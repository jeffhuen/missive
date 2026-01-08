//! Mailjet API provider.
//!
//! For reference: [Mailjet API docs](https://dev.mailjet.com/guides/#send-api-v3-1)
//!
//! # Example
//!
//! ```rust,ignore
//! use missive::providers::MailjetMailer;
//!
//! let mailer = MailjetMailer::new("api_key", "secret_key");
//! ```
//!
//! ## Provider Options
//!
//! Mailjet-specific options can be set via `provider_option`:
//!
//! ```rust,ignore
//! let email = Email::new()
//!     .from("sender@example.com")
//!     .to("recipient@example.com")
//!     .subject("Hello")
//!     .provider_option("template_id", 123)
//!     .provider_option("template_error_deliver", true)
//!     .provider_option("template_error_reporting", "developer@example.com")
//!     .provider_option("variables", json!({"firstname": "John", "lastname": "Doe"}))
//!     .provider_option("custom_id", "my-custom-id")
//!     .provider_option("event_payload", "custom-payload-string");
//! ```
//!
//! ## Provider Options Reference
//!
//! * `template_id` (integer) - ID of the template to use
//! * `template_error_deliver` (boolean) - Send even if template has errors
//! * `template_error_reporting` (string) - Email address to notify on template errors
//! * `variables` (map) - Key/value variables for template substitution
//! * `custom_id` (string) - Custom ID for tracking
//! * `event_payload` (string or map) - Custom payload for webhook events

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::email::Email;
use crate::error::MailError;
use crate::mailer::{DeliveryResult, Mailer};

const MAILJET_API_URL: &str = "https://api.mailjet.com/v3.1";

/// Mailjet API email provider.
pub struct MailjetMailer {
    api_key: String,
    secret_key: String,
    client: Client,
    base_url: String,
}

