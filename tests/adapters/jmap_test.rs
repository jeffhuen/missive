//! JMAP adapter tests.

use missive::providers::JmapMailer;
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
        .text_body("Hello")
}

fn session_response(api_url: &str) -> serde_json::Value {
    json!({
        "capabilities": {
            "urn:ietf:params:jmap:core": {},
            "urn:ietf:params:jmap:mail": {},
            "urn:ietf:params:jmap:submission": {}
        },
        "accounts": {
            "u123456": {
                "name": "test@example.com",
                "isPersonal": true,
                "isReadOnly": false,
                "accountCapabilities": {
                    "urn:ietf:params:jmap:core": {},
                    "urn:ietf:params:jmap:mail": {},
                    "urn:ietf:params:jmap:submission": {}
                }
            }
        },
        "primaryAccounts": {
            "urn:ietf:params:jmap:mail": "u123456",
            "urn:ietf:params:jmap:submission": "u123456"
        },
        "username": "test@example.com",
        "apiUrl": api_url,
        "downloadUrl": format!("{}/download/{{accountId}}/{{blobId}}/{{name}}", api_url),
        "uploadUrl": format!("{}/upload/{{accountId}}", api_url),
        "eventSourceUrl": format!("{}/eventsource", api_url),
        "state": "abc123"
    })
}

fn success_response() -> serde_json::Value {
    json!({
        "methodResponses": [
            ["Email/set", {
                "accountId": "u123456",
                "oldState": "1",
                "newState": "2",
                "created": {
                    "draft": {
                        "id": "M123456",
                        "blobId": "B123456",
                        "threadId": "T123456",
                        "size": 1024
                    }
                }
            }, "e0"],
            ["EmailSubmission/set", {
                "accountId": "u123456",
                "oldState": "1",
                "newState": "2",
                "created": {
                    "sub": {
                        "id": "ES123456",
                        "emailId": "M123456",
                        "threadId": "T123456",
                        "sendAt": "2024-01-01T00:00:00Z"
                    }
                }
            }, "s0"]
        ],
        "sessionState": "abc123"
    })
}

// ============================================================================
// Basic Delivery Tests
// ============================================================================

#[tokio::test]
async fn successful_delivery_returns_ok() {
    let server = MockServer::start().await;
    let api_url = format!("{}/api", server.uri());

    // Use test_session to bypass session discovery
    let mailer = JmapMailer::new(&server.uri())
        .credentials("user", "pass")
        .test_session_with_mailbox(&api_url, "u123456", "mb-drafts")
        .build();

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(header("Content-Type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success_response()))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_ok());
    let delivery = result.unwrap();
    assert_eq!(delivery.message_id, "ES123456");
}

#[tokio::test]
async fn text_only_delivery_returns_ok() {
    let server = MockServer::start().await;
    let api_url = format!("{}/api", server.uri());

    let mailer = JmapMailer::new(&server.uri())
        .credentials("user", "pass")
        .test_session_with_mailbox(&api_url, "u123456", "mb-drafts")
        .build();

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .text_body("Hello");

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success_response()))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn html_only_delivery_returns_ok() {
    let server = MockServer::start().await;
    let api_url = format!("{}/api", server.uri());

    let mailer = JmapMailer::new(&server.uri())
        .credentials("user", "pass")
        .test_session_with_mailbox(&api_url, "u123456", "mb-drafts")
        .build();

    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello, Avengers!")
        .html_body("<h1>Hello</h1>");

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success_response()))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&email).await;
    assert!(result.is_ok());
}

// ============================================================================
// Session Discovery Tests
// ============================================================================

#[tokio::test]
async fn session_discovery_works() {
    let server = MockServer::start().await;
    let api_url = format!("{}/api", server.uri());

    let mailer = JmapMailer::new(&server.uri())
        .credentials("user", "pass")
        .build();

    // Mock session discovery
    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(session_response(&api_url)))
        .expect(1)
        .mount(&server)
        .await;

    // Mock all POST requests to /api (identity fetch, mailbox fetch, email send)
    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "methodResponses": [
                ["Identity/get", {
                    "accountId": "u123456",
                    "state": "1",
                    "list": [
                        {"id": "id-default", "name": "Default", "email": "test@example.com"}
                    ]
                }, "i0"],
                ["Mailbox/get", {
                    "accountId": "u123456",
                    "state": "1",
                    "list": [
                        {"id": "mb-inbox", "name": "Inbox", "role": "inbox"},
                        {"id": "mb-drafts", "name": "Drafts", "role": "drafts"}
                    ]
                }, "m0"],
                ["Email/set", {
                    "accountId": "u123456",
                    "oldState": "1",
                    "newState": "2",
                    "created": {
                        "draft": {
                            "id": "M123456",
                            "blobId": "B123456",
                            "threadId": "T123456",
                            "size": 1024
                        }
                    }
                }, "e0"],
                ["EmailSubmission/set", {
                    "accountId": "u123456",
                    "oldState": "1",
                    "newState": "2",
                    "created": {
                        "sub": {
                            "id": "ES123456",
                            "emailId": "M123456",
                            "threadId": "T123456",
                            "sendAt": "2024-01-01T00:00:00Z"
                        }
                    }
                }, "s0"]
            ]
        })))
        .expect(3) // Identity fetch + Mailbox fetch + Email send
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_ok());
}

