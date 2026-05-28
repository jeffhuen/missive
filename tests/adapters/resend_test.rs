//! Resend adapter tests.
//!
//! Ported from Swoosh's resend_test.exs

use chrono::{TimeZone, Utc};
use missive::providers::{ResendEmailExt, ResendMailer};
use missive::{Attachment, Email, MailError, Mailer, PreparedEmail};
use serde_json::json;
use std::fs;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ============================================================================
// Helper Functions
// ============================================================================

fn valid_email() -> Email {
    Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>")
        .text_body("Hello")
}

fn success_response() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "id": "049b9217-30b5-4f61-a8e3-4d2d12f9f5a7"
    }))
}

// ============================================================================
// Basic Delivery Tests
// ============================================================================

#[tokio::test]
async fn successful_delivery_returns_ok() {
    let server = MockServer::start().await;
    let mailer = ResendMailer::new("re_123456789").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/emails"))
        .and(header("Authorization", "Bearer re_123456789"))
        .and(header("Content-Type", "application/json"))
        .and(body_json(json!({
            "from": "tony.stark@example.com",
            "to": ["steve.rogers@example.com"],
            "subject": "Hello, Avengers!",
            "html": "<h1>Hello</h1>",
            "text": "Hello"
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_ok());
    let delivery = result.unwrap();
    assert_eq!(delivery.message_id, "049b9217-30b5-4f61-a8e3-4d2d12f9f5a7");
}

#[tokio::test]
async fn text_only_delivery_returns_ok() {
    let server = MockServer::start().await;
    let mailer = ResendMailer::new("re_123456789").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .text_body("Hello");

    Mock::given(method("POST"))
        .and(path("/emails"))
        .and(body_json(json!({
            "from": "tony.stark@example.com",
            "to": ["steve.rogers@example.com"],
            "subject": "Hello, Avengers!",
            "text": "Hello"
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn html_only_delivery_returns_ok() {
    let server = MockServer::start().await;
    let mailer = ResendMailer::new("re_123456789").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>");

    Mock::given(method("POST"))
        .and(path("/emails"))
        .and(body_json(json!({
            "from": "tony.stark@example.com",
            "to": ["steve.rogers@example.com"],
            "subject": "Hello, Avengers!",
            "html": "<h1>Hello</h1>"
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

// ============================================================================
// All Fields Tests
// ============================================================================

#[tokio::test]
async fn deliver_with_all_fields_returns_ok() {
    let server = MockServer::start().await;
    let mailer = ResendMailer::new("re_123456789").base_url(server.uri());

    let email = Email::new()
        .from(("T Stark", "tony.stark@example.com"))
        .to(("Steve Rogers", "steve.rogers@example.com"))
        .to("bruce.banner@example.com")
        .reply_to("hulk.smash@example.com")
        .cc("hulk.smash@example.com")
        .cc(("Janet Pym", "wasp.avengers@example.com"))
        .bcc("thor.odinson@example.com")
        .bcc(("Henry McCoy", "beast.avengers@example.com"))
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>")
        .text_body("Hello");

    Mock::given(method("POST"))
        .and(path("/emails"))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

// ============================================================================
// Provider Options Tests
// ============================================================================

#[tokio::test]
async fn deliver_with_tags_returns_ok() {
    let server = MockServer::start().await;
    let mailer = ResendMailer::new("re_123456789").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .text_body("Hello")
        .provider_option(
            "tags",
            json!([
                {"name": "category", "value": "confirm_email"},
                {"name": "user_id", "value": "123"}
            ]),
        );

    Mock::given(method("POST"))
        .and(path("/emails"))
        .and(body_json(json!({
            "from": "tony.stark@example.com",
            "to": ["steve.rogers@example.com"],
            "subject": "Hello, Avengers!",
            "text": "Hello",
            "tags": [
                {"name": "category", "value": "confirm_email"},
                {"name": "user_id", "value": "123"}
            ]
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn typed_resend_options_are_sent() {
    let server = MockServer::start().await;
    let mailer = ResendMailer::new("re_123456789").base_url(server.uri());
    let scheduled_at = Utc.with_ymd_and_hms(2024, 12, 1, 0, 0, 0).unwrap();

    Mock::given(method("POST"))
        .and(path("/emails"))
        .and(header("Idempotency-Key", "welcome-123"))
        .and(body_json(json!({
            "from": "tony.stark@example.com",
            "to": ["steve.rogers@example.com"],
            "subject": "Hello, Avengers!",
            "html": "<h1>Hello</h1>",
            "text": "Hello",
            "tags": [
                {"name": "category", "value": "welcome"},
                {"name": "source", "value": "signup"}
            ],
            "scheduled_at": "2024-12-01T00:00:00+00:00",
            "template": {
                "id": "welcome-template",
                "variables": {
                    "name": "Steve"
                }
            }
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let email = valid_email()
        .resend_tag("category", "welcome")
        .resend_tag("source", "signup")
        .resend_scheduled_at(scheduled_at)
        .resend_idempotency_key("welcome-123")
        .resend_template(json!({
            "id": "welcome-template",
            "variables": {
                "name": "Steve"
            }
        }))
        .unwrap();

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_scheduled_at_returns_ok() {
    let server = MockServer::start().await;
    let mailer = ResendMailer::new("re_123456789").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .text_body("Hello")
        .provider_option("scheduled_at", "2024-08-05T11:52:01.858Z");

    Mock::given(method("POST"))
        .and(path("/emails"))
        .and(body_json(json!({
            "from": "tony.stark@example.com",
            "to": ["steve.rogers@example.com"],
            "subject": "Hello, Avengers!",
            "text": "Hello",
            "scheduled_at": "2024-08-05T11:52:01.858Z"
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_formats_named_and_idn_addresses_for_provider_api() {
    let server = MockServer::start().await;
    let mailer = ResendMailer::new("re_123456789").base_url(server.uri());

    let email = Email::new()
        .from(("Tony, \"Iron\" Stark", "tony@例え.jp"))
        .to(("Steve, Rogers", "steve@例え.jp"))
        .reply_to(("Pepper \"Rescue\" Potts", "pepper@例え.jp"))
        .subject("Hello, Avengers!")
        .text_body("Hello");

    Mock::given(method("POST"))
        .and(path("/emails"))
        .and(body_json(json!({
            "from": "\"Tony, \\\"Iron\\\" Stark\" <tony@xn--r8jz45g.jp>",
            "to": ["\"Steve, Rogers\" <steve@xn--r8jz45g.jp>"],
            "reply_to": "\"Pepper \\\"Rescue\\\" Potts\" <pepper@xn--r8jz45g.jp>",
            "subject": "Hello, Avengers!",
            "text": "Hello"
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_custom_headers_returns_ok() {
    let server = MockServer::start().await;
    let mailer = ResendMailer::new("re_123456789").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .text_body("Hello")
        .header("X-Custom-Header", "CustomValue")
        .header("X-Another-Header", "AnotherValue");

    Mock::given(method("POST"))
        .and(path("/emails"))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_missing_lazy_attachment_returns_attachment_error() {
    let server = MockServer::start().await;
    let mailer = ResendMailer::new("re_123456789").base_url(server.uri());
    let path = std::env::temp_dir().join(format!(
        "missive-resend-missing-attachment-{}.txt",
        std::process::id()
    ));
    fs::write(&path, b"content").unwrap();

    let email = valid_email().attachment(Attachment::from_path_lazy(&path).unwrap());
    fs::remove_file(&path).unwrap();

    let err = mailer.deliver(&email).await.unwrap_err();
    assert!(matches!(err, MailError::AttachmentFileNotFound(_)));
}

#[tokio::test]
async fn deliver_with_lazy_attachment_sends_file_contents() {
    let server = MockServer::start().await;
    let mailer = ResendMailer::new("re_123456789").base_url(server.uri());
    let filename = format!("missive-resend-lazy-attachment-{}.txt", std::process::id());
    let attachment_path = std::env::temp_dir().join(&filename);
    fs::write(&attachment_path, b"assemble").unwrap();

    let email = valid_email().attachment(Attachment::from_path_lazy(&attachment_path).unwrap());

    Mock::given(method("POST"))
        .and(path("/emails"))
        .and(body_json(json!({
            "from": "tony.stark@example.com",
            "to": ["steve.rogers@example.com"],
            "subject": "Hello, Avengers!",
            "html": "<h1>Hello</h1>",
            "text": "Hello",
            "attachments": [{
                "filename": filename,
                "content": "YXNzZW1ibGU=",
                "content_type": "text/plain"
            }]
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    fs::remove_file(&attachment_path).unwrap();

    assert!(result.is_ok());
}

// ============================================================================
// Error Response Tests
// ============================================================================

#[tokio::test]
async fn deliver_with_400_response() {
    let server = MockServer::start().await;
    let mailer = ResendMailer::new("re_123456789").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/emails"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "statusCode": 400,
            "message": "Missing required field: 'to'",
            "name": "validation_error"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Missing required field"));
}

#[tokio::test]
async fn deliver_with_429_response() {
    let server = MockServer::start().await;
    let mailer = ResendMailer::new("re_123456789").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/emails"))
        .respond_with(ResponseTemplate::new(429).set_body_json(json!({
            "statusCode": 429,
            "message": "Too many requests",
            "name": "rate_limit_exceeded"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Too many requests"));
    assert_eq!(err.kind(), "provider_error");
    assert_eq!(err.provider_status(), Some(429));
    match err {
        MailError::ProviderError {
            provider, status, ..
        } => {
            assert_eq!(provider, "resend");
            assert_eq!(status, Some(429));
        }
        other => panic!("expected provider error, got {other:?}"),
    }
}

#[tokio::test]
async fn deliver_with_500_response() {
    let server = MockServer::start().await;
    let mailer = ResendMailer::new("re_123456789").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/emails"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_err());
}

// ============================================================================
// Validation Tests
// ============================================================================

#[tokio::test]
async fn deliver_without_from_returns_error() {
    let server = MockServer::start().await;
    let mailer = ResendMailer::new("re_123456789").base_url(server.uri());

    let email = Email::new()
        .to("steve.rogers@example.com")
        .subject("Hello!")
        .text_body("Hi");

    let result = mailer.deliver(&email).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("from"));
}

#[tokio::test]
async fn deliver_without_to_returns_error() {
    let server = MockServer::start().await;
    let mailer = ResendMailer::new("re_123456789").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .subject("Hello!")
        .text_body("Hi");

    let result = mailer.deliver(&email).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("to"));
}

// ============================================================================
// Batch Delivery Tests (validate_batch)
// ============================================================================

#[test]
fn validate_batch_rejects_scheduled_at() {
    let mailer = ResendMailer::new("re_123456789");

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Test")
        .text_body("Test")
        .provider_option("scheduled_at", "2024-08-05T11:52:01.858Z");

    let email = PreparedEmail::new(email).unwrap();
    let result = mailer.validate_batch(&[email]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("scheduled_at"));
}

#[test]
fn validate_batch_rejects_attachments() {
    let mailer = ResendMailer::new("re_123456789");

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("With attachment")
        .text_body("See attached")
        .attachment(Attachment::from_bytes("file.txt", b"Content".to_vec()));

    let email = PreparedEmail::new(email).unwrap();
    let result = mailer.validate_batch(&[email]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("attachments"));
}

#[test]
fn validate_batch_rejects_if_any_email_has_scheduled_at() {
    let mailer = ResendMailer::new("re_123456789");

    let email1 = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Test 1")
        .text_body("Test");

    let email2 = Email::new()
        .from("tony.stark@example.com")
        .to("natasha.romanova@example.com")
        .subject("Test 2")
        .text_body("Test")
        .provider_option("scheduled_at", "2024-08-05T11:52:01.858Z");

    let email1 = PreparedEmail::new(email1).unwrap();
    let email2 = PreparedEmail::new(email2).unwrap();
    let result = mailer.validate_batch(&[email1, email2]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("scheduled_at"));
}

#[test]
fn validate_batch_rejects_if_any_email_has_attachments() {
    let mailer = ResendMailer::new("re_123456789");

    let email1 = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Test 1")
        .text_body("Test");

    let email2 = Email::new()
        .from("tony.stark@example.com")
        .to("natasha.romanova@example.com")
        .subject("Test 2")
        .text_body("Test")
        .attachment(Attachment::from_bytes("file.txt", b"Content".to_vec()));

    let email1 = PreparedEmail::new(email1).unwrap();
    let email2 = PreparedEmail::new(email2).unwrap();
    let result = mailer.validate_batch(&[email1, email2]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("attachments"));
}

// ============================================================================
// Template Tests
// ============================================================================

#[tokio::test]
async fn deliver_with_template_returns_ok() {
    let server = MockServer::start().await;
    let mailer = ResendMailer::new("re_123456789").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello!")
        .provider_option(
            "template",
            json!({
                "id": "tmpl_123",
                "data": {"name": "Steve"}
            }),
        );

    Mock::given(method("POST"))
        .and(path("/emails"))
        .and(body_json(json!({
            "from": "tony.stark@example.com",
            "to": ["steve.rogers@example.com"],
            "subject": "Hello!",
            "template": {
                "id": "tmpl_123",
                "data": {"name": "Steve"}
            }
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_idempotency_key_sets_header() {
    let server = MockServer::start().await;
    let mailer = ResendMailer::new("re_123456789").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello!")
        .text_body("Hello")
        .provider_option("idempotency_key", "unique-key-123");

    Mock::given(method("POST"))
        .and(path("/emails"))
        .and(header("Idempotency-Key", "unique-key-123"))
        .and(body_json(json!({
            "from": "tony.stark@example.com",
            "to": ["steve.rogers@example.com"],
            "subject": "Hello!",
            "text": "Hello"
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

// ============================================================================
// Batch Delivery Tests (deliver_many)
// ============================================================================

#[tokio::test]
async fn deliver_many_with_empty_list_returns_ok() {
    let mailer = ResendMailer::new("re_123456789");
    let result = mailer.deliver_many(&[]).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn deliver_many_with_two_emails_returns_ok() {
    let server = MockServer::start().await;
    let mailer = ResendMailer::new("re_123456789").base_url(server.uri());

    let email1 = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello Steve!")
        .text_body("Hi Steve");

    let email2 = Email::new()
        .from("tony.stark@example.com")
        .to("natasha.romanova@example.com")
        .subject("Hello Natasha!")
        .text_body("Hi Natasha");

    Mock::given(method("POST"))
        .and(path("/emails/batch"))
        .and(body_json(json!([
            {
                "from": "tony.stark@example.com",
                "to": ["steve.rogers@example.com"],
                "subject": "Hello Steve!",
                "text": "Hi Steve"
            },
            {
                "from": "tony.stark@example.com",
                "to": ["natasha.romanova@example.com"],
                "subject": "Hello Natasha!",
                "text": "Hi Natasha"
            }
        ])))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [
                {"id": "msg-id-1"},
                {"id": "msg-id-2"}
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver_many(&[email1, email2]).await;
    assert!(result.is_ok());
    let results = result.unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].message_id, "msg-id-1");
    assert_eq!(results[1].message_id, "msg-id-2");
}

// ============================================================================
// Provider Name Test
// ============================================================================

#[test]
fn provider_name_returns_resend() {
    let mailer = ResendMailer::new("re_123456789");
    assert_eq!(mailer.provider_name(), "resend");
}
