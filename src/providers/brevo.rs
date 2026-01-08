//! Brevo API provider (formerly Sendinblue).
//!
//! For reference: [Brevo API docs](https://developers.brevo.com/reference/sendtransacemail)
//!
//! # Example
//!
//! ```rust,ignore
//! use missive::providers::BrevoMailer;
//!
//! let mailer = BrevoMailer::new("your-api-key");
//! ```
//!
//! ## Provider Options
//!
//! Brevo-specific options can be set via `provider_option`:
//!
//! ```rust,ignore
//! use chrono::{Utc, Duration};
//!
//! let email = Email::new()
//!     .from("sender@example.com")
//!     .to("recipient@example.com")
//!     .subject("Hello")
//!     .provider_option("sender_id", 42)
//!     .provider_option("template_id", 123)
//!     .provider_option("params", json!({"name": "John", "order_id": 456}))
//!     .provider_option("tags", vec!["welcome", "onboarding"])
//!     .provider_option("schedule_at", (Utc::now() + Duration::hours(1)).to_rfc3339());
//! ```
//!
//! ## Provider Options Reference
//!
//! * `sender_id` (integer) - Use a sender ID instead of email address
//! * `template_id` (integer) - ID of the active transactional email template
//! * `params` (map) - Key/value attributes to customize the template
//! * `tags` (list[string]) - Tags for filtering in Brevo dashboard
//! * `schedule_at` (string) - RFC3339 UTC datetime to schedule the email
//!
//! ## Using Template Default Sender
//!
//! When using a template, you can omit the sender and use the template's
//! default sender by setting the from email to "TEMPLATE":
//!
//! ```rust,ignore
//! let email = Email::new()
//!     .from(("", "TEMPLATE"))  // Uses template's default sender
//!     .to("recipient@example.com")
//!     .provider_option("template_id", 123);
//! ```

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::email::Email;
use crate::error::MailError;
use crate::mailer::{DeliveryResult, Mailer};

const BREVO_BASE_URL: &str = "https://api.brevo.com/v3";
const BREVO_API_ENDPOINT: &str = "/smtp/email";

/// Brevo API email provider.
pub struct BrevoMailer {
    api_key: String,
    base_url: String,
    client: Client,
}

impl BrevoMailer {
    /// Create a new Brevo mailer with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: BREVO_BASE_URL.to_string(),
            client: Client::new(),
        }
    }

    /// Create with a custom reqwest client.
    pub fn with_client(api_key: impl Into<String>, client: Client) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: BREVO_BASE_URL.to_string(),
            client,
        }
    }

    /// Set a custom base URL (for testing or EU endpoint).
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    fn build_request(&self, email: &Email) -> Result<BrevoRequest, MailError> {
        let from = email.from.as_ref().ok_or(MailError::MissingField("from"))?;

        if email.to.is_empty() {
            return Err(MailError::MissingField("to"));
        }

        let mut request = BrevoRequest {
            sender: prepare_sender(from, email),
            to: email.to.iter().map(prepare_recipient).collect(),
            cc: if email.cc.is_empty() {
                None
            } else {
                Some(email.cc.iter().map(prepare_recipient).collect())
            },
            bcc: if email.bcc.is_empty() {
                None
            } else {
                Some(email.bcc.iter().map(prepare_recipient).collect())
            },
            reply_to: email.reply_to.first().map(prepare_recipient),
            subject: if email.subject.is_empty() {
                None
            } else {
                Some(email.subject.clone())
            },
            text_content: email.text_body.clone(),
            html_content: email.html_body.clone(),
            template_id: None,
            headers: if email.headers.is_empty() {
                None
            } else {
                Some(email.headers.clone())
            },
            params: None,
            tags: None,
            attachment: None,
            scheduled_at: None,
        };

        // Provider-specific options
        if let Some(template_id) = email.provider_options.get("template_id") {
            request.template_id = template_id.as_i64();
        }
        if let Some(params) = email.provider_options.get("params") {
            if let Some(obj) = params.as_object() {
                request.params = Some(obj.clone().into_iter().collect());
            }
        }
        if let Some(tags) = email.provider_options.get("tags") {
            request.tags = serde_json::from_value(tags.clone()).ok();
        }
        if let Some(schedule_at) = email.provider_options.get("schedule_at") {
            request.scheduled_at = schedule_at.as_str().map(|s| s.to_string());
        }

        // Add attachments
        if !email.attachments.is_empty() {
            request.attachment = Some(
                email
                    .attachments
                    .iter()
                    .map(|a| BrevoAttachment {
                        name: a.filename.clone(),
                        content: a.base64_data(),
                    })
                    .collect(),
            );
        }

        Ok(request)
    }
}

