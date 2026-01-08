//! Postmark API provider.
//!
//! # Example
//!
//! ```rust,ignore
//! use missive::providers::PostmarkMailer;
//!
//! let mailer = PostmarkMailer::new("xxxxx-xxxx-xxxx-xxxx-xxxxxx");
//! ```
//!
//! ## Provider Options
//!
//! Postmark-specific options can be set via `provider_option`:
//!
//! ```rust,ignore
//! let email = Email::new()
//!     .from("sender@example.com")
//!     .to("recipient@example.com")
//!     .subject("Hello")
//!     .provider_option("tag", "welcome")
//!     .provider_option("track_opens", true)
//!     .provider_option("track_links", "HtmlAndText")
//!     .provider_option("message_stream", "outbound")
//!     .provider_option("metadata", json!({"user_id": "123"}))
//!     .provider_option("inline_css", true);
//! ```
//!
//! ## Template Support
//!
//! Send emails using Postmark templates:
//!
//! ```rust,ignore
//! // Using template ID
//! let email = Email::new()
//!     .from("sender@example.com")
//!     .to("recipient@example.com")
//!     .provider_option("template_id", 12345)
//!     .provider_option("template_model", json!({
//!         "name": "John",
//!         "product": "Awesome App"
//!     }));
//!
//! // Using template alias
//! let email = Email::new()
//!     .from("sender@example.com")
//!     .to("recipient@example.com")
//!     .provider_option("template_alias", "welcome-email")
//!     .provider_option("template_model", json!({
//!         "name": "John"
//!     }));
//! ```
//!
//! ## Batch Sending
//!
//! Use `deliver_many` for batch sending (up to 500 emails per batch):
//!
//! ```rust,ignore
//! let emails = vec![email1, email2, email3];
//! let results = mailer.deliver_many(&emails).await?;
//! ```

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::email::Email;
use crate::error::MailError;
use crate::mailer::{DeliveryResult, Mailer};

const POSTMARK_API_URL: &str = "https://api.postmarkapp.com";

/// Postmark API email provider.
pub struct PostmarkMailer {
    api_token: String,
    client: Client,
    base_url: String,
}

impl PostmarkMailer {
    /// Create a new Postmark mailer with the given server token.
    pub fn new(api_token: impl Into<String>) -> Self {
        Self {
            api_token: api_token.into(),
            client: Client::new(),
            base_url: POSTMARK_API_URL.to_string(),
        }
    }

    /// Create with a custom reqwest client.
    pub fn with_client(api_token: impl Into<String>, client: Client) -> Self {
        Self {
            api_token: api_token.into(),
            client,
            base_url: POSTMARK_API_URL.to_string(),
        }
    }

    /// Set a custom base URL (for testing).
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Check if this email uses a template.
    fn is_template_email(email: &Email) -> bool {
        email.provider_options.contains_key("template_id")
            || email.provider_options.contains_key("template_alias")
    }

    fn build_request(&self, email: &Email) -> Result<PostmarkRequest, MailError> {
        let from = email.from.as_ref().ok_or(MailError::MissingField("from"))?;

        if email.to.is_empty() {
            return Err(MailError::MissingField("to"));
        }

        let mut request = PostmarkRequest {
            from: from.formatted(),
            to: email
                .to
                .iter()
                .map(|a| a.formatted())
                .collect::<Vec<_>>()
                .join(", "),
            subject: if email.subject.is_empty() {
                None
            } else {
                Some(email.subject.clone())
            },
            html_body: email.html_body.clone(),
            text_body: email.text_body.clone(),
            cc: if email.cc.is_empty() {
                None
            } else {
                Some(
                    email
                        .cc
                        .iter()
                        .map(|a| a.formatted())
                        .collect::<Vec<_>>()
                        .join(", "),
                )
            },
            bcc: if email.bcc.is_empty() {
                None
            } else {
                Some(
                    email
                        .bcc
                        .iter()
                        .map(|a| a.formatted())
                        .collect::<Vec<_>>()
                        .join(", "),
                )
            },
            reply_to: email.reply_to.first().map(|a| a.formatted()),
            tag: None,
            track_opens: None,
            track_links: None,
            message_stream: None,
            metadata: None,
            inline_css: None,
            template_id: None,
            template_alias: None,
            template_model: None,
            headers: None,
            attachments: None,
        };

        // Add attachments
        if !email.attachments.is_empty() {
            request.attachments = Some(
                email
                    .attachments
                    .iter()
                    .map(|a| {
                        let content_id = if a.is_inline() {
                            // Postmark requires "cid:" prefix for inline attachments
                            a.content_id.as_ref().map(|cid| format!("cid:{}", cid))
                        } else {
                            None
                        };
                        PostmarkAttachment {
                            name: a.filename.clone(),
                            content: a.base64_data(),
                            content_type: a.content_type.clone(),
                            content_id,
                        }
                    })
                    .collect(),
            );
        }

        // Custom headers
        if !email.headers.is_empty() {
            request.headers = Some(
                email
                    .headers
                    .iter()
                    .map(|(name, value)| PostmarkHeader {
                        name: name.clone(),
                        value: value.clone(),
                    })
                    .collect(),
            );
        }

        // Provider-specific options
        if let Some(tag) = email.provider_options.get("tag") {
            request.tag = tag.as_str().map(|s| s.to_string());
        }
        if let Some(track_opens) = email.provider_options.get("track_opens") {
            request.track_opens = track_opens.as_bool();
        }
        if let Some(track_links) = email.provider_options.get("track_links") {
            request.track_links = track_links.as_str().map(|s| s.to_string());
        }
        if let Some(message_stream) = email.provider_options.get("message_stream") {
            request.message_stream = message_stream.as_str().map(|s| s.to_string());
        }
        if let Some(metadata) = email.provider_options.get("metadata") {
            request.metadata = Some(metadata.clone());
        }
        if let Some(inline_css) = email.provider_options.get("inline_css") {
            request.inline_css = inline_css.as_bool();
        }

        // Template options
        if let Some(template_id) = email.provider_options.get("template_id") {
            request.template_id = template_id.as_i64();
        }
        if let Some(template_alias) = email.provider_options.get("template_alias") {
            request.template_alias = template_alias.as_str().map(|s| s.to_string());
        }
        if let Some(template_model) = email.provider_options.get("template_model") {
            request.template_model = Some(template_model.clone());
        }

        Ok(request)
    }

