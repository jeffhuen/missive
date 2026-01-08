//! Logger mailer that only logs emails.
//!
//! Useful for staging environments or when you want to see what would be sent
//! without actually sending or storing emails.

use async_trait::async_trait;

use crate::email::Email;
use crate::error::MailError;
use crate::mailer::{DeliveryResult, Mailer};

/// Logger mailer that emits tracing events for emails.
pub struct LoggerMailer {
    /// If true, log full email details. If false, just log recipient summary.
    log_full: bool,
}

impl LoggerMailer {
    /// Create a logger mailer with brief output (just recipients).
    pub fn new() -> Self {
        Self { log_full: false }
    }

    /// Create a logger mailer with full email details.
    pub fn full() -> Self {
        Self { log_full: true }
    }

    /// Set whether to log full email details.
    pub fn log_full(mut self, full: bool) -> Self {
        self.log_full = full;
        self
    }
}

impl Default for LoggerMailer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Mailer for LoggerMailer {
    async fn deliver(&self, email: &Email) -> Result<DeliveryResult, MailError> {
        let message_id = uuid::Uuid::new_v4().to_string();

        if self.log_full {
            // Log full email details
            tracing::info!(
                message_id = %message_id,
                from = ?email.from.as_ref().map(|a| a.formatted()),
                to = ?email.to.iter().map(|a| a.formatted()).collect::<Vec<_>>(),
                cc = ?email.cc.iter().map(|a| a.formatted()).collect::<Vec<_>>(),
                bcc = ?email.bcc.iter().map(|a| a.formatted()).collect::<Vec<_>>(),
                subject = %email.subject,
                has_html = email.html_body.is_some(),
                has_text = email.text_body.is_some(),
                attachments = email.attachments.len(),
                "Email logged (full)"
            );

            // Also log bodies at debug level
            if let Some(ref text) = email.text_body {
                tracing::debug!(body = %text, "Text body");
            }
            if let Some(ref html) = email.html_body {
                tracing::debug!(body = %html, "HTML body");
            }
        } else {
            // Brief log - just recipients
            tracing::info!(
                message_id = %message_id,
                to = ?email.to.iter().map(|a| &a.email).collect::<Vec<_>>(),
                subject = %email.subject,
                "Email logged"
            );
        }

        Ok(DeliveryResult::new(message_id))
    }

    fn provider_name(&self) -> &'static str {
        "logger"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Address;

    #[tokio::test]
    async fn test_logger_brief() {
        let mailer = LoggerMailer::new();

        let email = Email::new()
            .from(Address::new("sender@example.com"))
            .to(Address::new("recipient@example.com"))
            .subject("Test Subject")
            .text_body("Hello, World!");

        let result = mailer.deliver(&email).await;
        assert!(result.is_ok());

        let delivery = result.unwrap();
        assert!(!delivery.message_id.is_empty());
    }

    #[tokio::test]
    async fn test_logger_full() {
        let mailer = LoggerMailer::full();

        let email = Email::new()
            .from(Address::with_name("Alice", "alice@example.com"))
            .to(Address::new("bob@example.com"))
            .cc(Address::new("charlie@example.com"))
            .subject("Test Subject")
            .text_body("Plain text")
            .html_body("<p>HTML</p>");

        let result = mailer.deliver(&email).await;
        assert!(result.is_ok());

        let delivery = result.unwrap();
        assert!(!delivery.message_id.is_empty());
    }

    #[tokio::test]
    async fn test_logger_builder() {
        let mailer = LoggerMailer::new().log_full(true);
        assert!(mailer.log_full);

        let mailer = LoggerMailer::new().log_full(false);
        assert!(!mailer.log_full);
    }

    #[test]
    fn test_provider_name() {
        let mailer = LoggerMailer::new();
        assert_eq!(mailer.provider_name(), "logger");
    }

    #[test]
    fn test_default() {
        let mailer = LoggerMailer::default();
        assert!(!mailer.log_full);
    }
}
