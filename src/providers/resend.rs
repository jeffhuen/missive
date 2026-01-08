//! Resend API provider.
//!
//! # Example
//!
//! ```rust,ignore
//! use missive::providers::ResendMailer;
//!
//! let mailer = ResendMailer::new("re_xxxxx");
//! ```
//!
//! ## Provider Options
//!
//! Resend-specific options can be set via `provider_option`:
//!
//! ```rust,ignore
//! let email = Email::new()
//!     .from("sender@example.com")
//!     .to("recipient@example.com")
//!     .subject("Hello")
//!     .provider_option("tags", vec![json!({"name": "category", "value": "welcome"})])
//!     .provider_option("scheduled_at", "2024-01-01T00:00:00Z")
//!     .provider_option("idempotency_key", "unique-key-123");
//! ```
//!
//! ## Template Support
//!
//! Send emails using Resend templates:
//!
//! ```rust,ignore
//! // Template without variables
//! let email = Email::new()
//!     .from("sender@example.com")
//!     .to("recipient@example.com")
//!     .provider_option("template", json!({"id": "welcome-template"}));
//!
//! // Template with variables
//! let email = Email::new()
//!     .from("sender@example.com")
//!     .to("recipient@example.com")
//!     .provider_option("template", json!({
//!         "id": "welcome-template",
//!         "variables": {
//!             "name": "John",
//!             "action_url": "https://example.com/activate"
//!         }
//!     }));
//! ```

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::email::Email;
use crate::error::MailError;
use crate::mailer::{DeliveryResult, Mailer};

const RESEND_API_URL: &str = "https://api.resend.com";

/// Resend API email provider.
pub struct ResendMailer {
    api_key: String,
    client: Client,
    base_url: String,
}

impl ResendMailer {
    /// Create a new Resend mailer with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            client: Client::new(),
            base_url: RESEND_API_URL.to_string(),
        }
    }

    /// Create with a custom reqwest client.
    pub fn with_client(api_key: impl Into<String>, client: Client) -> Self {
        Self {
            api_key: api_key.into(),
            client,
            base_url: RESEND_API_URL.to_string(),
        }
    }

    /// Set a custom base URL (for testing).
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    fn build_request(&self, email: &Email) -> Result<ResendRequest, MailError> {
        let from = email.from.as_ref().ok_or(MailError::MissingField("from"))?;

        if email.to.is_empty() {
            return Err(MailError::MissingField("to"));
        }

        let mut request = ResendRequest {
            from: from.formatted(),
            to: email.to.iter().map(|a| a.formatted()).collect(),
            subject: if email.subject.is_empty() {
                None
            } else {
                Some(email.subject.clone())
            },
            html: email.html_body.clone(),
            text: email.text_body.clone(),
            cc: if email.cc.is_empty() {
                None
            } else {
                Some(email.cc.iter().map(|a| a.formatted()).collect())
            },
            bcc: if email.bcc.is_empty() {
                None
            } else {
                Some(email.bcc.iter().map(|a| a.formatted()).collect())
            },
            reply_to: email.reply_to.first().map(|a| a.formatted()),
            headers: if email.headers.is_empty() {
                None
            } else {
                Some(
                    email
                        .headers
                        .iter()
                        .map(|(k, v)| ResendHeader {
                            name: k.clone(),
                            value: v.clone(),
                        })
                        .collect(),
                )
            },
            attachments: None,
            tags: None,
            scheduled_at: None,
            template: None,
        };

        // Add attachments
        if !email.attachments.is_empty() {
            let attachments: Vec<ResendAttachment> = email
                .attachments
                .iter()
                .map(|a| {
                    // Only include content_id for inline attachments
                    let content_id = if a.is_inline() {
                        a.content_id.clone()
                    } else {
                        None
                    };
                    ResendAttachment {
                        filename: a.filename.clone(),
                        content: a.base64_data(),
                        content_type: Some(a.content_type.clone()),
                        content_id,
                    }
                })
                .collect();
            request.attachments = Some(attachments);
        }

        // Add provider-specific options
        if let Some(tags) = email.provider_options.get("tags") {
            request.tags = serde_json::from_value(tags.clone()).ok();
        }
        if let Some(scheduled_at) = email.provider_options.get("scheduled_at") {
            request.scheduled_at = scheduled_at.as_str().map(|s| s.to_string());
        }
        if let Some(template) = email.provider_options.get("template") {
            request.template = Some(template.clone());
        }

        Ok(request)
    }
}

