//! SendGrid adapter tests.
//!
//! Ported from Swoosh's sendgrid_test.exs

use missive::providers::SendGridMailer;
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
    ResponseTemplate::new(200)
        .insert_header("X-Message-Id", "123-xyz")
        .set_body_json(json!({"message": "success"}))
}

// ============================================================================
// Basic Delivery Tests
// ============================================================================

#[tokio::test]
async fn successful_delivery_returns_ok() {
    let server = MockServer::start().await;
    let mailer = SendGridMailer::new("SG.test-api-key").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/mail/send"))
        .and(header("Authorization", "Bearer SG.test-api-key"))
        .and(header("Content-Type", "application/json"))
        .and(body_json(json!({
            "from": {"email": "tony.stark@example.com"},
            "personalizations": [{"to": [{"email": "steve.rogers@example.com"}]}],
            "content": [
                {"type": "text/plain", "value": "Hello"},
                {"type": "text/html", "value": "<h1>Hello</h1>"}
            ],
            "subject": "Hello, Avengers!"
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_ok());
    let delivery = result.unwrap();
    assert_eq!(delivery.message_id, "123-xyz");
}

#[tokio::test]
async fn text_only_delivery_returns_ok() {
    let server = MockServer::start().await;
    let mailer = SendGridMailer::new("SG.test-api-key").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .text_body("Hello");

    Mock::given(method("POST"))
        .and(path("/mail/send"))
        .and(body_json(json!({
            "from": {"email": "tony.stark@example.com"},
            "personalizations": [{"to": [{"email": "steve.rogers@example.com"}]}],
            "content": [{"type": "text/plain", "value": "Hello"}],
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
    let mailer = SendGridMailer::new("SG.test-api-key").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>");

    Mock::given(method("POST"))
        .and(path("/mail/send"))
        .and(body_json(json!({
            "from": {"email": "tony.stark@example.com"},
            "personalizations": [{"to": [{"email": "steve.rogers@example.com"}]}],
            "content": [{"type": "text/html", "value": "<h1>Hello</h1>"}],
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
    let mailer = SendGridMailer::new("SG.test-api-key").base_url(server.uri());

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
        .and(path("/mail/send"))
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
    let mailer = SendGridMailer::new("SG.test-api-key").base_url(server.uri());

    let email = Email::new()
        .from(("T Stark", "tony.stark@example.com"))
        .to(("Steve Rogers", "steve.rogers@example.com"))
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>")
        .text_body("Hello")
        .provider_option("template_id", "Welcome");

    Mock::given(method("POST"))
        .and(path("/mail/send"))
        .and(body_json(json!({
            "from": {"name": "T Stark", "email": "tony.stark@example.com"},
            "personalizations": [
                {"to": [{"name": "Steve Rogers", "email": "steve.rogers@example.com"}]}
            ],
            "content": [
                {"type": "text/plain", "value": "Hello"},
                {"type": "text/html", "value": "<h1>Hello</h1>"}
            ],
            "subject": "Hello, Avengers!",
            "template_id": "Welcome"
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_dynamic_template_data_returns_ok() {
    let server = MockServer::start().await;
    let mailer = SendGridMailer::new("SG.test-api-key").base_url(server.uri());

    let email = Email::new()
        .from(("T Stark", "tony.stark@example.com"))
        .to(("Steve Rogers", "steve.rogers@example.com"))
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>")
        .text_body("Hello")
        .provider_option("dynamic_template_data", json!({"name": "Steve Rogers"}));

    Mock::given(method("POST"))
        .and(path("/mail/send"))
        .and(body_json(json!({
            "from": {"name": "T Stark", "email": "tony.stark@example.com"},
            "personalizations": [
                {
                    "to": [{"name": "Steve Rogers", "email": "steve.rogers@example.com"}],
                    "dynamic_template_data": {"name": "Steve Rogers"}
                }
            ],
            "content": [
                {"type": "text/plain", "value": "Hello"},
                {"type": "text/html", "value": "<h1>Hello</h1>"}
            ],
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
async fn deliver_with_categories_returns_ok() {
    let server = MockServer::start().await;
    let mailer = SendGridMailer::new("SG.test-api-key").base_url(server.uri());

    let email = Email::new()
        .from(("T Stark", "tony.stark@example.com"))
        .to(("Steve Rogers", "steve.rogers@example.com"))
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>")
        .text_body("Hello")
        .provider_option("categories", json!(["welcome"]));

    Mock::given(method("POST"))
        .and(path("/mail/send"))
        .and(body_json(json!({
            "from": {"name": "T Stark", "email": "tony.stark@example.com"},
            "categories": ["welcome"],
            "personalizations": [
                {"to": [{"name": "Steve Rogers", "email": "steve.rogers@example.com"}]}
            ],
            "content": [
                {"type": "text/plain", "value": "Hello"},
                {"type": "text/html", "value": "<h1>Hello</h1>"}
            ],
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
async fn deliver_with_429_response() {
    let server = MockServer::start().await;
    let mailer = SendGridMailer::new("SG.test-api-key").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/mail/send"))
        .respond_with(ResponseTemplate::new(429).set_body_json(json!({
            "errors": [{"field": null, "message": "too many requests"}]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("too many requests"));
}

#[tokio::test]
async fn deliver_with_400_response() {
    let server = MockServer::start().await;
    let mailer = SendGridMailer::new("SG.test-api-key").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/mail/send"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "errors": [{"field": "identifier1", "message": "error message explained"}]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("error message explained"));
}

#[tokio::test]
async fn deliver_with_500_response() {
    let server = MockServer::start().await;
    let mailer = SendGridMailer::new("SG.test-api-key").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/mail/send"))
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
    let mailer = SendGridMailer::new("SG.test-api-key").base_url(server.uri());

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
    let mailer = SendGridMailer::new("SG.test-api-key").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .subject("Hello!")
        .text_body("Hi");

    let result = mailer.deliver(&email).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("to"));
}

// ============================================================================
// Additional Provider Options Tests
// ============================================================================

#[tokio::test]
async fn deliver_with_custom_args_returns_ok() {
    let server = MockServer::start().await;
    let mailer = SendGridMailer::new("SG.test-api-key").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .text_body("Hello")
        .provider_option(
            "custom_args",
            json!({
                "my_var": {"my_message_id": 123},
                "my_other_var": {"my_other_id": 1}
            }),
        );

    Mock::given(method("POST"))
        .and(path("/mail/send"))
        .and(body_json(json!({
            "from": {"email": "tony.stark@example.com"},
            "personalizations": [{
                "to": [{"email": "steve.rogers@example.com"}],
                "custom_args": {
                    "my_var": {"my_message_id": 123},
                    "my_other_var": {"my_other_id": 1}
                }
            }],
            "content": [{"type": "text/plain", "value": "Hello"}],
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
async fn deliver_with_substitutions_returns_ok() {
    let server = MockServer::start().await;
    let mailer = SendGridMailer::new("SG.test-api-key").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, -name-!")
        .text_body("Hello -name-")
        .provider_option("substitutions", json!({"-name-": "Steve"}));

    Mock::given(method("POST"))
        .and(path("/mail/send"))
        .and(body_json(json!({
            "from": {"email": "tony.stark@example.com"},
            "personalizations": [{
                "to": [{"email": "steve.rogers@example.com"}],
                "substitutions": {"-name-": "Steve"}
            }],
            "content": [{"type": "text/plain", "value": "Hello -name-"}],
            "subject": "Hello, -name-!"
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_asm_returns_ok() {
    let server = MockServer::start().await;
    let mailer = SendGridMailer::new("SG.test-api-key").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello!")
        .text_body("Hello")
        .provider_option(
            "asm",
            json!({
                "group_id": 1,
                "groups_to_display": [1, 2, 3]
            }),
        );

    Mock::given(method("POST"))
        .and(path("/mail/send"))
        .and(body_json(json!({
            "from": {"email": "tony.stark@example.com"},
            "personalizations": [{"to": [{"email": "steve.rogers@example.com"}]}],
            "content": [{"type": "text/plain", "value": "Hello"}],
            "subject": "Hello!",
            "asm": {"group_id": 1, "groups_to_display": [1, 2, 3]}
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_mail_settings_sandbox_mode_returns_ok() {
    let server = MockServer::start().await;
    let mailer = SendGridMailer::new("SG.test-api-key").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello!")
        .text_body("Hello")
        .provider_option("mail_settings", json!({"sandbox_mode": {"enable": true}}));

    Mock::given(method("POST"))
        .and(path("/mail/send"))
        .and(body_json(json!({
            "from": {"email": "tony.stark@example.com"},
            "personalizations": [{"to": [{"email": "steve.rogers@example.com"}]}],
            "content": [{"type": "text/plain", "value": "Hello"}],
            "subject": "Hello!",
            "mail_settings": {"sandbox_mode": {"enable": true}}
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_tracking_settings_returns_ok() {
    let server = MockServer::start().await;
    let mailer = SendGridMailer::new("SG.test-api-key").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello!")
        .text_body("Hello")
        .provider_option(
            "tracking_settings",
            json!({"subscription_tracking": {"enable": false}}),
        );

    Mock::given(method("POST"))
        .and(path("/mail/send"))
        .and(body_json(json!({
            "from": {"email": "tony.stark@example.com"},
            "personalizations": [{"to": [{"email": "steve.rogers@example.com"}]}],
            "content": [{"type": "text/plain", "value": "Hello"}],
            "subject": "Hello!",
            "tracking_settings": {"subscription_tracking": {"enable": false}}
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_scheduling_returns_ok() {
    let server = MockServer::start().await;
    let mailer = SendGridMailer::new("SG.test-api-key").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello!")
        .text_body("Hello")
        .provider_option("batch_id", "batch-123")
        .provider_option("send_at", 1617260400);

    Mock::given(method("POST"))
        .and(path("/mail/send"))
        .and(body_json(json!({
            "from": {"email": "tony.stark@example.com"},
            "personalizations": [{"to": [{"email": "steve.rogers@example.com"}]}],
            "content": [{"type": "text/plain", "value": "Hello"}],
            "subject": "Hello!",
            "batch_id": "batch-123",
            "send_at": 1617260400
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_ip_pool_name_returns_ok() {
    let server = MockServer::start().await;
    let mailer = SendGridMailer::new("SG.test-api-key").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello!")
        .text_body("Hello")
        .provider_option("ip_pool_name", "my-pool");

    Mock::given(method("POST"))
        .and(path("/mail/send"))
        .and(body_json(json!({
            "from": {"email": "tony.stark@example.com"},
            "personalizations": [{"to": [{"email": "steve.rogers@example.com"}]}],
            "content": [{"type": "text/plain", "value": "Hello"}],
            "subject": "Hello!",
            "ip_pool_name": "my-pool"
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
    let mailer = SendGridMailer::new("SG.test-api-key").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello!")
        .text_body("Hello")
        .header("X-Custom-Header", "CustomValue");

    Mock::given(method("POST"))
        .and(path("/mail/send"))
        .and(body_json(json!({
            "from": {"email": "tony.stark@example.com"},
            "personalizations": [{"to": [{"email": "steve.rogers@example.com"}]}],
            "content": [{"type": "text/plain", "value": "Hello"}],
            "subject": "Hello!",
            "headers": {"X-Custom-Header": "CustomValue"}
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_multiple_reply_to_returns_ok() {
    let server = MockServer::start().await;
    let mailer = SendGridMailer::new("SG.test-api-key").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .reply_to("reply1@example.com")
        .reply_to("reply2@example.com")
        .subject("Hello!")
        .text_body("Hello");

    Mock::given(method("POST"))
        .and(path("/mail/send"))
        .and(body_json(json!({
            "from": {"email": "tony.stark@example.com"},
            "personalizations": [{"to": [{"email": "steve.rogers@example.com"}]}],
            "content": [{"type": "text/plain", "value": "Hello"}],
            "subject": "Hello!",
            "reply_to_list": [
                {"email": "reply1@example.com"},
                {"email": "reply2@example.com"}
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
async fn deliver_with_custom_personalizations_returns_ok() {
    let server = MockServer::start().await;
    let mailer = SendGridMailer::new("SG.test-api-key").base_url(server.uri());

    // When using custom personalizations, no `to` field is required on the email
    // since the personalizations contain their own recipients.
    let email = Email::new()
        .from("tony.stark@example.com")
        .subject("Hello!")
        .text_body("Hello")
        .provider_option(
            "personalizations",
            json!([
                {"to": [{"email": "user1@example.com"}], "subject": "Custom Subject 1"},
                {"to": [{"email": "user2@example.com"}], "subject": "Custom Subject 2"}
            ]),
        );

    // Accept any POST to /mail/send
    Mock::given(method("POST"))
        .and(path("/mail/send"))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
}

// ============================================================================
// Provider Name Test
// ============================================================================

#[test]
fn provider_name_returns_sendgrid() {
    let mailer = SendGridMailer::new("SG.test-api-key");
    assert_eq!(mailer.provider_name(), "sendgrid");
}
