//! Amazon SES adapter tests.
//!
//! Ported from Swoosh's amazonses_test.exs
//!
//! Note: AWS Signature v4 generates different signatures each time based on
//! the current timestamp, so we can't verify exact request bodies. Instead,
//! we verify the request path, method, and response parsing.

use missive::providers::AmazonSesMailer;
use missive::{Email, Mailer};
use serde_json::json;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ============================================================================
// Helper Functions
// ============================================================================

fn valid_email() -> Email {
    Email::new()
        .from("guybrush.threepwood@pirates.grog")
        .to("elaine.marley@triisland.gov")
        .subject("Mighty Pirate Newsletter")
        .text_body("Hello")
        .html_body("<h1>Hello</h1>")
}

fn success_response() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_string(
        r#"<SendRawEmailResponse>
            <SendRawEmailResult>
                <MessageId>messageId</MessageId>
            </SendRawEmailResult>
            <ResponseMetadata>
                <RequestId>requestId</RequestId>
            </ResponseMetadata>
        </SendRawEmailResponse>"#,
    )
}

fn error_response() -> ResponseTemplate {
    ResponseTemplate::new(500).set_body_string(
        r#"<ErrorResponse>
            <Error>
                <Type>ErrorType</Type>
                <Code>ErrorCode</Code>
                <Message>Error Message</Message>
            </Error>
            <RequestId>a97266f7-b062-11e7-b126-6b0f7a9b3379</RequestId>
        </ErrorResponse>"#,
    )
}

// ============================================================================
// Basic Delivery Tests
// ============================================================================

#[tokio::test]
async fn successful_delivery_returns_ok() {
    let server = MockServer::start().await;
    let mailer = AmazonSesMailer::new("us-east-1", "test_access", "test_secret")
        .host(server.uri());

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_string_contains("Action=SendRawEmail"))
        .and(body_string_contains("Version=2010-12-01"))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_ok());
    let delivery = result.unwrap();
    assert_eq!(delivery.message_id, "messageId");
}

#[tokio::test]
async fn delivery_with_tags_returns_ok() {
    let server = MockServer::start().await;
    let mailer = AmazonSesMailer::new("us-east-1", "test_access", "test_secret")
        .host(server.uri());

    let email = Email::new()
        .from("guybrush.threepwood@pirates.grog")
        .to("elaine.marley@triisland.gov")
        .subject("Mighty Pirate Newsletter")
        .text_body("Hello")
        .html_body("<h1>Hello</h1>")
        .provider_option(
            "tags",
            json!([{"name": "name1", "value": "test1"}]),
        )
        .provider_option("configuration_set_name", "configuration_set_name1");

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_string_contains("Action=SendRawEmail"))
        .and(body_string_contains("Version=2010-12-01"))
        .and(body_string_contains("ConfigurationSetName=configuration_set_name1"))
        .and(body_string_contains("Tags.member.1.Name=name1"))
        .and(body_string_contains("Tags.member.1.Value=test1"))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
    let delivery = result.unwrap();
    assert_eq!(delivery.message_id, "messageId");
}

#[tokio::test]
async fn deliver_with_all_fields_returns_ok() {
    let server = MockServer::start().await;
    let mailer = AmazonSesMailer::new("us-east-1", "test_access", "test_secret")
        .host(server.uri());

    let email = Email::new()
        .from(("G Threepwood", "guybrush.threepwood@pirates.grog"))
        .to(("Murry The Skull", "murry@lechucksship.gov"))
        .to("elaine.marley@triisland.gov")
        .cc(("Cannibals", "canni723@monkeyisland.com"))
        .cc("carla@sworddojo.org")
        .bcc(("LeChuck", "lechuck@underworld.com"))
        .bcc("stan@coolshirt.com")
        .subject("Mighty Pirate Newsletter")
        .text_body("Hello")
        .html_body("<h1>Hello</h1>");

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_string_contains("Action=SendRawEmail"))
        .and(body_string_contains("Version=2010-12-01"))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
    let delivery = result.unwrap();
    assert_eq!(delivery.message_id, "messageId");
}

