//! Local mailer for development and testing.
//!
//! Stores emails in memory for viewing via the mailbox preview UI or for
//! programmatic assertions in tests.
//!
//! # Development Usage
//!
//! ```rust,ignore
//! use missive::providers::LocalMailer;
//! use missive::preview::mailbox_router;
//!
//! let mailer = LocalMailer::new();
//! let storage = mailer.storage();
//!
//! // Mount preview UI
//! let app = Router::new()
//!     .nest("/dev/mailbox", mailbox_router(storage));
//! ```
//!
//! # Testing Usage
//!
//! ```rust,ignore
//! use missive::providers::LocalMailer;
//! use missive::testing::*;
//!
//! #[tokio::test]
//! async fn test_sends_welcome_email() {
//!     let mailer = LocalMailer::new();
//!
//!     // Code under test
//!     send_welcome_email(&mailer, "user@example.com").await;
//!
//!     // Assertions
//!     assert_email_sent(&mailer);
//!     assert_email_to(&mailer, "user@example.com");
//!     assert_email_subject_contains(&mailer, "Welcome");
//! }
//! ```

use async_trait::async_trait;
use std::sync::Arc;

use crate::email::Email;
use crate::error::MailError;
use crate::mailer::{DeliveryResult, Mailer};
use crate::storage::{MemoryStorage, Storage, StoredEmail};

/// Local mailer that stores emails in memory.
///
/// Use for:
/// - **Development**: View emails via the [preview UI](crate::preview)
/// - **Testing**: Assert on sent emails with [testing helpers](crate::testing)
pub struct LocalMailer {
    storage: Arc<MemoryStorage>,
    /// If set, deliver() will return this error (for testing error paths).
    fail_with: std::sync::RwLock<Option<String>>,
}

impl LocalMailer {
    /// Create a new local mailer with fresh storage.
    pub fn new() -> Self {
        Self {
            storage: MemoryStorage::shared(),
            fail_with: std::sync::RwLock::new(None),
        }
    }

    /// Create a local mailer with existing storage.
    ///
    /// Useful for sharing storage between mailer and preview UI.
    pub fn with_storage(storage: Arc<MemoryStorage>) -> Self {
        Self {
            storage,
            fail_with: std::sync::RwLock::new(None),
        }
    }

    // =========================================================================
    // Storage Access (for preview UI)
    // =========================================================================

    /// Get a reference to the underlying storage.
    ///
    /// Use this to share storage with the mailbox preview router.
    pub fn storage(&self) -> Arc<MemoryStorage> {
        Arc::clone(&self.storage)
    }

    // =========================================================================
    // Failure Simulation (for testing)
    // =========================================================================

    /// Configure the mailer to fail with an error message.
    ///
    /// Useful for testing error handling paths.
    ///
    /// ```rust,ignore
    /// let mailer = LocalMailer::new();
    /// mailer.set_failure("SMTP connection refused");
    ///
    /// let result = deliver_with(&email, &mailer).await;
    /// assert!(result.is_err());
    /// ```
    pub fn set_failure(&self, message: impl Into<String>) {
        *self.fail_with.write().unwrap() = Some(message.into());
    }

    /// Clear the failure state.
    pub fn clear_failure(&self) {
        *self.fail_with.write().unwrap() = None;
    }

    // =========================================================================
    // Email Access (for testing assertions)
    // =========================================================================

    /// Get all captured emails (newest first).
    pub fn emails(&self) -> Vec<StoredEmail> {
        self.storage.all()
    }

    /// Get the most recently sent email.
    pub fn last_email(&self) -> Option<StoredEmail> {
        self.storage.all().into_iter().next()
    }

    /// Get the count of sent emails.
    pub fn email_count(&self) -> usize {
        self.storage.count()
    }

    /// Clear all captured emails.
    pub fn clear(&self) {
        self.storage.clear();
    }

    /// Remove and return all captured emails.
    ///
    /// Useful for multi-phase tests where you want to check emails
    /// from one phase without them affecting assertions in the next.
    pub fn flush(&self) -> Vec<StoredEmail> {
        self.storage.flush()
    }

    /// Check if any email was sent.
    pub fn has_emails(&self) -> bool {
        self.storage.count() > 0
    }

    // =========================================================================
    // Query Helpers (for testing)
    // =========================================================================

    /// Check if an email was sent to a specific address.
    pub fn sent_to(&self, email: &str) -> bool {
        self.storage.all().iter().any(|stored| {
            stored
                .email
                .to
                .iter()
                .any(|addr| addr.email.eq_ignore_ascii_case(email))
        })
    }

