//! Mailer trait and delivery result types.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::email::Email;
use crate::error::MailError;

/// Result of a successful email delivery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryResult {
    /// Message ID assigned by the provider
    pub message_id: String,
    /// Optional provider-specific response data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_response: Option<serde_json::Value>,
}

impl DeliveryResult {
    /// Create a new delivery result with just a message ID.
    pub fn new(message_id: impl Into<String>) -> Self {
        Self {
            message_id: message_id.into(),
            provider_response: None,
        }
    }

    /// Create a delivery result with provider response.
    pub fn with_response(message_id: impl Into<String>, response: serde_json::Value) -> Self {
        Self {
            message_id: message_id.into(),
            provider_response: Some(response),
        }
    }
}

/// Trait for email delivery providers.
///
/// All email providers (SMTP, Resend, SendGrid, etc.) implement this trait.
///
/// # Example
///
/// ```ignore
/// use missive::{Email, Mailer};
/// use missive::providers::SmtpMailer;
///
/// let mailer = SmtpMailer::new("smtp.example.com", 587, "user", "pass");
///
/// let email = Email::new()
///     .from("sender@example.com")
///     .to("recipient@example.com")
///     .subject("Hello")
///     .text_body("World");
///
/// let result = mailer.deliver(&email).await?;
/// println!("Sent with ID: {}", result.message_id);
/// ```
#[async_trait]
pub trait Mailer: Send + Sync {
    /// Send a single email.
    ///
    /// Returns the message ID on success.
    async fn deliver(&self, email: &Email) -> Result<DeliveryResult, MailError>;

    /// Validate emails before batch sending.
    ///
    /// Override this in providers that have batch limitations.
    /// Called by `deliver_many()` before sending.
    ///
    /// # Example
    ///
    /// ```ignore
    /// fn validate_batch(&self, emails: &[Email]) -> Result<(), MailError> {
    ///     for email in emails {
    ///         if !email.attachments.is_empty() {
    ///             return Err(MailError::UnsupportedFeature(
    ///                 "attachments not supported in batch sends".into()
    ///             ));
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    fn validate_batch(&self, _emails: &[Email]) -> Result<(), MailError> {
        Ok(()) // Default: no restrictions
    }

    /// Send multiple emails.
    ///
    /// Default implementation calls `validate_batch()` first, then `deliver()` for each email.
    /// Providers with batch APIs can override for better performance.
    async fn deliver_many(&self, emails: &[Email]) -> Result<Vec<DeliveryResult>, MailError> {
        // Validate batch before sending
        self.validate_batch(emails)?;

        let mut results = Vec::with_capacity(emails.len());
        for email in emails {
            results.push(self.deliver(email).await?);
        }
        Ok(results)
    }

    /// Get the provider name (for logging/debugging).
    fn provider_name(&self) -> &'static str {
        "unknown"
    }

    /// Validate configuration.
    ///
    /// Called at startup to verify required configuration is present.
    /// Override in providers that require specific config (API keys, etc.).
    fn validate_config(&self) -> Result<(), MailError> {
        Ok(())
    }
}

/// Extension trait for optional mailer operations.
pub trait MailerExt: Mailer {
    /// Validate an email before sending.
    fn validate(&self, email: &Email) -> Result<(), MailError> {
        if email.from.is_none() {
            return Err(MailError::MissingField("from"));
        }
        if email.to.is_empty() {
            return Err(MailError::MissingField("to"));
        }
        Ok(())
    }
}

// Auto-implement MailerExt for all Mailers
impl<T: Mailer> MailerExt for T {}