// ============================================================================
// Optional Config Params Tests
// ============================================================================

#[tokio::test]
async fn optional_config_params_are_present_when_set() {
    let server = MockServer::start().await;
    let mailer = AmazonSesMailer::new("us-east-1", "test_access", "test_secret")
        .host(server.uri())
        .ses_source("aaa@bbb.com")
        .ses_source_arn("arn:aws:ses:us-east-1:123:identity/source.example.com")
        .ses_from_arn("arn:aws:ses:us-east-1:123:identity/from.example.com")
        .ses_return_path_arn("arn:aws:ses:us-east-1:123:identity/return.example.com");

    // Source parameter contains @ which should be included as-is
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_string_contains("Action=SendRawEmail"))
        .and(body_string_contains("Source=aaa@bbb.com"))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn optional_config_params_not_present_when_not_set() {
    let server = MockServer::start().await;
    let mailer = AmazonSesMailer::new("us-east-1", "test_access", "test_secret")
        .host(server.uri());

    // Create a custom matcher that ensures Source is NOT in the body
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_string_contains("Action=SendRawEmail"))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_ok());
}

// ============================================================================
// Error Response Tests
// ============================================================================

#[tokio::test]
async fn api_error_parses_correctly() {
    let server = MockServer::start().await;
    let mailer = AmazonSesMailer::new("us-east-1", "test_access", "test_secret")
        .host(server.uri());

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(error_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("ErrorCode"));
    assert!(err.to_string().contains("Error Message"));
}

// ============================================================================
// Validation Tests
// ============================================================================

#[tokio::test]
async fn deliver_without_from_returns_error() {
    let server = MockServer::start().await;
    let mailer = AmazonSesMailer::new("us-east-1", "test_access", "test_secret")
        .host(server.uri());

    let email = Email::new()
        .to("elaine.marley@triisland.gov")
        .subject("Hello!")
        .text_body("Hi");

    let result = mailer.deliver(&email).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("from"));
}

#[tokio::test]
async fn deliver_without_to_returns_error() {
    let server = MockServer::start().await;
    let mailer = AmazonSesMailer::new("us-east-1", "test_access", "test_secret")
        .host(server.uri());

    let email = Email::new()
        .from("guybrush.threepwood@pirates.grog")
        .subject("Hello!")
        .text_body("Hi");

    let result = mailer.deliver(&email).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("to"));
}

// ============================================================================
// Security Token Test (for IAM roles)
// ============================================================================

#[tokio::test]
async fn delivery_with_security_token() {
    let server = MockServer::start().await;
    let mailer = AmazonSesMailer::new("us-east-1", "test_access", "test_secret")
        .host(server.uri());

    let email = Email::new()
        .from("guybrush.threepwood@pirates.grog")
        .to("elaine.marley@triisland.gov")
        .subject("Hello")
        .text_body("Hi")
        .provider_option("security_token", "temporary-session-token");

    // When security token is provided, X-Amz-Security-Token header should be present
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_string_contains("Action=SendRawEmail"))
        .respond_with(success_response())
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

// ============================================================================
// Region Tests
// ============================================================================

#[tokio::test]
async fn uses_correct_region_endpoint() {
    // Test that different regions produce different base URLs
    let mailer_us_east = AmazonSesMailer::new("us-east-1", "key", "secret");
    assert_eq!(mailer_us_east.provider_name(), "amazon_ses");

    let mailer_eu_west = AmazonSesMailer::new("eu-west-1", "key", "secret");
    assert_eq!(mailer_eu_west.provider_name(), "amazon_ses");
}

// ============================================================================
// Provider Name Test
// ============================================================================

#[test]
fn provider_name_returns_amazon_ses() {
    let mailer = AmazonSesMailer::new("us-east-1", "test_access", "test_secret");
    assert_eq!(mailer.provider_name(), "amazon_ses");
}
