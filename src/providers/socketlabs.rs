//! SocketLabs Injection API provider.
//!
//! For reference: [SocketLabs API docs](https://docs.socketlabs.com/)
//!
//! # Example
//!
//! ```rust,ignore
//! use missive::providers::SocketLabsMailer;
//!
//! // The API key from SocketLabs dashboard is used as a Bearer token
//! let mailer = SocketLabsMailer::new("12345", "your-api-key");
//! ```
//!
//! ## Provider Options
//!
//! SocketLabs-specific options can be set via `provider_option`:
//!
//! ```rust,ignore
//! use serde_json::json;
//!
//! let email = Email::new()
//!     .from("sender@example.com")
//!     .to("recipient@example.com")
//!     .subject("Hello")
//!     .provider_option("api_template", "template-id")
//!     .provider_option("mailing_id", "batch-001")
//!     .provider_option("message_id", "msg-001")
//!     .provider_option("charset", "UTF-8")
//!     .provider_option("amp_body", "<html amp4email>...</html>")
//!     .provider_option("merge_data", json!({
//!         "PerMessage": [{"key": "value"}],
//!         "Global": {"global_key": "global_value"}
//!     }));
//! ```
//!
//! ## Provider Options Reference
//!
//! | Option | Type | Description |
//! |--------|------|-------------|
//! | `api_template` | string | Template identifier from Email Content Manager |
//! | `mailing_id` | string | Special header for tracking email batches |
//! | `message_id` | string | Special header for tracking individual messages |
//! | `charset` | string | Character encoding (default: UTF-8) |
//! | `amp_body` | string | AMP HTML content for dynamic emails |
//! | `merge_data` | object | Data for inline merge feature |

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::email::Email;
use crate::error::MailError;
use crate::mailer::{DeliveryResult, Mailer};

const SOCKETLABS_API_URL: &str = "https://inject-cx.socketlabs.com/api/v1";

/// SocketLabs Injection API email provider.
pub struct SocketLabsMailer {
    server_id: String,
    api_key: String,
    client: Client,
    base_url: String,
}

impl SocketLabsMailer {
    /// Create a new SocketLabs mailer with the given server ID and API key.
    ///
    /// # Arguments
    ///
    /// * `server_id` - Your SocketLabs server ID
    /// * `api_key` - Your SocketLabs API key
    pub fn new(server_id: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            server_id: server_id.into(),
            api_key: api_key.into(),
            client: Client::new(),
            base_url: SOCKETLABS_API_URL.to_string(),
        }
    }

    /// Create with a custom reqwest client.
    pub fn with_client(
        server_id: impl Into<String>,
        api_key: impl Into<String>,
        client: Client,
    ) -> Self {
        Self {
            server_id: server_id.into(),
            api_key: api_key.into(),
            client,
            base_url: SOCKETLABS_API_URL.to_string(),
        }
    }

    /// Set a custom base URL (for testing).
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    fn build_message(&self, email: &Email) -> Result<SocketLabsMessage, MailError> {
        let from = email.from.as_ref().ok_or(MailError::MissingField("from"))?;

        if email.to.is_empty() {
            return Err(MailError::MissingField("to"));
        }

        let mut message = SocketLabsMessage {
            from: SocketLabsAddress::from_address(from),
            to: email
                .to
                .iter()
                .map(SocketLabsAddress::from_address)
                .collect(),
            cc: if email.cc.is_empty() {
                None
            } else {
                Some(
                    email
                        .cc
                        .iter()
                        .map(SocketLabsAddress::from_address)
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
                        .map(SocketLabsAddress::from_address)
                        .collect(),
                )
            },
            subject: email.subject.clone(),
            html_body: email.html_body.clone(),
            text_body: email.text_body.clone(),
            amp_body: None,
            reply_to: email.reply_to.first().map(SocketLabsAddress::from_address),
            attachments: None,
            custom_headers: None,
            api_template: None,
            message_id: None,
            mailing_id: None,
            charset: None,
            merge_data: None,
        };

        // Add attachments
        if !email.attachments.is_empty() {
            message.attachments = Some(
                email
                    .attachments
                    .iter()
                    .map(|a| SocketLabsAttachment {
                        name: a.filename.clone(),
                        content_type: a.content_type.clone(),
                        content: a.base64_data(),
                        // Use filename as ContentId for inline attachments
                        content_id: if a.is_inline() {
                            a.content_id.clone().unwrap_or_else(|| a.filename.clone())
                        } else {
                            a.filename.clone()
                        },
                    })
                    .collect(),
            );
        }

        // Custom headers
        if !email.headers.is_empty() {
            message.custom_headers = Some(
                email
                    .headers
                    .iter()
                    .map(|(k, v)| SocketLabsCustomHeader {
                        name: k.clone(),
                        value: v.clone(),
                    })
                    .collect(),
            );
        }

        // Provider-specific options
        if let Some(api_template) = email.provider_options.get("api_template") {
            message.api_template = api_template.as_str().map(|s| s.to_string());
        }
        if let Some(message_id) = email.provider_options.get("message_id") {
            message.message_id = message_id.as_str().map(|s| s.to_string());
        }
        if let Some(mailing_id) = email.provider_options.get("mailing_id") {
            message.mailing_id = mailing_id.as_str().map(|s| s.to_string());
        }
        if let Some(charset) = email.provider_options.get("charset") {
            message.charset = charset.as_str().map(|s| s.to_string());
        }
        if let Some(amp_body) = email.provider_options.get("amp_body") {
            message.amp_body = amp_body.as_str().map(|s| s.to_string());
        }
        if let Some(merge_data) = email.provider_options.get("merge_data") {
            message.merge_data = Some(merge_data.clone());
        }

        Ok(message)
    }

    fn build_request(
        &self,
        messages: Vec<SocketLabsMessage>,
    ) -> Result<SocketLabsRequest, MailError> {
        let server_id: i64 = self.server_id.parse().map_err(|_| {
            MailError::Configuration(format!("Invalid server_id: {}", self.server_id))
        })?;

        Ok(SocketLabsRequest {
            server_id,
            messages,
        })
    }
}

