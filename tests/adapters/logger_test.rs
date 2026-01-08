//! Logger adapter tests.
//!
//! Ported from Swoosh's logger_test.exs

use missive::providers::LoggerMailer;
use missive::{Email, Mailer};

// ============================================================================
// Basic Delivery Tests
// ============================================================================

#[tokio::test]
async fn deliver_returns_ok() {
    let mailer = LoggerMailer::new();

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .text_body("Hello!");

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());

    let delivery = result.unwrap();
    assert!(!delivery.message_id.is_empty());
}

#[tokio::test]
async fn deliver_with_full_logging_returns_ok() {
    let mailer = LoggerMailer::full();

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello!</h1>")
        .text_body("Hello!");

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());

    let delivery = result.unwrap();
    assert!(!delivery.message_id.is_empty());
}

// ============================================================================
// All Fields Tests
// ============================================================================

#[tokio::test]
async fn deliver_with_all_fields_returns_ok() {
    let mailer = LoggerMailer::new();

    let email = Email::new()
        .from(("T Stark", "tony.stark@example.com"))
        .to("steve.rogers@example.com")
        .to(("Bruce Banner", "bruce.banner@example.com"))
        .cc("natasha.romanoff@example.com")
        .bcc("nick.fury@example.com")
        .reply_to("pepper.potts@example.com")
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>")
        .text_body("Hello");

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

// ============================================================================
// Builder Tests
// ============================================================================

#[tokio::test]
async fn new_creates_working_logger() {
    let mailer = LoggerMailer::new();
    let email = Email::new().from("a@b.com").to("c@d.com").subject("Test");
    assert!(mailer.deliver(&email).await.is_ok());
}

#[tokio::test]
async fn full_creates_working_logger() {
    let mailer = LoggerMailer::full();
    let email = Email::new().from("a@b.com").to("c@d.com").subject("Test");
    assert!(mailer.deliver(&email).await.is_ok());
}

#[tokio::test]
async fn log_full_builder_method_works() {
    let mailer = LoggerMailer::new().log_full(true);
    let email = Email::new().from("a@b.com").to("c@d.com").subject("Test");
    assert!(mailer.deliver(&email).await.is_ok());
}

#[tokio::test]
async fn default_creates_working_logger() {
    let mailer = LoggerMailer::default();
    let email = Email::new().from("a@b.com").to("c@d.com").subject("Test");
    assert!(mailer.deliver(&email).await.is_ok());
}

// ============================================================================
// Provider Name Test
// ============================================================================

#[test]
fn provider_name_returns_logger() {
    let mailer = LoggerMailer::new();
    assert_eq!(mailer.provider_name(), "logger");
}

// ============================================================================
// deliver_many Tests
// ============================================================================

#[tokio::test]
async fn deliver_many_returns_ok() {
    let mailer = LoggerMailer::new();

    let email1 = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Email 1");

    let email2 = Email::new()
        .from("tony.stark@example.com")
        .to("natasha.romanoff@example.com")
        .subject("Email 2");

    let result = mailer.deliver_many(&[email1, email2]).await;
    assert!(result.is_ok());

    let deliveries = result.unwrap();
    assert_eq!(deliveries.len(), 2);
    assert!(!deliveries[0].message_id.is_empty());
    assert!(!deliveries[1].message_id.is_empty());
}

// ============================================================================
// Edge Cases
// ============================================================================

#[tokio::test]
async fn deliver_empty_email_returns_ok() {
    let mailer = LoggerMailer::new();

    // Logger doesn't validate, it just logs
    let email = Email::new().subject("Empty email");

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_html_only_returns_ok() {
    let mailer = LoggerMailer::full();

    let email = Email::new()
        .from("sender@example.com")
        .to("recipient@example.com")
        .subject("HTML only")
        .html_body("<h1>Hello</h1>");

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_text_only_returns_ok() {
    let mailer = LoggerMailer::full();

    let email = Email::new()
        .from("sender@example.com")
        .to("recipient@example.com")
        .subject("Text only")
        .text_body("Hello");

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}
