//! Email struct tests.
//!
//! Ported from Swoosh's email_test.exs

use missive::{Address, Email};
use serde_json::json;

// ============================================================================
// Constructor Tests
// ============================================================================

#[test]
fn new_creates_empty_email() {
    let email = Email::new();
    assert!(email.from.is_none());
    assert!(email.to.is_empty());
    assert!(email.subject.is_empty());
    assert!(email.html_body.is_none());
    assert!(email.text_body.is_none());
}

// ============================================================================
// From Tests
// ============================================================================

#[test]
fn from_sets_sender_from_string() {
    let email = Email::new().from("tony.stark@example.com");
    let from = email.from.unwrap();
    assert_eq!(from.email, "tony.stark@example.com");
    assert!(from.name.is_none());
}

#[test]
fn from_sets_sender_from_tuple() {
    let email = Email::new().from(("Steve Rogers", "steve.rogers@example.com"));
    let from = email.from.unwrap();
    assert_eq!(from.email, "steve.rogers@example.com");
    assert_eq!(from.name.as_deref(), Some("Steve Rogers"));
}

#[test]
fn from_replaces_previous_sender() {
    let email = Email::new()
        .from("tony.stark@example.com")
        .from(("Steve Rogers", "steve.rogers@example.com"));
    let from = email.from.unwrap();
    assert_eq!(from.email, "steve.rogers@example.com");
}

// ============================================================================
// Subject Tests
// ============================================================================

#[test]
fn subject_sets_subject() {
    let email = Email::new().subject("Hello, Avengers!");
    assert_eq!(email.subject, "Hello, Avengers!");
}

#[test]
fn subject_replaces_previous_subject() {
    let email = Email::new()
        .subject("Hello, Avengers!")
        .subject("Welcome, I am Jarvis");
    assert_eq!(email.subject, "Welcome, I am Jarvis");
}

// ============================================================================
// Body Tests
// ============================================================================

#[test]
fn html_body_sets_html_body() {
    let email = Email::new().html_body("<h1>Hello, Avengers!</h1>");
    assert_eq!(email.html_body.as_deref(), Some("<h1>Hello, Avengers!</h1>"));
}

#[test]
fn html_body_replaces_previous_html_body() {
    let email = Email::new()
        .html_body("<h1>Hello, Avengers!</h1>")
        .html_body("<h1>Welcome, I am Jarvis</h1>");
    assert_eq!(
        email.html_body.as_deref(),
        Some("<h1>Welcome, I am Jarvis</h1>")
    );
}

#[test]
fn text_body_sets_text_body() {
    let email = Email::new().text_body("Hello, Avengers!");
    assert_eq!(email.text_body.as_deref(), Some("Hello, Avengers!"));
}

#[test]
fn text_body_replaces_previous_text_body() {
    let email = Email::new()
        .text_body("Hello, Avengers!")
        .text_body("Welcome, I am Jarvis");
    assert_eq!(email.text_body.as_deref(), Some("Welcome, I am Jarvis"));
}

// ============================================================================
// Reply-To Tests
// ============================================================================

#[test]
fn reply_to_sets_reply_to_from_string() {
    let email = Email::new().reply_to("welcome.avengers@example.com");
    assert_eq!(email.reply_to.len(), 1);
    assert_eq!(email.reply_to[0].email, "welcome.avengers@example.com");
}

#[test]
fn reply_to_sets_reply_to_from_tuple() {
    let email = Email::new().reply_to(("Jarvis Assist", "help.jarvis@example.com"));
    assert_eq!(email.reply_to.len(), 1);
    assert_eq!(email.reply_to[0].email, "help.jarvis@example.com");
    assert_eq!(email.reply_to[0].name.as_deref(), Some("Jarvis Assist"));
}

// ============================================================================
// To Tests
// ============================================================================

#[test]
fn to_adds_recipient() {
    let email = Email::new().to("tony.stark@example.com");
    assert_eq!(email.to.len(), 1);
    assert_eq!(email.to[0].email, "tony.stark@example.com");
}

#[test]
fn to_adds_multiple_recipients() {
    let email = Email::new()
        .to("tony.stark@example.com")
        .to(("Steve Rogers", "steve.rogers@example.com"));

    assert_eq!(email.to.len(), 2);
    assert_eq!(email.to[0].email, "tony.stark@example.com");
    assert_eq!(email.to[1].email, "steve.rogers@example.com");
    assert_eq!(email.to[1].name.as_deref(), Some("Steve Rogers"));
}

#[test]
fn to_adds_recipient_with_name() {
    let email = Email::new().to(("Thor Odinson", "thor.odinson@example.com"));
    assert_eq!(email.to[0].email, "thor.odinson@example.com");
    assert_eq!(email.to[0].name.as_deref(), Some("Thor Odinson"));
}

// ============================================================================
// CC Tests
// ============================================================================

#[test]
fn cc_adds_recipient() {
    let email = Email::new().cc("natasha.romanoff@example.com");
    assert_eq!(email.cc.len(), 1);
    assert_eq!(email.cc[0].email, "natasha.romanoff@example.com");
}

