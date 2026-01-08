//! Mailtrap adapter tests.
//!
//! Ported from Swoosh's mailtrap_test.exs

use missive::providers::MailtrapMailer;
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
        "success": true,
        "message_ids": ["0c7fd939-02cf-11ed-88c2-0a58a9feac02"]
    }))
}

// ============================================================================
// Basic Delivery Tests
// ============================================================================

#[tokio::test]
async fn successful_delivery_returns_ok() {
    let server = MockServer::start().await;
    let mailer = MailtrapMailer::new("test-api-key").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/api/send"))
        .and(header("Authorization", "Bearer test-api-key"))
        .and(header("Content-Type", "application/json"))
        .and(body_json(json!({
            "from": {"email": "tony.stark@example.com"},
            "to": [{"email": "steve.rogers@example.com"}],
            "text": "Hello",
            "html": "<h1>Hello</h1>",
            "subject": "Hello, Avengers!"
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_ok());
    let delivery = result.unwrap();
    assert_eq!(delivery.message_id, "0c7fd939-02cf-11ed-88c2-0a58a9feac02");
}

#[tokio::test]
async fn text_only_delivery_returns_ok() {
    let server = MockServer::start().await;
    let mailer = MailtrapMailer::new("test-api-key").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .text_body("Hello");

    Mock::given(method("POST"))
        .and(path("/api/send"))
        .and(body_json(json!({
            "from": {"email": "tony.stark@example.com"},
            "to": [{"email": "steve.rogers@example.com"}],
            "text": "Hello",
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
    let mailer = MailtrapMailer::new("test-api-key").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>");

    Mock::given(method("POST"))
        .and(path("/api/send"))
        .and(body_json(json!({
            "from": {"email": "tony.stark@example.com"},
            "to": [{"email": "steve.rogers@example.com"}],
            "html": "<h1>Hello</h1>",
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
    let mailer = MailtrapMailer::new("test-api-key").base_url(server.uri());

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
        .and(path("/api/send"))
        .and(body_json(json!({
            "from": {"name": "T Stark", "email": "tony.stark@example.com"},
            "headers": {"Reply-To": "hulk.smash@example.com"},
            "to": [{"name": "Steve Rogers", "email": "steve.rogers@example.com"}],
            "cc": [
                {"email": "hulk.smash@example.com"},
                {"name": "Janet Pym", "email": "wasp.avengers@example.com"}
            ],
            "bcc": [
                {"email": "thor.odinson@example.com"},
                {"name": "Henry McCoy", "email": "beast.avengers@example.com"}
            ],
            "text": "Hello",
            "html": "<h1>Hello</h1>",
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
async fn deliver_with_custom_variables_returns_ok() {
    let server = MockServer::start().await;
    let mailer = MailtrapMailer::new("test-api-key").base_url(server.uri());

    let email = Email::new()
        .from(("T Stark", "tony.stark@example.com"))
        .to(("Steve Rogers", "steve.rogers@example.com"))
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>")
        .text_body("Hello")
        .provider_option(
            "custom_variables",
            json!({
                "my_var": {"my_message_id": 123},
                "my_other_var": {"my_other_id": 1, "stuff": 2}
            }),
        );

    Mock::given(method("POST"))
        .and(path("/api/send"))
        .and(body_json(json!({
            "from": {"name": "T Stark", "email": "tony.stark@example.com"},
            "to": [{"name": "Steve Rogers", "email": "steve.rogers@example.com"}],
            "text": "Hello",
            "html": "<h1>Hello</h1>",
            "subject": "Hello, Avengers!",
            "custom_variables": {
                "my_var": {"my_message_id": 123},
                "my_other_var": {"my_other_id": 1, "stuff": 2}
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
async fn deliver_with_category_returns_ok() {
    let server = MockServer::start().await;
    let mailer = MailtrapMailer::new("test-api-key").base_url(server.uri());

    let email = Email::new()
        .from(("T Stark", "tony.stark@example.com"))
        .to(("Steve Rogers", "steve.rogers@example.com"))
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>")
        .text_body("Hello")
        .provider_option("category", "alert");

    Mock::given(method("POST"))
        .and(path("/api/send"))
        .and(body_json(json!({
            "from": {"name": "T Stark", "email": "tony.stark@example.com"},
            "to": [{"name": "Steve Rogers", "email": "steve.rogers@example.com"}],
            "text": "Hello",
            "html": "<h1>Hello</h1>",
            "subject": "Hello, Avengers!",
            "category": "alert"
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
    let mailer = MailtrapMailer::new("test-api-key").base_url(server.uri());

    let email = Email::new()
        .from(("T Stark", "tony.stark@example.com"))
        .to(("Steve Rogers", "steve.rogers@example.com"))
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>")
        .text_body("Hello")
        .header("In-Reply-To", "<1234@example.com>")
        .header("X-Accept-Language", "en")
        .header("X-Mailer", "missive");

    Mock::given(method("POST"))
        .and(path("/api/send"))
        .and(body_json(json!({
            "from": {"name": "T Stark", "email": "tony.stark@example.com"},
            "to": [{"name": "Steve Rogers", "email": "steve.rogers@example.com"}],
            "text": "Hello",
            "html": "<h1>Hello</h1>",
            "subject": "Hello, Avengers!",
            "headers": {
                "In-Reply-To": "<1234@example.com>",
                "X-Accept-Language": "en",
                "X-Mailer": "missive"
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
async fn deliver_with_reply_to_and_custom_headers_returns_ok() {
    let server = MockServer::start().await;
    let mailer = MailtrapMailer::new("test-api-key").base_url(server.uri());

    let email = Email::new()
        .from(("T Stark", "tony.stark@example.com"))
        .to(("Steve Rogers", "steve.rogers@example.com"))
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>")
        .text_body("Hello")
        .reply_to("hulk.smash@example.com")
        .header("In-Reply-To", "<1234@example.com>")
        .header("X-Accept-Language", "en")
        .header("X-Mailer", "missive");

    Mock::given(method("POST"))
        .and(path("/api/send"))
        .and(body_json(json!({
            "from": {"name": "T Stark", "email": "tony.stark@example.com"},
            "to": [{"name": "Steve Rogers", "email": "steve.rogers@example.com"}],
            "text": "Hello",
            "html": "<h1>Hello</h1>",
            "subject": "Hello, Avengers!",
            "headers": {
                "Reply-To": "hulk.smash@example.com",
                "In-Reply-To": "<1234@example.com>",
                "X-Accept-Language": "en",
                "X-Mailer": "missive"
            }
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

// ============================================================================
// Sandbox Mode Tests
// ============================================================================

#[tokio::test]
async fn deliver_with_sandbox_config_returns_ok() {
    let server = MockServer::start().await;
    let mailer = MailtrapMailer::new("test-api-key")
        .base_url(server.uri())
        .sandbox_inbox_id("11111");

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>")
        .text_body("Hello");

    Mock::given(method("POST"))
        .and(path("/api/send/11111"))
        .and(body_json(json!({
            "from": {"email": "tony.stark@example.com"},
            "to": [{"email": "steve.rogers@example.com"}],
            "text": "Hello",
            "html": "<h1>Hello</h1>",
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
// Error Response Tests
// ============================================================================

#[tokio::test]
async fn deliver_with_400_response() {
    let server = MockServer::start().await;
    let mailer = MailtrapMailer::new("test-api-key").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/api/send"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "errors": ["bla bla"],
            "success": false
        })))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("bla bla"));
}

#[tokio::test]
async fn deliver_with_500_response() {
    let server = MockServer::start().await;
    let mailer = MailtrapMailer::new("test-api-key").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/api/send"))
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
    let mailer = MailtrapMailer::new("test-api-key").base_url(server.uri());

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
    let mailer = MailtrapMailer::new("test-api-key").base_url(server.uri());

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
fn provider_name_returns_mailtrap() {
    let mailer = MailtrapMailer::new("test-api-key");
    assert_eq!(mailer.provider_name(), "mailtrap");
}
