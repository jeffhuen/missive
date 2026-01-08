# Testing

Missive provides first-class testing support with `LocalMailer` and assertion helpers.

## Setup

Enable the `local` feature:

```toml
[dev-dependencies]
missive = { version = "0.1", features = ["local"] }
```

## Basic Usage

```rust
use missive::{Email, deliver_with, configure};
use missive::providers::LocalMailer;
use missive::testing::*;

#[tokio::test]
async fn test_sends_welcome_email() {
    // Create test mailer
    let mailer = LocalMailer::new();

    // Configure as global mailer (optional)
    configure(mailer.clone());

    // Your code that sends email
    let email = Email::new()
        .to("user@example.com")
        .subject("Welcome!")
        .text_body("Thanks for signing up.");

    deliver_with(&email, &mailer).await.unwrap();

    // Assertions
    assert_email_sent(&mailer);
    assert_email_to(&mailer, "user@example.com");
    assert_email_subject(&mailer, "Welcome!");
}
```

## Available Assertions

### Positive Assertions

| Function | Description |
|----------|-------------|
| `assert_email_sent(&mailer)` | At least one email was sent |
| `assert_email_count(&mailer, n)` | Exactly `n` emails were sent |
| `assert_email_to(&mailer, email)` | Email sent to this address |
| `assert_email_from(&mailer, email)` | Last email was from this address |
| `assert_email_subject(&mailer, text)` | Email with exact subject exists |
| `assert_email_subject_contains(&mailer, text)` | Subject contains text |
| `assert_email_html_contains(&mailer, text)` | HTML body contains text |
| `assert_email_text_contains(&mailer, text)` | Text body contains text |
| `assert_email_has_attachment(&mailer, filename)` | Has attachment with filename |
| `assert_email_matches(&mailer, predicate)` | Custom predicate matches |

### Negative Assertions

| Function | Description |
|----------|-------------|
| `assert_no_emails_sent(&mailer)` | No emails were sent |
| `refute_email_to(&mailer, email)` | No email sent to this address |
| `refute_email_subject(&mailer, text)` | No email with this subject |
| `refute_email_matches(&mailer, predicate)` | No email matches predicate |

### Regex Assertions

Requires the `local` feature (includes `regex`):

| Function | Description |
|----------|-------------|
| `assert_email_subject_matches(&mailer, regex)` | Subject matches regex |
| `assert_email_html_matches(&mailer, regex)` | HTML body matches regex |
| `assert_email_text_matches(&mailer, regex)` | Text body matches regex |

## Error Messages

Assertions provide detailed error messages showing actual emails:

```
Expected an email to be sent to 'wrong@example.com'.

Emails sent:
  1. To: [user@example.com], From: sender@example.com, Subject: "Welcome!"
  2. To: [admin@example.com], From: noreply@example.com, Subject: "Alert"
```

## Custom Predicates

Use `assert_email_matches` for complex assertions:

```rust
assert_email_matches(&mailer, |email| {
    email.to.iter().any(|a| a.email.ends_with("@company.com"))
        && email.subject.contains("Invoice")
        && email.attachments.len() > 0
});
```

## Simulating Failures

Test error handling by configuring the mailer to fail:

```rust
#[tokio::test]
async fn test_handles_email_failure() {
    let mailer = LocalMailer::new();

    // Configure to fail
    mailer.set_failure("SMTP connection refused");

    let email = Email::new()
        .to("user@example.com")
        .subject("Test");

    let result = deliver_with(&email, &mailer).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("connection refused"));

    // Clear failure for subsequent tests
    mailer.clear_failure();
}
```

## Flush Emails

Atomically get and clear all emails:

```rust
#[tokio::test]
async fn test_multiple_phases() {
    let mailer = LocalMailer::new();

    // Phase 1: User signup
    send_signup_emails(&mailer).await;

    let signup_emails = flush_emails(&mailer);
    assert_eq!(signup_emails.len(), 2);  // welcome + verification

    // Mailer is now empty
    assert_no_emails_sent(&mailer);

    // Phase 2: Password reset
    send_password_reset(&mailer).await;

    assert_email_count(&mailer, 1);
    assert_email_subject_contains(&mailer, "Password");
}
```

## Inspecting Emails

Access the raw `Email` struct for detailed inspection:

```rust
let mailer = LocalMailer::new();

// ... send emails ...

// Get all emails
let emails = mailer.emails();

// Get most recent email
if let Some(stored) = mailer.last_email() {
    let email = &stored.email;
    println!("To: {:?}", email.to);
    println!("Subject: {}", email.subject);
    println!("HTML: {:?}", email.html_body);
    println!("Attachments: {}", email.attachments.len());
}

// Find specific emails
let welcome_emails = mailer.find_emails(|e| e.subject.contains("Welcome"));
```

## Integration with Test Frameworks

### With tokio::test

```rust
#[tokio::test]
async fn test_email() {
    let mailer = LocalMailer::new();
    // ...
}
```

### Shared Test Fixtures

```rust
fn test_mailer() -> LocalMailer {
    let mailer = LocalMailer::new();
    missive::configure(mailer.clone());
    mailer
}

#[tokio::test]
async fn test_one() {
    let mailer = test_mailer();
    // ...
}
```

### Clearing Between Tests

Each `LocalMailer::new()` creates fresh storage, so tests are isolated by default. If sharing a mailer:

```rust
mailer.clear();  // Remove all captured emails
```

## Best Practices

1. **Use `deliver_with`** - Pass the test mailer explicitly for clarity
2. **Assert specifics** - Check subject, recipients, not just "email sent"
3. **Test error paths** - Use `set_failure()` to test error handling
4. **Use `flush_emails`** - When testing multi-step flows
5. **Check attachments** - Verify attachments are included when expected
