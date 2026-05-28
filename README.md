# Missive

Compose, deliver, test, and preview emails in Rust. Plug and play.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/jeffhuen/missive/main/docs/images/preview-dark.webp">
  <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/jeffhuen/missive/main/docs/images/preview-light.webp">
  <img alt="Mailbox Preview UI" src="https://raw.githubusercontent.com/jeffhuen/missive/main/docs/images/preview-dark.webp">
</picture>

Missive comes with adapters for popular transactional email providers including Amazon SES, Gmail, JMAP, Mailgun, Resend, SendGrid, Postmark, Proton Bridge, SocketLabs, SMTP, and more. For local development, it includes an in-memory mailbox with a web-based preview UI, plus a logger provider for debugging.

## Requirements

Rust 1.75+ (async traits)

## Quick Start

Create an `EmailClient` with your provider and pass it through your application state:

```rust
use missive::{Email, EmailClient};
use missive::providers::ResendMailer;

let mailer = ResendMailer::new(std::env::var("RESEND_API_KEY")?);
let client = EmailClient::new(mailer)
    .with_default_from("noreply@example.com");

let email = Email::new()
    .to("user@example.com")
    .subject("Welcome!")
    .text_body("Thanks for signing up.");

client.deliver(email).await?;
```

The legacy `deliver(&email)` global facade remains available for small apps and compatibility, but `EmailClient` is the primary API.

If you want environment-based setup, opt into it explicitly:

```rust
let client = EmailClient::from_env()?;
```

## Installation

Add missive to your `Cargo.toml`:

```toml
[dependencies]
missive = { version = "0.6.2", features = ["resend"] }
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
| SocketLabs | `socketlabs` | `SOCKETLABS_SERVER_ID`, `SOCKETLABS_API_KEY` |
| Gmail | `gmail` | `GMAIL_ACCESS_TOKEN` |
| JMAP | `jmap` | `JMAP_URL`<br>`JMAP_USERNAME`<br>`JMAP_PASSWORD` |
| Proton Bridge | `protonbridge` | `PROTONBRIDGE_USERNAME`<br>`PROTONBRIDGE_PASSWORD` |
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
missive = { version = "0.6.2", features = ["resend"] }
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
missive = { version = "0.6.2", features = ["smtp", "resend", "local"] }
```

Then configure per environment in `.env`:

```bash
# ---- Missive Email ----
# Development: local in-memory mailer
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

`EmailClient::from_env()` and the compatibility global facade can load provider configuration from environment variables. When `EMAIL_PROVIDER` is not set, Missive detects which provider to use based on:

1. **Available API keys** - checks for `RESEND_API_KEY`, `SENDGRID_API_KEY`, etc.
2. **Enabled features** - only considers providers whose feature is compiled in
3. **Fallback to local** - if the `local` feature is enabled and no API keys found

**Detection order:** Resend → SendGrid → Postmark → Unsent → Brevo → Mailgun → Amazon SES → Mailtrap → SocketLabs → Gmail → Proton Bridge → JMAP → SMTP → Local

This means minimal env setup for simple cases that explicitly call `EmailClient::from_env()`:

```toml
missive = { version = "0.6.2", features = ["resend"] }
```

```bash
RESEND_API_KEY=re_xxxxx
# No EMAIL_PROVIDER needed when using EmailClient::from_env()
```

Use `EMAIL_PROVIDER` explicitly when:
- Multiple providers are enabled and you want to choose one
- You want to override auto-detection
- You're using `logger` or `logger_full` (no API key to detect)

### Bundles

```toml
# Development setup (local + preview UI)
missive = { version = "0.6.2", features = ["dev"] }

