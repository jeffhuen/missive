//! Tests for the interceptor functionality.
//!
//! These tests define the expected behavior for interceptors.

use missive::providers::LocalMailer;
use missive::{Address, Email, Interceptor, InterceptorExt, MailError, Mailer};

/// Test that a basic interceptor can modify an email.
#[tokio::test]
async fn test_interceptor_modifies_email() {
    let local = LocalMailer::new();
    let mailer = local
        .clone()
        .with_interceptor(|email: Email| Ok(email.header("X-Modified", "true")));

    let email = Email::new()
        .from("sender@example.com")
        .to("recipient@example.com")
        .subject("Test")
        .text_body("Hello");

    mailer.deliver(&email).await.unwrap();

    let emails = local.emails();
    assert_eq!(emails.len(), 1);
    assert_eq!(
        emails[0].email.headers.get("X-Modified"),
        Some(&"true".to_string())
    );
}

/// Test that an interceptor can redirect recipients.
#[tokio::test]
async fn test_interceptor_redirects_recipients() {
    let local = LocalMailer::new();
    let mailer = local.clone().with_interceptor(|email: Email| {
        Ok(email
            .put_to(vec![Address::new("test@dev.example.com")])
            .put_cc(vec![])
            .put_bcc(vec![]))
    });

    let email = Email::new()
        .from("sender@example.com")
        .to("real-user@example.com")
        .cc("another@example.com")
        .subject("Test")
        .text_body("Hello");

    mailer.deliver(&email).await.unwrap();

    let emails = local.emails();
    assert_eq!(emails.len(), 1);
    assert_eq!(emails[0].email.to.len(), 1);
    assert_eq!(emails[0].email.to[0].email, "test@dev.example.com");
    assert!(emails[0].email.cc.is_empty());
}

/// Test that an interceptor can block an email by returning an error.
#[tokio::test]
async fn test_interceptor_blocks_email() {
    let local = LocalMailer::new();
    let mailer = local.clone().with_interceptor(|email: Email| {
        if email.to.iter().any(|a| a.email.ends_with("@blocked.com")) {
            return Err(MailError::SendError("Blocked domain".into()));
        }
        Ok(email)
    });

    let email = Email::new()
        .from("sender@example.com")
        .to("victim@blocked.com")
        .subject("Test")
        .text_body("Hello");

    let result = mailer.deliver(&email).await;
    assert!(result.is_err());

    // Email should not have been stored
    let emails = local.emails();
    assert!(emails.is_empty());
}

/// Test that multiple interceptors all apply their transformations.
#[tokio::test]
async fn test_interceptor_chaining() {
    let local = LocalMailer::new();
    let mailer = local
        .clone()
        .with_interceptor(|email: Email| Ok(email.header("X-First", "1")))
        .with_interceptor(|email: Email| Ok(email.header("X-Second", "2")))
        .with_interceptor(|email: Email| Ok(email.header("X-Third", "3")));

    let email = Email::new()
        .from("sender@example.com")
        .to("recipient@example.com")
        .subject("Test")
        .text_body("Hello");

    mailer.deliver(&email).await.unwrap();

    let emails = local.emails();
    assert_eq!(emails.len(), 1);
    // All interceptors should have applied their headers
    assert_eq!(
        emails[0].email.headers.get("X-First"),
        Some(&"1".to_string())
    );
    assert_eq!(
        emails[0].email.headers.get("X-Second"),
        Some(&"2".to_string())
    );
    assert_eq!(
        emails[0].email.headers.get("X-Third"),
        Some(&"3".to_string())
    );
}

/// Test that a struct implementing Interceptor works.
#[tokio::test]
async fn test_struct_interceptor() {
    struct AddHeader {
        name: String,
        value: String,
    }

    impl Interceptor for AddHeader {
        fn intercept(&self, email: Email) -> Result<Email, MailError> {
            Ok(email.header(&self.name, &self.value))
        }
    }

    let local = LocalMailer::new();
    let mailer = local.clone().with_interceptor(AddHeader {
        name: "X-Custom".into(),
        value: "custom-value".into(),
    });

    let email = Email::new()
        .from("sender@example.com")
        .to("recipient@example.com")
        .subject("Test")
        .text_body("Hello");

    mailer.deliver(&email).await.unwrap();

    let emails = local.emails();
    assert_eq!(
        emails[0].email.headers.get("X-Custom"),
        Some(&"custom-value".to_string())
    );
}

/// Test that interceptors work with deliver_many (batch sending).
#[tokio::test]
async fn test_interceptor_with_deliver_many() {
    let local = LocalMailer::new();
    let mailer = local
        .clone()
        .with_interceptor(|email: Email| Ok(email.header("X-Batch", "true")));

    let emails: Vec<Email> = (0..3)
        .map(|i| {
            Email::new()
                .from("sender@example.com")
                .to(format!("user{}@example.com", i))
                .subject(format!("Test {}", i))
                .text_body("Hello")
        })
        .collect();

    mailer.deliver_many(&emails).await.unwrap();

    let stored = local.emails();
    assert_eq!(stored.len(), 3);
    for email in stored {
        assert_eq!(
            email.email.headers.get("X-Batch"),
            Some(&"true".to_string())
        );
    }
}

/// Test that provider_name is preserved through the interceptor wrapper.
#[tokio::test]
async fn test_interceptor_preserves_provider_name() {
    let mailer = LocalMailer::new().with_interceptor(|email: Email| Ok(email));

    assert_eq!(mailer.provider_name(), "local");
}

/// Test that if any interceptor blocks, the email is not sent.
#[tokio::test]
async fn test_blocking_interceptor_prevents_delivery() {
    let local = LocalMailer::new();
    let mailer = local
        .clone()
        .with_interceptor(|email: Email| Ok(email.header("X-Before", "1")))
        .with_interceptor(|_email: Email| Err(MailError::SendError("Blocked".into())))
        .with_interceptor(|email: Email| Ok(email.header("X-After", "1")));

    let email = Email::new()
        .from("sender@example.com")
        .to("recipient@example.com")
        .subject("Test")
        .text_body("Hello");

    let result = mailer.deliver(&email).await;
    assert!(result.is_err());

    // Email should not have been delivered
    assert!(local.emails().is_empty());
}

/// Test that validate_config delegates to inner mailer.
#[tokio::test]
async fn test_validate_config_delegates() {
    let mailer = LocalMailer::new().with_interceptor(|email: Email| Ok(email));

    // LocalMailer's validate_config always succeeds
    assert!(mailer.validate_config().is_ok());
}

/// Test that deliver_many fails fast when any email fails interception.
#[tokio::test]
async fn test_deliver_many_fails_on_interceptor_error() {
    let local = LocalMailer::new();
    let mailer = local.clone().with_interceptor(|email: Email| {
        if email.subject.contains("bad") {
            return Err(MailError::SendError("Blocked bad email".into()));
        }
        Ok(email)
    });

    let emails = vec![
        Email::new()
            .from("sender@example.com")
            .to("user1@example.com")
            .subject("good email"),
        Email::new()
            .from("sender@example.com")
            .to("user2@example.com")
            .subject("bad email"),
        Email::new()
            .from("sender@example.com")
            .to("user3@example.com")
            .subject("another good email"),
    ];

    let result = mailer.deliver_many(&emails).await;
    assert!(result.is_err());

    // No emails should have been sent (fail-fast behavior)
    assert!(local.emails().is_empty());
}