    /// Check if an email with matching subject was sent.
    pub fn sent_with_subject(&self, subject: &str) -> bool {
        self.storage
            .all()
            .iter()
            .any(|stored| stored.email.subject == subject)
    }

    /// Check if an email with subject containing text was sent.
    pub fn sent_with_subject_containing(&self, text: &str) -> bool {
        self.storage
            .all()
            .iter()
            .any(|stored| stored.email.subject.contains(text))
    }

    /// Find emails matching a predicate.
    pub fn find_emails<F>(&self, predicate: F) -> Vec<StoredEmail>
    where
        F: Fn(&Email) -> bool,
    {
        self.storage
            .all()
            .into_iter()
            .filter(|stored| predicate(&stored.email))
            .collect()
    }
}

impl Default for LocalMailer {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for LocalMailer {
    fn clone(&self) -> Self {
        Self {
            storage: Arc::clone(&self.storage),
            fail_with: std::sync::RwLock::new(self.fail_with.read().unwrap().clone()),
        }
    }
}

#[async_trait]
impl Mailer for LocalMailer {
    async fn deliver(&self, email: &Email) -> Result<DeliveryResult, MailError> {
        // Check for configured failure
        if let Some(ref message) = *self.fail_with.read().unwrap() {
            return Err(MailError::SendError(message.clone()));
        }

        let message_id = self.storage.push(email.clone());
        Ok(DeliveryResult::new(message_id))
    }

    fn provider_name(&self) -> &'static str {
        "local"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_mailer() {
        let mailer = LocalMailer::new();

        let email = Email::new()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Test Email")
            .text_body("Hello!");

        let result = mailer.deliver(&email).await.unwrap();
        assert!(!result.message_id.is_empty());

        // Verify stored
        let storage = mailer.storage();
        assert_eq!(storage.count(), 1);

        let stored = storage.get(&result.message_id).unwrap();
        assert_eq!(stored.email.subject, "Test Email");
    }

    #[tokio::test]
    async fn test_shared_storage() {
        let storage = MemoryStorage::shared();
        let mailer = LocalMailer::with_storage(Arc::clone(&storage));

        let email = Email::new().subject("Shared Test");
        mailer.deliver(&email).await.unwrap();

        // Storage is shared
        assert_eq!(storage.count(), 1);
        assert_eq!(mailer.storage().count(), 1);
    }

    #[tokio::test]
    async fn test_captures_emails() {
        let mailer = LocalMailer::new();

        let email = Email::new()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Test Subject");

        mailer.deliver(&email).await.unwrap();

        assert!(mailer.has_emails());
        assert_eq!(mailer.email_count(), 1);
        assert!(mailer.sent_to("recipient@example.com"));
        assert!(mailer.sent_with_subject("Test Subject"));
        assert!(mailer.sent_with_subject_containing("Subject"));
    }

    #[tokio::test]
    async fn test_can_fail() {
        let mailer = LocalMailer::new();
        mailer.set_failure("Simulated failure");

        let email = Email::new().subject("Test");
        let result = mailer.deliver(&email).await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Simulated failure"));

        // Clear failure and try again
        mailer.clear_failure();
        let result = mailer.deliver(&email).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_find_emails() {
        let mailer = LocalMailer::new();

        mailer
            .deliver(&Email::new().to("a@example.com").subject("Welcome"))
            .await
            .unwrap();

        mailer
            .deliver(&Email::new().to("b@example.com").subject("Goodbye"))
            .await
            .unwrap();

        let welcome_emails = mailer.find_emails(|e| e.subject.contains("Welcome"));
        assert_eq!(welcome_emails.len(), 1);
        assert!(welcome_emails[0]
            .email
            .to
            .iter()
            .any(|a| a.email == "a@example.com"));
    }

    #[tokio::test]
    async fn test_flush() {
        let mailer = LocalMailer::new();

        mailer
            .deliver(&Email::new().subject("Email 1"))
            .await
            .unwrap();
        mailer
            .deliver(&Email::new().subject("Email 2"))
            .await
            .unwrap();

        let flushed = mailer.flush();
        assert_eq!(flushed.len(), 2);
        assert_eq!(mailer.email_count(), 0);
    }

    #[tokio::test]
    async fn test_clone() {
        let mailer = LocalMailer::new();
        mailer.deliver(&Email::new().subject("Test")).await.unwrap();

        let cloned = mailer.clone();

        // Storage is shared
        assert_eq!(cloned.email_count(), 1);

        // Deliver through clone
        cloned.deliver(&Email::new().subject("Test 2")).await.unwrap();
        assert_eq!(mailer.email_count(), 2);
    }
}
