//! Testing utilities and assertion helpers.
//!
//! Provides convenient assertion macros and functions for testing email functionality.
//!
//! # Example
//!
//! ```rust,ignore
//! use missive::providers::LocalMailer;
//! use missive::testing::*;
//!
//! #[tokio::test]
//! async fn test_welcome_flow() {
//!     let mailer = LocalMailer::new();
//!
//!     // ... trigger email sending ...
//!
//!     assert_email_sent(&mailer);
//!     assert_email_to(&mailer, "user@example.com");
//!     assert_email_subject_contains(&mailer, "Welcome");
//!     refute_email_to(&mailer, "admin@example.com");
//!
//!     // Regex matching
//!     assert_email_subject_matches(&mailer, r"Welcome.*!");
//!     assert_email_html_matches(&mailer, r"<h1>.*</h1>");
//! }
//! ```

use regex::Regex;

use crate::providers::LocalMailer;
use crate::storage::StoredEmail;

// ============================================================================
// Helper Functions
// ============================================================================

/// Format a list of emails for error messages.
fn format_email_summary(emails: &[StoredEmail]) -> String {
    if emails.is_empty() {
        return "  (no emails sent)".to_string();
    }

    emails
        .iter()
        .enumerate()
        .map(|(i, stored)| {
            let e = &stored.email;
            let to = e
                .to
                .iter()
                .map(|a| a.email.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let from = e
                .from
                .as_ref()
                .map(|a| a.email.as_str())
                .unwrap_or("<none>");
            format!(
                "  {}. To: [{}], From: {}, Subject: \"{}\"",
                i + 1,
                to,
                from,
                e.subject
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ============================================================================
// Basic Assertions
// ============================================================================

/// Assert that at least one email was sent.
///
/// # Panics
///
/// Panics if no emails were sent.
pub fn assert_email_sent(mailer: &LocalMailer) {
    assert!(
        mailer.has_emails(),
        "Expected at least one email to be sent, but none were sent"
    );
}

/// Assert that no emails were sent.
///
/// # Panics
///
/// Panics if any email was sent.
pub fn assert_no_emails_sent(mailer: &LocalMailer) {
    let emails = mailer.emails();
    assert!(
        emails.is_empty(),
        "Expected no emails to be sent, but {} were sent.\n\nEmails sent:\n{}",
        emails.len(),
        format_email_summary(&emails)
    );
}

/// Assert that exactly N emails were sent.
///
/// # Panics
///
/// Panics if the count doesn't match.
pub fn assert_email_count(mailer: &LocalMailer, expected: usize) {
    let actual = mailer.email_count();
    assert!(
        actual == expected,
        "Expected {} email(s) to be sent, but {} were sent.\n\nEmails sent:\n{}",
        expected,
        actual,
        format_email_summary(&mailer.emails())
    );
}

/// Assert that an email was sent to a specific address.
///
/// # Panics
///
/// Panics if no email was sent to the address.
pub fn assert_email_to(mailer: &LocalMailer, email: &str) {
    let emails = mailer.emails();
    let found = emails
        .iter()
        .any(|stored| stored.email.to.iter().any(|a| a.email.eq_ignore_ascii_case(email)));

    assert!(
        found,
        "Expected an email to be sent to '{}'.\n\nEmails sent:\n{}",
        email,
        format_email_summary(&emails)
    );
}

/// Assert that no email was sent to a specific address.
///
/// # Panics
///
/// Panics if an email was sent to the address.
pub fn assert_no_emails_to(mailer: &LocalMailer, email: &str) {
    let emails = mailer.emails();
    let found = emails.iter().find(|stored| {
        stored
            .email
            .to
            .iter()
            .any(|a| a.email.eq_ignore_ascii_case(email))
    });

    if let Some(found_email) = found {
        panic!(
            "Expected no email to be sent to '{}', but found one.\n\nMatching email:\n  Subject: \"{}\"\n  From: {}\n\nAll emails:\n{}",
            email,
            found_email.email.subject,
            found_email.email.from.as_ref().map(|a| a.email.as_str()).unwrap_or("<none>"),
            format_email_summary(&emails)
        );
    }
}

/// Assert that an email with the exact subject was sent.
///
/// # Panics
///
/// Panics if no email with the subject was found.
pub fn assert_email_subject(mailer: &LocalMailer, subject: &str) {
    let emails = mailer.emails();
    let found = emails.iter().any(|stored| stored.email.subject == subject);

    assert!(
        found,
        "Expected an email with subject '{}'.\n\nEmails sent:\n{}",
        subject,
        format_email_summary(&emails)
    );
}

/// Assert that an email with subject containing text was sent.
///
/// # Panics
///
/// Panics if no matching email was found.
pub fn assert_email_subject_contains(mailer: &LocalMailer, text: &str) {
    let emails = mailer.emails();
    let found = emails.iter().any(|stored| stored.email.subject.contains(text));

    assert!(
        found,
        "Expected an email with subject containing '{}'.\n\nEmails sent:\n{}",
        text,
        format_email_summary(&emails)
    );
}

/// Assert that an email matching a predicate was sent.
///
/// # Panics
///
/// Panics if no matching email was found.
pub fn assert_email_matches<F>(mailer: &LocalMailer, predicate: F)
where
    F: Fn(&crate::email::Email) -> bool,
{
    let matches = mailer.find_emails(predicate);
    assert!(
        !matches.is_empty(),
        "Expected an email matching the predicate, but none was found.\n\nEmails sent:\n{}",
        format_email_summary(&mailer.emails())
    );
}

/// Get the last email sent, or panic if none.
///
/// # Panics
///
/// Panics if no emails were sent.
pub fn get_last_email(mailer: &LocalMailer) -> StoredEmail {
    mailer
        .last_email()
        .expect("Expected at least one email to be sent, but none were sent")
}

/// Flush and return all emails from the mailer.
///
/// This removes all emails from the mailer's storage and returns them.
/// Useful when you need to inspect emails and then clear the state.
pub fn flush_emails(mailer: &LocalMailer) -> Vec<StoredEmail> {
    mailer.flush()
}

/// Get all emails sent to a specific address.
pub fn get_emails_to(mailer: &LocalMailer, email: &str) -> Vec<StoredEmail> {
    mailer.find_emails(|e| {
        e.to
            .iter()
            .any(|addr| addr.email.eq_ignore_ascii_case(email))
    })
}

/// Assert the last email was sent from a specific address.
///
/// # Panics
///
/// Panics if no email was sent or from address doesn't match.
pub fn assert_email_from(mailer: &LocalMailer, from_email: &str) {
    let emails = mailer.emails();
    assert!(
        !emails.is_empty(),
        "Expected at least one email to check 'from', but none were sent"
    );

    let last = &emails[0];
    let actual_from = last
        .email
        .from
        .as_ref()
        .map(|a| a.email.as_str())
        .unwrap_or("<none>");

    assert!(
        actual_from.eq_ignore_ascii_case(from_email),
        "Expected last email from '{}', but was from '{}'.\n\nEmails sent:\n{}",
        from_email,
        actual_from,
        format_email_summary(&emails)
    );
}

/// Assert the last email has HTML body containing text.
///
/// # Panics
///
/// Panics if no email was sent or HTML body doesn't contain text.
pub fn assert_email_html_contains(mailer: &LocalMailer, text: &str) {
    let emails = mailer.emails();
    let last = emails
        .first()
        .expect("Expected at least one email to be sent, but none were sent");
    let html = last.email.html_body.as_deref().unwrap_or("");

    assert!(
        html.contains(text),
        "Expected HTML body to contain '{}', but it didn't.\n\nLast email:\n{}\n\nHTML body (first 500 chars):\n{}",
        text,
        format_email_summary(&[last.clone()]),
        &html[..html.len().min(500)]
    );
}

/// Assert the last email has text body containing text.
///
/// # Panics
///
/// Panics if no email was sent or text body doesn't contain text.
pub fn assert_email_text_contains(mailer: &LocalMailer, text: &str) {
    let emails = mailer.emails();
    let last = emails
        .first()
        .expect("Expected at least one email to be sent, but none were sent");
    let body = last.email.text_body.as_deref().unwrap_or("");

    assert!(
        body.contains(text),
        "Expected text body to contain '{}', but it didn't.\n\nLast email:\n{}\n\nText body (first 500 chars):\n{}",
        text,
        format_email_summary(&[last.clone()]),
        &body[..body.len().min(500)]
    );
}

/// Assert the last email has an attachment with the given filename.
///
/// # Panics
///
/// Panics if no email was sent or no attachment with that name exists.
pub fn assert_email_has_attachment(mailer: &LocalMailer, filename: &str) {
    let emails = mailer.emails();
    let last = emails
        .first()
        .expect("Expected at least one email to be sent, but none were sent");
    let has_attachment = last.email.attachments.iter().any(|a| a.filename == filename);

    let attachment_list = last
        .email
        .attachments
        .iter()
        .map(|a| a.filename.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    assert!(
        has_attachment,
        "Expected email to have attachment '{}'.\n\nLast email:\n{}\n\nAttachments: [{}]",
        filename,
        format_email_summary(&[last.clone()]),
        attachment_list
    );
}

// ============================================================================
// Regex Matching
// ============================================================================

/// Assert the last email subject matches a regex pattern.
///
/// # Panics
///
/// Panics if no email was sent or subject doesn't match.
pub fn assert_email_subject_matches(mailer: &LocalMailer, pattern: &str) {
    let emails = mailer.emails();
    let last = emails
        .first()
        .expect("Expected at least one email to be sent, but none were sent");
    let re = Regex::new(pattern).expect("Invalid regex pattern");

    assert!(
        re.is_match(&last.email.subject),
        "Expected subject to match pattern '{}', but was '{}'.\n\nLast email:\n{}",
        pattern,
        last.email.subject,
        format_email_summary(&[last.clone()])
    );
}

/// Assert the last email HTML body matches a regex pattern.
///
/// # Panics
///
/// Panics if no email was sent or HTML body doesn't match.
pub fn assert_email_html_matches(mailer: &LocalMailer, pattern: &str) {
    let emails = mailer.emails();
    let last = emails
        .first()
        .expect("Expected at least one email to be sent, but none were sent");
    let html = last.email.html_body.as_deref().unwrap_or("");
    let re = Regex::new(pattern).expect("Invalid regex pattern");

    assert!(
        re.is_match(html),
        "Expected HTML body to match pattern '{}', but it didn't.\n\nLast email:\n{}\n\nHTML body (first 500 chars):\n{}",
        pattern,
        format_email_summary(&[last.clone()]),
        &html[..html.len().min(500)]
    );
}

/// Assert the last email text body matches a regex pattern.
///
/// # Panics
///
/// Panics if no email was sent or text body doesn't match.
pub fn assert_email_text_matches(mailer: &LocalMailer, pattern: &str) {
    let emails = mailer.emails();
    let last = emails
        .first()
        .expect("Expected at least one email to be sent, but none were sent");
    let text = last.email.text_body.as_deref().unwrap_or("");
    let re = Regex::new(pattern).expect("Invalid regex pattern");

    assert!(
        re.is_match(text),
        "Expected text body to match pattern '{}', but it didn't.\n\nLast email:\n{}\n\nText body (first 500 chars):\n{}",
        pattern,
        format_email_summary(&[last.clone()]),
        &text[..text.len().min(500)]
    );
}

// ============================================================================
// Refute Assertions
// ============================================================================

/// Refute that any email was sent (alias for assert_no_emails_sent).
///
/// # Panics
///
/// Panics if any email was sent.
pub fn refute_email_sent(mailer: &LocalMailer) {
    assert_no_emails_sent(mailer);
}

/// Refute that an email was sent to a specific address (alias for assert_no_emails_to).
///
/// # Panics
///
/// Panics if an email was sent to the address.
pub fn refute_email_to(mailer: &LocalMailer, email: &str) {
    assert_no_emails_to(mailer, email);
}

/// Refute that an email with the exact subject was sent.
///
/// # Panics
///
/// Panics if an email with that subject was sent.
pub fn refute_email_subject(mailer: &LocalMailer, subject: &str) {
    let emails = mailer.emails();
    let found = emails.iter().find(|stored| stored.email.subject == subject);

    if let Some(found_email) = found {
        panic!(
            "Expected no email with subject '{}', but found one.\n\nMatching email:\n  To: [{}]\n  From: {}\n\nAll emails:\n{}",
            subject,
            found_email.email.to.iter().map(|a| a.email.as_str()).collect::<Vec<_>>().join(", "),
            found_email.email.from.as_ref().map(|a| a.email.as_str()).unwrap_or("<none>"),
            format_email_summary(&emails)
        );
    }
}

/// Refute that an email matching the predicate was sent.
///
/// # Panics
///
/// Panics if a matching email was found.
pub fn refute_email_matches<F>(mailer: &LocalMailer, predicate: F)
where
    F: Fn(&crate::email::Email) -> bool,
{
    let matches = mailer.find_emails(predicate);
    if !matches.is_empty() {
        panic!(
            "Expected no emails matching the predicate, but {} were found.\n\nMatching emails:\n{}",
            matches.len(),
            format_email_summary(&matches)
        );
    }
}

// ============================================================================
// Batch Assertions
// ============================================================================

/// Assert exactly N emails were sent (alias for assert_email_count).
pub fn assert_emails_sent_count(mailer: &LocalMailer, expected: usize) {
    assert_email_count(mailer, expected);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::email::Email;
    use crate::mailer::Mailer;

    #[tokio::test]
    async fn test_assertions() {
        let mailer = LocalMailer::new();

        mailer
            .deliver(
                &Email::new()
                    .from("sender@example.com")
                    .to("recipient@example.com")
                    .subject("Welcome aboard!")
                    .html_body("<h1>Hello</h1>")
                    .text_body("Hello"),
            )
            .await
            .unwrap();

        assert_email_sent(&mailer);
        assert_email_count(&mailer, 1);
        assert_email_to(&mailer, "recipient@example.com");
        assert_email_from(&mailer, "sender@example.com");
        assert_email_subject(&mailer, "Welcome aboard!");
        assert_email_subject_contains(&mailer, "Welcome");
        assert_email_html_contains(&mailer, "<h1>Hello</h1>");
        assert_email_text_contains(&mailer, "Hello");
        assert_no_emails_to(&mailer, "other@example.com");
    }

    #[tokio::test]
    #[should_panic(expected = "Expected at least one email")]
    async fn test_assert_sent_fails_when_empty() {
        let mailer = LocalMailer::new();
        assert_email_sent(&mailer);
    }

    #[tokio::test]
    #[should_panic(expected = "Expected no emails")]
    async fn test_assert_no_emails_fails_when_sent() {
        let mailer = LocalMailer::new();
        mailer.deliver(&Email::new().subject("Test")).await.unwrap();
        assert_no_emails_sent(&mailer);
    }
}
