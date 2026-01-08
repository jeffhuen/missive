//! Postmark adapter tests.
//!
//! Ported from Swoosh's postmark_test.exs

use missive::providers::PostmarkMailer;
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
        "ErrorCode": 0,
        "Message": "OK",
        "MessageID": "b7bc2f4a-e38e-4336-af7d-e6c392c2f817",
        "SubmittedAt": "2010-11-26T12:01:05.1794748-05:00",
        "To": "tony.stark@example.com"
    }))
}

// ============================================================================
// Basic Delivery Tests
// ============================================================================

#[tokio::test]
async fn successful_delivery_returns_ok() {
    let server = MockServer::start().await;
    let mailer = PostmarkMailer::new("jarvis").base_url(server.uri());

    // Postmark uses PascalCase for keys
    Mock::given(method("POST"))
        .and(path("/email"))
        .and(header("X-Postmark-Server-Token", "jarvis"))
        .and(header("Content-Type", "application/json"))
        .and(body_string_contains("\"Subject\":\"Hello, Avengers!\""))
        .and(body_string_contains("\"To\":\"tony.stark@example.com\""))
        .and(body_string_contains(
            "\"From\":\"steve.rogers@example.com\"",
        ))
        .and(body_string_contains("\"HtmlBody\":\"<h1>Hello</h1>\""))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_ok());
    let delivery = result.unwrap();
    assert_eq!(delivery.message_id, "b7bc2f4a-e38e-4336-af7d-e6c392c2f817");
}

