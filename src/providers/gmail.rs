//! Gmail API provider.
//!
//! Sends emails through Gmail's API using RFC 2822 formatted messages via media upload.
//!
//! # Example
//!
//! ```rust,ignore
//! use missive::providers::GmailMailer;
//!
//! // Create with OAuth2 access token
//! let mailer = GmailMailer::new("your-access-token");
//! ```
//!
//! ## Authentication
//!
//! Gmail requires an OAuth2 access token with one of these scopes:
//! - `https://www.googleapis.com/auth/gmail.send` (recommended, minimal permissions)
//! - `https://www.googleapis.com/auth/gmail.compose`
//! - `https://www.googleapis.com/auth/gmail.modify`
//! - `https://mail.google.com/` (full access)
//!
//! The token must be obtained through Google's OAuth2 flow. Common approaches:
//!
//! - Service account with domain-wide delegation
//! - OAuth2 web/desktop flow for user consent
//! - Google Cloud Application Default Credentials
//!
//! This provider does NOT handle token refresh - you must provide a valid access token.
//!
//! ## Technical Details
//!
//! This provider uses Gmail's media upload endpoint with `uploadType=media` and
//! `Content-Type: message/rfc822`. The RFC 2822 message is built using lettre
//! (shared with the SMTP provider).

use async_trait::async_trait;
use lettre::{
    message::{
        header::{ContentType, HeaderName, HeaderValue},
        Attachment as LettreAttachment, Mailbox, MultiPart, SinglePart,
    },
    Message,
};
use reqwest::Client;
use serde::Deserialize;

use crate::address::Address;
use crate::attachment::AttachmentType;
use crate::email::{Email, PreparedEmail};
use crate::error::MailError;
use crate::mailer::{DeliveryResult, Mailer};

const GMAIL_API_URL: &str =
    "https://www.googleapis.com/upload/gmail/v1/users/me/messages/send?uploadType=media";

/// Gmail API email provider.
pub struct GmailMailer {
    access_token: String,
    client: Client,
    base_url: String,
}

impl GmailMailer {
    /// Create a new Gmail mailer with an OAuth2 access token.
    ///
    /// # Arguments
    ///
    /// * `access_token` - OAuth2 access token with `gmail.send` scope
    pub fn new(access_token: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
            client: Client::new(),
            base_url: GMAIL_API_URL.to_string(),
        }
    }

    /// Create with a custom reqwest client.
    pub fn with_client(access_token: impl Into<String>, client: Client) -> Self {
        Self {
            access_token: access_token.into(),
            client,
            base_url: GMAIL_API_URL.to_string(),
        }
    }

    /// Set a custom base URL (for testing).
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Build a lettre Message from our Email struct (RFC 2822 format).
    async fn build_message(&self, email: &Email) -> Result<Message, MailError> {
        let from = email.from.as_ref().ok_or(MailError::MissingField("from"))?;

        if email.to.is_empty() {
            return Err(MailError::MissingField("to"));
        }

        let mut builder = Message::builder()
            .from(address_to_mailbox(from)?)
            .subject(&email.subject);

        // Add recipients
        for to in &email.to {
            builder = builder.to(address_to_mailbox(to)?);
        }
        for cc in &email.cc {
            builder = builder.cc(address_to_mailbox(cc)?);
        }
        for bcc in &email.bcc {
            builder = builder.bcc(address_to_mailbox(bcc)?);
        }

        // Reply-to
        if let Some(reply_to) = email.reply_to.first() {
            builder = builder.reply_to(address_to_mailbox(reply_to)?);
        }

        for (name, value) in &email.headers {
            let name = HeaderName::new_from_ascii(name.clone())
                .map_err(|_| MailError::BuildError(format!("Invalid header name: {}", name)))?;
            builder = builder.raw_header(HeaderValue::new(name, value.clone()));
        }

        // Build body
        let message = if email.attachments.is_empty() {
            // Simple message without attachments
            match (&email.html_body, &email.text_body) {
                (Some(html), Some(text)) => builder.multipart(
                    MultiPart::alternative_plain_html(text.clone(), html.clone()),
                )?,
                (Some(html), None) => builder.header(ContentType::TEXT_HTML).body(html.clone())?,
                (None, Some(text)) => builder.header(ContentType::TEXT_PLAIN).body(text.clone())?,
                (None, None) => builder
                    .header(ContentType::TEXT_PLAIN)
                    .body(String::new())?,
            }
        } else {
            // Message with attachments - build mixed multipart
            let body_part = match (&email.html_body, &email.text_body) {
                (Some(html), Some(text)) => {
                    MultiPart::alternative_plain_html(text.clone(), html.clone())
                }
                (Some(html), None) => MultiPart::mixed().singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(html.clone()),
                ),
                (None, Some(text)) => MultiPart::mixed().singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_PLAIN)
                        .body(text.clone()),
                ),
                (None, None) => MultiPart::mixed().singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_PLAIN)
                        .body(String::new()),
                ),
            };

            // Start with body and add attachments
            let mut multipart = MultiPart::mixed().multipart(body_part);

            for attachment in &email.attachments {
                let data = attachment.get_data_async().await?;
                let content_type: ContentType = attachment
                    .content_type
                    .parse()
                    .unwrap_or(ContentType::TEXT_PLAIN);

                let lettre_attachment = match attachment.disposition {
                    AttachmentType::Inline => {
                        let cid = attachment
                            .content_id
                            .as_ref()
                            .unwrap_or(&attachment.filename);
                        LettreAttachment::new_inline(cid.clone()).body(data, content_type)
                    }
                    AttachmentType::Attachment => {
                        LettreAttachment::new(attachment.filename.clone()).body(data, content_type)
                    }
                };

                multipart = multipart.singlepart(lettre_attachment);
            }

            builder.multipart(multipart)?
        };

        Ok(message)
    }
}

#[async_trait]
impl Mailer for GmailMailer {
    async fn deliver_prepared(&self, email: &PreparedEmail) -> Result<DeliveryResult, MailError> {
        let message = self.build_message(email).await?;

        // Convert lettre Message to RFC 2822 bytes
        let raw_message = message.formatted();

        let response = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "message/rfc822")
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("User-Agent", format!("missive/{}", crate::VERSION))
            .body(raw_message)
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            let result: GmailResponse = response.json().await?;
            Ok(DeliveryResult::with_response(
                result.id.clone(),
                serde_json::json!({
                    "provider": "gmail",
                    "id": result.id,
                    "threadId": result.thread_id,
                    "labelIds": result.label_ids,
                }),
            ))
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(MailError::provider_with_status(
                "gmail",
                error_text,
                status.as_u16(),
            ))
        }
    }

    fn provider_name(&self) -> &'static str {
        "gmail"
    }
}

// ============================================================================
// Gmail API Types
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GmailResponse {
    id: String,
    thread_id: String,
    #[serde(default)]
    label_ids: Vec<String>,
}

/// Convert our Address to lettre's Mailbox.
fn address_to_mailbox(addr: &Address) -> Result<Mailbox, MailError> {
    let email = addr.to_ascii()?.parse()?;

    Ok(Mailbox::new(addr.name.clone(), email))
}
