//! Standalone preview server integration tests.
//!
//! Tests the tiny_http-based standalone preview server.
//!
//! Run with: cargo test --features preview --test preview_standalone_test

#![cfg(feature = "preview")]

use std::sync::Arc;
use std::time::Duration;

use missive::preview::PreviewServer;
use missive::{Email, MemoryStorage, Storage};

/// Create test storage with sample emails.
fn create_test_storage() -> Arc<MemoryStorage> {
    let storage = MemoryStorage::shared();

    let email = Email::new()
        .subject("Test Email")
        .from("sender@example.com")
        .to("recipient@example.com")
        .text_body("Hello, world!")
        .html_body("<p>Hello, world!</p>");

    storage.push(email);
    storage
}

/// Find an available port for testing.
fn get_test_addr() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    format!("127.0.0.1:{}", port)
}

#[test]
fn test_server_starts_and_responds() {
    let storage = create_test_storage();
    let addr = get_test_addr();

    let server = PreviewServer::new(&addr, storage).expect("Failed to create server");
    server.spawn();

    // Give the server a moment to start
    std::thread::sleep(Duration::from_millis(50));

    // Test index route
    let response = ureq::get(&format!("http://{}/", addr)).call();
    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.status(), 200);
    assert!(response.content_type().contains("text/html"));
}

#[test]
fn test_json_endpoint() {
    let storage = create_test_storage();
    let addr = get_test_addr();

    PreviewServer::new(&addr, storage).unwrap().spawn();
    std::thread::sleep(Duration::from_millis(50));

    let response = ureq::get(&format!("http://{}/json", addr))
        .call()
        .expect("Request failed");

    assert_eq!(response.status(), 200);
    assert!(response.content_type().contains("application/json"));

    let body: serde_json::Value = response.into_json().unwrap();
    assert!(body["data"].is_array());
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
}

#[test]
fn test_view_single_email() {
    let storage = create_test_storage();
    let addr = get_test_addr();

    // Get the email ID
    let emails = storage.all();
    let email_id = &emails[0].id;

    PreviewServer::new(&addr, Arc::clone(&storage))
        .unwrap()
        .spawn();
    std::thread::sleep(Duration::from_millis(50));

    let response = ureq::get(&format!("http://{}/{}", addr, email_id))
        .call()
        .expect("Request failed");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.into_json().unwrap();
    assert_eq!(body["subject"], "Test Email");
}

#[test]
fn test_email_html_body() {
    let storage = create_test_storage();
    let addr = get_test_addr();

    let emails = storage.all();
    let email_id = &emails[0].id;

    PreviewServer::new(&addr, Arc::clone(&storage))
        .unwrap()
        .spawn();
    std::thread::sleep(Duration::from_millis(50));

    let response = ureq::get(&format!("http://{}/{}/html", addr, email_id))
        .call()
        .expect("Request failed");

    assert_eq!(response.status(), 200);
    assert!(response.content_type().contains("text/html"));

    let body = response.into_string().unwrap();
    assert!(body.contains("<p>Hello, world!</p>"));
}

#[test]
fn test_not_found() {
    let storage = create_test_storage();
    let addr = get_test_addr();

    PreviewServer::new(&addr, storage).unwrap().spawn();
    std::thread::sleep(Duration::from_millis(50));

    let response = ureq::get(&format!(
        "http://{}/00000000-0000-0000-0000-000000000000",
        addr
    ))
    .call();

    assert!(response.is_err());
    let err = response.unwrap_err();
    assert!(matches!(err, ureq::Error::Status(404, _)));
}

#[test]
fn test_clear_emails() {
    let storage = create_test_storage();
    let addr = get_test_addr();

    assert_eq!(storage.count(), 1);

    PreviewServer::new(&addr, Arc::clone(&storage))
        .unwrap()
        .spawn();
    std::thread::sleep(Duration::from_millis(50));

    let response = ureq::post(&format!("http://{}/clear", addr))
        .call()
        .expect("Request failed");

    assert_eq!(response.status(), 204);
    assert_eq!(storage.count(), 0);
}