# Broad bundle: all providers + local + templates + Axum preview UI
missive = { version = "0.6.2", features = ["full"] }
```

The `full` bundle does not include `metrics`, the standalone `preview` server,
or `preview-actix`; enable those features explicitly when you need them.

### Available Features

| Feature | Description |
|---------|-------------|
| `smtp` | SMTP provider via lettre |
| `resend` | Resend API |
| `sendgrid` | SendGrid API |
| `postmark` | Postmark API |
| `brevo` | Brevo API (formerly Sendinblue) |
| `mailgun` | Mailgun API |
| `mailjet` | Mailjet API |
| `amazon_ses` | Amazon SES API |
| `mailtrap` | Mailtrap API |
| `socketlabs` | SocketLabs Injection API |
| `gmail` | Gmail API (OAuth2) |
| `jmap` | JMAP (RFC 8621) - works with Stalwart, Fastmail, Cyrus, etc. |
| `protonbridge` | Proton Mail via local Bridge |
| `unsent` | Unsent API |
| `local` | LocalMailer - in-memory storage + test assertions |
| `preview` | Standalone preview server via tiny_http; also enables `local` |
| `preview-axum` | Preview UI embedded in Axum; also enables `local` |
| `preview-actix` | Preview UI embedded in Actix; also enables `local` |
| `templates` | Askama template integration |
| `metrics` | Prometheus-style metrics |
| `dev` | Enables `local` + `preview` |
| `full` | All providers + `local` + `templates` + `preview-axum`; excludes `metrics`, `preview`, and `preview-actix` |

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
use chrono::{Duration, Utc};
use missive::Email;
use missive::providers::ResendEmailExt;
use serde_json::json;

// Resend: typed tags, scheduling, and idempotency
let email = Email::new()
    .to("user@example.com")
    .subject("Welcome")
    .resend_tag("category", "welcome")
    .resend_scheduled_at(Utc::now() + Duration::hours(1))
    .resend_idempotency_key("welcome-123");

// SendGrid: raw options remain available for provider features
// that do not have typed helpers yet
let email = Email::new()
    .to("user@example.com")
    .subject("Welcome")
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
use missive::{Email, EmailClient};
use missive::providers::LocalMailer;
use missive::testing::*;

#[tokio::test]
async fn test_welcome_email() {
    let mailer = LocalMailer::new();
    let client = EmailClient::new(mailer.clone())
        .with_default_from("noreply@example.com");

    let email = Email::new()
        .to("user@example.com")
        .subject("Welcome")
        .text_body("Thanks for signing up.");
    client.deliver(email).await.unwrap();

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
let client = EmailClient::new(mailer.clone())
    .with_default_from("noreply@example.com");
mailer.set_failure("SMTP connection refused");

let result = client.deliver(email).await;
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

### Standalone Server (Recommended)

The simplest option - runs on a separate port with no framework dependencies:

```rust
use missive::providers::LocalMailer;
use missive::preview::PreviewServer;

let mailer = LocalMailer::new();
let storage = mailer.storage();
missive::configure(mailer);

PreviewServer::new("127.0.0.1:3025", storage)
    .expect("Failed to start preview server")
    .spawn();

println!("Preview UI at http://127.0.0.1:3025");
```

If your preview emails omit a `from` address, set a default sender:

```bash
EMAIL_FROM=noreply@example.com
```

### Axum Integration

Embed the preview UI into your Axum app:

```rust
use missive::providers::LocalMailer;
use missive::preview::mailbox_router;

let mailer = LocalMailer::new();
let storage = mailer.storage();
missive::configure(mailer);

let mut app = Router::new()
    .route("/", get(home));

// Use .nest_service() if your app has custom state (Router<AppState>)
// Use .nest() if your app is Router<()>
app = app.nest_service("/dev/mailbox", mailbox_router(storage));
```

Then visit `http://localhost:3000/dev/mailbox`. See [docs/preview.md](./docs/preview.md) for more details.

### Actix Integration

See [docs/preview.md](./docs/preview.md) for Actix configuration.

### Features

- View all sent emails
- HTML and plain text preview
- View email headers
- Download attachments
- Delete individual emails or clear all
- Dark mode toggle
- JSON API for programmatic access

## Interceptors

Interceptors let you modify or block emails before they are sent. Use them to add headers, redirect recipients in development, or enforce business rules.

```rust
use missive::{Email, InterceptorExt, MailError};
use missive::providers::ResendMailer;

let mailer = ResendMailer::new(api_key)
    // Add tracking header to all emails
    .with_interceptor(|email: Email| {
        Ok(email.header("X-Request-ID", get_request_id()))
    })
    // Block emails to certain domains
    .with_interceptor(|email: Email| {
        for recipient in email.to_addresses() {
            if recipient.email().ends_with("@blocked.com") {
                return Err(MailError::SendError("Blocked domain".into()));
            }
        }
        Ok(email)
    });
```

