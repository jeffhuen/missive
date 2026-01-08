//! Mailjet adapter tests.
//!
//! Ported from Swoosh's mailjet_test.exs

use base64::Engine;
use missive::providers::MailjetMailer;
use missive::{Email, Mailer};
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ============================================================================
// Helper Functions
// ============================================================================

fn valid_email() -> Email {
    Email::new()
        .from("sender@example.com")
        .to("receiver@example.com")
        .subject("Hello, world!")
}

fn success_response() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "Messages": [
            {
                "Status": "success",
                "CustomID": "",
                "To": [
                    {
                        "Email": "receiver@example.com",
                        "MessageUUID": "12345-12345-12345",
                        "MessageID": 123456789,
                        "MessageHref": "https://api.mailjet.com/v3/REST/message/123456789"
                    }
                ],
                "Cc": [],
                "Bcc": []
            }
        ]
    }))
}

// ============================================================================
// Basic Delivery Tests
// ============================================================================

#[tokio::test]
async fn successful_delivery_returns_ok() {
    let server = MockServer::start().await;
    let mailer = MailjetMailer::new("public_key", "private_key").base_url(server.uri());

    let email = valid_email().html_body("<h1>Hello</h1>").text_body("Hello");

    Mock::given(method("POST"))
        .and(path("/send"))
        .and(header("Content-Type", "application/json"))
        .and(body_json(json!({
            "Messages": [
                {
                    "From": {"Email": "sender@example.com", "Name": ""},
                    "To": [{"Email": "receiver@example.com", "Name": ""}],
                    "Subject": "Hello, world!",
                    "TextPart": "Hello",
                    "HTMLPart": "<h1>Hello</h1>"
                }
            ]
        })))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
    let delivery = result.unwrap();
    assert_eq!(delivery.message_id, "123456789");
}

#[tokio::test]
async fn sends_valid_auth_header() {
    let server = MockServer::start().await;
    let mailer = MailjetMailer::new("public_key", "private_key").base_url(server.uri());

    // Base64 of "public_key:private_key"
    let expected_auth = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode("public_key:private_key")
    );

    Mock::given(method("POST"))
        .and(path("/send"))
        .and(header("Authorization", expected_auth.as_str()))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let _ = mailer.deliver(&valid_email().text_body("Hello")).await;
}

// ============================================================================
// Provider Options Tests
// ============================================================================

