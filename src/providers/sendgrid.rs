//! SendGrid API provider.
//!
//! # Example
//!
//! ```rust,ignore
//! use missive::providers::SendGridMailer;
//!
//! let mailer = SendGridMailer::new("SG.xxxxx");
//! ```
//!
//! ## Provider Options
//!
//! SendGrid-specific options can be set via `provider_option`:
//!
//! ### Personalization Options
//!
//! ```rust,ignore
//! let email = Email::new()
//!     .from("sender@example.com")
//!     .to("recipient@example.com")
//!     .subject("Hello")
//!     .provider_option("dynamic_template_data", json!({"name": "John"}))
//!     .provider_option("custom_args", json!({"campaign_id": "123"}))
//!     .provider_option("substitutions", json!({"-name-": "John"}));
//! ```
//!
//! ### Body Options
//!
//! ```rust,ignore
//! let email = Email::new()
//!     .from("sender@example.com")
//!     .to("recipient@example.com")
//!     .subject("Hello")
//!     .provider_option("template_id", "d-xxxxx")
//!     .provider_option("categories", vec!["welcome", "user"])
//!     .provider_option("asm", json!({"group_id": 1, "groups_to_display": [1, 2, 3]}))
//!     .provider_option("mail_settings", json!({"sandbox_mode": {"enable": true}}))
//!     .provider_option("tracking_settings", json!({"click_tracking": {"enable": true}}))
//!     .provider_option("send_at", 1617260400)
//!     .provider_option("batch_id", "batch-123")
//!     .provider_option("ip_pool_name", "my-pool");
//! ```
//!
//! ### Custom Personalizations
//!
//! For advanced use cases, you can override the entire personalizations array:
//!
//! ```rust,ignore
//! let email = Email::new()
//!     .from("sender@example.com")
//!     .subject("Hello")
//!     .provider_option("personalizations", json!([
//!         {"to": [{"email": "user1@example.com"}], "subject": "Custom 1"},
//!         {"to": [{"email": "user2@example.com"}], "subject": "Custom 2"}
//!     ]));
//! ```

use async_trait::async_trait;
use flate2::write::GzEncoder;
use flate2::Compression;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Write;

use crate::email::Email;
use crate::error::MailError;
use crate::mailer::{DeliveryResult, Mailer};

const SENDGRID_API_URL: &str = "https://api.sendgrid.com/v3";

/// SendGrid API email provider.
pub struct SendGridMailer {
    api_key: String,
    client: Client,
    base_url: String,
    compress: bool,
}

impl SendGridMailer {
    /// Create a new SendGrid mailer with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            client: Client::new(),
            base_url: SENDGRID_API_URL.to_string(),
            compress: false,
        }
    }

    /// Create with a custom reqwest client.
    pub fn with_client(api_key: impl Into<String>, client: Client) -> Self {
        Self {
            api_key: api_key.into(),
            client,
            base_url: SENDGRID_API_URL.to_string(),
            compress: false,
        }
    }

    /// Set a custom base URL (for testing).
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Enable gzip compression for requests.
    pub fn compress(mut self, enabled: bool) -> Self {
        self.compress = enabled;
        self
    }

    fn build_request(&self, email: &Email) -> Result<SendGridRequest, MailError> {
        let from = email.from.as_ref().ok_or(MailError::MissingField("from"))?;

        // Check if custom personalizations are provided
        let personalizations = if let Some(custom) = email.provider_options.get("personalizations")
        {
            serde_json::from_value(custom.clone()).map_err(|e| {
                MailError::provider("sendgrid", format!("Invalid personalizations: {}", e))
            })?
        } else {
            if email.to.is_empty() {
                return Err(MailError::MissingField("to"));
            }
            vec![self.build_personalization(email)]
        };

        // Build content
        let mut content = Vec::new();
        if let Some(ref text) = email.text_body {
            content.push(SendGridContent {
                content_type: "text/plain".to_string(),
                value: text.clone(),
            });
        }
        if let Some(ref html) = email.html_body {
            content.push(SendGridContent {
                content_type: "text/html".to_string(),
                value: html.clone(),
            });
        }

        // Build reply_to or reply_to_list
        let (reply_to, reply_to_list) = if email.reply_to.len() > 1 {
            (
                None,
                Some(
                    email
                        .reply_to
                        .iter()
                        .map(|a| SendGridAddress {
                            email: a.email.clone(),
                            name: a.name.clone(),
                        })
                        .collect(),
                ),
            )
        } else {
            (
                email.reply_to.first().map(|a| SendGridAddress {
                    email: a.email.clone(),
                    name: a.name.clone(),
                }),
                None,
            )
        };

        let mut request = SendGridRequest {
            personalizations,
            from: SendGridAddress {
                email: from.email.clone(),
                name: from.name.clone(),
            },
            reply_to,
            reply_to_list,
            subject: email.subject.clone(),
            content: if content.is_empty() {
                None
            } else {
                Some(content)
            },
            attachments: None,
            headers: None,
            template_id: None,
            categories: None,
            asm: None,
            mail_settings: None,
            tracking_settings: None,
            send_at: None,
            batch_id: None,
            ip_pool_name: None,
        };

        // Add attachments
        if !email.attachments.is_empty() {
            request.attachments = Some(
                email
                    .attachments
                    .iter()
                    .map(|a| {
                        let (disposition, content_id) = match a.disposition {
                            crate::attachment::AttachmentType::Inline => {
                                // For inline attachments, use content_id if provided, else filename
                                let cid =
                                    a.content_id.clone().unwrap_or_else(|| a.filename.clone());
                                ("inline".to_string(), Some(cid))
                            }
                            crate::attachment::AttachmentType::Attachment => {
                                ("attachment".to_string(), None)
                            }
                        };
                        SendGridAttachment {
                            content: a.base64_data(),
                            filename: a.filename.clone(),
                            content_type: Some(a.content_type.clone()),
                            disposition: Some(disposition),
                            content_id,
                        }
                    })
                    .collect(),
            );
        }

        // Custom headers
        if !email.headers.is_empty() {
            request.headers = Some(email.headers.clone());
        }

        // Provider-specific body options
        if let Some(template_id) = email.provider_options.get("template_id") {
            request.template_id = template_id.as_str().map(|s| s.to_string());
        }
        if let Some(categories) = email.provider_options.get("categories") {
            request.categories = serde_json::from_value(categories.clone()).ok();
        }
        if let Some(asm) = email.provider_options.get("asm") {
            request.asm = Some(asm.clone());
        }
        if let Some(mail_settings) = email.provider_options.get("mail_settings") {
            request.mail_settings = Some(mail_settings.clone());
        }
        if let Some(tracking_settings) = email.provider_options.get("tracking_settings") {
            request.tracking_settings = Some(tracking_settings.clone());
        }
        if let Some(send_at) = email.provider_options.get("send_at") {
            request.send_at = send_at.as_i64();
        }
        if let Some(batch_id) = email.provider_options.get("batch_id") {
            request.batch_id = batch_id.as_str().map(|s| s.to_string());
        }
        if let Some(ip_pool_name) = email.provider_options.get("ip_pool_name") {
            request.ip_pool_name = ip_pool_name.as_str().map(|s| s.to_string());
        }

        Ok(request)
    }

    fn build_personalization(&self, email: &Email) -> SendGridPersonalization {
        let mut personalization = SendGridPersonalization {
            to: email
                .to
                .iter()
                .map(|a| SendGridAddress {
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
                        .map(|a| SendGridAddress {
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
                        .map(|a| SendGridAddress {
                            email: a.email.clone(),
                            name: a.name.clone(),
                        })
                        .collect(),
                )
            },
            dynamic_template_data: None,
            custom_args: None,
            substitutions: None,
        };

        // Personalization-level provider options
        if let Some(data) = email.provider_options.get("dynamic_template_data") {
            personalization.dynamic_template_data = Some(data.clone());
        }
        if let Some(args) = email.provider_options.get("custom_args") {
            personalization.custom_args = Some(args.clone());
        }
        if let Some(subs) = email.provider_options.get("substitutions") {
            personalization.substitutions = Some(subs.clone());
        }

        personalization
    }

    fn compress_body(&self, body: &[u8]) -> Result<Vec<u8>, MailError> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(body).map_err(|e| {
            MailError::provider("sendgrid", format!("Failed to compress body: {}", e))
        })?;
        encoder.finish().map_err(|e| {
            MailError::provider("sendgrid", format!("Failed to finish compression: {}", e))
        })
    }
}

