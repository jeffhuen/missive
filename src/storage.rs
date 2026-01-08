//! Storage trait and implementations for local/test mailers.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::email::Email;

/// A stored email with metadata.
#[derive(Debug, Clone)]
pub struct StoredEmail {
    /// Unique identifier for this email.
    pub id: String,
    /// The email content.
    pub email: Email,
    /// When the email was "sent" (stored).
    pub sent_at: DateTime<Utc>,
}

/// Trait for email storage backends.
pub trait Storage: Send + Sync {
    /// Store an email and return its ID.
    fn push(&self, email: Email) -> String;

    /// Pop and return the most recent email.
    fn pop(&self) -> Option<StoredEmail>;

    /// Get an email by ID.
    fn get(&self, id: &str) -> Option<StoredEmail>;

    /// Get all stored emails, newest first.
    fn all(&self) -> Vec<StoredEmail>;

    /// Delete an email by ID.
    fn delete(&self, id: &str) -> bool;

    /// Clear all stored emails.
    fn clear(&self);

    /// Get the count of stored emails.
    fn count(&self) -> usize;

    /// Remove and return all stored emails.
    fn flush(&self) -> Vec<StoredEmail>;
}

/// Thread-safe in-memory storage for emails.
///
/// Used by `LocalMailer` for development and testing.
#[derive(Debug, Default)]
pub struct MemoryStorage {
    emails: RwLock<HashMap<String, StoredEmail>>,
    /// Order of email IDs for maintaining insertion order.
    order: RwLock<Vec<String>>,
}

impl MemoryStorage {
    /// Create a new empty storage.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create storage wrapped in an Arc for sharing.
    pub fn shared() -> Arc<Self> {
        Arc::new(Self::new())
    }
}

impl Storage for MemoryStorage {
    fn push(&self, email: Email) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let sent_at = Utc::now();

        // Store sent_at in the email's private field
        let mut email = email;
        email
            .private
            .insert("sent_at".to_string(), serde_json::json!(sent_at.to_rfc3339()));

        let stored = StoredEmail {
            id: id.clone(),
            email,
            sent_at,
        };

        {
            let mut emails = self.emails.write().unwrap();
            let mut order = self.order.write().unwrap();
            emails.insert(id.clone(), stored);
            order.push(id.clone());
        }

        id
    }

    fn pop(&self) -> Option<StoredEmail> {
        let mut emails = self.emails.write().unwrap();
        let mut order = self.order.write().unwrap();

        if let Some(id) = order.pop() {
            emails.remove(&id)
        } else {
            None
        }
    }

    fn get(&self, id: &str) -> Option<StoredEmail> {
        let emails = self.emails.read().unwrap();
        emails.get(id).cloned()
    }

    fn all(&self) -> Vec<StoredEmail> {
        let emails = self.emails.read().unwrap();
        let order = self.order.read().unwrap();

        // Return in reverse order (newest first)
        order
            .iter()
            .rev()
            .filter_map(|id| emails.get(id).cloned())
            .collect()
    }

    fn delete(&self, id: &str) -> bool {
        let mut emails = self.emails.write().unwrap();
        let mut order = self.order.write().unwrap();

        if emails.remove(id).is_some() {
            order.retain(|x| x != id);
            true
        } else {
            false
        }
    }

    fn clear(&self) {
        let mut emails = self.emails.write().unwrap();
        let mut order = self.order.write().unwrap();
        emails.clear();
        order.clear();
    }

    fn count(&self) -> usize {
        let emails = self.emails.read().unwrap();
        emails.len()
    }

    fn flush(&self) -> Vec<StoredEmail> {
        let mut emails = self.emails.write().unwrap();
        let mut order = self.order.write().unwrap();

        // Get all in order (newest first)
        let result: Vec<StoredEmail> = order
            .iter()
            .rev()
            .filter_map(|id| emails.get(id).cloned())
            .collect();

        // Clear storage
        emails.clear();
        order.clear();

        result
    }
}

impl Storage for Arc<MemoryStorage> {
    fn push(&self, email: Email) -> String {
        (**self).push(email)
    }

    fn pop(&self) -> Option<StoredEmail> {
        (**self).pop()
    }

    fn get(&self, id: &str) -> Option<StoredEmail> {
        (**self).get(id)
    }

    fn all(&self) -> Vec<StoredEmail> {
        (**self).all()
    }

    fn delete(&self, id: &str) -> bool {
        (**self).delete(id)
    }

    fn clear(&self) {
        (**self).clear()
    }

    fn count(&self) -> usize {
        (**self).count()
    }

    fn flush(&self) -> Vec<StoredEmail> {
        (**self).flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_storage() {
        let storage = MemoryStorage::new();

        let email = Email::new()
            .from("test@example.com")
            .to("recipient@example.com")
            .subject("Test");

        // Push and retrieve
        let id = storage.push(email.clone());
        assert_eq!(storage.count(), 1);

        let stored = storage.get(&id).unwrap();
        assert_eq!(stored.email.subject, "Test");

        // All returns newest first
        let email2 = Email::new().subject("Second");
        let id2 = storage.push(email2);

        let all = storage.all();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, id2); // Newest first

        // Delete
        assert!(storage.delete(&id));
        assert_eq!(storage.count(), 1);
        assert!(storage.get(&id).is_none());

        // Clear
        storage.clear();
        assert_eq!(storage.count(), 0);
    }

    #[test]
    fn test_flush() {
        let storage = MemoryStorage::new();

        // Add some emails
        let email1 = Email::new()
            .from("test@example.com")
            .to("recipient@example.com")
            .subject("First");
        let email2 = Email::new()
            .from("test@example.com")
            .to("recipient@example.com")
            .subject("Second");
        let email3 = Email::new()
            .from("test@example.com")
            .to("recipient@example.com")
            .subject("Third");

        storage.push(email1);
        storage.push(email2);
        storage.push(email3);
        assert_eq!(storage.count(), 3);

        // Flush returns all emails (newest first) and clears storage
        let flushed = storage.flush();
        assert_eq!(flushed.len(), 3);
        assert_eq!(flushed[0].email.subject, "Third"); // Newest first
        assert_eq!(flushed[1].email.subject, "Second");
        assert_eq!(flushed[2].email.subject, "First");

        // Storage should be empty after flush
        assert_eq!(storage.count(), 0);
        assert!(storage.all().is_empty());

        // Flush on empty storage returns empty vec
        let empty_flush = storage.flush();
        assert!(empty_flush.is_empty());
    }
}
