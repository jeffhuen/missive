//! Preview UI tests.
//!
//! Tests for the mailbox preview functionality, inspired by Swoosh's MailboxPreviewTest.
//!
//! Run with: cargo test --features dev --test preview_test

use std::sync::Arc;

use http_body_util::BodyExt;
use missive::{Attachment, Email, MemoryStorage, Storage};
use tower::ServiceExt;

// Re-use axum types from missive's dependency
use missive::preview::reexports::*;

/// Create test storage with sample emails.
fn create_test_storage() -> Arc<MemoryStorage> {
    let storage = MemoryStorage::shared();

    // Email 1: Full-featured email with all fields
    let email1 = Email::new()
        .subject("Peace, love, not war")
        .from(("Admin", "admin@avengers.org"))
        .reply_to("maria.hill@avengers.org")
        .to("random@villain.me")
        .cc("ironman@avengers.org")
        .cc(("Thor", "thor@avengers.org"))
        .bcc("thanos@villain.me")
        .bcc(("Bob", "hahaha@minions.org"))
        .text_body("Lorem ipsum dolor sit amet")
        .html_body("<p>Lorem ipsum dolor sit amet</p>")
        .header("X-Magic-Number", "7")
        .provider_option(
            "template_model",
            serde_json::json!({"name": "Steve", "email": "steve@avengers.com"}),
        )
        .attachment(
            Attachment::from_bytes("file.png", b"fake png data".to_vec()).content_type("image/png"),
        );

    // Email 2: Minimal email with emoji in subject
    let email2 = Email::new()
        .subject("Avengers Assemble! 🦸‍♂️")
        .from("noreply@shield.gov")
        .to("avengers@shield.gov")
        .text_body("Lorem ipsum dolor sit amet")
        .html_body("<p>Lorem ipsum dolor sit amet</p>");

    storage.push(email1);
    storage.push(email2);

    storage
}

/// Create empty storage.
fn create_empty_storage() -> Arc<MemoryStorage> {
    MemoryStorage::shared()
}

// ============================================================================
// JSON API Tests
// ============================================================================

#[tokio::test]
async fn test_json_renders_emails() {
    let storage = create_test_storage();
    let app = missive::preview::mailbox_router(storage);

    let response = app
        .oneshot(Request::builder().uri("/json").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.contains("application/json"));

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Check structure
    assert!(json.get("data").is_some());
    let data = json["data"].as_array().unwrap();
    assert_eq!(data.len(), 2);

    // Emails are returned newest-first, so find by subject
    let email1 = data
        .iter()
        .find(|e| e["subject"] == "Peace, love, not war")
        .unwrap();
    assert_eq!(email1["from"], "\"Admin\" <admin@avengers.org>");
    assert_eq!(email1["reply_to"], "maria.hill@avengers.org");

    // Check to/cc/bcc arrays
    let to = email1["to"].as_array().unwrap();
    assert_eq!(to.len(), 1);
    assert_eq!(to[0], "random@villain.me");

    let cc = email1["cc"].as_array().unwrap();
    assert_eq!(cc.len(), 2);

    let bcc = email1["bcc"].as_array().unwrap();
    assert_eq!(bcc.len(), 2);

    // Check headers
    assert_eq!(email1["headers"]["X-Magic-Number"], "7");

    // Check provider_options
    let provider_options = email1["provider_options"].as_array().unwrap();
    assert_eq!(provider_options.len(), 1);
    assert_eq!(provider_options[0]["key"], "template_model");

    // Check attachments
    let attachments = email1["attachments"].as_array().unwrap();
    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0]["filename"], "file.png");
    assert_eq!(attachments[0]["content_type"], "image/png");
    assert_eq!(attachments[0]["type"], "attachment");

    // Check second email (with emoji)
    let email2 = data
        .iter()
        .find(|e| e["subject"] == "Avengers Assemble! 🦸‍♂️")
        .unwrap();
    assert!(email2["bcc"].as_array().unwrap().is_empty());
    assert!(email2["cc"].as_array().unwrap().is_empty());
    assert!(email2["attachments"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_json_empty_storage() {
    let storage = create_empty_storage();
    let app = missive::preview::mailbox_router(storage);

    let response = app
        .oneshot(Request::builder().uri("/json").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let data = json["data"].as_array().unwrap();
    assert!(data.is_empty());
}

// ============================================================================
// Index Tests
// ============================================================================

#[tokio::test]
async fn test_index_renders_html() {
    let storage = create_test_storage();
    let app = missive::preview::mailbox_router(storage);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.contains("text/html"));

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).unwrap();

    // Should contain email subjects
    assert!(html.contains("Peace, love, not war"));
    assert!(html.contains("Avengers Assemble!"));
    assert!(html.contains("2 messages"));
}

#[tokio::test]
async fn test_index_empty_state() {
    let storage = create_empty_storage();
    let app = missive::preview::mailbox_router(storage);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).unwrap();

    assert!(html.contains("No emails yet"));
    assert!(html.contains("0 messages"));
}

// ============================================================================
// Single Email View Tests
// ============================================================================

#[tokio::test]
async fn test_view_email_by_id() {
    let storage = create_test_storage();
    let emails = storage.all();
    // Find the email with attachments (Peace, love, not war)
    let target_email = emails
        .iter()
        .find(|e| e.email.subject == "Peace, love, not war")
        .unwrap();
    let target_id = &target_email.id;

    let app = missive::preview::mailbox_router(storage);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/{}", target_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["subject"], "Peace, love, not war");
}

#[tokio::test]
async fn test_view_email_not_found() {
    let storage = create_test_storage();
    let app = missive::preview::mailbox_router(storage);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/nonexistent-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// HTML Body Tests
// ============================================================================

#[tokio::test]
async fn test_email_html_body() {
    let storage = create_test_storage();
    let emails = storage.all();
    let target_email = emails
        .iter()
        .find(|e| e.email.subject == "Peace, love, not war")
        .unwrap();
    let target_id = &target_email.id;

    let app = missive::preview::mailbox_router(storage);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/{}/html", target_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.contains("text/html"));

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).unwrap();

    assert!(html.contains("<p>Lorem ipsum dolor sit amet</p>"));
}

// ============================================================================
// Attachment Tests
// ============================================================================

#[tokio::test]
async fn test_download_attachment() {
    let storage = create_test_storage();
    let emails = storage.all();
    // Find email with attachments
    let target_email = emails
        .iter()
        .find(|e| !e.email.attachments.is_empty())
        .unwrap();
    let target_id = &target_email.id;

    let app = missive::preview::mailbox_router(storage);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/{}/attachments/0", target_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let content_disposition = response
        .headers()
        .get("content-disposition")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(content_disposition, "attachment; filename=\"file.png\"");

    let content_type = response
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(content_type, "image/png");

    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"fake png data");
}

#[tokio::test]
async fn test_download_attachment_not_found() {
    let storage = create_test_storage();
    let emails = storage.all();
    let target_email = emails
        .iter()
        .find(|e| !e.email.attachments.is_empty())
        .unwrap();
    let target_id = &target_email.id;

    let app = missive::preview::mailbox_router(storage);

    // Attachment index out of bounds
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/{}/attachments/99", target_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Clear All Tests
// ============================================================================

#[tokio::test]
async fn test_clear_all() {
    let storage = create_test_storage();
    assert_eq!(storage.all().len(), 2);

    let app = missive::preview::mailbox_router(Arc::clone(&storage));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/clear")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(storage.all().is_empty());
}