See [docs/interceptors.md](./docs/interceptors.md) for more examples including development redirects and multi-tenant branding.

## Multiple Clients

Use separate clients when different mailers or sender defaults are needed:

```rust
use missive::{Email, EmailClient};
use missive::providers::ResendMailer;

// Use a different API key for this one email
let special_client = EmailClient::new(ResendMailer::new("different_api_key"))
    .with_default_from("vip@example.com");

let email = Email::new()
    .to("vip@example.com")
    .subject("Special delivery");

special_client.deliver(email).await?;
```

## Async Emails

Missive delivery is async. For fire-and-forget sending:

```rust
// Using tokio::spawn
let client = client.clone();
tokio::spawn(async move {
    if let Err(e) = client.deliver(email).await {
        tracing::error!("Failed to send email: {}", e);
    }
});
```

For reliable delivery, use a job queue like [apalis](https://github.com/geofmureithi/apalis):

```rust
use apalis::prelude::*;
use missive::{EmailClient, Mailer};

#[derive(Debug, Serialize, Deserialize)]
struct SendEmailJob {
    to: String,
    subject: String,
    body: String,
}

async fn send_email<M: Mailer>(
    job: SendEmailJob,
    client: &EmailClient<M>,
    _ctx: JobContext,
) -> Result<(), Error> {
    let email = Email::new()
        .to(&job.to)
        .subject(&job.subject)
        .text_body(&job.body);

    client.deliver(email).await?;
    Ok(())
}
```

## Metrics

Enable Prometheus-style metrics with `features = ["metrics"]`:

```toml
missive = { version = "0.6.2", features = ["resend", "metrics"] }
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
missive.deliver { provider="resend", recipient_count=1, attachment_count=0, status="success", duration_ms=42 }
```

Configure with any tracing subscriber:

```rust
tracing_subscriber::fmt::init();
```

## Error Handling

Delivery errors are returned to the caller - missive does not automatically retry or crash. Errors are logged via `tracing::error!` for observability.

```rust
match client.deliver(email).await {
    Ok(result) => println!("Sent: {}", result.message_id),
    Err(e) => {
        // You decide: retry, alert, queue for later, ignore, etc.
        println!("Failed: {}", e);
    }
}
```

Error variants for granular handling:

```rust
use missive::MailError;

match client.deliver(email).await {
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

## Migrating To The v0.7 API

The v0.7 cleanup makes the explicit client API the primary path and keeps the
global delivery facade only for compatibility.

Before:

```rust
use missive::providers::ResendMailer;

missive::configure(ResendMailer::new(api_key));
missive::deliver(&email).await?;
```

After:

```rust
use missive::EmailClient;
use missive::providers::ResendMailer;

let client = EmailClient::new(ResendMailer::new(api_key))
    .with_default_from("noreply@example.com");

client.deliver(email).await?;
```

For environment-based setup, prefer an explicit startup call:

```rust
use missive::EmailClient;

let client = EmailClient::from_env()?;
client.deliver(email).await?;
```

Other breaking migrations include private struct fields with accessor methods,
typed provider option helpers, attachment read errors that now fail delivery,
and `MailError` variants that preserve source errors. See the
[v0.7 migration guide](./docs/migration-v0.7.md) for old-to-new examples and
rationale.

## API Reference

### Core API

| Function | Description |
|----------|-------------|
| `EmailClient::new(mailer)` | Create an explicit delivery client |
| `EmailClient::from_env()` | Create a client from environment variables explicitly |
| `client.with_default_from(addr)` | Set the sender used when an email omits `from` |
| `client.deliver(email)` | Send one email |
| `client.deliver_many(emails)` | Send multiple emails |
| `deliver(&email)` | Compatibility facade using the global mailer |
| `deliver_with(&email, &mailer)` | Compatibility helper for a specific mailer |
| `configure(mailer)` | Set the global compatibility mailer |
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
- [v0.7 Migration](./docs/migration-v0.7.md) - Breaking API changes and old-to-new examples

## Acknowledgments

Missive's design is inspired by [Swoosh](https://github.com/swoosh/swoosh), the excellent Elixir email library.

## License

MIT