#[tokio::test]
async fn deliver_with_template_id_and_variables_returns_ok() {
    let server = MockServer::start().await;
    let mailer = MailjetMailer::new("public_key", "private_key").base_url(server.uri());

    let email = valid_email()
        .provider_option("template_id", 123)
        .provider_option("template_error_deliver", true)
        .provider_option("template_error_reporting", "developer@example.com")
        .provider_option(
            "variables",
            json!({
                "firstname": "Pan",
                "lastname": "Michal"
            }),
        );

    Mock::given(method("POST"))
        .and(path("/send"))
        .and(body_json(json!({
            "Messages": [
                {
                    "From": {"Email": "sender@example.com", "Name": ""},
                    "To": [{"Email": "receiver@example.com", "Name": ""}],
                    "Subject": "Hello, world!",
                    "TemplateID": 123,
                    "TemplateLanguage": true,
                    "TemplateErrorDeliver": true,
                    "TemplateErrorReporting": {"Email": "developer@example.com", "Name": ""},
                    "Variables": {"firstname": "Pan", "lastname": "Michal"}
                }
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
async fn deliver_with_custom_id_returns_ok() {
    let server = MockServer::start().await;
    let mailer = MailjetMailer::new("public_key", "private_key").base_url(server.uri());

    let email = valid_email()
        .text_body("Hello")
        .html_body("<h1>Hello</h1>")
        .provider_option("custom_id", "my-great-custom-id");

    Mock::given(method("POST"))
        .and(path("/send"))
        .and(body_json(json!({
            "Messages": [
                {
                    "From": {"Email": "sender@example.com", "Name": ""},
                    "To": [{"Email": "receiver@example.com", "Name": ""}],
                    "Subject": "Hello, world!",
                    "TextPart": "Hello",
                    "HTMLPart": "<h1>Hello</h1>",
                    "CustomID": "my-great-custom-id"
                }
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
async fn deliver_with_binary_event_payload_returns_ok() {
    let server = MockServer::start().await;
    let mailer = MailjetMailer::new("public_key", "private_key").base_url(server.uri());

    let email = valid_email()
        .text_body("Hello")
        .provider_option("event_payload", "Eticket,1234,row,15,seat,B");

    Mock::given(method("POST"))
        .and(path("/send"))
        .and(body_json(json!({
            "Messages": [
                {
                    "From": {"Email": "sender@example.com", "Name": ""},
                    "To": [{"Email": "receiver@example.com", "Name": ""}],
                    "Subject": "Hello, world!",
                    "TextPart": "Hello",
                    "EventPayload": "Eticket,1234,row,15,seat,B"
                }
            ]
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
    let mailer = MailjetMailer::new("public_key", "private_key").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/send"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "Messages": [
                {
                    "Status": "error",
                    "Errors": [
                        {
                            "ErrorIdentifier": "error id",
                            "ErrorCode": "mj-0004",
                            "StatusCode": 400,
                            "ErrorMessage": "Type mismatch. Expected type \"array of emails\".",
                            "ErrorRelatedTo": ["HTMLPart", "TemplateID"]
                        }
                    ]
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email().text_body("Hello")).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Type mismatch"));
}

#[tokio::test]
async fn deliver_with_global_400_error() {
    let server = MockServer::start().await;
    let mailer = MailjetMailer::new("public_key", "private_key").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/send"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "ErrorIdentifier": "error id",
            "ErrorCode": "mj-0002",
            "StatusCode": 400,
            "ErrorMessage": "Malformed JSON, please review the syntax and properties types.",
            "Messages": []
        })))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email().text_body("Hello")).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Malformed JSON"));
}

// ============================================================================
// Validation Tests
// ============================================================================

#[tokio::test]
async fn deliver_without_from_returns_error() {
    let server = MockServer::start().await;
    let mailer = MailjetMailer::new("public_key", "private_key").base_url(server.uri());

    let email = Email::new()
        .to("receiver@example.com")
        .subject("Hello!")
        .text_body("Hi");

    let result = mailer.deliver(&email).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("from"));
}

#[tokio::test]
async fn deliver_without_to_returns_error() {
    let server = MockServer::start().await;
    let mailer = MailjetMailer::new("public_key", "private_key").base_url(server.uri());

    let email = Email::new()
        .from("sender@example.com")
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
    let mailer = MailjetMailer::new("public_key", "private_key");
    let result = mailer.deliver_many(&[]).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn deliver_many_with_two_emails_returns_ok() {
    let server = MockServer::start().await;
    let mailer = MailjetMailer::new("public_key", "private_key").base_url(server.uri());

    let email1 = Email::new()
        .from("sender@example.com")
        .to("receiver1@example.com")
        .subject("Hello 1");

    let email2 = Email::new()
        .from("sender@example.com")
        .to("receiver2@example.com")
        .subject("Hello 2")
        .provider_option("template_id", 123)
        .provider_option("template_error_deliver", true)
        .provider_option("template_error_reporting", "developer@example.com")
        .provider_option(
            "variables",
            json!({
                "firstname": "Pan",
                "lastname": "Michal"
            }),
        );

    Mock::given(method("POST"))
        .and(path("/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "Messages": [
                {
                    "Status": "success",
                    "To": [{"MessageID": 123456789}]
                },
                {
                    "Status": "success",
                    "To": [{"MessageID": 23456789}]
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver_many(&[email1, email2]).await;
    assert!(result.is_ok());
    let results = result.unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].message_id, "123456789");
    assert_eq!(results[1].message_id, "23456789");
}

// ============================================================================
// Provider Name Test
// ============================================================================

#[test]
fn provider_name_returns_mailjet() {
    let mailer = MailjetMailer::new("public_key", "private_key");
    assert_eq!(mailer.provider_name(), "mailjet");
}
