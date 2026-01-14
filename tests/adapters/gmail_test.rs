//! Gmail adapter tests.
//!
//! Ported from Swoosh's gmail_test.exs

use missive::providers::GmailMailer;
use missive::{Attachment, Email, Mailer};
use serde_json::json;
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ============================================================================
// Helper Functions
// ============================================================================

fn valid_email() -> Email {
    Email::new()
        .from("steve.rogers@example.com")
        .to("tony.stark@example.com")
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>")
}

fn success_response() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "id": "msg-12345",
        "threadId": "thread-67890",
        "labelIds": ["SENT"]
    }))
}

// ============================================================================
// Basic Delivery Tests
// ============================================================================

#[tokio::test]
async fn successful_delivery_returns_ok() {
    let server = MockServer::start().await;
    let mailer = GmailMailer::new("test_access_token").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/"))
        .and(header("Content-Type", "message/rfc822"))
        .and(header("Authorization", "Bearer test_access_token"))
        // Check RFC 2822 message contains expected fields
        .and(body_string_contains("Subject: Hello, Avengers!"))
        .and(body_string_contains("To: tony.stark@example.com"))
        .and(body_string_contains("From: steve.rogers@example.com"))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_ok());
    let delivery = result.unwrap();
    assert_eq!(delivery.message_id, "msg-12345");
}

#[tokio::test]
async fn text_only_delivery_returns_ok() {
    let server = MockServer::start().await;
    let mailer = GmailMailer::new("test_access_token").base_url(server.uri());

    let email = Email::new()
        .from("steve.rogers@example.com")
        .to("tony.stark@example.com")
        .subject("Hello, Avengers!")
        .text_body("Hello");

    Mock::given(method("POST"))
        .and(path("/"))
        .and(header("Content-Type", "message/rfc822"))
        .and(body_string_contains("Subject: Hello, Avengers!"))
        .and(body_string_contains("Hello"))
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
    let mailer = GmailMailer::new("test_access_token").base_url(server.uri());

    let email = Email::new()
        .from(("T Stark", "tony.stark@example.com"))
        .to("wasp.avengers@example.com")
        .to(("Steve Rogers", "steve.rogers@example.com"))
        .cc(("Bruce Banner", "hulk.smash@example.com"))
        .cc("thor.odinson@example.com")
        .bcc(("Clinton Francis Barton", "hawk.eye@example.com"))
        .bcc("beast.avengers@example.com")
        .reply_to("iron.stark@example.com")
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>")
        .text_body("Hello");

    Mock::given(method("POST"))
        .and(path("/"))
        .and(header("Content-Type", "message/rfc822"))
        // Check for Subject
        .and(body_string_contains("Subject: Hello, Avengers!"))
        // Check for From with name
        .and(body_string_contains("T Stark"))
        // Check for To addresses
        .and(body_string_contains("wasp.avengers@example.com"))
        .and(body_string_contains("steve.rogers@example.com"))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

// ============================================================================
// Attachment Tests
// ============================================================================

#[tokio::test]
async fn deliver_with_attachment_returns_ok() {
    let server = MockServer::start().await;
    let mailer = GmailMailer::new("test_access_token").base_url(server.uri());

    let attachment = Attachment::from_bytes("test.txt", b"Hello World".to_vec());

    let email = Email::new()
        .from("steve.rogers@example.com")
        .to("tony.stark@example.com")
        .subject("With Attachment")
        .text_body("See attached")
        .attachment(attachment);

    Mock::given(method("POST"))
        .and(path("/"))
        .and(header("Content-Type", "message/rfc822"))
        .and(body_string_contains("Subject: With Attachment"))
        .and(body_string_contains("test.txt"))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

// ============================================================================
// Error Response Tests
// ============================================================================

#[tokio::test]
async fn deliver_with_unauthorized_error() {
    let server = MockServer::start().await;
    let mailer = GmailMailer::new("invalid_token").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": {
                "code": 401,
                "message": "Invalid Credentials",
                "status": "UNAUTHENTICATED"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("401") || err.contains("Invalid"));
}

#[tokio::test]
async fn deliver_with_500_response() {
    let server = MockServer::start().await;
    let mailer = GmailMailer::new("test_access_token").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "error": {
                "code": 500,
                "message": "Internal Server Error"
            }
        })))
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
    let mailer = GmailMailer::new("test_access_token").base_url(server.uri());

    let email = Email::new()
        .to("tony.stark@example.com")
        .subject("Hello!")
        .text_body("Hi");

    let result = mailer.deliver(&email).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("from"));
}

#[tokio::test]
async fn deliver_without_to_returns_error() {
    let server = MockServer::start().await;
    let mailer = GmailMailer::new("test_access_token").base_url(server.uri());

    let email = Email::new()
        .from("steve.rogers@example.com")
        .subject("Hello!")
        .text_body("Hi");

    let result = mailer.deliver(&email).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("to"));
}

// ============================================================================
// Response Data Tests
// ============================================================================

#[tokio::test]
async fn delivery_result_contains_thread_id() {
    let server = MockServer::start().await;
    let mailer = GmailMailer::new("test_access_token").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg-abc123",
            "threadId": "thread-xyz789",
            "labelIds": ["SENT", "IMPORTANT"]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_ok());
    let delivery = result.unwrap();
    assert_eq!(delivery.message_id, "msg-abc123");

    // Check provider response contains thread info
    let response = delivery.provider_response.unwrap();
    assert_eq!(response["threadId"], "thread-xyz789");
    assert_eq!(response["labelIds"], json!(["SENT", "IMPORTANT"]));
}

// ============================================================================
// Provider Name Test
// ============================================================================

#[test]
fn provider_name_returns_gmail() {
    let mailer = GmailMailer::new("test_access_token");
    assert_eq!(mailer.provider_name(), "gmail");
}