impl MailjetMailer {
    /// Create a new Mailjet mailer with the given API key and secret key.
    pub fn new(api_key: impl Into<String>, secret_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            secret_key: secret_key.into(),
            client: Client::new(),
            base_url: MAILJET_API_URL.to_string(),
        }
    }

    /// Create with a custom reqwest client.
    pub fn with_client(
        api_key: impl Into<String>,
        secret_key: impl Into<String>,
        client: Client,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            secret_key: secret_key.into(),
            client,
            base_url: MAILJET_API_URL.to_string(),
        }
    }

    /// Set a custom base URL (for testing).
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    fn auth_header(&self) -> String {
        let credentials = format!("{}:{}", self.api_key, self.secret_key);
        format!("Basic {}", BASE64.encode(credentials.as_bytes()))
    }

    fn build_message(&self, email: &Email) -> Result<MailjetMessage, MailError> {
        let from = email.from.as_ref().ok_or(MailError::MissingField("from"))?;

        if email.to.is_empty() {
            return Err(MailError::MissingField("to"));
        }

        let mut message = MailjetMessage {
            from: MailjetAddress {
                email: from.email.clone(),
                name: from.name.clone().unwrap_or_default(),
            },
            to: email
                .to
                .iter()
                .map(|a| MailjetAddress {
                    email: a.email.clone(),
                    name: a.name.clone().unwrap_or_default(),
                })
                .collect(),
            cc: if email.cc.is_empty() {
                None
            } else {
                Some(
                    email
                        .cc
                        .iter()
                        .map(|a| MailjetAddress {
                            email: a.email.clone(),
                            name: a.name.clone().unwrap_or_default(),
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
                        .map(|a| MailjetAddress {
                            email: a.email.clone(),
                            name: a.name.clone().unwrap_or_default(),
                        })
                        .collect(),
                )
            },
            reply_to: email.reply_to.first().map(|a| MailjetAddress {
                email: a.email.clone(),
                name: a.name.clone().unwrap_or_default(),
            }),
            subject: email.subject.clone(),
            text_part: email.text_body.clone(),
            html_part: email.html_body.clone(),
            headers: if email.headers.is_empty() {
                None
            } else {
                Some(email.headers.clone())
            },
            attachments: None,
            inlined_attachments: None,
            template_id: None,
            template_language: None,
            template_error_deliver: None,
            template_error_reporting: None,
            variables: None,
            custom_id: None,
            event_payload: None,
        };

        // Add attachments
        if !email.attachments.is_empty() {
            let (inline, regular): (Vec<_>, Vec<_>) = email
                .attachments
                .iter()
                .partition(|a| a.disposition == crate::attachment::AttachmentType::Inline);

            if !regular.is_empty() {
                message.attachments = Some(
                    regular
                        .iter()
                        .map(|a| MailjetAttachment {
                            content_type: a.content_type.clone(),
                            filename: a.filename.clone(),
                            base64_content: a.base64_data(),
                            content_id: a.content_id.clone().unwrap_or_else(|| a.filename.clone()),
                        })
                        .collect(),
                );
            }

            if !inline.is_empty() {
                message.inlined_attachments = Some(
                    inline
                        .iter()
                        .map(|a| MailjetAttachment {
                            content_type: a.content_type.clone(),
                            filename: a.filename.clone(),
                            base64_content: a.base64_data(),
                            content_id: a.content_id.clone().unwrap_or_else(|| a.filename.clone()),
                        })
                        .collect(),
                );
            }
        }

        // Provider-specific options
        if let Some(template_id) = email.provider_options.get("template_id") {
            message.template_id = template_id.as_i64();
            message.template_language = Some(true);

            // Template error handling
            if let Some(deliver) = email.provider_options.get("template_error_deliver") {
                message.template_error_deliver = deliver.as_bool();
            }

            if let Some(reporting) = email.provider_options.get("template_error_reporting") {
                if let Some(email_str) = reporting.as_str() {
                    message.template_error_reporting = Some(MailjetAddress {
                        email: email_str.to_string(),
                        name: String::new(),
                    });
                }
            }
        }

        if let Some(variables) = email.provider_options.get("variables") {
            message.variables = Some(variables.clone());
        }

        if let Some(custom_id) = email.provider_options.get("custom_id") {
            message.custom_id = custom_id.as_str().map(|s| s.to_string());
        }

        if let Some(event_payload) = email.provider_options.get("event_payload") {
            if let Some(s) = event_payload.as_str() {
                message.event_payload = Some(s.to_string());
            } else {
                // Serialize map to JSON string
                message.event_payload = Some(serde_json::to_string(event_payload)?);
            }
        }

        Ok(message)
    }
}

#[async_trait]
impl Mailer for MailjetMailer {
    async fn deliver(&self, email: &Email) -> Result<DeliveryResult, MailError> {
        let message = self.build_message(email)?;
        let request = MailjetRequest {
            messages: vec![message],
        };

        let url = format!("{}/send", self.base_url);
        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .header("User-Agent", format!("missive/{}", crate::VERSION))
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        let body: MailjetResponse = response.json().await?;

        if status.is_success() {
            if let Some(msg) = body.messages.first() {
                if msg.status == "success" {
                    let message_id = msg
                        .to
                        .as_ref()
                        .and_then(|to| to.first())
                        .and_then(|t| t.message_id)
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

                    return Ok(DeliveryResult::with_response(
                        message_id,
                        serde_json::json!({ "provider": "mailjet" }),
                    ));
                } else if let Some(errors) = &msg.errors {
                    let error_msg = errors
                        .iter()
                        .map(|e| e.error_message.clone())
                        .collect::<Vec<_>>()
                        .join("; ");
                    return Err(MailError::provider_with_status(
                        "mailjet",
                        error_msg,
                        status.as_u16(),
                    ));
                }
            }
            // Fallback success
            Ok(DeliveryResult::with_response(
                uuid::Uuid::new_v4().to_string(),
                serde_json::json!({ "provider": "mailjet" }),
            ))
        } else {
            // Check for per-message errors
            if let Some(msg) = body.messages.first() {
                if let Some(errors) = &msg.errors {
                    let error_msg = errors
                        .iter()
                        .map(|e| e.error_message.clone())
                        .collect::<Vec<_>>()
                        .join("; ");
                    return Err(MailError::provider_with_status(
                        "mailjet",
                        error_msg,
                        status.as_u16(),
                    ));
                }
            }
            // Global error
            let error_msg = body
                .error_message
                .unwrap_or_else(|| "Unknown error".to_string());
            Err(MailError::provider_with_status(
                "mailjet",
                error_msg,
                status.as_u16(),
            ))
        }
    }

    async fn deliver_many(&self, emails: &[Email]) -> Result<Vec<DeliveryResult>, MailError> {
        if emails.is_empty() {
            return Ok(vec![]);
        }

        let messages: Result<Vec<_>, _> = emails.iter().map(|e| self.build_message(e)).collect();
        let request = MailjetRequest {
            messages: messages?,
        };

        let url = format!("{}/send", self.base_url);
        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .header("User-Agent", format!("missive/{}", crate::VERSION))
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        let body: MailjetResponse = response.json().await?;

        if status.is_success() {
            Ok(body
                .messages
                .iter()
                .map(|msg| {
                    let message_id = msg
                        .to
                        .as_ref()
                        .and_then(|to| to.first())
                        .and_then(|t| t.message_id)
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

                    DeliveryResult::with_response(
                        message_id,
                        serde_json::json!({
                            "provider": "mailjet",
                            "status": msg.status
                        }),
                    )
                })
                .collect())
        } else {
            let error_msg = body
                .error_message
                .unwrap_or_else(|| "Unknown error".to_string());
            Err(MailError::provider_with_status(
                "mailjet",
                error_msg,
                status.as_u16(),
            ))
        }
    }

    fn provider_name(&self) -> &'static str {
        "mailjet"
    }
}

// ============================================================================
// Mailjet API Types
// ============================================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct MailjetRequest {
    messages: Vec<MailjetMessage>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct MailjetMessage {
    from: MailjetAddress,
    to: Vec<MailjetAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cc: Option<Vec<MailjetAddress>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bcc: Option<Vec<MailjetAddress>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<MailjetAddress>,
    subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text_part: Option<String>,
    #[serde(rename = "HTMLPart", skip_serializing_if = "Option::is_none")]
    html_part: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachments: Option<Vec<MailjetAttachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inlined_attachments: Option<Vec<MailjetAttachment>>,
    #[serde(rename = "TemplateID", skip_serializing_if = "Option::is_none")]
    template_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    template_language: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    template_error_deliver: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    template_error_reporting: Option<MailjetAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    variables: Option<Value>,
    #[serde(rename = "CustomID", skip_serializing_if = "Option::is_none")]
    custom_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    event_payload: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MailjetAddress {
    email: String,
    name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct MailjetAttachment {
    content_type: String,
    filename: String,
    #[serde(rename = "Base64Content")]
    base64_content: String,
    #[serde(rename = "ContentId")]
    content_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MailjetResponse {
    #[serde(default)]
    messages: Vec<MailjetMessageResult>,
    #[serde(default)]
    error_message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MailjetMessageResult {
    #[serde(default)]
    status: String,
    #[serde(default)]
    to: Option<Vec<MailjetRecipientResult>>,
    #[serde(default)]
    errors: Option<Vec<MailjetError>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MailjetRecipientResult {
    #[serde(rename = "MessageID")]
    message_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MailjetError {
    error_message: String,
}