    async fn send_request(
        &self,
        url: &str,
        body: &impl Serialize,
    ) -> Result<reqwest::Response, MailError> {
        Ok(self
            .client
            .post(url)
            .header("X-Postmark-Server-Token", &self.api_token)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("User-Agent", format!("missive/{}", crate::VERSION))
            .json(body)
            .send()
            .await?)
    }

    fn parse_response(_status: reqwest::StatusCode, result: PostmarkResponse) -> DeliveryResult {
        DeliveryResult::with_response(
            result.message_id,
            serde_json::json!({
                "provider": "postmark",
                "submitted_at": result.submitted_at,
            }),
        )
    }

    fn parse_error(status: reqwest::StatusCode, error: PostmarkError) -> MailError {
        MailError::provider_with_status(
            "postmark",
            format!("[{}] {}", error.error_code, error.message),
            status.as_u16(),
        )
    }
}

#[async_trait]
impl Mailer for PostmarkMailer {
    async fn deliver(&self, email: &Email) -> Result<DeliveryResult, MailError> {
        let request = self.build_request(email)?;

        // Use template endpoint if template_id or template_alias is set
        let url = if Self::is_template_email(email) {
            format!("{}/email/withTemplate", self.base_url)
        } else {
            format!("{}/email", self.base_url)
        };

        let response = self.send_request(&url, &request).await?;
        let status = response.status();

        if status.is_success() {
            let result: PostmarkResponse = response.json().await?;
            Ok(Self::parse_response(status, result))
        } else {
            let error: PostmarkError = response.json().await.unwrap_or(PostmarkError {
                error_code: 0,
                message: "Unknown error".to_string(),
            });
            Err(Self::parse_error(status, error))
        }
    }

    async fn deliver_many(&self, emails: &[Email]) -> Result<Vec<DeliveryResult>, MailError> {
        if emails.is_empty() {
            return Ok(vec![]);
        }

        // Check if any emails use templates
        let has_templates = emails.iter().any(Self::is_template_email);

        // Build requests
        let requests: Vec<PostmarkRequest> = emails
            .iter()
            .map(|email| self.build_request(email))
            .collect::<Result<Vec<_>, _>>()?;

        // Use appropriate batch endpoint
        let url = if has_templates {
            format!("{}/email/batchWithTemplates", self.base_url)
        } else {
            format!("{}/email/batch", self.base_url)
        };

        // For template batch, wrap in Messages object
        let response = if has_templates {
            let batch = PostmarkTemplateBatchRequest { messages: requests };
            self.send_request(&url, &batch).await?
        } else {
            self.send_request(&url, &requests).await?
        };

        let status = response.status();

        if status.is_success() {
            let results: Vec<PostmarkBatchResponse> = response.json().await?;
            Ok(results
                .into_iter()
                .map(|r| {
                    DeliveryResult::with_response(
                        r.message_id,
                        serde_json::json!({
                            "provider": "postmark",
                            "error_code": r.error_code,
                            "message": r.message,
                            "to": r.to,
                            "submitted_at": r.submitted_at,
                        }),
                    )
                })
                .collect())
        } else {
            let error: PostmarkError = response.json().await.unwrap_or(PostmarkError {
                error_code: 0,
                message: "Unknown error".to_string(),
            });
            Err(Self::parse_error(status, error))
        }
    }

    fn provider_name(&self) -> &'static str {
        "postmark"
    }
}

// ============================================================================
// Postmark API Types
// ============================================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct PostmarkRequest {
    from: String,
    to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    html_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bcc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    track_opens: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    track_links: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message_stream: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inline_css: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    template_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    template_alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    template_model: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<Vec<PostmarkHeader>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachments: Option<Vec<PostmarkAttachment>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct PostmarkHeader {
    name: String,
    value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct PostmarkAttachment {
    name: String,
    content: String, // Base64 encoded
    content_type: String,
    #[serde(rename = "ContentID", skip_serializing_if = "Option::is_none")]
    content_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct PostmarkTemplateBatchRequest {
    messages: Vec<PostmarkRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PostmarkResponse {
    #[serde(rename = "MessageID")]
    message_id: String,
    submitted_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PostmarkBatchResponse {
    #[serde(rename = "MessageID")]
    message_id: String,
    error_code: i32,
    message: String,
    #[serde(default)]
    to: String,
    #[serde(default)]
    submitted_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PostmarkError {
    error_code: i32,
    message: String,
}
