# Missive

Compose, deliver, test, and preview emails in Rust. Plug and play.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/jeffhuen/missive/main/docs/images/preview-dark.webp">
  <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/jeffhuen/missive/main/docs/images/preview-light.webp">
  <img alt="Mailbox Preview UI" src="https://raw.githubusercontent.com/jeffhuen/missive/main/docs/images/preview-dark.webp">
</picture>

Missive comes with adapters for popular transactional email providers including Amazon SES, Mailgun, Resend, SendGrid, Postmark, SMTP, and more. For local development, it includes an in-memory mailbox with a web-based preview UI, plus a logger provider for debugging. Zero configuration required for most setups.

## Requirements

Rust 1.75+ (async traits)

## Quick Start

Add to your `.env`:

```bash
# ---- Missive Email ----
EMAIL_PROVIDER=resend
EMAIL_FROM=noreply@example.com
RESEND_API_KEY=re_xxxxx
```

Send emails:

```rust
use missive::{Email, deliver};

let email = Email::new()
    .to("user@example.com")
    .subject("Welcome!")
    .text_body("Thanks for signing up.");

deliver(&email).await?;
```

That's it. No configuration code, no builder structs, no initialization.

## Installation

Add missive to your `Cargo.toml`:

```toml
[dependencies]
missive = { version = "0.3", features = ["resend"] }
```

