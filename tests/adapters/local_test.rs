//! Local adapter tests.
//!
//! Ported from Swoosh's local_test.exs

use missive::providers::LocalMailer;
use missive::{Email, Mailer};

// ============================================================================
// Basic Delivery Tests (matching Swoosh local_test.exs)
// ============================================================================

#[tokio::test]
async fn deliver_returns_ok() {
    let mailer = LocalMailer::new();

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .text_body("Hello!");

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_many_returns_ok() {
    let mailer = LocalMailer::new();

    let email_to_steve = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .text_body("Hello!");

    let email_to_natasha = Email::new()
        .from("tony.stark@example.com")
        .to("natasha.romanoff@example.com")
        .subject("Hello, Avengers!")
        .text_body("Hello!");

    let result = mailer.deliver_many(&[email_to_steve, email_to_natasha]).await;
    assert!(result.is_ok());
    let ids = result.unwrap();
    assert_eq!(ids.len(), 2);
}

// ============================================================================
// Storage Tests
// ============================================================================

#[tokio::test]
async fn captures_sent_emails() {
    let mailer = LocalMailer::new();

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .text_body("Hello!");

    mailer.deliver(&email).await.unwrap();

    assert!(mailer.has_emails());
    assert_eq!(mailer.email_count(), 1);
    assert!(mailer.sent_to("steve.rogers@example.com"));
    assert!(mailer.sent_with_subject("Hello, Avengers!"));
}

#[tokio::test]
async fn can_flush_emails() {
    let mailer = LocalMailer::new();

    mailer
        .deliver(&Email::new().from("a@b.com").to("c@d.com").subject("Test 1"))
        .await
        .unwrap();
    mailer
        .deliver(&Email::new().from("a@b.com").to("c@d.com").subject("Test 2"))
        .await
        .unwrap();

    let flushed = mailer.flush();
    assert_eq!(flushed.len(), 2);
    assert_eq!(mailer.email_count(), 0);
}

// ============================================================================
// Failure Simulation Tests
// ============================================================================

#[tokio::test]
async fn can_simulate_failure() {
    let mailer = LocalMailer::new();
    mailer.set_failure("SMTP connection refused");

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Test");

    let result = mailer.deliver(&email).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("SMTP connection refused"));

    // Clear failure
    mailer.clear_failure();
    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

// ============================================================================
// Provider Name Test
// ============================================================================

#[test]
fn provider_name_returns_local() {
    let mailer = LocalMailer::new();
    assert_eq!(mailer.provider_name(), "local");
}