/// Check if sender should use template default (email is "TEMPLATE").
fn is_template_sender(from: &crate::Address) -> bool {
    from.email == "TEMPLATE"
}

fn prepare_sender(from: &crate::Address, email: &Email) -> Option<BrevoSender> {
    // When from email is "TEMPLATE", don't send sender - use template default
    if is_template_sender(from) {
        return None;
    }

    // Check for sender_id provider option
    if let Some(sender_id) = email.provider_options.get("sender_id") {
        if let Some(id) = sender_id.as_i64() {
            return Some(BrevoSender {
                id: Some(id),
                email: Some(from.email.clone()),
                name: None,
            });
        }
    }

    Some(BrevoSender {
        id: None,
        email: Some(from.email.clone()),
        name: from.name.clone(),
    })
}

fn prepare_recipient(addr: &crate::Address) -> BrevoRecipient {
    BrevoRecipient {
        email: addr.email.clone(),
        name: addr.name.clone(),
    }
}

#[async_trait]
impl Mailer for BrevoMailer {
    async fn deliver(&self, email: &Email) -> Result<DeliveryResult, MailError> {
        let request = self.build_request(email)?;
        let url = format!("{}{}", self.base_url, BREVO_API_ENDPOINT);

        let response = self
            .client
            .post(&url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("User-Agent", format!("missive/{}", crate::VERSION))
            .header("Api-Key", &self.api_key)
            .json(&request)
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            let result: BrevoResponse = response.json().await?;
            Ok(DeliveryResult::with_response(
                result.message_id,
                serde_json::json!({ "provider": "brevo" }),
            ))
        } else {
            let error: BrevoError = response.json().await.unwrap_or(BrevoError {
                code: "unknown".to_string(),
                message: "Unknown error".to_string(),
            });
            Err(MailError::provider_with_status(
                "brevo",
                format!("[{}] {}", error.code, error.message),
                status.as_u16(),
            ))
        }
    }

