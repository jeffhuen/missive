//! Mailgun API provider.
//!
//! For reference: [Mailgun API docs](https://documentation.mailgun.com/en/latest/api-sending.html#sending)
//!
//! # Example
//!
//! ```rust,ignore
//! use missive::providers::MailgunMailer;
//!
//! let mailer = MailgunMailer::new("your-api-key", "mg.yourdomain.com");
//! ```
//!
//! ## Configuration
//!
//! * `api_key` - Your Mailgun API key
//! * `domain` - Your sending domain (e.g., "mg.yourdomain.com" or sandbox domain)
//!
//! For EU domains, use `.base_url("https://api.eu.mailgun.net/v3")`.
//!
//! ## Provider Options
//!
//! Mailgun-specific options can be set via `provider_option`:
//!
//! ```rust,ignore
//! let email = Email::new()
//!     .from("sender@example.com")
//!     .to("recipient@example.com")
//!     .subject("Hello")
//!     .provider_option("custom_vars", json!({"user_id": "123"}))
//!     .provider_option("recipient_vars", json!({
//!         "bob@example.com": {"name": "Bob"},
//!         "alice@example.com": {"name": "Alice"}
//!     }))
//!     .provider_option("sending_options", json!({"tracking": "yes", "dkim": "yes"}))
//!     .provider_option("tags", vec!["welcome", "onboarding"])
//!     .provider_option("template_name", "welcome-template")
//!     .provider_option("template_options", json!({"version": "v2", "text": "yes"}));
//! ```
//!
//! ## Provider Options Reference
//!
//! * `custom_vars` (map) - Custom variables sent as `h:X-Mailgun-Variables` header
//! * `recipient_vars` (map) - Per-recipient variables for batch sending
//! * `sending_options` (map) - Mailgun options like `tracking`, `dkim`, `testmode`
//! * `tags` (list[string]) - Tags for analytics (max 3)
//! * `template_name` (string) - Name of stored Mailgun template
//! * `template_options` (map) - Template options like `version`, `text`

use async_trait::async_trait;
use base64::Engine;
use reqwest::{
    multipart::{Form, Part},
    Client,
};
use serde::Deserialize;
use serde_json::Value;

use crate::email::Email;
use crate::error::MailError;
use crate::mailer::{DeliveryResult, Mailer};

const MAILGUN_BASE_URL: &str = "https://api.mailgun.net/v3";

/// Mailgun API email provider.
pub struct MailgunMailer {
    api_key: String,
    domain: String,
    base_url: String,
    client: Client,
}

