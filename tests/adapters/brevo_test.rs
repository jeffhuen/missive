//! Brevo adapter tests.
//!
//! Ported from Swoosh's brevo_test.exs

use missive::providers::BrevoMailer;
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
        "messageId": "<42.11@relay.example.com>"
    }))
}

// ============================================================================
// Basic Delivery Tests
// ============================================================================

#[tokio::test]
async fn successful_delivery_returns_ok() {
    let server = MockServer::start().await;
    let mailer = BrevoMailer::new("test-api-key").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/smtp/email"))
        .and(header("Api-Key", "test-api-key"))
        .and(header("Content-Type", "application/json"))
        .and(body_json(json!({
            "sender": {"email": "tony.stark@example.com"},
            "to": [{"email": "steve.rogers@example.com"}],
            "htmlContent": "<h1>Hello</h1>",
            "textContent": "Hello",
            "subject": "Hello, Avengers!"
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_ok());
    let delivery = result.unwrap();
    assert_eq!(delivery.message_id, "<42.11@relay.example.com>");
}

#[tokio::test]
async fn text_only_delivery_returns_ok() {
    let server = MockServer::start().await;
    let mailer = BrevoMailer::new("test-api-key").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .text_body("Hello");

    Mock::given(method("POST"))
        .and(path("/smtp/email"))
        .and(body_json(json!({
            "sender": {"email": "tony.stark@example.com"},
            "to": [{"email": "steve.rogers@example.com"}],
            "textContent": "Hello",
            "subject": "Hello, Avengers!"
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
    let mailer = BrevoMailer::new("test-api-key").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>");

    Mock::given(method("POST"))
        .and(path("/smtp/email"))
        .and(body_json(json!({
            "sender": {"email": "tony.stark@example.com"},
            "to": [{"email": "steve.rogers@example.com"}],
            "htmlContent": "<h1>Hello</h1>",
            "subject": "Hello, Avengers!"
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
    let mailer = BrevoMailer::new("test-api-key").base_url(server.uri());

    let email = Email::new()
        .from(("T Stark", "tony.stark@example.com"))
        .to(("Steve Rogers", "steve.rogers@example.com"))
        .reply_to("hulk.smash@example.com")
        .cc("hulk.smash@example.com")
        .cc(("Janet Pym", "wasp.avengers@example.com"))
        .bcc("thor.odinson@example.com")
        .bcc(("Henry McCoy", "beast.avengers@example.com"))
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>")
        .text_body("Hello");

    Mock::given(method("POST"))
        .and(path("/smtp/email"))
        .and(body_json(json!({
            "sender": {"name": "T Stark", "email": "tony.stark@example.com"},
            "replyTo": {"email": "hulk.smash@example.com"},
            "to": [{"name": "Steve Rogers", "email": "steve.rogers@example.com"}],
            "cc": [
                {"email": "hulk.smash@example.com"},
                {"name": "Janet Pym", "email": "wasp.avengers@example.com"}
            ],
            "bcc": [
                {"email": "thor.odinson@example.com"},
                {"name": "Henry McCoy", "email": "beast.avengers@example.com"}
            ],
            "textContent": "Hello",
            "htmlContent": "<h1>Hello</h1>",
            "subject": "Hello, Avengers!"
        })))
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
async fn deliver_with_template_id_returns_ok() {
    let server = MockServer::start().await;
    let mailer = BrevoMailer::new("test-api-key").base_url(server.uri());

    let email = Email::new()
        .from(("T Stark", "tony.stark@example.com"))
        .to(("Steve Rogers", "steve.rogers@example.com"))
        .subject("Hello, Avengers!")
        .provider_option("template_id", 42);

    Mock::given(method("POST"))
        .and(path("/smtp/email"))
        .and(body_json(json!({
            "sender": {"name": "T Stark", "email": "tony.stark@example.com"},
            "to": [{"name": "Steve Rogers", "email": "steve.rogers@example.com"}],
            "subject": "Hello, Avengers!",
            "templateId": 42
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_template_id_and_params_returns_ok() {
    let server = MockServer::start().await;
    let mailer = BrevoMailer::new("test-api-key").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .text_body("Hello")
        .provider_option("template_id", 42)
        .provider_option(
            "params",
            json!({
                "sample_template_param": "sample value",
                "another_one": 99
            }),
        );

    Mock::given(method("POST"))
        .and(path("/smtp/email"))
        .and(body_json(json!({
            "sender": {"email": "tony.stark@example.com"},
            "to": [{"email": "steve.rogers@example.com"}],
            "textContent": "Hello",
            "subject": "Hello, Avengers!",
            "templateId": 42,
            "params": {
                "sample_template_param": "sample value",
                "another_one": 99
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
async fn deliver_with_tags_returns_ok() {
    let server = MockServer::start().await;
    let mailer = BrevoMailer::new("test-api-key").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .text_body("Hello")
        .provider_option("tags", json!(["welcome", "onboarding"]));

    Mock::given(method("POST"))
        .and(path("/smtp/email"))
        .and(body_json(json!({
            "sender": {"email": "tony.stark@example.com"},
            "to": [{"email": "steve.rogers@example.com"}],
            "textContent": "Hello",
            "subject": "Hello, Avengers!",
            "tags": ["welcome", "onboarding"]
        })))
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
async fn deliver_with_429_response() {
    let server = MockServer::start().await;
    let mailer = BrevoMailer::new("test-api-key").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/smtp/email"))
        .respond_with(ResponseTemplate::new(429).set_body_json(json!({
            "code": "too_many_requests",
            "message": "The expected rate limit is exceeded."
        })))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("too_many_requests"));
}

#[tokio::test]
async fn deliver_with_400_response() {
    let server = MockServer::start().await;
    let mailer = BrevoMailer::new("test-api-key").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/smtp/email"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "code": "invalid_parameter",
            "message": "error message explained."
        })))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("invalid_parameter"));
}

#[tokio::test]
async fn deliver_with_500_response() {
    let server = MockServer::start().await;
    let mailer = BrevoMailer::new("test-api-key").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/smtp/email"))
        .respond_with(ResponseTemplate::new(500).set_body_string(""))
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
    let mailer = BrevoMailer::new("test-api-key").base_url(server.uri());

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
    let mailer = BrevoMailer::new("test-api-key").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .subject("Hello!")
        .text_body("Hi");

    let result = mailer.deliver(&email).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("to"));
}

// ============================================================================
// Batch Delivery Tests (deliver_many)
// ============================================================================

#[tokio::test]
async fn deliver_many_with_empty_list_returns_ok() {
    let mailer = BrevoMailer::new("test-api-key");
    let result = mailer.deliver_many(&[]).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn deliver_many_with_two_emails_returns_ok() {
    let server = MockServer::start().await;
    let mailer = BrevoMailer::new("test-api-key").base_url(server.uri());

    let email1 = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Steve!")
        .html_body("<h1>Hello Steve</h1>");

    let email2 = Email::new()
        .from("tony.stark@example.com")
        .to("natasha.romanova@example.com")
        .subject("Hello, Natasha!")
        .html_body("<h1>Hello Natasha</h1>");

    Mock::given(method("POST"))
        .and(path("/smtp/email"))
        .and(body_json(json!({
            "sender": {"email": "tony.stark@example.com"},
            "subject": "Hello, Steve!",
            "htmlContent": "<h1>Hello Steve</h1>",
            "messageVersions": [
                {
                    "to": [{"email": "steve.rogers@example.com"}],
                    "subject": "Hello, Steve!",
                    "htmlContent": "<h1>Hello Steve</h1>"
                },
                {
                    "to": [{"email": "natasha.romanova@example.com"}],
                    "subject": "Hello, Natasha!",
                    "htmlContent": "<h1>Hello Natasha</h1>"
                }
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "messageIds": [
                "<42.11@relay.example.com>",
                "<53.22@relay.example.com>"
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver_many(&[email1, email2]).await;
    assert!(result.is_ok());
    let results = result.unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].message_id, "<42.11@relay.example.com>");
    assert_eq!(results[1].message_id, "<53.22@relay.example.com>");
}

#[tokio::test]
async fn deliver_many_with_400_response() {
    let server = MockServer::start().await;
    let mailer = BrevoMailer::new("test-api-key").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/smtp/email"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "code": "missing_parameter",
            "message": "subject is required"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver_many(&[valid_email()]).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("missing_parameter"));
}

// ============================================================================
// TEMPLATE Sender Tests
// ============================================================================

#[tokio::test]
async fn deliver_with_template_sender_omits_sender() {
    let server = MockServer::start().await;
    let mailer = BrevoMailer::new("test-api-key").base_url(server.uri());

    // When from email is "TEMPLATE", sender should be omitted
    let email = Email::new()
        .from(("", "TEMPLATE"))
        .to("steve.rogers@example.com")
        .provider_option("template_id", 42);

    Mock::given(method("POST"))
        .and(path("/smtp/email"))
        .and(body_json(json!({
            "to": [{"email": "steve.rogers@example.com"}],
            "templateId": 42
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_sender_id_returns_ok() {
    let server = MockServer::start().await;
    let mailer = BrevoMailer::new("test-api-key").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello!")
        .text_body("Hello")
        .provider_option("sender_id", 42);

    Mock::given(method("POST"))
        .and(path("/smtp/email"))
        .and(body_json(json!({
            "sender": {"id": 42, "email": "tony.stark@example.com"},
            "to": [{"email": "steve.rogers@example.com"}],
            "textContent": "Hello",
            "subject": "Hello!"
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

// ============================================================================
// Provider Name Test
// ============================================================================

#[test]
fn provider_name_returns_brevo() {
    let mailer = BrevoMailer::new("test-api-key");
    assert_eq!(mailer.provider_name(), "brevo");
}
