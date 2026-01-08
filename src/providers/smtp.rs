//! SMTP provider using lettre.
//!
//! # Example
//!
//! ```rust,ignore
//! use missive::providers::SmtpMailer;
//!
//! // With authentication
//! let mailer = SmtpMailer::new("smtp.example.com", 587)
//!     .credentials("username", "password")
//!     .build();
//!
//! // Without authentication (local relay)
//! let mailer = SmtpMailer::localhost();
//! ```

use async_trait::async_trait;
use lettre::{
    message::{
        header::ContentType, Attachment as LettreAttachment, Mailbox, MultiPart, SinglePart,
    },
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

use crate::address::Address;
use crate::attachment::AttachmentType;
use crate::email::Email;
use crate::error::MailError;
use crate::mailer::{DeliveryResult, Mailer};

/// SMTP email provider.
pub struct SmtpMailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
}

impl SmtpMailer {
    /// Create a new SMTP mailer builder with TLS (STARTTLS on port 587).
    pub fn new(host: &str, port: u16) -> SmtpBuilder {
        SmtpBuilder {
            host: host.to_string(),
            port,
            credentials: None,
            tls: TlsMode::StartTls,
        }
    }

    /// Create a new SMTP mailer for localhost (no TLS, no auth).
    pub fn localhost() -> Self {
        let transport = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("localhost")
            .port(25)
            .build();

        Self { transport }
    }

    /// Build a lettre Message from our Email struct.
    fn build_message(&self, email: &Email) -> Result<Message, MailError> {
        let from = email
            .from
            .as_ref()
            .ok_or(MailError::MissingField("from"))?;

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

        // Reply-to (supports multiple, use first one for SMTP)
        if let Some(reply_to) = email.reply_to.first() {
            builder = builder.reply_to(address_to_mailbox(reply_to)?);
        }

        // Custom headers - note: lettre requires implementing Header trait for custom headers.
        // For now, we skip custom headers in SMTP. Use provider_options for SMTP-specific headers.
        // TODO: Add support for common custom headers (X-Priority, X-Mailer, etc.)
        let _ = &email.headers; // Acknowledge but don't use

        // Build body
        let message = if email.attachments.is_empty() {
            // Simple message without attachments
            match (&email.html_body, &email.text_body) {
                (Some(html), Some(text)) => builder
                    .multipart(MultiPart::alternative_plain_html(text.clone(), html.clone()))?,
                (Some(html), None) => builder.header(ContentType::TEXT_HTML).body(html.clone())?,
                (None, Some(text)) => {
                    builder.header(ContentType::TEXT_PLAIN).body(text.clone())?
                }
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
                        LettreAttachment::new_inline(cid.clone())
                            .body(attachment.data.clone(), content_type)
                    }
                    AttachmentType::Attachment => LettreAttachment::new(attachment.filename.clone())
                        .body(attachment.data.clone(), content_type),
                };

                multipart = multipart.singlepart(lettre_attachment);
            }

            builder.multipart(multipart)?
        };

        Ok(message)
    }
}

#[async_trait]
impl Mailer for SmtpMailer {
    async fn deliver(&self, email: &Email) -> Result<DeliveryResult, MailError> {
        let message = self.build_message(email)?;

        let response = self
            .transport
            .send(message)
            .await
            .map_err(|e| MailError::SendError(e.to_string()))?;

        // Extract message ID from SMTP response, or generate one
        let message_id = response
            .message()
            .next()
            .and_then(|m| m.lines().next())
            .map(|s| s.to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        Ok(DeliveryResult::new(message_id))
    }

    fn provider_name(&self) -> &'static str {
        "smtp"
    }
}

/// TLS mode for SMTP connection.
#[derive(Debug, Clone, Copy)]
pub enum TlsMode {
    /// No TLS (dangerous, only for localhost)
    None,
    /// STARTTLS - upgrade to TLS after connecting (port 587)
    StartTls,
    /// Implicit TLS - connect with TLS from start (port 465)
    Tls,
}

/// Builder for SmtpMailer.
pub struct SmtpBuilder {
    host: String,
    port: u16,
    credentials: Option<Credentials>,
    tls: TlsMode,
}

impl SmtpBuilder {
    /// Set SMTP credentials.
    pub fn credentials(mut self, username: &str, password: &str) -> Self {
        self.credentials = Some(Credentials::new(username.to_string(), password.to_string()));
        self
    }

    /// Set TLS mode.
    pub fn tls(mut self, mode: TlsMode) -> Self {
        self.tls = mode;
        self
    }

    /// Disable TLS (dangerous, only for localhost/testing).
    pub fn no_tls(mut self) -> Self {
        self.tls = TlsMode::None;
        self
    }

    /// Build the SmtpMailer.
    pub fn build(self) -> SmtpMailer {
        let transport = match self.tls {
            TlsMode::None => {
                let mut t = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&self.host)
                    .port(self.port);
                if let Some(creds) = self.credentials {
                    t = t.credentials(creds);
                }
                t.build()
            }
            TlsMode::StartTls => {
                let mut t = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.host)
                    .unwrap_or_else(|_| {
                        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&self.host)
                    })
                    .port(self.port);
                if let Some(creds) = self.credentials {
                    t = t.credentials(creds);
                }
                t.build()
            }
            TlsMode::Tls => {
                let mut t = AsyncSmtpTransport::<Tokio1Executor>::relay(&self.host)
                    .unwrap_or_else(|_| {
                        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&self.host)
                    })
                    .port(self.port);
                if let Some(creds) = self.credentials {
                    t = t.credentials(creds);
                }
                t.build()
            }
        };

        SmtpMailer { transport }
    }
}

/// Convert our Address to lettre's Mailbox.
fn address_to_mailbox(addr: &Address) -> Result<Mailbox, MailError> {
    let email = addr
        .email
        .parse()
        .map_err(|e: lettre::address::AddressError| MailError::InvalidAddress(e.to_string()))?;

    Ok(Mailbox::new(addr.name.clone(), email))
}