impl MailgunMailer {
    /// Create a new Mailgun mailer with the given API key and domain.
    pub fn new(api_key: impl Into<String>, domain: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            domain: domain.into(),
            base_url: MAILGUN_BASE_URL.to_string(),
            client: Client::new(),
        }
    }

    /// Create with a custom reqwest client.
    pub fn with_client(
        api_key: impl Into<String>,
        domain: impl Into<String>,
        client: Client,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            domain: domain.into(),
            base_url: MAILGUN_BASE_URL.to_string(),
            client,
        }
    }

    /// Set a custom base URL (e.g., for EU: "https://api.eu.mailgun.net/v3").
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    fn auth_header(&self) -> String {
        let credentials = format!("api:{}", self.api_key);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
        format!("Basic {}", encoded)
    }

    fn build_form(&self, email: &Email) -> Result<Form, MailError> {
        let from = email
            .from
            .as_ref()
            .ok_or(MailError::MissingField("from"))?;

        if email.to.is_empty() {
            return Err(MailError::MissingField("to"));
        }

        let mut form = Form::new();

        // Required fields
        form = form.text("from", from.formatted());
        form = form.text(
            "to",
            email
                .to
                .iter()
                .map(|a| a.formatted())
                .collect::<Vec<_>>()
                .join(", "),
        );
        form = form.text("subject", email.subject.clone());

        // Optional body content
        if let Some(ref text) = email.text_body {
            form = form.text("text", text.clone());
        }
        if let Some(ref html) = email.html_body {
            form = form.text("html", html.clone());
        }

        // CC/BCC
        if !email.cc.is_empty() {
            form = form.text(
                "cc",
                email
                    .cc
                    .iter()
                    .map(|a| a.formatted())
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }
        if !email.bcc.is_empty() {
            form = form.text(
                "bcc",
                email
                    .bcc
                    .iter()
                    .map(|a| a.formatted())
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }

        // Reply-To (Mailgun uses h:Reply-To header)
        if let Some(reply_to) = email.reply_to.first() {
            form = form.text("h:Reply-To", reply_to.email.clone());
        }

        // Custom headers
        for (name, value) in &email.headers {
            form = form.text(format!("h:{}", name), value.clone());
        }

        // Provider options: custom_vars -> h:X-Mailgun-Variables
        if let Some(custom_vars) = email.provider_options.get("custom_vars") {
            if let Ok(json_str) = serde_json::to_string(custom_vars) {
                form = form.text("h:X-Mailgun-Variables", json_str);
            }
        }

        // Provider options: recipient_vars -> recipient-variables
        if let Some(recipient_vars) = email.provider_options.get("recipient_vars") {
            if let Ok(json_str) = serde_json::to_string(recipient_vars) {
                form = form.text("recipient-variables", json_str);
            }
        }

        // Provider options: sending_options -> o:key
        if let Some(sending_options) = email.provider_options.get("sending_options") {
            if let Some(obj) = sending_options.as_object() {
                for (key, value) in obj {
                    let value_str = encode_variable(value);
                    form = form.text(format!("o:{}", key), value_str);
                }
            }
        }

        // Provider options: tags -> o:tag (can have multiple)
        if let Some(tags) = email.provider_options.get("tags") {
            if let Some(arr) = tags.as_array() {
                for tag in arr {
                    if let Some(tag_str) = tag.as_str() {
                        form = form.text("o:tag", tag_str.to_string());
                    }
                }
            }
        }

        // Provider options: template_name -> template
        if let Some(template_name) = email.provider_options.get("template_name") {
            if let Some(name) = template_name.as_str() {
                form = form.text("template", name.to_string());
            }
        }

        // Provider options: template_options -> t:key
        if let Some(template_options) = email.provider_options.get("template_options") {
            if let Some(obj) = template_options.as_object() {
                for (key, value) in obj {
                    let value_str = encode_variable(value);
                    form = form.text(format!("t:{}", key), value_str);
                }
            }
        }

        // Attachments
        for attachment in &email.attachments {
            let data = attachment.get_data().map_err(|e| {
                MailError::AttachmentError(format!("{}: {}", attachment.filename, e))
            })?;

            let field_name = match attachment.disposition {
                crate::attachment::AttachmentType::Inline => "inline",
                crate::attachment::AttachmentType::Attachment => "attachment",
            };

            let part = Part::bytes(data)
                .file_name(attachment.filename.clone())
                .mime_str(&attachment.content_type)
                .map_err(|e| MailError::AttachmentError(e.to_string()))?;

            form = form.part(field_name, part);
        }

        Ok(form)
    }
}

fn encode_variable(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Bool(b) => if *b { "yes" } else { "no" }.to_string(),
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}

#[async_trait]
impl Mailer for MailgunMailer {
    async fn deliver(&self, email: &Email) -> Result<DeliveryResult, MailError> {
        let form = self.build_form(email)?;
        let url = format!("{}/{}/messages", self.base_url, self.domain);

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("User-Agent", format!("missive/{}", crate::VERSION))
            .multipart(form)
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            let result: MailgunResponse = response.json().await?;
            Ok(DeliveryResult::with_response(
                result.id,
                serde_json::json!({
                    "provider": "mailgun",
                    "message": result.message,
                }),
            ))
        } else {
            let error_body = response.text().await.unwrap_or_default();
            let error_msg = serde_json::from_str::<MailgunError>(&error_body)
                .map(|e| e.message)
                .unwrap_or(error_body);

            Err(MailError::provider_with_status(
                "mailgun",
                error_msg,
                status.as_u16(),
            ))
        }
    }

    fn provider_name(&self) -> &'static str {
        "mailgun"
    }
}

// ============================================================================
// Mailgun API Types
// ============================================================================

#[derive(Debug, Deserialize)]
struct MailgunResponse {
    id: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct MailgunError {
    message: String,
}