#[async_trait]
impl Mailer for SocketLabsMailer {
    async fn deliver(&self, email: &Email) -> Result<DeliveryResult, MailError> {
        let message = self.build_message(email)?;
        let request = self.build_request(vec![message])?;

        let url = format!("{}/email", self.base_url);
        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("User-Agent", format!("missive/{}", crate::VERSION))
            .json(&request)
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            let result: SocketLabsResponse = response.json().await?;

            // SocketLabs returns "Success" for successful requests
            if result.error_code == "Success" {
                Ok(DeliveryResult::with_response(
                    result.transaction_receipt.unwrap_or_default(),
                    serde_json::json!({
                        "provider": "socketlabs",
                        "error_code": result.error_code,
                        "message_results": result.message_results,
                    }),
                ))
            } else {
                Err(MailError::provider_with_status(
                    "socketlabs",
                    format!(
                        "[{}] {}",
                        result.error_code,
                        result
                            .message_results
                            .as_ref()
                            .and_then(|r| r.first())
                            .and_then(|m| m.error_code.clone())
                            .unwrap_or_else(|| "Unknown error".to_string())
                    ),
                    status.as_u16(),
                ))
            }
        } else {
            let error: SocketLabsResponse = response.json().await.unwrap_or(SocketLabsResponse {
                error_code: "UnknownError".to_string(),
                message_results: None,
                transaction_receipt: None,
            });
            Err(MailError::provider_with_status(
                "socketlabs",
                format!("[{}] Request failed", error.error_code),
                status.as_u16(),
            ))
        }
    }

    async fn deliver_many(&self, emails: &[Email]) -> Result<Vec<DeliveryResult>, MailError> {
        if emails.is_empty() {
            return Ok(vec![]);
        }

        // Build all messages
        let messages: Vec<SocketLabsMessage> = emails
            .iter()
            .map(|email| self.build_message(email))
            .collect::<Result<Vec<_>, _>>()?;

        let request = self.build_request(messages)?;

        let url = format!("{}/email", self.base_url);
        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("User-Agent", format!("missive/{}", crate::VERSION))
            .json(&request)
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            let result: SocketLabsResponse = response.json().await?;

            if result.error_code == "Success" {
                // Map message results to delivery results
                let message_results = result.message_results.unwrap_or_default();
                let receipt = result.transaction_receipt.unwrap_or_default();

                Ok(message_results
                    .into_iter()
                    .enumerate()
                    .map(|(i, mr)| {
                        DeliveryResult::with_response(
                            // Use individual message ID or transaction receipt + index
                            mr.index
                                .map(|idx| format!("{}-{}", receipt, idx))
                                .unwrap_or_else(|| format!("{}-{}", receipt, i)),
                            serde_json::json!({
                                "provider": "socketlabs",
                                "error_code": mr.error_code,
                                "address_result": mr.address_result,
                            }),
                        )
                    })
                    .collect())
            } else {
                Err(MailError::provider_with_status(
                    "socketlabs",
                    format!("[{}] Batch request failed", result.error_code),
                    status.as_u16(),
                ))
            }
        } else {
            let error: SocketLabsResponse = response.json().await.unwrap_or(SocketLabsResponse {
                error_code: "UnknownError".to_string(),
                message_results: None,
                transaction_receipt: None,
            });
            Err(MailError::provider_with_status(
                "socketlabs",
                format!("[{}] Request failed", error.error_code),
                status.as_u16(),
            ))
        }
    }

    fn provider_name(&self) -> &'static str {
        "socketlabs"
    }
}

// ============================================================================
// SocketLabs API Types
// ============================================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct SocketLabsRequest {
    #[serde(rename = "serverId")]
    server_id: i64,
    #[serde(rename = "Messages")]
    messages: Vec<SocketLabsMessage>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct SocketLabsMessage {
    from: SocketLabsAddress,
    to: Vec<SocketLabsAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "CC")]
    cc: Option<Vec<SocketLabsAddress>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "BCC")]
    bcc: Option<Vec<SocketLabsAddress>>,
    subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    html_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    amp_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<SocketLabsAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachments: Option<Vec<SocketLabsAttachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    custom_headers: Option<Vec<SocketLabsCustomHeader>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mailing_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    charset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    merge_data: Option<Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SocketLabsAddress {
    email_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    friendly_name: Option<String>,
}

impl SocketLabsAddress {
    fn from_address(addr: &crate::Address) -> Self {
        Self {
            email_address: addr.email.clone(),
            friendly_name: addr.name.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct SocketLabsAttachment {
    name: String,
    content_type: String,
    content: String, // Base64 encoded
    content_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct SocketLabsCustomHeader {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SocketLabsResponse {
    error_code: String,
    #[serde(default)]
    message_results: Option<Vec<SocketLabsMessageResult>>,
    #[serde(default)]
    transaction_receipt: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct SocketLabsMessageResult {
    #[serde(default)]
    index: Option<i32>,
    #[serde(default)]
    error_code: Option<String>,
    #[serde(default)]
    address_result: Option<Vec<SocketLabsAddressResult>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct SocketLabsAddressResult {
    #[serde(default)]
    email_address: Option<String>,
    #[serde(default)]
    accepted: Option<bool>,
    #[serde(default)]
    error_code: Option<String>,
}
