//! Mailgun adapter tests.
//!
//! Ported from Swoosh's mailgun_test.exs

use missive::providers::MailgunMailer;
use missive::{Email, Mailer};
use serde_json::json;
use wiremock::matchers::{header, method, path};
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
}

fn success_response() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "message": "Queued. Thank you.",
        "id": "<20111114174239.25659.5817@samples.mailgun.org>"
    }))
}

// ============================================================================
// Basic Delivery Tests
// ============================================================================

#[tokio::test]
async fn successful_delivery_returns_ok() {
    let server = MockServer::start().await;
    let mailer = MailgunMailer::new("fake-api-key", "avengers.com").base_url(server.uri());

    // Mailgun uses Basic auth with "api:key" format
    let expected_auth = format!(
        "Basic {}",
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            "api:fake-api-key"
        )
    );

    Mock::given(method("POST"))
        .and(path("/avengers.com/messages"))
        .and(header("Authorization", expected_auth.as_str()))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_ok());
    let delivery = result.unwrap();
    assert_eq!(
        delivery.message_id,
        "<20111114174239.25659.5817@samples.mailgun.org>"
    );
}

#[tokio::test]
async fn deliver_with_all_fields_returns_ok() {
    let server = MockServer::start().await;
    let mailer = MailgunMailer::new("fake-api-key", "avengers.com").base_url(server.uri());

    let email = Email::new()
        .from(("T Stark", "tony.stark@example.com"))
        .to(("Steve Rogers", "steve.rogers@example.com"))
        .to("wasp.avengers@example.com")
        .reply_to("office.avengers@example.com")
        .cc(("Bruce Banner", "hulk.smash@example.com"))
        .cc("thor.odinson@example.com")
        .bcc(("Clinton Francis Barton", "hawk.eye@example.com"))
        .bcc("beast.avengers@example.com")
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>")
        .text_body("Hello");

    Mock::given(method("POST"))
        .and(path("/avengers.com/messages"))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
    let delivery = result.unwrap();
    assert_eq!(
        delivery.message_id,
        "<20111114174239.25659.5817@samples.mailgun.org>"
    );
}

// ============================================================================
// Provider Options Tests
// ============================================================================

#[tokio::test]
async fn deliver_with_custom_vars_returns_ok() {
    let server = MockServer::start().await;
    let mailer = MailgunMailer::new("fake-api-key", "avengers.com").base_url(server.uri());

    let custom_vars = json!({
        "my_var": [{"my_message_id": 123}],
        "my_other_var": {"my_other_id": 1, "stuff": 2}
    });

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>")
        .provider_option("custom_vars", custom_vars);

    Mock::given(method("POST"))
        .and(path("/avengers.com/messages"))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_sending_options_returns_ok() {
    let server = MockServer::start().await;
    let mailer = MailgunMailer::new("fake-api-key", "avengers.com").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>")
        .provider_option(
            "sending_options",
            json!({"dkim": "yes", "tracking": "no"}),
        );

    Mock::given(method("POST"))
        .and(path("/avengers.com/messages"))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_template_options_returns_ok() {
    let server = MockServer::start().await;
    let mailer = MailgunMailer::new("fake-api-key", "avengers.com").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>")
        .provider_option("template_name", "avengers-templates")
        .provider_option(
            "template_options",
            json!({
                "version": "initial",
                "text": "yes",
                "variables": {"a": 1}
            }),
        );

    Mock::given(method("POST"))
        .and(path("/avengers.com/messages"))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_recipient_vars_returns_ok() {
    let server = MockServer::start().await;
    let mailer = MailgunMailer::new("fake-api-key", "avengers.com").base_url(server.uri());

    let recipient_vars = json!({
        "steve.rogers@example.com": {"var1": 123},
        "juan.diaz@example.com": {"var1": 456}
    });

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .to("juan.diaz@example.com")
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>")
        .provider_option("recipient_vars", recipient_vars);

    Mock::given(method("POST"))
        .and(path("/avengers.com/messages"))
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
    let mailer = MailgunMailer::new("fake-api-key", "avengers.com").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>")
        .provider_option("tags", json!(["worldwide-peace", "unity"]));

    Mock::given(method("POST"))
        .and(path("/avengers.com/messages"))
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
    let mailer = MailgunMailer::new("fake-api-key", "avengers.com").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>")
        .header("In-Reply-To", "<1234@example.com>")
        .header("X-Accept-Language", "en")
        .header("X-Mailer", "missive");

    Mock::given(method("POST"))
        .and(path("/avengers.com/messages"))
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
async fn deliver_with_401_response() {
    let server = MockServer::start().await;
    let mailer = MailgunMailer::new("fake-api-key", "avengers.com").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/avengers.com/messages"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Forbidden"))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Forbidden") || err.to_string().contains("401"));
}

#[tokio::test]
async fn deliver_with_500_response() {
    let server = MockServer::start().await;
    let mailer = MailgunMailer::new("fake-api-key", "avengers.com").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/avengers.com/messages"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "errors": ["The provided authorization grant is invalid, expired, or revoked"],
            "message": "error"
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
    let mailer = MailgunMailer::new("fake-api-key", "avengers.com").base_url(server.uri());

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
    let mailer = MailgunMailer::new("fake-api-key", "avengers.com").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .subject("Hello!")
        .text_body("Hi");

    let result = mailer.deliver(&email).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("to"));
}

// ============================================================================
// EU Base URL Test
// ============================================================================

#[tokio::test]
async fn deliver_with_eu_base_url_returns_ok() {
    let server = MockServer::start().await;
    // Simulate EU endpoint by using custom base_url
    let mailer = MailgunMailer::new("fake-api-key", "avengers.eu").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/avengers.eu/messages"))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_ok());
}

// ============================================================================
// Provider Name Test
// ============================================================================

#[test]
fn provider_name_returns_mailgun() {
    let mailer = MailgunMailer::new("fake-api-key", "avengers.com");
    assert_eq!(mailer.provider_name(), "mailgun");
}
