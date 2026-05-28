//! Mailer trait and delivery result types.
//!
//! # Architecture: Why `async_trait`?
//!
//! This module uses `#[async_trait]` instead of native async traits (Rust 1.75+)
//! because the library requires dynamic dispatch via `Arc<dyn Mailer>`.
//!
//! ## The tradeoff
//!
//! Native async traits are not object-safe - you can't use `dyn Trait` with them.
//! The `async_trait` macro boxes futures, enabling dynamic dispatch at the cost
//! of one heap allocation per method call.
//!
//! ## Why this cost is acceptable
//!
//! Email sending is I/O-bound. Network latency (50-500ms) completely dominates
//! the ~10ns heap allocation. The boxing overhead is unmeasurable in practice.
//!
//! ## What dynamic dispatch enables
//!
//! - **Runtime provider selection**: Choose providers from environment variables
//!   without recompilation. Deploy the same binary to staging (LocalMailer) and
//!   production (ResendMailer).
//!
//! - **Global mailer pattern**: The `deliver(&email)` API stores an `Arc<dyn Mailer>`
//!   internally, auto-configured from environment variables.
//!
//! - **Custom providers**: Users can implement `Mailer` for their own types and
//!   use them with the global `configure()` function.
//!
//! ## Zero-cost alternative
//!
//! Users who want to avoid boxing can call methods directly on concrete types:
//!
//! ```ignore
//! let mailer = ResendMailer::new(api_key);
//! mailer.deliver(&email).await?;  // No dynamic dispatch
//! ```
//!
//! The boxing only occurs when using `Arc<dyn Mailer>` (global mailer, runtime
//! provider selection).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::address::Address;
use crate::email::{Email, PreparedEmail};
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
    /// Apply provider-specific validation and convert an email into a prepared
    /// message.
    ///
    /// Providers with alternate recipient models can override this while still
    /// ensuring their adapter receives `PreparedEmail` rather than raw `Email`.
    fn prepare_email(
        &self,
        email: Email,
        default_from: Option<Address>,
    ) -> Result<PreparedEmail, MailError> {
        PreparedEmail::with_default_from(email, default_from)
    }

    /// Validate and send a single email.
    ///
    /// Provider implementations should normally implement
    /// [`deliver_prepared`](Self::deliver_prepared), not override this method,
    /// so every direct mailer call goes through Missive's shared validation
    /// path before provider-specific serialization.
    async fn deliver(&self, email: &Email) -> Result<DeliveryResult, MailError> {
        let email = self.prepare_email(email.clone(), None)?;
        self.deliver_prepared(&email).await
    }

    /// Send an email that has passed Missive's shared validation.
    ///
    /// Returns the message ID on success.
    async fn deliver_prepared(&self, email: &PreparedEmail) -> Result<DeliveryResult, MailError>;

    /// Validate emails before batch sending.
    ///
    /// Override this in providers that have batch limitations.
    /// Called by `deliver_many()` before sending.
    ///
    /// # Example
    ///
    /// ```ignore
    /// fn validate_batch(&self, emails: &[PreparedEmail]) -> Result<(), MailError> {
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
    fn validate_batch(&self, _emails: &[PreparedEmail]) -> Result<(), MailError> {
        Ok(()) // Default: no restrictions
    }

    /// Validate and send multiple emails.
    ///
    /// Default implementation prepares all messages first, calls
    /// `validate_batch()`, then sends each prepared email. Providers with batch
    /// APIs should override [`deliver_many_prepared`](Self::deliver_many_prepared).
    async fn deliver_many(&self, emails: &[Email]) -> Result<Vec<DeliveryResult>, MailError> {
        let emails = emails
            .iter()
            .cloned()
            .map(|email| self.prepare_email(email, None))
            .collect::<Result<Vec<_>, _>>()?;

        self.deliver_many_prepared(&emails).await
    }

    /// Send multiple already-prepared emails.
    ///
    /// Default implementation calls `validate_batch()` first, then
    /// `deliver_prepared()` for each email.
    /// Providers with batch APIs can override for better performance.
    async fn deliver_many_prepared(
        &self,
        emails: &[PreparedEmail],
    ) -> Result<Vec<DeliveryResult>, MailError> {
        self.validate_batch(emails)?;

        let mut results = Vec::with_capacity(emails.len());
        for email in emails {
            results.push(self.deliver_prepared(email).await?);
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
    ///
    /// # Deprecated
    ///
    /// This method does not account for the `EMAIL_FROM` environment variable fallback,
    /// so it incorrectly rejects emails that rely on the default sender. Use
    /// [`Email::is_valid()`] for a quick check, or rely on the validation in
    /// [`deliver()`]/[`deliver_with()`] which handles environment defaults correctly.
    #[deprecated(
        since = "0.6.2",
        note = "does not handle EMAIL_FROM fallback; use Email::is_valid() or rely on deliver() validation"
    )]
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

#[async_trait]
impl<T: Mailer + ?Sized> Mailer for &T {
    fn prepare_email(
        &self,
        email: Email,
        default_from: Option<Address>,
    ) -> Result<PreparedEmail, MailError> {
        (**self).prepare_email(email, default_from)
    }

    async fn deliver(&self, email: &Email) -> Result<DeliveryResult, MailError> {
        (**self).deliver(email).await
    }

    async fn deliver_prepared(&self, email: &PreparedEmail) -> Result<DeliveryResult, MailError> {
        (**self).deliver_prepared(email).await
    }

    fn validate_batch(&self, emails: &[PreparedEmail]) -> Result<(), MailError> {
        (**self).validate_batch(emails)
    }

    async fn deliver_many(&self, emails: &[Email]) -> Result<Vec<DeliveryResult>, MailError> {
        (**self).deliver_many(emails).await
    }

    async fn deliver_many_prepared(
        &self,
        emails: &[PreparedEmail],
    ) -> Result<Vec<DeliveryResult>, MailError> {
        (**self).deliver_many_prepared(emails).await
    }

    fn provider_name(&self) -> &'static str {
        (**self).provider_name()
    }

    fn validate_config(&self) -> Result<(), MailError> {
        (**self).validate_config()
    }
}

#[async_trait]
impl<T: Mailer + ?Sized> Mailer for Arc<T> {
    fn prepare_email(
        &self,
        email: Email,
        default_from: Option<Address>,
    ) -> Result<PreparedEmail, MailError> {
        (**self).prepare_email(email, default_from)
    }

    async fn deliver(&self, email: &Email) -> Result<DeliveryResult, MailError> {
        (**self).deliver(email).await
    }

    async fn deliver_prepared(&self, email: &PreparedEmail) -> Result<DeliveryResult, MailError> {
        (**self).deliver_prepared(email).await
    }

    fn validate_batch(&self, emails: &[PreparedEmail]) -> Result<(), MailError> {
        (**self).validate_batch(emails)
    }

    async fn deliver_many(&self, emails: &[Email]) -> Result<Vec<DeliveryResult>, MailError> {
        (**self).deliver_many(emails).await
    }

    async fn deliver_many_prepared(
        &self,
        emails: &[PreparedEmail],
    ) -> Result<Vec<DeliveryResult>, MailError> {
        (**self).deliver_many_prepared(emails).await
    }

    fn provider_name(&self) -> &'static str {
        (**self).provider_name()
    }

    fn validate_config(&self) -> Result<(), MailError> {
        (**self).validate_config()
    }
}