    /// Send multiple emails in a single API call using Brevo's messageVersions.
    ///
    /// Global parameters (from first email): sender, attachments, tags, scheduled_at
    /// Per-email parameters: to, cc, bcc, subject, content, template_id, params, headers, reply_to
    async fn deliver_many(&self, emails: &[Email]) -> Result<Vec<DeliveryResult>, MailError> {
        if emails.is_empty() {
            return Ok(vec![]);
        }

        let first_email = &emails[0];
        let from = first_email
            .from
            .as_ref()
            .ok_or(MailError::MissingField("from"))?;

        // Build batch request with messageVersions
        let batch_request = BrevoBatchRequest {
            sender: prepare_sender(from, first_email),
            subject: if first_email.subject.is_empty() {
                None
            } else {
                Some(first_email.subject.clone())
            },
            text_content: first_email.text_body.clone(),
            html_content: first_email.html_body.clone(),
            template_id: first_email
                .provider_options
                .get("template_id")
                .and_then(|v| v.as_i64()),
            tags: first_email
                .provider_options
                .get("tags")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            attachment: if first_email.attachments.is_empty() {
                None
            } else {
                Some(
                    first_email
                        .attachments
                        .iter()
                        .map(|a| BrevoAttachment {
                            name: a.filename.clone(),
                            content: a.base64_data(),
                        })
                        .collect(),
                )
            },
            scheduled_at: first_email
                .provider_options
                .get("schedule_at")
                .and_then(|v| v.as_str().map(|s| s.to_string())),
            message_versions: emails.iter().map(prepare_message_version).collect(),
        };

        // If first email has no subject but template_id, that's fine
        if batch_request.subject.is_none() && batch_request.template_id.is_none() {
            // Check if any email has a subject
            if !emails.iter().any(|e| !e.subject.is_empty()) {
                return Err(MailError::MissingField("subject"));
            }
        }

        let url = format!("{}{}", self.base_url, BREVO_API_ENDPOINT);

        let response = self
            .client
            .post(&url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("User-Agent", format!("missive/{}", crate::VERSION))
            .header("Api-Key", &self.api_key)
            .json(&batch_request)
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            let result: BrevoBatchResponse = response.json().await?;
            Ok(result
                .message_ids
                .into_iter()
                .map(|id| {
                    DeliveryResult::with_response(id, serde_json::json!({ "provider": "brevo" }))
                })
                .collect())
        } else {
            let error: BrevoError = response.json().await.unwrap_or(BrevoError {
                code: "unknown".to_string(),
                message: "Unknown error".to_string(),
            });
            Err(MailError::provider_with_status(
                "brevo",
                format!("[{}] {}", error.code, error.message),
                status.as_u16(),
            ))
        }
    }

    fn provider_name(&self) -> &'static str {
        "brevo"
    }
}

fn prepare_message_version(email: &Email) -> BrevoMessageVersion {
    BrevoMessageVersion {
        to: email.to.iter().map(prepare_recipient).collect(),
        cc: if email.cc.is_empty() {
            None
        } else {
            Some(email.cc.iter().map(prepare_recipient).collect())
        },
        bcc: if email.bcc.is_empty() {
            None
        } else {
            Some(email.bcc.iter().map(prepare_recipient).collect())
        },
        reply_to: email.reply_to.first().map(prepare_recipient),
        subject: if email.subject.is_empty() {
            None
        } else {
            Some(email.subject.clone())
        },
        text_content: email.text_body.clone(),
        html_content: email.html_body.clone(),
        template_id: email
            .provider_options
            .get("template_id")
            .and_then(|v| v.as_i64()),
        headers: if email.headers.is_empty() {
            None
        } else {
            Some(email.headers.clone())
        },
        params: email
            .provider_options
            .get("params")
            .and_then(|v| v.as_object().map(|obj| obj.clone().into_iter().collect())),
    }
}

// ============================================================================
// Brevo API Types
// ============================================================================

#[derive(Debug, Serialize)]
struct BrevoSender {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Debug, Serialize)]
struct BrevoRecipient {
    email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BrevoRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    sender: Option<BrevoSender>,
    to: Vec<BrevoRecipient>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cc: Option<Vec<BrevoRecipient>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bcc: Option<Vec<BrevoRecipient>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<BrevoRecipient>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    html_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    template_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachment: Option<Vec<BrevoAttachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scheduled_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BrevoBatchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    sender: Option<BrevoSender>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    html_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    template_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachment: Option<Vec<BrevoAttachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scheduled_at: Option<String>,
    message_versions: Vec<BrevoMessageVersion>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BrevoMessageVersion {
    to: Vec<BrevoRecipient>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cc: Option<Vec<BrevoRecipient>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bcc: Option<Vec<BrevoRecipient>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<BrevoRecipient>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    html_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    template_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Serialize)]
struct BrevoAttachment {
    name: String,
    content: String, // Base64 encoded
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrevoResponse {
    message_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrevoBatchResponse {
    #[serde(default)]
    message_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct BrevoError {
    code: String,
    message: String,
}
