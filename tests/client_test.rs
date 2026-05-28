#![cfg(feature = "local")]

use missive::providers::LocalMailer;
use missive::{deliver_with, Email, EmailClient, MailError, Mailer, Storage};

#[tokio::test]
async fn email_client_delivers_with_default_sender() {
    let mailer = LocalMailer::new();
    let client = EmailClient::new(mailer.clone()).with_default_from("noreply@example.com");
    let email = Email::new()
        .to("user@example.com")
        .subject("Welcome")
        .text_body("Hello");

    let result = client.deliver(email).await.unwrap();

    let stored = mailer.storage().get(&result.message_id).unwrap();
    assert_eq!(
        stored.email.from.as_ref().map(|addr| addr.email.as_str()),
        Some("noreply@example.com")
    );
    assert_eq!(stored.email.to[0].email, "user@example.com");
}

#[tokio::test]
async fn email_client_deliver_many_prepares_and_sends_all_emails() {
    let mailer = LocalMailer::new();
    let client = EmailClient::new(mailer.clone()).with_default_from("noreply@example.com");
    let emails = vec![
        Email::new()
            .to("one@example.com")
            .subject("One")
            .text_body("Hello one"),
        Email::new()
            .to("two@example.com")
            .subject("Two")
            .text_body("Hello two"),
    ];

    let results = client.deliver_many(emails).await.unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(mailer.email_count(), 2);
    assert!(mailer.sent_to("one@example.com"));
    assert!(mailer.sent_to("two@example.com"));
}

#[tokio::test]
async fn email_client_requires_sender_or_default_sender() {
    let client = EmailClient::new(LocalMailer::new());
    let email = Email::new().to("user@example.com");

    let error = client.deliver(email).await.unwrap_err();

    assert!(matches!(error, MailError::MissingField("from")));
}

#[tokio::test]
async fn direct_mailer_rejects_invalid_email_before_storage() {
    let mailer = LocalMailer::new();
    let email = Email::new().to("user@example.com");

    let error = mailer.deliver(&email).await.unwrap_err();

    assert!(matches!(error, MailError::MissingField("from")));
    assert_eq!(mailer.email_count(), 0);
}

#[tokio::test]
async fn direct_mailer_batch_rejects_invalid_email_before_storage() {
    let mailer = LocalMailer::new();
    let emails = vec![
        Email::new().from("noreply@example.com"),
        Email::new()
            .from("noreply@example.com")
            .to("user@example.com"),
    ];

    let error = mailer.deliver_many(&emails).await.unwrap_err();

    assert!(matches!(error, MailError::MissingField("to")));
    assert_eq!(mailer.email_count(), 0);
}

#[tokio::test]
async fn delivery_facade_rejects_invalid_email_before_provider() {
    let mailer = LocalMailer::new();
    let email = Email::new().from("noreply@example.com");

    let error = deliver_with(&email, &mailer).await.unwrap_err();

    assert!(matches!(error, MailError::MissingField("to")));
    assert_eq!(mailer.email_count(), 0);
}
