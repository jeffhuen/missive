# Email Providers

Missive supports multiple email providers out of the box. This guide covers configuration details for each.

## SMTP

Traditional SMTP delivery via [lettre](https://github.com/lettre/lettre).

**Feature:** `smtp`

**Environment Variables:**

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `SMTP_HOST` | Yes | - | SMTP server hostname |
| `SMTP_PORT` | No | `587` | SMTP server port |
| `SMTP_USERNAME` | No | - | Authentication username |
| `SMTP_PASSWORD` | No | - | Authentication password |
| `SMTP_TLS` | No | `required` | TLS mode: `required`, `opportunistic`, `none` |

**Example:**

```bash
EMAIL_PROVIDER=smtp
SMTP_HOST=smtp.example.com
SMTP_PORT=587
SMTP_USERNAME=apikey
SMTP_PASSWORD=your-api-key
```

**Programmatic Configuration:**

```rust
use missive::providers::SmtpMailer;

let mailer = SmtpMailer::new("smtp.example.com", 587)
    .credentials("username", "password")
    .tls_required()  // or .tls_opportunistic() or .tls_none()
    .build();
```

---

## Resend

[Resend](https://resend.com) - Modern email API.

**Feature:** `resend`

**Environment Variables:**

| Variable | Required | Description |
|----------|----------|-------------|
| `RESEND_API_KEY` | Yes | Your Resend API key (starts with `re_`) |

**Provider Options:**

```rust
use missive::Email;
use serde_json::json;

let email = Email::new()
    .to("user@example.com")
    .subject("Hello")
    // Resend-specific options
    .provider_option("tags", json!([
        {"name": "category", "value": "welcome"},
        {"name": "source", "value": "signup"}
    ]))
    .provider_option("scheduled_at", "2024-12-01T00:00:00Z");
```

**Available Options:**

| Option | Type | Description |
|--------|------|-------------|
| `tags` | Array | Email tags for analytics |
| `scheduled_at` | String (ISO 8601) | Schedule email for later delivery |

---

## SendGrid

[SendGrid](https://sendgrid.com) - Twilio's email platform.

**Feature:** `sendgrid`

**Environment Variables:**

| Variable | Required | Description |
|----------|----------|-------------|
| `SENDGRID_API_KEY` | Yes | Your SendGrid API key |

**Provider Options:**

```rust
use missive::Email;
use serde_json::json;

let email = Email::new()
    .to("user@example.com")
    .subject("Hello")
    // SendGrid-specific options
    .provider_option("categories", json!(["transactional", "welcome"]))
    .provider_option("send_at", 1734048000)  // Unix timestamp
    .provider_option("tracking_settings", json!({
        "click_tracking": {"enable": true},
        "open_tracking": {"enable": true}
    }));
```

**Available Options:**

| Option | Type | Description |
|--------|------|-------------|
| `categories` | Array | Email categories (max 10) |
| `send_at` | Integer | Unix timestamp for scheduled send |
| `tracking_settings` | Object | Click/open tracking configuration |
| `asm` | Object | Unsubscribe group settings |

---

## Postmark

[Postmark](https://postmarkapp.com) - Transactional email service.

**Feature:** `postmark`

**Environment Variables:**

| Variable | Required | Description |
|----------|----------|-------------|
| `POSTMARK_API_KEY` | Yes | Your Postmark server token |

**Provider Options:**

```rust
use missive::Email;
use serde_json::json;

let email = Email::new()
    .to("user@example.com")
    .subject("Hello")
    // Postmark-specific options
    .provider_option("Tag", "welcome-email")
    .provider_option("TrackOpens", true)
    .provider_option("TrackLinks", "HtmlAndText")
    .provider_option("MessageStream", "outbound");
```

**Available Options:**

| Option | Type | Description |
|--------|------|-------------|
| `Tag` | String | Email tag for analytics |
| `TrackOpens` | Boolean | Enable open tracking |
| `TrackLinks` | String | Link tracking: `None`, `HtmlAndText`, `HtmlOnly`, `TextOnly` |
| `MessageStream` | String | Message stream ID |
| `Metadata` | Object | Custom metadata |

---

## Mailgun

[Mailgun](https://mailgun.com) - Email API by Sinch.

**Feature:** `mailgun`

**Environment Variables:**

| Variable | Required | Description |
|----------|----------|-------------|
| `MAILGUN_API_KEY` | Yes | Your Mailgun API key |
| `MAILGUN_DOMAIN` | Yes | Your sending domain (e.g., `mg.yourdomain.com`) |

**Programmatic Configuration:**

```rust
use missive::providers::MailgunMailer;

let mailer = MailgunMailer::new("your-api-key", "mg.yourdomain.com");

// For EU region:
let mailer = MailgunMailer::new("your-api-key", "mg.yourdomain.com")
    .base_url("https://api.eu.mailgun.net/v3");
```

**Provider Options:**

```rust
use missive::Email;
use serde_json::json;

let email = Email::new()
    .to("user@example.com")
    .subject("Hello")
    // Mailgun-specific options
    .provider_option("tags", json!(["welcome", "onboarding"]))
    .provider_option("custom_vars", json!({"user_id": "123"}))
    .provider_option("recipient_vars", json!({
        "bob@example.com": {"name": "Bob"},
        "alice@example.com": {"name": "Alice"}
    }))
    .provider_option("sending_options", json!({"tracking": "yes", "dkim": "yes"}))
    .provider_option("template_name", "welcome-template")
    .provider_option("template_options", json!({"version": "v2", "text": "yes"}));
```

**Available Options:**

| Option | Type | Description |
|--------|------|-------------|
| `tags` | Array | Tags for analytics (max 3) |
| `custom_vars` | Object | Custom variables (sent as `h:X-Mailgun-Variables`) |
| `recipient_vars` | Object | Per-recipient variables for batch sending |
| `sending_options` | Object | Options like `tracking`, `dkim`, `testmode` |
| `template_name` | String | Name of stored Mailgun template |
| `template_options` | Object | Template options like `version`, `text` |

---

## Amazon SES

[Amazon SES](https://aws.amazon.com/ses/) - AWS Simple Email Service.

**Feature:** `amazon_ses`

**Environment Variables:**

| Variable | Required | Description |
|----------|----------|-------------|
| `AWS_REGION` | Yes | AWS region (e.g., `us-east-1`) |
| `AWS_ACCESS_KEY_ID` | Yes | IAM access key ID |
| `AWS_SECRET_ACCESS_KEY` | Yes | IAM secret access key |

**Programmatic Configuration:**

```rust
use missive::providers::AmazonSesMailer;

let mailer = AmazonSesMailer::new("us-east-1", "AKIAIOSFODNN7EXAMPLE", "your-secret-key");

// With additional SES parameters:
let mailer = AmazonSesMailer::new("us-east-1", "access-key", "secret-key")
    .ses_source_arn("arn:aws:ses:us-east-1:123456789:identity/example.com")
    .ses_from_arn("arn:aws:ses:us-east-1:123456789:identity/example.com");
```

**Provider Options:**

```rust
use missive::Email;
use serde_json::json;

let email = Email::new()
    .to("user@example.com")
    .subject("Hello")
    // SES-specific options
    .provider_option("configuration_set_name", "my-config-set")
    .provider_option("tags", json!([
        {"name": "campaign", "value": "welcome"},
        {"name": "env", "value": "production"}
    ]))
    // For IAM role authentication:
    .provider_option("security_token", "temporary-session-token");
```

**Available Options:**

| Option | Type | Description |
|--------|------|-------------|
| `configuration_set_name` | String | SES configuration set name |
| `tags` | Array | Message tags (`[{name, value}]`) for tracking |
| `security_token` | String | Temporary security token for IAM roles |

---

## Mailtrap

[Mailtrap](https://mailtrap.io) - Email testing and sending platform.

**Feature:** `mailtrap`

**Environment Variables:**

| Variable | Required | Description |
|----------|----------|-------------|
| `MAILTRAP_API_KEY` | Yes | Your Mailtrap API key |

**Programmatic Configuration:**

```rust
use missive::providers::MailtrapMailer;

let mailer = MailtrapMailer::new("your-api-key");

// For sandbox mode (testing):
let mailer = MailtrapMailer::new("your-api-key")
    .sandbox_inbox_id("111111");
```

**Provider Options:**

```rust
use missive::Email;
use serde_json::json;

let email = Email::new()
    .to("user@example.com")
    .subject("Hello")
    // Mailtrap-specific options
    .provider_option("category", "welcome")
    .provider_option("custom_variables", json!({
        "my_var": {"my_message_id": 123},
        "my_other_var": {"my_other_id": 1}
    }));
```

**Available Options:**

| Option | Type | Description |
|--------|------|-------------|
| `category` | String | Email category for filtering |
| `custom_variables` | Object | Custom variables for tracking |

---

## Brevo

[Brevo](https://brevo.com) (formerly Sendinblue) - Marketing and transactional email platform.

**Feature:** `brevo`

**Environment Variables:**

| Variable | Required | Description |
|----------|----------|-------------|
| `BREVO_API_KEY` | Yes | Your Brevo API key |

**Provider Options:**

```rust
use missive::Email;
use serde_json::json;

let email = Email::new()
    .to("user@example.com")
    .subject("Hello")
    // Brevo-specific options
    .provider_option("template_id", 123)
    .provider_option("params", json!({"name": "John", "order_id": 456}))
    .provider_option("tags", json!(["welcome", "onboarding"]))
    .provider_option("schedule_at", "2024-12-01T00:00:00Z");
```

**Available Options:**

| Option | Type | Description |
|--------|------|-------------|
| `template_id` | Integer | ID of the transactional email template |
| `params` | Object | Key/value attributes to customize the template |
| `tags` | Array | Tags for filtering in Brevo dashboard |
| `schedule_at` | String (RFC 3339) | UTC datetime to schedule the email |
| `sender_id` | Integer | Use a sender ID instead of email address |

---

## Mailjet

[Mailjet](https://mailjet.com) - Email delivery and marketing platform.

**Feature:** `mailjet`

**Environment Variables:**

| Variable | Required | Description |
|----------|----------|-------------|
| `MAILJET_API_KEY` | Yes | Your Mailjet API key |
| `MAILJET_SECRET_KEY` | Yes | Your Mailjet secret key |

**Programmatic Configuration:**

```rust
use missive::providers::MailjetMailer;

let mailer = MailjetMailer::new("api-key", "secret-key");
```

**Provider Options:**

```rust
use missive::Email;
use serde_json::json;

let email = Email::new()
    .to("user@example.com")
    .subject("Hello")
    // Mailjet-specific options
    .provider_option("template_id", 123)
    .provider_option("variables", json!({"firstname": "John", "lastname": "Doe"}))
    .provider_option("custom_id", "my-custom-id")
    .provider_option("event_payload", "custom-payload-string")
    .provider_option("template_error_deliver", true)
    .provider_option("template_error_reporting", "developer@example.com");
```

**Available Options:**

| Option | Type | Description |
|--------|------|-------------|
| `template_id` | Integer | ID of the template to use |
| `variables` | Object | Key/value variables for template substitution |
| `custom_id` | String | Custom ID for tracking |
| `event_payload` | String/Object | Custom payload for webhook events |
| `template_error_deliver` | Boolean | Send even if template has errors |
| `template_error_reporting` | String | Email to notify on template errors |

---

## Unsent

[Unsent](https://unsent.dev) - Developer-friendly email API.

**Feature:** `unsent`

**Environment Variables:**

| Variable | Required | Description |
|----------|----------|-------------|
| `UNSENT_API_KEY` | Yes | Your Unsent API key |

---

## Development Providers

These providers don't send real emails - they're for development, testing, and debugging.

| Provider | Storage | Use Case |
|----------|---------|----------|
| `LocalMailer` | `MemoryStorage` | Development (preview UI) and testing (assertions) |
| `LoggerMailer` | None | Staging, CI, console debugging |

### Local

Stores emails in `MemoryStorage` for development and testing.

**Feature:** `local`

**For development** - view emails via the [preview UI](./preview.md):

```rust
use missive::providers::LocalMailer;
use missive::preview::mailbox_router;

let mailer = LocalMailer::new();
let storage = mailer.storage();

let app = Router::new()
    .nest("/mailbox", mailbox_router(storage));
```

**For testing** - assert on sent emails:

```rust
use missive::providers::LocalMailer;
use missive::testing::*;

let mailer = LocalMailer::new();

// ... code that sends email ...

assert_email_sent(&mailer);
assert_email_to(&mailer, "user@example.com");
```

See [Testing](./testing.md) for more details.

### Logger

Logs emails to console via tracing. No storage - emails are not retained.

**Always available** (no feature flag required).

| `EMAIL_PROVIDER` | Output |
|------------------|--------|
| `logger` | Brief: recipients + subject |
| `logger_full` | Full: all fields, bodies at debug level |

```rust
use missive::providers::LoggerMailer;

let mailer = LoggerMailer::new();       // Brief
let mailer = LoggerMailer::full();      // Full details
```
