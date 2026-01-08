//! Unsent adapter tests.
//!
//! Unsent is a simple email API similar to Resend.

use missive::providers::UnsentMailer;
use missive::{Email, Mailer};
use serde_json::json;
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
        "emailId": "049b9217-30b5-4f61-a8e3-4d2d12f9f5a7"
    }))
}

// ============================================================================
// Basic Delivery Tests
// ============================================================================

#[tokio::test]
async fn successful_delivery_returns_ok() {
    let server = MockServer::start().await;
    let mailer = UnsentMailer::new("unsent_123456789").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/emails"))
        .and(header("Authorization", "Bearer unsent_123456789"))
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
    let mailer = UnsentMailer::new("unsent_123456789").base_url(server.uri());

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
    let mailer = UnsentMailer::new("unsent_123456789").base_url(server.uri());

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
    let mailer = UnsentMailer::new("unsent_123456789").base_url(server.uri());

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
// Error Response Tests
// ============================================================================

#[tokio::test]
async fn deliver_with_400_response() {
    let server = MockServer::start().await;
    let mailer = UnsentMailer::new("unsent_123456789").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/emails"))
        .respond_with(
            ResponseTemplate::new(400).set_body_string("Missing required field: 'to'"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Missing required field"));
}

#[tokio::test]
async fn deliver_with_500_response() {
    let server = MockServer::start().await;
    let mailer = UnsentMailer::new("unsent_123456789").base_url(server.uri());

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
    let mailer = UnsentMailer::new("unsent_123456789").base_url(server.uri());

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
    let mailer = UnsentMailer::new("unsent_123456789").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .subject("Hello!")
        .text_body("Hi");

    let result = mailer.deliver(&email).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("to"));
}

// ============================================================================
// Provider Name Test
// ============================================================================

#[test]
fn provider_name_returns_unsent() {
    let mailer = UnsentMailer::new("unsent_123456789");
    assert_eq!(mailer.provider_name(), "unsent");
}