#[test]
fn cc_adds_multiple_recipients() {
    let email = Email::new()
        .cc("natasha.romanoff@example.com")
        .cc(("Steve Rogers", "steve.rogers@example.com"));

    assert_eq!(email.cc.len(), 2);
    assert_eq!(email.cc[0].email, "natasha.romanoff@example.com");
    assert_eq!(email.cc[1].email, "steve.rogers@example.com");
}

// ============================================================================
// BCC Tests
// ============================================================================

#[test]
fn bcc_adds_recipient() {
    let email = Email::new().bcc("loki.odinson@example.com");
    assert_eq!(email.bcc.len(), 1);
    assert_eq!(email.bcc[0].email, "loki.odinson@example.com");
}

#[test]
fn bcc_adds_multiple_recipients() {
    let email = Email::new()
        .bcc("loki.odinson@example.com")
        .bcc(("Bruce Banner", "hulk.smash@example.com"));

    assert_eq!(email.bcc.len(), 2);
    assert_eq!(email.bcc[0].email, "loki.odinson@example.com");
    assert_eq!(email.bcc[1].email, "hulk.smash@example.com");
}

// ============================================================================
// Header Tests
// ============================================================================

#[test]
fn header_adds_header() {
    let email = Email::new().header("X-Accept-Language", "en");
    assert_eq!(email.headers.get("X-Accept-Language"), Some(&"en".to_string()));
}

#[test]
fn header_adds_multiple_headers() {
    let email = Email::new()
        .header("X-Accept-Language", "en")
        .header("X-Mailer", "missive");

    assert_eq!(email.headers.get("X-Accept-Language"), Some(&"en".to_string()));
    assert_eq!(email.headers.get("X-Mailer"), Some(&"missive".to_string()));
}

#[test]
fn header_replaces_existing_header() {
    let email = Email::new()
        .header("X-Mailer", "old-mailer")
        .header("X-Mailer", "missive");

    assert_eq!(email.headers.get("X-Mailer"), Some(&"missive".to_string()));
}

// ============================================================================
// Private Data Tests
// ============================================================================

#[test]
fn put_private_adds_private_data() {
    let email = Email::new().put_private("phoenix_layout", json!(false));
    assert_eq!(email.private.get("phoenix_layout"), Some(&json!(false)));
}

#[test]
fn put_private_adds_multiple_private_data() {
    let email = Email::new()
        .put_private("key1", json!("value1"))
        .put_private("key2", json!(42));

    assert_eq!(email.private.get("key1"), Some(&json!("value1")));
    assert_eq!(email.private.get("key2"), Some(&json!(42)));
}

// ============================================================================
// Provider Options Tests
// ============================================================================

#[test]
fn provider_option_adds_option() {
    let email = Email::new().provider_option("tag", "welcome");
    assert_eq!(
        email.provider_options.get("tag"),
        Some(&json!("welcome"))
    );
}

#[test]
fn provider_option_adds_complex_option() {
    let email = Email::new().provider_option(
        "tags",
        json!([{"name": "category", "value": "confirm_email"}]),
    );
    assert_eq!(
        email.provider_options.get("tags"),
        Some(&json!([{"name": "category", "value": "confirm_email"}]))
    );
}

// ============================================================================
// Assign Tests
// ============================================================================

#[test]
fn assign_adds_template_variable() {
    let email = Email::new()
        .assign("name", "Tony Stark")
        .assign("team", "Avengers");

    assert_eq!(email.assigns.get("name"), Some(&json!("Tony Stark")));
    assert_eq!(email.assigns.get("team"), Some(&json!("Avengers")));
}

// ============================================================================
// Validation Tests
// ============================================================================

#[test]
fn is_valid_returns_true_for_complete_email() {
    let email = Email::new()
        .from("tony.stark@example.com")
        .to("steve.rogers@example.com")
        .subject("Hello")
        .text_body("Hi there");

    assert!(email.is_valid());
}

#[test]
fn is_valid_returns_false_without_from() {
    let email = Email::new()
        .to("steve.rogers@example.com")
        .subject("Hello")
        .text_body("Hi there");

    assert!(!email.is_valid());
}

#[test]
fn is_valid_returns_false_without_to() {
    let email = Email::new()
        .from("tony.stark@example.com")
        .subject("Hello")
        .text_body("Hi there");

    assert!(!email.is_valid());
}

// ============================================================================
// Address Formatting Tests
// ============================================================================

#[test]
fn address_formatted_without_name() {
    let addr = Address::new("tony.stark@example.com");
    assert_eq!(addr.formatted(), "tony.stark@example.com");
}

#[test]
fn address_formatted_with_name() {
    let addr = Address::with_name("Tony Stark", "tony.stark@example.com");
    assert_eq!(addr.formatted(), "Tony Stark <tony.stark@example.com>");
}

#[test]
fn address_formatted_rfc5322_escapes_special_chars() {
    let addr = Address::with_name("Stark, Tony", "tony.stark@example.com");
    assert_eq!(
        addr.formatted_rfc5322(),
        "\"Stark, Tony\" <tony.stark@example.com>"
    );
}