// ============================================================================
// All Fields Tests
// ============================================================================

#[tokio::test]
async fn deliver_with_all_fields_returns_ok() {
    let server = MockServer::start().await;
    let api_url = format!("{}/api", server.uri());

    let mailer = JmapMailer::new(&server.uri())
        .credentials("user", "pass")
        .test_session_with_mailbox(&api_url, "u123456", "mb-drafts")
        .build();

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
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success_response()))
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
async fn deliver_with_jmap_error_response() {
    let server = MockServer::start().await;
    let api_url = format!("{}/api", server.uri());

    let mailer = JmapMailer::new(&server.uri())
        .credentials("user", "pass")
        .test_session_with_mailbox(&api_url, "u123456", "mb-drafts")
        .build();

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "methodResponses": [
                ["error", {
                    "type": "invalidArguments",
                    "description": "Missing required field: from"
                }, "e0"]
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("invalidArguments"));
}

#[tokio::test]
async fn deliver_with_email_not_created_error() {
    let server = MockServer::start().await;
    let api_url = format!("{}/api", server.uri());

    let mailer = JmapMailer::new(&server.uri())
        .credentials("user", "pass")
        .test_session_with_mailbox(&api_url, "u123456", "mb-drafts")
        .build();

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "methodResponses": [
                ["Email/set", {
                    "accountId": "u123456",
                    "oldState": "1",
                    "newState": "1",
                    "notCreated": {
                        "draft": {
                            "type": "invalidProperties",
                            "description": "Invalid email format"
                        }
                    }
                }, "e0"]
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("invalidProperties"));
}

#[tokio::test]
async fn deliver_with_http_error() {
    let server = MockServer::start().await;
    let api_url = format!("{}/api", server.uri());

    let mailer = JmapMailer::new(&server.uri())
        .credentials("user", "pass")
        .test_session_with_mailbox(&api_url, "u123456", "mb-drafts")
        .build();

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn deliver_with_401_unauthorized() {
    let server = MockServer::start().await;
    let api_url = format!("{}/api", server.uri());

    let mailer = JmapMailer::new(&server.uri())
        .credentials("user", "pass")
        .test_session_with_mailbox(&api_url, "u123456", "mb-drafts")
        .build();

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
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
    let api_url = format!("{}/api", server.uri());

    let mailer = JmapMailer::new(&server.uri())
        .credentials("user", "pass")
        .test_session_with_mailbox(&api_url, "u123456", "mb-drafts")
        .build();

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
    let api_url = format!("{}/api", server.uri());

    let mailer = JmapMailer::new(&server.uri())
        .credentials("user", "pass")
        .test_session_with_mailbox(&api_url, "u123456", "mb-drafts")
        .build();

    let email = Email::new()
        .from("tony.stark@example.com")
        .subject("Hello!")
        .text_body("Hi");

    let result = mailer.deliver(&email).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("to"));
}

// ============================================================================
// Authentication Tests
// ============================================================================

#[tokio::test]
async fn basic_auth_sends_authorization_header() {
    let server = MockServer::start().await;
    let api_url = format!("{}/api", server.uri());

    let mailer = JmapMailer::new(&server.uri())
        .credentials("testuser", "testpass")
        .test_session_with_mailbox(&api_url, "u123456", "mb-drafts")
        .build();

    // Basic auth header for "testuser:testpass" is "dGVzdHVzZXI6dGVzdHBhc3M="
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(header("Authorization", "Basic dGVzdHVzZXI6dGVzdHBhc3M="))
        .respond_with(ResponseTemplate::new(200).set_body_json(success_response()))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn bearer_auth_sends_authorization_header() {
    let server = MockServer::start().await;
    let api_url = format!("{}/api", server.uri());

    let mailer = JmapMailer::new(&server.uri())
        .bearer_token("my-oauth-token")
        .test_session_with_mailbox(&api_url, "u123456", "mb-drafts")
        .build();

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(header("Authorization", "Bearer my-oauth-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success_response()))
        .expect(1)
        .mount(&server)
        .await;

    let result = mailer.deliver(&valid_email()).await;
    assert!(result.is_ok());
}

// ============================================================================
// Request Body Verification Tests
// ============================================================================

#[tokio::test]
async fn request_body_contains_mailbox_id() {
    use wiremock::matchers::body_string_contains;

    let server = MockServer::start().await;
    let api_url = format!("{}/api", server.uri());

    let mailer = JmapMailer::new(&server.uri())
        .credentials("user", "pass")
        .test_session_with_mailbox(&api_url, "u123456", "mb-drafts")
        .build();

    // Verify the request body contains mailboxIds with our drafts mailbox
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(body_string_contains(r#""mailboxIds":{"mb-drafts":true}"#))
        .respond_with(ResponseTemplate::new(200).set_body_json(success_response()))
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
fn provider_name_returns_jmap() {
    let mailer = JmapMailer::new("https://jmap.example.com")
        .credentials("user", "pass")
        .build();
    assert_eq!(mailer.provider_name(), "jmap");
}