#[async_trait]
impl Mailer for SendGridMailer {
    async fn deliver(&self, email: &Email) -> Result<DeliveryResult, MailError> {
        let request = self.build_request(email)?;

        let url = format!("{}/mail/send", self.base_url);
        let json_body = serde_json::to_vec(&request)?;

        let mut req = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("User-Agent", format!("missive/{}", crate::VERSION));

        let body = if self.compress {
            req = req.header("Content-Encoding", "gzip");
            self.compress_body(&json_body)?
        } else {
            json_body
        };

        let response = req.body(body).send().await?;

        let status = response.status();

        // SendGrid returns 202 Accepted on success with no body
        if status.is_success() {
            // Extract message ID from X-Message-Id header if present
            let message_id = response
                .headers()
                .get("X-Message-Id")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            Ok(DeliveryResult::with_response(
                message_id,
                serde_json::json!({ "provider": "sendgrid" }),
            ))
        } else {
            let error: SendGridError = response.json().await.unwrap_or(SendGridError {
                errors: vec![SendGridErrorDetail {
                    message: "Unknown error".to_string(),
                    field: None,
                    help: None,
                }],
            });

            let error_msg = error
                .errors
                .iter()
                .map(|e| e.message.clone())
                .collect::<Vec<_>>()
                .join("; ");

            Err(MailError::provider_with_status(
                "sendgrid",
                error_msg,
                status.as_u16(),
            ))
        }
    }

    fn provider_name(&self) -> &'static str {
        "sendgrid"
    }
}

// ============================================================================
// SendGrid API Types
// ============================================================================

#[derive(Debug, Serialize)]
struct SendGridRequest {
    personalizations: Vec<SendGridPersonalization>,
    from: SendGridAddress,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<SendGridAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to_list: Option<Vec<SendGridAddress>>,
    subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<Vec<SendGridContent>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachments: Option<Vec<SendGridAttachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    template_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    categories: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    asm: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mail_settings: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tracking_settings: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    send_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    batch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ip_pool_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SendGridPersonalization {
    to: Vec<SendGridAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cc: Option<Vec<SendGridAddress>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bcc: Option<Vec<SendGridAddress>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dynamic_template_data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    custom_args: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    substitutions: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SendGridAddress {
    email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Debug, Serialize)]
struct SendGridContent {
    #[serde(rename = "type")]
    content_type: String,
    value: String,
}

#[derive(Debug, Serialize)]
struct SendGridAttachment {
    content: String, // Base64 encoded
    filename: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disposition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SendGridError {
    errors: Vec<SendGridErrorDetail>,
}

#[derive(Debug, Deserialize)]
struct SendGridErrorDetail {
    message: String,
    #[allow(dead_code)]
    field: Option<String>,
    #[allow(dead_code)]
    help: Option<String>,
}