#[tokio::test]
async fn text_only_delivery_returns_ok() {
    let server = MockServer::start().await;
    let mailer = PostmarkMailer::new("jarvis").base_url(server.uri());

    let email = Email::new()
        .from("steve.rogers@example.com")
        .to("tony.stark@example.com")
        .subject("Hello, Avengers!")
        .text_body("Hello");

    Mock::given(method("POST"))
        .and(path("/email"))
        .and(body_string_contains("\"Subject\":\"Hello, Avengers!\""))
        .and(body_string_contains("\"To\":\"tony.stark@example.com\""))
        .and(body_string_contains(
            "\"From\":\"steve.rogers@example.com\"",
        ))
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
    let mailer = PostmarkMailer::new("jarvis").base_url(server.uri());

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
        .and(body_string_contains(
            "\"ReplyTo\":\"iron.stark@example.com\"",
        ))
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
async fn deliver_with_tag_returns_ok() {
    let server = MockServer::start().await;
    let mailer = PostmarkMailer::new("jarvis").base_url(server.uri());

    let email = Email::new()
        .from("steve.rogers@example.com")
        .to("tony.stark@example.com")
        .subject("Hello!")
        .text_body("Hello")
        .provider_option("tag", "top-secret");

    Mock::given(method("POST"))
        .and(path("/email"))
        .and(body_string_contains("\"Subject\":\"Hello!\""))
        .and(body_string_contains("\"Tag\":\"top-secret\""))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_track_opens_returns_ok() {
    let server = MockServer::start().await;
    let mailer = PostmarkMailer::new("jarvis").base_url(server.uri());

    let email = Email::new()
        .from("steve.rogers@example.com")
        .to("tony.stark@example.com")
        .subject("Hello!")
        .text_body("Hello")
        .provider_option("track_opens", false);

    Mock::given(method("POST"))
        .and(path("/email"))
        .and(body_string_contains("\"Subject\":\"Hello!\""))
        .and(body_string_contains("\"TrackOpens\":false"))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_track_links_returns_ok() {
    let server = MockServer::start().await;
    let mailer = PostmarkMailer::new("jarvis").base_url(server.uri());

    let email = Email::new()
        .from("steve.rogers@example.com")
        .to("tony.stark@example.com")
        .subject("Hello!")
        .text_body("Hello")
        .provider_option("track_links", "HtmlAndText");

    Mock::given(method("POST"))
        .and(path("/email"))
        .and(body_string_contains("\"Subject\":\"Hello!\""))
        .and(body_string_contains("\"TrackLinks\":\"HtmlAndText\""))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_message_stream_returns_ok() {
    let server = MockServer::start().await;
    let mailer = PostmarkMailer::new("jarvis").base_url(server.uri());

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("avengers@example.com")
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>")
        .text_body("Hello")
        .provider_option("message_stream", "test-stream-name");

    Mock::given(method("POST"))
        .and(path("/email"))
        .and(body_string_contains("\"Subject\":\"Hello, Avengers!\""))
        .and(body_string_contains(
            "\"MessageStream\":\"test-stream-name\"",
        ))
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
async fn deliver_with_422_response() {
    let server = MockServer::start().await;
    let mailer = PostmarkMailer::new("jarvis").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/email"))
        .respond_with(ResponseTemplate::new(422).set_body_json(json!({
            "ErrorCode": 400,
            "Message": "The provided authorization grant is invalid, expired, or revoked"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn deliver_with_500_response() {
    let server = MockServer::start().await;
    let mailer = PostmarkMailer::new("jarvis").base_url(server.uri());

    Mock::given(method("POST"))
        .and(path("/email"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "ErrorCode": 500,
            "Message": "Internal error"
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
    let mailer = PostmarkMailer::new("jarvis").base_url(server.uri());

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
    let mailer = PostmarkMailer::new("jarvis").base_url(server.uri());

    let email = Email::new()
        .from("steve.rogers@example.com")
        .subject("Hello!")
        .text_body("Hi");

    let result = mailer.deliver(&email).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("to"));
}

// ============================================================================
// Template Tests
// ============================================================================

#[tokio::test]
async fn deliver_with_template_id_returns_ok() {
    let server = MockServer::start().await;
    let mailer = PostmarkMailer::new("jarvis").base_url(server.uri());

    let email = Email::new()
        .from("steve.rogers@example.com")
        .to("tony.stark@example.com")
        .subject("Hello!")
        .provider_option("template_id", 1234)
        .provider_option(
            "template_model",
            json!({
                "name": "Tony",
                "company": "Stark Industries"
            }),
        );

    Mock::given(method("POST"))
        .and(path("/email/withTemplate"))
        .and(body_string_contains("\"TemplateId\":1234"))
        .and(body_string_contains("\"TemplateModel\""))
        .and(body_string_contains("\"name\":\"Tony\""))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_template_alias_returns_ok() {
    let server = MockServer::start().await;
    let mailer = PostmarkMailer::new("jarvis").base_url(server.uri());

    let email = Email::new()
        .from("steve.rogers@example.com")
        .to("tony.stark@example.com")
        .subject("Hello!")
        .provider_option("template_alias", "welcome-email")
        .provider_option(
            "template_model",
            json!({
                "name": "Tony"
            }),
        );

    Mock::given(method("POST"))
        .and(path("/email/withTemplate"))
        .and(body_string_contains("\"TemplateAlias\":\"welcome-email\""))
        .and(body_string_contains("\"TemplateModel\""))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

// ============================================================================
// Additional Provider Options Tests
// ============================================================================

#[tokio::test]
async fn deliver_with_metadata_returns_ok() {
    let server = MockServer::start().await;
    let mailer = PostmarkMailer::new("jarvis").base_url(server.uri());

    let email = Email::new()
        .from("steve.rogers@example.com")
        .to("tony.stark@example.com")
        .subject("Hello!")
        .text_body("Hello")
        .provider_option(
            "metadata",
            json!({
                "user_id": "123",
                "campaign": "welcome"
            }),
        );

    Mock::given(method("POST"))
        .and(path("/email"))
        .and(body_string_contains("\"Metadata\""))
        .and(body_string_contains("\"user_id\":\"123\""))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deliver_with_inline_css_returns_ok() {
    let server = MockServer::start().await;
    let mailer = PostmarkMailer::new("jarvis").base_url(server.uri());

    let email = Email::new()
        .from("steve.rogers@example.com")
        .to("tony.stark@example.com")
        .subject("Hello!")
        .html_body("<style>.test{color:red}</style><p class='test'>Hello</p>")
        .provider_option("inline_css", true);

    Mock::given(method("POST"))
        .and(path("/email"))
        .and(body_string_contains("\"InlineCss\":true"))
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
    let mailer = PostmarkMailer::new("jarvis").base_url(server.uri());

    let email = Email::new()
        .from("steve.rogers@example.com")
        .to("tony.stark@example.com")
        .subject("Hello!")
        .text_body("Hello")
        .header("X-Custom-Header", "CustomValue");

    Mock::given(method("POST"))
        .and(path("/email"))
        .and(body_string_contains("\"Headers\""))
        .and(body_string_contains("\"Name\":\"X-Custom-Header\""))
        .and(body_string_contains("\"Value\":\"CustomValue\""))
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
    let mailer = PostmarkMailer::new("jarvis");
    let result = mailer.deliver_many(&[]).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn deliver_many_with_regular_emails_returns_ok() {
    let server = MockServer::start().await;
    let mailer = PostmarkMailer::new("jarvis").base_url(server.uri());

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
        .and(path("/email/batch"))
        .and(header("X-Postmark-Server-Token", "jarvis"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "ErrorCode": 0,
                "Message": "OK",
                "MessageID": "msg-id-1",
                "SubmittedAt": "2010-11-26T12:01:05Z",
                "To": "tony.stark@example.com"
            },
            {
                "ErrorCode": 0,
                "Message": "OK",
                "MessageID": "msg-id-2",
                "SubmittedAt": "2010-11-26T12:01:05Z",
                "To": "natasha.romanova@example.com"
            }
        ])))
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

#[tokio::test]
async fn deliver_many_with_template_emails_returns_ok() {
    let server = MockServer::start().await;
    let mailer = PostmarkMailer::new("jarvis").base_url(server.uri());

    let email1 = Email::new()
        .from("steve.rogers@example.com")
        .to("tony.stark@example.com")
        .subject("Hello!")
        .provider_option("template_id", 1234)
        .provider_option("template_model", json!({"name": "Tony"}));

    let email2 = Email::new()
        .from("steve.rogers@example.com")
        .to("natasha.romanova@example.com")
        .subject("Hello!")
        .provider_option("template_id", 1234)
        .provider_option("template_model", json!({"name": "Natasha"}));

    Mock::given(method("POST"))
        .and(path("/email/batchWithTemplates"))
        .and(header("X-Postmark-Server-Token", "jarvis"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "ErrorCode": 0,
                "Message": "OK",
                "MessageID": "template-msg-1",
                "SubmittedAt": "2010-11-26T12:01:05Z",
                "To": "tony.stark@example.com"
            },
            {
                "ErrorCode": 0,
                "Message": "OK",
                "MessageID": "template-msg-2",
                "SubmittedAt": "2010-11-26T12:01:05Z",
                "To": "natasha.romanova@example.com"
            }
        ])))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver_many(&[email1, email2]).await;
    assert!(result.is_ok());
    let results = result.unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].message_id, "template-msg-1");
    assert_eq!(results[1].message_id, "template-msg-2");
}

#[tokio::test]
async fn deliver_many_with_partial_failure_returns_ok() {
    let server = MockServer::start().await;
    let mailer = PostmarkMailer::new("jarvis").base_url(server.uri());

    let email1 = Email::new()
        .from("steve.rogers@example.com")
        .to("tony.stark@example.com")
        .subject("Hello!")
        .text_body("Hi");

    let email2 = Email::new()
        .from("steve.rogers@example.com")
        .to("invalid@example.com")
        .subject("Hello!")
        .text_body("Hi");

    Mock::given(method("POST"))
        .and(path("/email/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "ErrorCode": 0,
                "Message": "OK",
                "MessageID": "msg-id-1",
                "SubmittedAt": "2010-11-26T12:01:05Z",
                "To": "tony.stark@example.com"
            },
            {
                "ErrorCode": 406,
                "Message": "Inactive recipient",
                "MessageID": "",
                "SubmittedAt": "",
                "To": "invalid@example.com"
            }
        ])))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver_many(&[email1, email2]).await;
    assert!(result.is_ok());
    let results = result.unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].message_id, "msg-id-1");
    // The second result should still be returned (with empty message_id)
}

// ============================================================================
// Provider Name Test
// ============================================================================

#[test]
fn provider_name_returns_postmark() {
    let mailer = PostmarkMailer::new("jarvis");
    assert_eq!(mailer.provider_name(), "postmark");
}