Enable the feature for your email provider. See [Feature Flags](#feature-flags) for all options.

## Providers

Missive supports popular transactional email services out of the box:

| Provider | Feature | Environment Variables |
|----------|---------|----------------------|
| SMTP | `smtp` | `SMTP_HOST`, `SMTP_PORT`, `SMTP_USERNAME`, `SMTP_PASSWORD` |
| Resend | `resend` | `RESEND_API_KEY` |
| SendGrid | `sendgrid` | `SENDGRID_API_KEY` |
| Postmark | `postmark` | `POSTMARK_API_KEY` |
| Brevo | `brevo` | `BREVO_API_KEY` |
| Mailgun | `mailgun` | `MAILGUN_API_KEY`, `MAILGUN_DOMAIN` |
| Mailjet | `mailjet` | `MAILJET_API_KEY`, `MAILJET_SECRET_KEY` |
| Amazon SES | `amazon_ses` | `AWS_REGION`, `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` |
| Mailtrap | `mailtrap` | `MAILTRAP_API_KEY` |
| Unsent | `unsent` | `UNSENT_API_KEY` |
| Local | `local` | (none) |
| Logger | (always available) | (none) |

Configure which provider to use with the `EMAIL_PROVIDER` environment variable:

```bash
EMAIL_PROVIDER=sendgrid
```

## Feature Flags

Missive uses Cargo features for conditional compilation - only the providers you enable are compiled into your binary. This keeps binaries small and compile times fast.

### Minimal: Single Provider

If you only use one provider, enable just that feature:

```toml
[dependencies]
missive = { version = "0.3", features = ["resend"] }
```

```bash
RESEND_API_KEY=re_xxxxx
# EMAIL_PROVIDER is auto-detected when only one is enabled
```

This gives you the smallest binary and fastest compile. You'd need to recompile to switch providers.

### Flexible: Multiple Providers

For runtime flexibility (e.g., different providers per environment), enable multiple:

```toml
[dependencies]
missive = { version = "0.3", features = ["smtp", "resend", "local"] }
```

Then configure per environment in `.env`:

```bash
# ---- Missive Email ----
# Development: local mailbox preview at /dev/mailbox
EMAIL_PROVIDER=local
EMAIL_FROM=noreply@example.com
```

```bash
# ---- Missive Email ----
# Staging: test with Resend
EMAIL_PROVIDER=resend
EMAIL_FROM=noreply@example.com
RESEND_API_KEY=re_test_xxx
```

```bash
# ---- Missive Email ----
# Production: your own SMTP
EMAIL_PROVIDER=smtp
EMAIL_FROM=noreply@example.com
SMTP_HOST=mail.example.com
SMTP_USERNAME=apikey
SMTP_PASSWORD=your-api-key
```

Same compiled binary, different behavior per environment.

### Auto-Detection

When `EMAIL_PROVIDER` is not set, Missive automatically detects which provider to use based on:

1. **Available API keys** - checks for `RESEND_API_KEY`, `SENDGRID_API_KEY`, etc.
2. **Enabled features** - only considers providers whose feature is compiled in
3. **Fallback to local** - if the `local` feature is enabled and no API keys found

**Detection order:** Resend → SendGrid → Postmark → Unsent → SMTP → Local

This means zero-config for simple setups:

```toml
missive = { version = "0.3", features = ["resend"] }
```

```bash
RESEND_API_KEY=re_xxxxx
# No EMAIL_PROVIDER needed - Resend is auto-detected
```

Use `EMAIL_PROVIDER` explicitly when:
- Multiple providers are enabled and you want to choose one
- You want to override auto-detection
- You're using `logger` or `logger_full` (no API key to detect)

### Bundles

```toml
# Development setup (local + preview UI)
missive = { version = "0.3", features = ["dev"] }

# Everything (all providers + templates)
missive = { version = "0.3", features = ["full"] }
```

### Available Features

| Feature | Description |
|---------|-------------|
| `smtp` | SMTP provider via lettre |
| `resend` | Resend API |
| `sendgrid` | SendGrid API |
| `postmark` | Postmark API |
| `unsent` | Unsent API |
| `local` | LocalMailer - in-memory storage + test assertions |
| `preview` | Web UI for viewing local emails (Axum) |
| `preview-axum` | Preview UI with Axum |
| `preview-actix` | Preview UI with Actix |
| `templates` | Askama template integration |
| `metrics` | Prometheus-style metrics |
| `dev` | Enables `local` + `preview` |
| `full` | All providers + templates + preview |

## Environment Variables

### Global Settings

| Variable | Description | Default |
|----------|-------------|---------|
| `EMAIL_PROVIDER` | Which provider to use | `smtp` |
| `EMAIL_FROM` | Default sender email | (none) |
| `EMAIL_FROM_NAME` | Default sender name | (none) |

### Provider-Specific

**SMTP:**
| Variable | Description | Default |
|----------|-------------|---------|
| `SMTP_HOST` | SMTP server hostname | (required) |
| `SMTP_PORT` | SMTP server port | `587` |
| `SMTP_USERNAME` | SMTP username | (optional) |
| `SMTP_PASSWORD` | SMTP password | (optional) |
| `SMTP_TLS` | TLS mode: `required`, `opportunistic`, `none` | `required` |

**API Providers:**
| Variable | Provider |
|----------|----------|
| `RESEND_API_KEY` | Resend |
| `SENDGRID_API_KEY` | SendGrid |
| `POSTMARK_API_KEY` | Postmark |
| `UNSENT_API_KEY` | Unsent |

## Composing Emails

### Basic Email

```rust
use missive::Email;

let email = Email::new()
    .from("sender@example.com")
    .to("recipient@example.com")
    .subject("Hello!")
    .text_body("Plain text content")
    .html_body("<h1>HTML content</h1>");
```

### With Display Names

```rust
let email = Email::new()
    .from(("Alice Smith", "alice@example.com"))
    .to(("Bob Jones", "bob@example.com"))
    .subject("Meeting tomorrow");
```

### Multiple Recipients

```rust
let email = Email::new()
    .to("one@example.com")
    .to("two@example.com")
    .cc("cc@example.com")
    .bcc("bcc@example.com")
    .reply_to("replies@example.com");
```

### Custom Headers

```rust
let email = Email::new()
    .header("X-Custom-Header", "custom-value")
    .header("X-Priority", "1");
```

### Provider-Specific Options

Pass options specific to your email provider:

```rust
// Resend: tags and scheduling
let email = Email::new()
    .provider_option("tags", json!([{"name": "category", "value": "welcome"}]))
    .provider_option("scheduled_at", "2024-12-01T00:00:00Z");

// SendGrid: categories and tracking
let email = Email::new()
    .provider_option("categories", json!(["transactional", "welcome"]))
    .provider_option("tracking_settings", json!({"click_tracking": {"enable": true}}));
```

## Custom Recipient Types

Implement `ToAddress` for your types to use them directly in email builders:

```rust
use missive::{Address, ToAddress, Email};

struct User {
    name: String,
    email: String,
}

impl ToAddress for User {
    fn to_address(&self) -> Address {
        Address::with_name(&self.name, &self.email)
    }
}

// Now use directly:
let user = User { name: "Alice".into(), email: "alice@example.com".into() };
let email = Email::new()
    .to(&user)
    .subject("Welcome!");
```

## Email Validation

Missive provides email address validation:

```rust
use missive::Address;

// Lenient (logs warnings for suspicious input)
let addr = Address::new("user@example.com");

// Strict RFC 5321/5322 validation
let addr = Address::parse("user@example.com")?;
let addr = Address::parse_with_name("Alice", "alice@example.com")?;

// International domain names (IDN/Punycode)
let addr = Address::new("user@example.jp");
let ascii = addr.to_ascii()?;  // Converts to punycode if needed
```

## Attachments

### From Bytes

```rust
use missive::{Email, Attachment};

let email = Email::new()
    .to("user@example.com")
    .subject("Your report")
    .attachment(
        Attachment::from_bytes("report.pdf", pdf_bytes)
            .content_type("application/pdf")
    );
```

### From File

```rust
// Eager loading (reads file immediately)
let attachment = Attachment::from_path("/path/to/file.pdf")?;

// Lazy loading (reads file at send time)
let attachment = Attachment::from_path_lazy("/path/to/large-file.zip")?;
```

### Inline Attachments (HTML Embedding)

```rust
let email = Email::new()
    .html_body(r#"<img src="cid:logo">"#)
    .attachment(
        Attachment::from_bytes("logo.png", png_bytes)
            .inline()
            .content_id("logo")
    );
```

## Testing

Use `LocalMailer` to capture emails in tests:

```rust
use missive::{Email, deliver_with, configure};
use missive::providers::LocalMailer;
use missive::testing::*;

#[tokio::test]
async fn test_welcome_email() {
    let mailer = LocalMailer::new();
    configure(mailer.clone());

    // Your code that sends an email
    send_welcome_email("user@example.com").await;

    // Assertions
    assert_email_sent(&mailer);
    assert_email_to(&mailer, "user@example.com");
    assert_email_subject_contains(&mailer, "Welcome");
    assert_email_count(&mailer, 1);
}
```

### Available Assertions

| Function | Description |
|----------|-------------|
| `assert_email_sent(&mailer)` | At least one email was sent |
| `assert_no_emails_sent(&mailer)` | No emails were sent |
| `assert_email_count(&mailer, n)` | Exactly n emails were sent |
| `assert_email_to(&mailer, email)` | Email was sent to address |
| `assert_email_from(&mailer, email)` | Email was sent from address |
| `assert_email_subject(&mailer, subject)` | Email has exact subject |
| `assert_email_subject_contains(&mailer, text)` | Subject contains text |
| `assert_email_html_contains(&mailer, text)` | HTML body contains text |
| `assert_email_text_contains(&mailer, text)` | Text body contains text |
| `refute_email_to(&mailer, email)` | No email was sent to address |

### Simulating Failures

```rust
let mailer = LocalMailer::new();
mailer.set_failure("SMTP connection refused");

let result = deliver_with(&email, &mailer).await;
assert!(result.is_err());
```

### Flush Emails

```rust
// Get and clear all emails atomically
let emails = flush_emails(&mailer);
assert_eq!(emails.len(), 3);

// Mailer is now empty
assert_no_emails_sent(&mailer);
```

## Mailbox Preview

View sent emails in your browser during development.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/jeffhuen/missive/main/docs/images/preview-light.webp">
  <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/jeffhuen/missive/main/docs/images/preview-dark.webp">
  <img alt="Mailbox Preview UI" src="https://raw.githubusercontent.com/jeffhuen/missive/main/docs/images/preview-light.webp">
</picture>

```rust
use missive::providers::LocalMailer;
use missive::preview::mailbox_router;

// Create mailer and get shared storage
let mailer = LocalMailer::new();
let storage = mailer.storage();

// Configure as global mailer
missive::configure(mailer);

// Mount the preview UI in your Axum router
let app = Router::new()
    .nest("/mailbox", mailbox_router(storage))
    .route("/", get(home));
```

Then visit `http://localhost:3000/mailbox` to see sent emails.

### Features

- View all sent emails
- HTML and plain text preview
- View email headers
- Download attachments
- Delete individual emails or clear all
- JSON API for programmatic access

## Interceptors

Interceptors let you modify or block emails before they are sent. Use them to add headers, redirect recipients in development, or enforce business rules.

```rust
use missive::{Email, InterceptorExt};
use missive::providers::ResendMailer;

let mailer = ResendMailer::new(api_key)
    // Add tracking header to all emails
    .with_interceptor(|email: Email| {
        Ok(email.header("X-Request-ID", get_request_id()))
    })
    // Block emails to certain domains
    .with_interceptor(|email: Email| {
        for recipient in &email.to {
            if recipient.email.ends_with("@blocked.com") {
                return Err(MailError::SendError("Blocked domain".into()));
            }
        }
        Ok(email)
    });
```

See [docs/interceptors.md](./docs/interceptors.md) for more examples including development redirects and multi-tenant branding.

## Per-Call Mailer Override

Override the global mailer for specific emails:

```rust
use missive::{Email, deliver_with};
use missive::providers::ResendMailer;

// Use a different API key for this one email
let special_mailer = ResendMailer::new("different_api_key");

let email = Email::new()
    .to("vip@example.com")
    .subject("Special delivery");

deliver_with(&email, &special_mailer).await?;
```

## Async Emails

Missive's `deliver()` is already async. For fire-and-forget sending:

```rust
// Using tokio::spawn
tokio::spawn(async move {
    if let Err(e) = deliver(&email).await {
        tracing::error!("Failed to send email: {}", e);
    }
});
```

For reliable delivery, use a job queue like [apalis](https://github.com/geofmureithi/apalis):

```rust
use apalis::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
struct SendEmailJob {
    to: String,
    subject: String,
    body: String,
}

async fn send_email(job: SendEmailJob, _ctx: JobContext) -> Result<(), Error> {
    let email = Email::new()
        .to(&job.to)
        .subject(&job.subject)
        .text_body(&job.body);

    deliver(&email).await?;
    Ok(())
}
```

## Metrics

Enable Prometheus-style metrics with `features = ["metrics"]`:

```toml
missive = { version = "0.3", features = ["resend", "metrics"] }
```

Missive emits these metrics:

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `missive_emails_total` | Counter | provider, status | Total emails sent |
| `missive_delivery_duration_seconds` | Histogram | provider | Delivery duration |
| `missive_batch_total` | Counter | provider, status | Batch operations |
| `missive_batch_size` | Histogram | provider | Emails per batch |

Install a recorder in your app to collect them:

```rust
// Using metrics-exporter-prometheus
metrics_exporter_prometheus::PrometheusBuilder::new()
    .install()
    .expect("failed to install Prometheus recorder");
```

If you don't install a recorder, metric calls are no-ops (zero overhead).

## Observability

Missive uses the `tracing` crate for observability. All email deliveries create spans:

```
missive.deliver { provider="resend", to=["user@example.com"], subject="Hello" }
```

Configure with any tracing subscriber:

```rust
tracing_subscriber::fmt::init();
```

## Error Handling

Delivery errors are returned to the caller - missive does not automatically retry or crash. Errors are logged via `tracing::error!` for observability.

```rust
match deliver(&email).await {
    Ok(result) => println!("Sent: {}", result.message_id),
    Err(e) => {
        // You decide: retry, alert, queue for later, ignore, etc.
        println!("Failed: {}", e);
    }
}
```

Error variants for granular handling:

```rust
use missive::{deliver, MailError};

match deliver(&email).await {
    Ok(result) => println!("Sent with ID: {}", result.message_id),
    Err(MailError::MissingField(field)) => println!("Missing: {}", field),
    Err(MailError::InvalidAddress(msg)) => println!("Bad address: {}", msg),
    Err(MailError::ProviderError { provider, message, .. }) => {
        println!("{} error: {}", provider, message);
    }
    Err(e) => println!("Error: {}", e),
}
```

## Logger Provider

Use `EMAIL_PROVIDER=logger` to only log emails without sending:

```bash
# Brief logging (just recipients and subject)
EMAIL_PROVIDER=logger

# Full logging (all fields, bodies at debug level)
EMAIL_PROVIDER=logger_full
```

Useful for staging environments or debugging.

## Templates

Enable `features = ["templates"]` for Askama integration:

```rust
use missive::{Email, EmailTemplate};
use askama::Template;

#[derive(Template)]
#[template(path = "welcome.html")]
struct WelcomeEmail {
    username: String,
    action_url: String,
}

let template = WelcomeEmail {
    username: "Alice".into(),
    action_url: "https://example.com/verify".into(),
};

let email = Email::new()
    .to("alice@example.com")
    .subject("Welcome!")
    .render_html(&template)?;
```

## API Reference

### Core Functions

| Function | Description |
|----------|-------------|
| `deliver(&email)` | Send email using global mailer |
| `deliver_with(&email, &mailer)` | Send email using specific mailer |
| `deliver_many(&emails)` | Send multiple emails |
| `configure(mailer)` | Set the global mailer |
| `init()` | Initialize from environment variables |
| `is_configured()` | Check if email is properly configured |

### Email Builder

| Method | Description |
|--------|-------------|
| `.from(addr)` | Set sender |
| `.to(addr)` | Add recipient |
| `.cc(addr)` | Add CC recipient |
| `.bcc(addr)` | Add BCC recipient |
| `.reply_to(addr)` | Add reply-to address |
| `.subject(text)` | Set subject line |
| `.text_body(text)` | Set plain text body |
| `.html_body(html)` | Set HTML body |
| `.attachment(att)` | Add attachment |
| `.header(name, value)` | Add custom header |
| `.provider_option(key, value)` | Set provider-specific option |
| `.assign(key, value)` | Set template variable |

## Documentation

For more detailed guides, see the [docs/](./docs/) folder:

- [Interceptors](./docs/interceptors.md) - Modify or block emails before delivery
- [Providers](./docs/providers.md) - Detailed configuration for each email provider
- [Testing](./docs/testing.md) - Complete testing guide with all assertion functions
- [Observability](./docs/observability.md) - Telemetry, metrics, Grafana dashboards, and alerting
- [Preview](./docs/preview.md) - Mailbox preview UI configuration
- [Templates](./docs/templates.md) - Askama template integration

## Acknowledgments

Missive's design is inspired by [Swoosh](https://github.com/swoosh/swoosh), the excellent Elixir email library.

## License

MIT
