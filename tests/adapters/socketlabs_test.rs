//! SocketLabs adapter tests.
//!
//! Ported from Swoosh's socket_labs_test.exs

use missive::providers::SocketLabsMailer;
use missive::{Email, Mailer};
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
        "ErrorCode": "Success",
        "MessageResults": [
            {
                "Index": 0,
                "ErrorCode": "Success",
                "AddressResult": [
                    {
                        "EmailAddress": "tony.stark@example.com",
                        "Accepted": true,
                        "ErrorCode": "Success"
                    }
                ]
            }
        ],
        "TransactionReceipt": "receipt-12345"
    }))
}

// ============================================================================
// Basic Delivery Tests
// ============================================================================

#[tokio::test]
async fn successful_delivery_returns_ok() {
    let server = MockServer::start().await;
    let mailer = SocketLabsMailer::new("1234", "some_key").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/email"))
        .and(header("Content-Type", "application/json"))
        .and(body_string_contains("\"serverId\":1234"))
        .and(header("Authorization", "Bearer some_key"))
        .and(body_string_contains("\"Subject\":\"Hello, Avengers!\""))
        .and(body_string_contains("\"emailAddress\":\"tony.stark@example.com\""))
        .and(body_string_contains("\"emailAddress\":\"steve.rogers@example.com\""))
        .and(body_string_contains("\"HtmlBody\":\"<h1>Hello</h1>\""))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_ok());
    let delivery = result.unwrap();
    assert_eq!(delivery.message_id, "receipt-12345");
}

