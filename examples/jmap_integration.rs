//! JMAP integration test against a local Stalwart server.
//!
//! # Setup
//!
//! See `docs/jmap-testing.md` for full setup instructions.
//!
//! Quick start:
//! ```bash
//! # Start Stalwart (see docs for full config)
//! docker run -d --name stalwart-test -p 8080:8080 stalwartlabs/stalwart:latest
//!
//! # Run this example
//! cargo run --example jmap_integration --features jmap
//! ```

use missive::providers::JmapMailer;
use missive::{Email, Mailer};

#[tokio::main]
async fn main() {
    println!("JMAP Integration Test");
    println!("=====================\n");

    // Configuration - adjust these if your setup differs
    let jmap_url = std::env::var("JMAP_URL")
        .unwrap_or_else(|_| "http://localhost:8080/jmap/session".to_string());
    let username = std::env::var("JMAP_USERNAME").unwrap_or_else(|_| "demo".to_string());
    let password = std::env::var("JMAP_PASSWORD").unwrap_or_else(|_| "demopass".to_string());
    let from_email = std::env::var("JMAP_FROM").unwrap_or_else(|_| "demo@lvh.me".to_string());
    let to_email = std::env::var("JMAP_TO").unwrap_or_else(|_| "demo@lvh.me".to_string());

    println!("URL:      {}", jmap_url);
    println!("Username: {}", username);
    println!("From:     {}", from_email);
    println!("To:       {}\n", to_email);

    let mailer = JmapMailer::new(&jmap_url)
        .credentials(&username, &password)
        .build();

    let email = Email::new()
        .from(&*from_email)
        .to(&*to_email)
        .subject("Test from missive JMAP adapter!")
        .text_body("Hello from the Rust JMAP provider!\n\nThis email was sent using missive's JMAP adapter.");

    println!("Sending email...\n");

    match mailer.deliver(&email).await {
        Ok(result) => {
            println!("✅ Success!");
            println!("   Message ID: {}", result.message_id);
            println!("   Provider:   {}", mailer.provider_name());
        }
        Err(e) => {
            println!("❌ Error: {}", e);
            println!("\nTroubleshooting:");
            println!("  1. Is Stalwart running? docker ps | grep stalwart");
            println!("  2. Is the config correct? See docs/jmap-testing.md");
            println!("  3. Try: curl -u {}:{} {}", username, password, jmap_url);
            std::process::exit(1);
        }
    }
}
