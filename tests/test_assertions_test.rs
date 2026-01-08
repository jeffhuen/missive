//! Test assertions tests.
//!
//! Ported from Swoosh's test_assertions_test.exs

use missive::providers::LocalMailer;
use missive::testing::*;
use missive::{Email, Mailer};

// ============================================================================
// Helper Functions
// ============================================================================

async fn send_email(mailer: &LocalMailer) -> Email {
    let email = Email::new()
        .from("tony.stark@example.com")
        .reply_to("bruce.banner@example.com")
        .to("steve.rogers@example.com")
        .to("natasha.romanoff@example.com")
        .cc("thor.odinson@example.com")
        .cc("clint.barton@example.com")
        .bcc("loki.odinson@example.com")
        .header("Avengers", "Assemble")
        .subject("Hello, Avengers!")
        .html_body("<h1>Some html</h1>")
        .text_body("Some text");

    mailer.deliver(&email).await.unwrap();
    email
}

// ============================================================================
// Basic Assertions - assert_email_sent
// ============================================================================

#[tokio::test]
async fn assert_email_sent_passes_when_email_sent() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_sent(&mailer);
}

#[tokio::test]
#[should_panic(expected = "Expected at least one email")]
async fn assert_email_sent_fails_when_no_email_sent() {
    let mailer = LocalMailer::new();
    assert_email_sent(&mailer);
}

// ============================================================================
// assert_no_emails_sent / refute_email_sent
// ============================================================================

#[tokio::test]
async fn assert_no_emails_sent_passes_when_empty() {
    let mailer = LocalMailer::new();
    assert_no_emails_sent(&mailer);
}

#[tokio::test]
#[should_panic(expected = "Expected no emails")]
async fn assert_no_emails_sent_fails_when_email_sent() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_no_emails_sent(&mailer);
}

#[tokio::test]
async fn refute_email_sent_passes_when_empty() {
    let mailer = LocalMailer::new();
    refute_email_sent(&mailer);
}

#[tokio::test]
#[should_panic(expected = "Expected no emails")]
async fn refute_email_sent_fails_when_email_sent() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    refute_email_sent(&mailer);
}

// ============================================================================
// assert_email_to
// ============================================================================

#[tokio::test]
async fn assert_email_to_passes_for_valid_recipient() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_to(&mailer, "steve.rogers@example.com");
}

#[tokio::test]
async fn assert_email_to_passes_for_second_recipient() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_to(&mailer, "natasha.romanoff@example.com");
}

#[tokio::test]
#[should_panic(expected = "Expected an email to be sent to")]
async fn assert_email_to_fails_for_wrong_recipient() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_to(&mailer, "wrong@example.com");
}

// ============================================================================
// assert_no_emails_to / refute_email_to
// ============================================================================

#[tokio::test]
async fn assert_no_emails_to_passes_for_non_recipient() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_no_emails_to(&mailer, "not-a-recipient@example.com");
}

#[tokio::test]
#[should_panic(expected = "Expected no email to be sent to")]
async fn assert_no_emails_to_fails_for_actual_recipient() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_no_emails_to(&mailer, "steve.rogers@example.com");
}

#[tokio::test]
async fn refute_email_to_passes_for_non_recipient() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    refute_email_to(&mailer, "not-a-recipient@example.com");
}

// ============================================================================
// assert_email_subject
// ============================================================================

#[tokio::test]
async fn assert_email_subject_passes_for_correct_subject() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_subject(&mailer, "Hello, Avengers!");
}

#[tokio::test]
#[should_panic(expected = "Expected an email with subject")]
async fn assert_email_subject_fails_for_wrong_subject() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_subject(&mailer, "Hello, X-Men!");
}

// ============================================================================
// assert_email_subject_contains
// ============================================================================

#[tokio::test]
async fn assert_email_subject_contains_passes_for_partial_match() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_subject_contains(&mailer, "Avengers");
}

#[tokio::test]
#[should_panic(expected = "Expected an email with subject containing")]
async fn assert_email_subject_contains_fails_for_no_match() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_subject_contains(&mailer, "X-Men");
}

// ============================================================================
// assert_email_from
// ============================================================================

#[tokio::test]
async fn assert_email_from_passes_for_correct_sender() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_from(&mailer, "tony.stark@example.com");
}

#[tokio::test]
#[should_panic(expected = "Expected last email from")]
async fn assert_email_from_fails_for_wrong_sender() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_from(&mailer, "wrong@example.com");
}

// ============================================================================
// assert_email_html_contains
// ============================================================================

#[tokio::test]
async fn assert_email_html_contains_passes_for_valid_content() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_html_contains(&mailer, "<h1>Some html</h1>");
}

#[tokio::test]
async fn assert_email_html_contains_passes_for_partial_content() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_html_contains(&mailer, "Some html");
}

