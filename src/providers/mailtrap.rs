//! Mailtrap API provider.
//!
//! For reference: [Mailtrap API docs](https://api-docs.mailtrap.io/docs/mailtrap-api-docs/67f1d70aeb62c-send-email)
//!
//! # Example
//!
//! ```rust,ignore
//! use missive::providers::MailtrapMailer;
//!
//! let mailer = MailtrapMailer::new("your-api-key");
//! ```
//!
//! ## Sandbox Mode
//!
//! For [sandbox mode](https://api-docs.mailtrap.io/docs/mailtrap-api-docs/bcf61cdc1547e-send-email-early-access):
//!
//! ```rust,ignore
//! let mailer = MailtrapMailer::new("your-api-key")
//!     .sandbox_inbox_id("111111");
//! ```
//!
//! ## Provider Options
//!
//! Mailtrap-specific options can be set via `provider_option`:
//!
//! ```rust,ignore
//! let email = Email::new()
//!     .from("sender@example.com")
//!     .to("recipient@example.com")
//!     .subject("Hello")
//!     .provider_option("category", "welcome")
//!     .provider_option("custom_variables", json!({
//!         "my_var": {"my_message_id": 123},
//!         "my_other_var": {"my_other_id": 1}
//!     }));
//! ```
//!
//! ## Provider Options Reference
//!
//! * `category` (string) - Email category for filtering
//! * `custom_variables` (map) - Custom variables for tracking

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::email::Email;
use crate::error::MailError;
use crate::mailer::{DeliveryResult, Mailer};

const MAILTRAP_BASE_URL: &str = "https://send.api.mailtrap.io";
const MAILTRAP_SANDBOX_BASE_URL: &str = "https://sandbox.api.mailtrap.io";
const MAILTRAP_API_ENDPOINT: &str = "/api/send";

/// Mailtrap API email provider.
pub struct MailtrapMailer {
    api_key: String,
    base_url: Option<String>,
    sandbox_inbox_id: Option<String>,
    client: Client,
}

impl MailtrapMailer {
    /// Create a new Mailtrap mailer with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: None,
            sandbox_inbox_id: None,
            client: Client::new(),
        }
    }

    /// Create with a custom reqwest client.
    pub fn with_client(api_key: impl Into<String>, client: Client) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: None,
            sandbox_inbox_id: None,
            client,
        }
    }

    /// Set a custom base URL.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Enable sandbox mode with the given inbox ID.
    pub fn sandbox_inbox_id(mut self, inbox_id: impl Into<String>) -> Self {
        self.sandbox_inbox_id = Some(inbox_id.into());
        self
    }

    fn prepare_url(&self) -> String {
        if let Some(ref inbox_id) = self.sandbox_inbox_id {
            let base = self
                .base_url
                .as_deref()
                .unwrap_or(MAILTRAP_SANDBOX_BASE_URL);
            format!("{}{}/{}", base, MAILTRAP_API_ENDPOINT, inbox_id)
        } else {
            let base = self.base_url.as_deref().unwrap_or(MAILTRAP_BASE_URL);
            format!("{}{}", base, MAILTRAP_API_ENDPOINT)
        }
    }

    fn build_request(&self, email: &Email) -> Result<MailtrapRequest, MailError> {
        let from = email
            .from
            .as_ref()
            .ok_or(MailError::MissingField("from"))?;

        if email.to.is_empty() {
            return Err(MailError::MissingField("to"));
        }

        let mut request = MailtrapRequest {
            from: MailtrapEmailItem {
                email: from.email.clone(),
                name: from.name.clone(),
            },
            to: email
                .to
                .iter()
                .map(|a| MailtrapEmailItem {
                    email: a.email.clone(),
                    name: a.name.clone(),
                })
                .collect(),
            cc: if email.cc.is_empty() {
                None
            } else {
                Some(
                    email
                        .cc
                        .iter()
                        .map(|a| MailtrapEmailItem {
                            email: a.email.clone(),
                            name: a.name.clone(),
                        })
                        .collect(),
                )
            },
            bcc: if email.bcc.is_empty() {
                None
            } else {
                Some(
                    email
                        .bcc
                        .iter()
                        .map(|a| MailtrapEmailItem {
                            email: a.email.clone(),
                            name: a.name.clone(),
                        })
                        .collect(),
                )
            },
            subject: email.subject.clone(),
            text: email.text_body.clone(),
            html: email.html_body.clone(),
            attachments: None,
            headers: None,
            category: None,
            custom_variables: None,
        };

        // Add attachments
        if !email.attachments.is_empty() {
            request.attachments = Some(
                email
                    .attachments
                    .iter()
                    .map(|a| {
                        let mut attachment = MailtrapAttachment {
                            filename: a.filename.clone(),
                            content_type: a.content_type.clone(),
                            content: a.base64_data(),
                            disposition: if a.is_inline() {
                                "inline".to_string()
                            } else {
                                "attachment".to_string()
                            },
                            content_id: None,
                        };
                        if a.is_inline() {
                            attachment.content_id = Some(a.filename.clone());
                        }
                        attachment
                    })
                    .collect(),
            );
        }

        // Build headers (including Reply-To)
        let mut headers = email.headers.clone();
        if let Some(reply_to) = email.reply_to.first() {
            headers.insert("Reply-To".to_string(), reply_to.email.clone());
        }
        if !headers.is_empty() {
            request.headers = Some(headers);
        }

        // Provider options
        if let Some(category) = email.provider_options.get("category") {
            request.category = category.as_str().map(|s| s.to_string());
        }
        if let Some(custom_vars) = email.provider_options.get("custom_variables") {
            request.custom_variables = Some(custom_vars.clone());
        }

        Ok(request)
    }
}

#[async_trait]
impl Mailer for MailtrapMailer {
    async fn deliver(&self, email: &Email) -> Result<DeliveryResult, MailError> {
        let request = self.build_request(email)?;
        let url = self.prepare_url();

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("User-Agent", format!("missive/{}", crate::VERSION))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            let result: MailtrapResponse = response.json().await?;
            // Return the first message ID, or join them if multiple
            let message_id = result
                .message_ids
                .first()
                .cloned()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            Ok(DeliveryResult::with_response(
                message_id,
                serde_json::json!({
                    "provider": "mailtrap",
                    "message_ids": result.message_ids,
                }),
            ))
        } else {
            let error: MailtrapError = response.json().await.unwrap_or(MailtrapError {
                errors: vec!["Unknown error".to_string()],
            });
            Err(MailError::provider_with_status(
                "mailtrap",
                error.errors.join("; "),
                status.as_u16(),
            ))
        }
    }

    fn provider_name(&self) -> &'static str {
        "mailtrap"
    }
}

// ============================================================================
// Mailtrap API Types
// ============================================================================

#[derive(Debug, Serialize)]
struct MailtrapRequest {
    from: MailtrapEmailItem,
    to: Vec<MailtrapEmailItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cc: Option<Vec<MailtrapEmailItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bcc: Option<Vec<MailtrapEmailItem>>,
    subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachments: Option<Vec<MailtrapAttachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    custom_variables: Option<Value>,
}

#[derive(Debug, Serialize)]
struct MailtrapEmailItem {
    email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Debug, Serialize)]
struct MailtrapAttachment {
    filename: String,
    #[serde(rename = "type")]
    content_type: String,
    content: String, // Base64 encoded
    disposition: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MailtrapResponse {
    #[serde(default)]
    message_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MailtrapError {
    #[serde(default)]
    errors: Vec<String>,
}