#[tokio::test]
async fn text_only_delivery_returns_ok() {
    let server = MockServer::start().await;
    let mailer = SocketLabsMailer::new("1234", "some_key").base_url(server.uri());

    let email = Email::new()
        .from("steve.rogers@example.com")
        .to("tony.stark@example.com")
        .subject("Hello, Avengers!")
        .text_body("Hello");

    Mock::given(method("POST"))
        .and(path("/email"))
        .and(body_string_contains("\"Subject\":\"Hello, Avengers!\""))
        .and(body_string_contains("\"TextBody\":\"Hello\""))
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
    let mailer = SocketLabsMailer::new("1234", "some_key").base_url(server.uri());

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
        .and(path("/email"))
        .and(body_string_contains("\"Subject\":\"Hello, Avengers!\""))
        .and(body_string_contains("\"HtmlBody\":\"<h1>Hello</h1>\""))
        .and(body_string_contains("\"TextBody\":\"Hello\""))
        // Check for friendlyName in from
        .and(body_string_contains("\"friendlyName\":\"T Stark\""))
        // Check for CC
        .and(body_string_contains("\"CC\""))
        // Check for BCC
        .and(body_string_contains("\"BCC\""))
        // Check for ReplyTo
        .and(body_string_contains("\"ReplyTo\""))
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
async fn deliver_with_api_template_returns_ok() {
    let server = MockServer::start().await;
    let mailer = SocketLabsMailer::new("1234", "some_key").base_url(server.uri());

    let email = Email::new()
        .from(("T Stark", "tony.stark@example.com"))
        .to("avengers@example.com")
        .subject("Hello!")
        .provider_option("api_template", "12345");

    Mock::given(method("POST"))
        .and(path("/email"))
        .and(body_string_contains("\"ApiTemplate\":\"12345\""))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_message_id_returns_ok() {
    let server = MockServer::start().await;
    let mailer = SocketLabsMailer::new("1234", "some_key").base_url(server.uri());

    let email = Email::new()
        .from(("T Stark", "tony.stark@example.com"))
        .to("avengers@example.com")
        .subject("Hello!")
        .provider_option("message_id", "12345");

    Mock::given(method("POST"))
        .and(path("/email"))
        .and(body_string_contains("\"MessageId\":\"12345\""))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_mailing_id_returns_ok() {
    let server = MockServer::start().await;
    let mailer = SocketLabsMailer::new("1234", "some_key").base_url(server.uri());

    let email = Email::new()
        .from(("T Stark", "tony.stark@example.com"))
        .to("avengers@example.com")
        .subject("Hello!")
        .provider_option("mailing_id", "12345");

    Mock::given(method("POST"))
        .and(path("/email"))
        .and(body_string_contains("\"MailingId\":\"12345\""))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_charset_returns_ok() {
    let server = MockServer::start().await;
    let mailer = SocketLabsMailer::new("1234", "some_key").base_url(server.uri());

    let email = Email::new()
        .from(("T Stark", "tony.stark@example.com"))
        .to("avengers@example.com")
        .subject("Hello!")
        .provider_option("charset", "UTF-8");

    Mock::given(method("POST"))
        .and(path("/email"))
        .and(body_string_contains("\"Charset\":\"UTF-8\""))
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
    let mailer = SocketLabsMailer::new("1234", "some_key").base_url(server.uri());

    let email = Email::new()
        .from(("T Stark", "tony.stark@example.com"))
        .to("avengers@example.com")
        .subject("Hello!")
        .text_body("Hi")
        .header("Header1", "1234567890")
        .header("Header2", "12345");

    Mock::given(method("POST"))
        .and(path("/email"))
        .and(body_string_contains("\"CustomHeaders\""))
        .and(body_string_contains("\"Name\":\"Header1\""))
        .and(body_string_contains("\"Value\":\"1234567890\""))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_amp_body_returns_ok() {
    let server = MockServer::start().await;
    let mailer = SocketLabsMailer::new("1234", "some_key").base_url(server.uri());

    let email = Email::new()
        .from(("T Stark", "tony.stark@example.com"))
        .to("avengers@example.com")
        .subject("Hello!")
        .html_body("<html>Fallback</html>")
        .provider_option("amp_body", "<html amp4email><body>Dynamic AMP content</body></html>");

    Mock::given(method("POST"))
        .and(path("/email"))
        .and(body_string_contains("\"AmpBody\":\"<html amp4email>"))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_merge_data_returns_ok() {
    let server = MockServer::start().await;
    let mailer = SocketLabsMailer::new("1234", "some_key").base_url(server.uri());

    let email = Email::new()
        .from(("T Stark", "tony.stark@example.com"))
        .to("avengers@example.com")
        .subject("Hello!")
        .provider_option(
            "merge_data",
            json!({
                "PerMessage": [
                    {"per_message1": "value1", "per_message2": "value2"}
                ],
                "Global": {
                    "global1": "value1",
                    "global2": "value2"
                }
            }),
        );

    Mock::given(method("POST"))
        .and(path("/email"))
        .and(body_string_contains("\"MergeData\""))
        .and(body_string_contains("\"Global\""))
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
async fn deliver_with_error_response() {
    let server = MockServer::start().await;
    let mailer = SocketLabsMailer::new("1234", "some_key").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/email"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ErrorCode": "AuthenticationFailed",
            "MessageResults": [],
            "TransactionReceipt": null
        })))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("AuthenticationFailed"));
}

#[tokio::test]
async fn deliver_with_500_response() {
    let server = MockServer::start().await;
    let mailer = SocketLabsMailer::new("1234", "some_key").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/email"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "ErrorCode": "InternalError",
            "MessageResults": [],
            "TransactionReceipt": null
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
    let mailer = SocketLabsMailer::new("1234", "some_key").base_url(server.uri());

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
    let mailer = SocketLabsMailer::new("1234", "some_key").base_url(server.uri());

    let email = Email::new()
        .from("steve.rogers@example.com")
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
    let mailer = SocketLabsMailer::new("1234", "some_key");
    let result = mailer.deliver_many(&[]).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn deliver_many_with_multiple_emails_returns_ok() {
    let server = MockServer::start().await;
    let mailer = SocketLabsMailer::new("1234", "some_key").base_url(server.uri());

    let email1 = Email::new()
        .from("steve.rogers@example.com")
        .to("tony.stark@example.com")
        .subject("Hello Tony!")
        .text_body("Hi Tony");

    let email2 = Email::new()
        .from("steve.rogers@example.com")
        .to("natasha.romanova@example.com")
        .subject("Hello Natasha!")
        .text_body("Hi Natasha");

    Mock::given(method("POST"))
        .and(path("/email"))
        .and(body_string_contains("\"serverId\":1234"))
        // Check that both messages are in the request
        .and(body_string_contains("tony.stark@example.com"))
        .and(body_string_contains("natasha.romanova@example.com"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ErrorCode": "Success",
            "MessageResults": [
                {
                    "Index": 0,
                    "ErrorCode": "Success",
                    "AddressResult": [
                        {"EmailAddress": "tony.stark@example.com", "Accepted": true, "ErrorCode": "Success"}
                    ]
                },
                {
                    "Index": 1,
                    "ErrorCode": "Success",
                    "AddressResult": [
                        {"EmailAddress": "natasha.romanova@example.com", "Accepted": true, "ErrorCode": "Success"}
                    ]
                }
            ],
            "TransactionReceipt": "batch-receipt-123"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver_many(&[email1, email2]).await;
    assert!(result.is_ok());
    let results = result.unwrap();
    assert_eq!(results.len(), 2);
}

// ============================================================================
// Provider Name Test
// ============================================================================

#[test]
fn provider_name_returns_socketlabs() {
    let mailer = SocketLabsMailer::new("1234", "some_key");
    assert_eq!(mailer.provider_name(), "socketlabs");
}