#[tokio::test]
#[should_panic(expected = "Expected HTML body to contain")]
async fn assert_email_html_contains_fails_for_wrong_content() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_html_contains(&mailer, "Wrong content");
}

// ============================================================================
// assert_email_text_contains
// ============================================================================

#[tokio::test]
async fn assert_email_text_contains_passes_for_valid_content() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_text_contains(&mailer, "Some text");
}

#[tokio::test]
#[should_panic(expected = "Expected text body to contain")]
async fn assert_email_text_contains_fails_for_wrong_content() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_text_contains(&mailer, "Wrong content");
}

// ============================================================================
// Regex Matching
// ============================================================================

#[tokio::test]
async fn assert_email_subject_matches_passes_for_regex() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_subject_matches(&mailer, r"Hello.*!");
}

#[tokio::test]
async fn assert_email_subject_matches_passes_for_avengers_regex() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_subject_matches(&mailer, r"Avengers");
}

#[tokio::test]
#[should_panic(expected = "Expected subject to match pattern")]
async fn assert_email_subject_matches_fails_for_no_match() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_subject_matches(&mailer, r"X-Men");
}

#[tokio::test]
async fn assert_email_html_matches_passes_for_regex() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_html_matches(&mailer, r"<h1>.*</h1>");
}

#[tokio::test]
async fn assert_email_text_matches_passes_for_regex() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_text_matches(&mailer, r"Some.*");
}

// ============================================================================
// refute_email_subject
// ============================================================================

#[tokio::test]
async fn refute_email_subject_passes_for_different_subject() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    refute_email_subject(&mailer, "Goodbye, Avengers!");
}

#[tokio::test]
#[should_panic(expected = "Expected no email with subject")]
async fn refute_email_subject_fails_for_matching_subject() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    refute_email_subject(&mailer, "Hello, Avengers!");
}

// ============================================================================
// assert_email_count
// ============================================================================

#[tokio::test]
async fn assert_email_count_passes_for_correct_count() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_count(&mailer, 1);
}

#[tokio::test]
async fn assert_email_count_passes_for_multiple_emails() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    send_email(&mailer).await;
    assert_email_count(&mailer, 2);
}

#[tokio::test]
#[should_panic(expected = "Expected 2 email(s) to be sent, but 1")]
async fn assert_email_count_fails_for_wrong_count() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_count(&mailer, 2);
}

// ============================================================================
// assert_email_matches (predicate)
// ============================================================================

#[tokio::test]
async fn assert_email_matches_passes_for_valid_predicate() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_matches(&mailer, |email| email.to.len() == 2);
}

#[tokio::test]
#[should_panic(expected = "Expected an email matching the predicate")]
async fn assert_email_matches_fails_for_no_match() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    assert_email_matches(&mailer, |email| email.to.is_empty());
}

// ============================================================================
// refute_email_matches (predicate)
// ============================================================================

#[tokio::test]
async fn refute_email_matches_passes_for_no_match() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    refute_email_matches(&mailer, |email| email.to.is_empty());
}

#[tokio::test]
#[should_panic(expected = "Expected no emails matching the predicate")]
async fn refute_email_matches_fails_for_matching() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    refute_email_matches(&mailer, |email| email.to.len() == 2);
}

// ============================================================================
// Utility Functions
// ============================================================================

#[tokio::test]
async fn get_last_email_returns_email() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    let last = get_last_email(&mailer);
    assert_eq!(last.email.subject, "Hello, Avengers!");
}

#[tokio::test]
#[should_panic(expected = "Expected at least one email")]
async fn get_last_email_panics_when_empty() {
    let mailer = LocalMailer::new();
    get_last_email(&mailer);
}

#[tokio::test]
async fn flush_emails_returns_all_and_clears() {
    let mailer = LocalMailer::new();
    send_email(&mailer).await;
    send_email(&mailer).await;

    let flushed = flush_emails(&mailer);
    assert_eq!(flushed.len(), 2);
    assert_eq!(mailer.email_count(), 0);
}

#[tokio::test]
async fn get_emails_to_returns_matching_emails() {
    let mailer = LocalMailer::new();

    // Send email to Steve
    mailer
        .deliver(
            &Email::new()
                .from("sender@example.com")
                .to("steve.rogers@example.com")
                .subject("For Steve"),
        )
        .await
        .unwrap();

    // Send email to Natasha
    mailer
        .deliver(
            &Email::new()
                .from("sender@example.com")
                .to("natasha.romanoff@example.com")
                .subject("For Natasha"),
        )
        .await
        .unwrap();

    let steve_emails = get_emails_to(&mailer, "steve.rogers@example.com");
    assert_eq!(steve_emails.len(), 1);
    assert_eq!(steve_emails[0].email.subject, "For Steve");
}
