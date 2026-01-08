//! Memory storage tests.
//!
//! Ported from Swoosh's memory_test.exs

use missive::{Email, MemoryStorage, Storage};

// ============================================================================
// Initialization Tests
// ============================================================================

#[test]
fn new_storage_starts_empty() {
    let storage = MemoryStorage::new();
    assert_eq!(storage.count(), 0);
    assert!(storage.all().is_empty());
}

// ============================================================================
// Push Tests
// ============================================================================

#[test]
fn push_adds_email_to_mailbox() {
    let storage = MemoryStorage::new();

    storage.push(Email::new().subject("Test"));
    assert_eq!(storage.count(), 1);
}

#[test]
fn push_returns_unique_id() {
    let storage = MemoryStorage::new();

    let id1 = storage.push(Email::new().subject("First"));
    let id2 = storage.push(Email::new().subject("Second"));

    assert_ne!(id1, id2);
    assert!(!id1.is_empty());
    assert!(!id2.is_empty());
}

// ============================================================================
// Get Tests
// ============================================================================

#[test]
fn get_retrieves_email_by_id() {
    let storage = MemoryStorage::new();

    storage.push(Email::new().subject("First"));
    let id = storage.push(Email::new().subject("Hello, Avengers!"));
    storage.push(Email::new().subject("Third"));

    let retrieved = storage.get(&id).unwrap();
    assert_eq!(retrieved.email.subject, "Hello, Avengers!");
}

#[test]
fn get_returns_none_for_unknown_id() {
    let storage = MemoryStorage::new();

    storage.push(Email::new().subject("Test"));
    assert!(storage.get("unknown-id").is_none());
}

// ============================================================================
// Pop Tests
// ============================================================================

#[test]
fn pop_removes_most_recent_email() {
    let storage = MemoryStorage::new();

    storage.push(Email::new().subject("Test 1"));
    storage.push(Email::new().subject("Test 2"));
    assert_eq!(storage.count(), 2);

    let email = storage.pop().unwrap();
    assert_eq!(email.email.subject, "Test 2");
    assert_eq!(storage.count(), 1);

    let email = storage.pop().unwrap();
    assert_eq!(email.email.subject, "Test 1");
    assert_eq!(storage.count(), 0);
}

#[test]
fn pop_returns_none_when_empty() {
    let storage = MemoryStorage::new();
    assert!(storage.pop().is_none());
}

// ============================================================================
// All Tests
// ============================================================================

#[test]
fn all_returns_emails_newest_first() {
    let storage = MemoryStorage::new();

    storage.push(Email::new().subject("First"));
    storage.push(Email::new().subject("Second"));
    storage.push(Email::new().subject("Third"));

    let all = storage.all();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].email.subject, "Third");
    assert_eq!(all[1].email.subject, "Second");
    assert_eq!(all[2].email.subject, "First");
}

// ============================================================================
// Delete Tests
// ============================================================================

#[test]
fn delete_removes_specific_email() {
    let storage = MemoryStorage::new();

    let id1 = storage.push(Email::new().subject("First"));
    let id2 = storage.push(Email::new().subject("Second"));

    assert!(storage.delete(&id1));
    assert_eq!(storage.count(), 1);
    assert!(storage.get(&id1).is_none());
    assert!(storage.get(&id2).is_some());
}

#[test]
fn delete_returns_false_for_unknown_id() {
    let storage = MemoryStorage::new();

    storage.push(Email::new().subject("Test"));
    assert!(!storage.delete("unknown-id"));
    assert_eq!(storage.count(), 1);
}

#[test]
fn clear_removes_all_emails() {
    let storage = MemoryStorage::new();

    storage.push(Email::new().subject("First"));
    storage.push(Email::new().subject("Second"));
    assert_eq!(storage.count(), 2);

    storage.clear();
    assert_eq!(storage.count(), 0);
    assert!(storage.all().is_empty());
}

// ============================================================================
// Flush Tests
// ============================================================================

#[test]
fn flush_returns_all_and_clears() {
    let storage = MemoryStorage::new();

    storage.push(Email::new().subject("First"));
    storage.push(Email::new().subject("Second"));
    storage.push(Email::new().subject("Third"));
    assert_eq!(storage.count(), 3);

    let flushed = storage.flush();
    assert_eq!(flushed.len(), 3);
    assert_eq!(flushed[0].email.subject, "Third"); // Newest first
    assert_eq!(flushed[1].email.subject, "Second");
    assert_eq!(flushed[2].email.subject, "First");

    // Storage should be empty after flush
    assert_eq!(storage.count(), 0);
    assert!(storage.all().is_empty());
}

#[test]
fn flush_on_empty_returns_empty() {
    let storage = MemoryStorage::new();
    let flushed = storage.flush();
    assert!(flushed.is_empty());
}

// ============================================================================
// Shared Storage Tests
// ============================================================================

#[test]
fn shared_storage_works_across_clones() {
    let storage = MemoryStorage::shared();
    let storage_clone = storage.clone();

    storage.push(Email::new().subject("From original"));
    storage_clone.push(Email::new().subject("From clone"));

    // Both see the same emails
    assert_eq!(storage.count(), 2);
    assert_eq!(storage_clone.count(), 2);

    // Pop from one affects the other
    storage.pop();
    assert_eq!(storage_clone.count(), 1);
}