#[async_trait]
impl Mailer for ResendMailer {
    async fn deliver(&self, email: &Email) -> Result<DeliveryResult, MailError> {
        let request = self.build_request(email)?;

        let url = format!("{}/emails", self.base_url);
        let mut req = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("User-Agent", format!("missive/{}", crate::VERSION));

        // Add idempotency key header if provided
        if let Some(idempotency_key) = email.provider_options.get("idempotency_key") {
            if let Some(key) = idempotency_key.as_str() {
                req = req.header("Idempotency-Key", key);
            }
        }

        let response = req.json(&request).send().await?;

        let status = response.status();

        if status.is_success() {
            let result: ResendResponse = response.json().await?;
            Ok(DeliveryResult::with_response(
                result.id,
                serde_json::json!({ "provider": "resend" }),
            ))
        } else {
            let error: ResendError = response.json().await.unwrap_or(ResendError {
                message: "Unknown error".to_string(),
                name: None,
            });
            Err(MailError::provider_with_status(
                "resend",
                error.message,
                status.as_u16(),
            ))
        }
    }

    /// Validate emails for Resend batch API limitations.
    ///
    /// Resend's batch API does not support:
    /// - `scheduled_at` option
    /// - Attachments
    fn validate_batch(&self, emails: &[Email]) -> Result<(), MailError> {
        for (i, email) in emails.iter().enumerate() {
            if email.provider_options.contains_key("scheduled_at") {
                return Err(MailError::UnsupportedFeature(format!(
                    "scheduled_at is not supported in batch sends (email {})",
                    i + 1
                )));
            }
            if !email.attachments.is_empty() {
                return Err(MailError::UnsupportedFeature(format!(
                    "attachments are not supported in Resend batch sends (email {})",
                    i + 1
                )));
            }
        }
        Ok(())
    }

    async fn deliver_many(&self, emails: &[Email]) -> Result<Vec<DeliveryResult>, MailError> {
        if emails.is_empty() {
            return Ok(vec![]);
        }

        // Validate batch restrictions
        self.validate_batch(emails)?;

        // Build requests
        let requests: Vec<ResendRequest> = emails
            .iter()
            .map(|email| self.build_request(email))
            .collect::<Result<Vec<_>, _>>()?;

        let url = format!("{}/emails/batch", self.base_url);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("User-Agent", format!("missive/{}", crate::VERSION))
            .json(&requests)
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            let result: ResendBatchResponse = response.json().await?;
            Ok(result
                .data
                .into_iter()
                .map(|r| {
                    DeliveryResult::with_response(r.id, serde_json::json!({ "provider": "resend" }))
                })
                .collect())
        } else {
            let error: ResendError = response.json().await.unwrap_or(ResendError {
                message: "Unknown error".to_string(),
                name: None,
            });
            Err(MailError::provider_with_status(
                "resend",
                error.message,
                status.as_u16(),
            ))
        }
    }

    fn provider_name(&self) -> &'static str {
        "resend"
    }
}

// ============================================================================
// Resend API Types
// ============================================================================

#[derive(Debug, Serialize)]
struct ResendRequest {
    from: String,
    to: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cc: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bcc: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<Vec<ResendHeader>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachments: Option<Vec<ResendAttachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<ResendTag>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scheduled_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    template: Option<Value>,
}

#[derive(Debug, Serialize)]
struct ResendHeader {
    name: String,
    value: String,
}

#[derive(Debug, Serialize)]
struct ResendAttachment {
    filename: String,
    content: String, // Base64 encoded
    #[serde(skip_serializing_if = "Option::is_none")]
    content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResendTag {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct ResendResponse {
    id: String,
}

#[derive(Debug, Deserialize)]
struct ResendBatchResponse {
    data: Vec<ResendResponse>,
}

#[derive(Debug, Deserialize)]
struct ResendError {
    message: String,
    #[serde(default)]
    #[allow(dead_code)]
    name: Option<String>,
}
