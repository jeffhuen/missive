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
use missive::EmailClient;
use missive::providers::SmtpMailer;

let mailer = SmtpMailer::new("smtp.example.com", 587)
    .credentials("username", "password")
    .tls_required()  // or .tls_opportunistic() or .tls_none()
    .build();
let client = EmailClient::new(mailer)
    .with_default_from("noreply@example.com");
```

---

## Resend

[Resend](https://resend.com) - Modern email API.

**Feature:** `resend`

**Environment Variables:**

| Variable | Required | Description |
|----------|----------|-------------|
| `RESEND_API_KEY` | Yes | Your Resend API key (starts with `re_`) |

**Typed Provider Options:**

```rust
use chrono::Utc;
use missive::Email;
use missive::providers::ResendEmailExt;

let email = Email::new()
    .to("user@example.com")
    .subject("Hello")
    .resend_tag("category", "welcome")
    .resend_scheduled_at(Utc::now())
    .resend_idempotency_key("welcome-123");
```

Raw `.provider_option(...)` remains available as an advanced escape hatch for
provider features that do not have typed helpers yet.

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
    .provider_option("template_error_reporting", "developer@example.com")
    .provider_option("track_opens", false)
    .provider_option("track_clicks", false)
    .provider_option("url_tags", "utm_source=transactional");
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
| `track_opens` | Boolean | Enable or disable open tracking |
| `track_clicks` | Boolean | Enable or disable click tracking |
| `url_tags` | String | URL query parameters to append to links |

---

## Unsent

[Unsent](https://unsent.dev) - Developer-friendly email API.

**Feature:** `unsent`

**Environment Variables:**

| Variable | Required | Description |
|----------|----------|-------------|
| `UNSENT_API_KEY` | Yes | Your Unsent API key |

---

## SocketLabs

[SocketLabs](https://socketlabs.com) - Email delivery and infrastructure platform.

**Feature:** `socketlabs`

> **Note:** Injection API Keys (Version 1) have been decommissioned by SocketLabs. You must generate a SocketLabs API Key with `Injection Api` access from the SocketLabs dashboard. The API key is used as a Bearer token for authentication.

**Environment Variables:**

| Variable | Required | Description |
|----------|----------|-------------|
| `SOCKETLABS_SERVER_ID` | Yes | Your SocketLabs server ID |
| `SOCKETLABS_API_KEY` | Yes | Your SocketLabs API key (with Injection Api access) |

**Programmatic Configuration:**

```rust
use missive::providers::SocketLabsMailer;

let mailer = SocketLabsMailer::new("12345", "your-api-key");
```

**Supported Features:**

All standard email features are supported including friendly names on addresses:

```rust
use missive::Email;

let email = Email::new()
    .from(("Sender Name", "sender@example.com"))
    .to(("Recipient Name", "recipient@example.com"))
    .cc("cc@example.com")
    .bcc("bcc@example.com")
    .reply_to("reply@example.com")
    .subject("Hello")
    .text_body("Plain text content")
    .html_body("<html>HTML content</html>")
    .header("X-Custom-Header", "value")
    .attachment(attachment);
```

**Provider Options:**

```rust
use missive::Email;
use serde_json::json;

let email = Email::new()
    .from("sender@example.com")
    .to("user@example.com")
    .subject("Hello")
    // SocketLabs-specific options
    .provider_option("api_template", "template-id")
    .provider_option("mailing_id", "batch-001")
    .provider_option("message_id", "msg-001")
    .provider_option("charset", "UTF-8")
    .provider_option("amp_body", "<html amp4email>...</html>")
    .provider_option("merge_data", json!({
        "PerMessage": [{"firstname": "John"}],
        "Global": {"company": "Acme Inc"}
    }));
```

**Available Options:**

| Option | Type | Description |
|--------|------|-------------|
| `api_template` | String | Template identifier from Email Content Manager |
| `mailing_id` | String | Special header for tracking email batches |
| `message_id` | String | Special header for tracking individual messages |
| `charset` | String | Character encoding (default: UTF-8) |
| `amp_body` | String | AMP HTML content for dynamic emails |
| `merge_data` | Object | Data for inline merge feature with `PerMessage` and `Global` keys |

---

## Gmail

[Gmail API](https://developers.google.com/gmail/api) - Send emails via Google's Gmail service.

**Feature:** `gmail`

**Environment Variables:**

| Variable | Required | Description |
|----------|----------|-------------|
| `GMAIL_ACCESS_TOKEN` | Yes | OAuth2 access token with `gmail.send` scope |

**Authentication:**

Gmail requires an OAuth2 access token with the `gmail.send` scope. This provider does NOT handle token refresh - you must provide a valid access token.

Common authentication approaches:
- **Service account with domain-wide delegation** - For automated systems sending as users in a Google Workspace domain
- **OAuth2 web/desktop flow** - For user-consented access to their Gmail account
- **Google Cloud Application Default Credentials** - When running on GCP with appropriate service account

**Programmatic Configuration:**

```rust
use missive::providers::GmailMailer;

let mailer = GmailMailer::new("your-oauth2-access-token");
```

**Example:**

```bash
EMAIL_PROVIDER=gmail
GMAIL_ACCESS_TOKEN=ya29.xxx...your-access-token
```

**Technical Details:**

Gmail API supports two methods for sending emails:
1. **JSON with base64url-encoded `raw` field** - standard approach
2. **Media upload with raw RFC 2822** - what this provider uses

This provider uses the media upload approach (`uploadType=media`) with `Content-Type: message/rfc822`. The RFC 2822 message is built using lettre internally (shared with the SMTP provider), so all standard email features (HTML, text, attachments, CC, BCC, etc.) are fully supported.

**Response Data:**

The `DeliveryResult` includes Gmail-specific data:

```rust
let result = client.deliver(email).await?;

// Message ID from Gmail
println!("Message ID: {}", result.message_id);

// Gmail-specific response includes threadId and labelIds
if let Some(response) = result.provider_response {
    println!("Thread ID: {}", response["threadId"]);
    println!("Labels: {:?}", response["labelIds"]);
}
```

---

## JMAP

[JMAP](https://jmap.io/) (JSON Meta Application Protocol) - Modern, stateless alternative to IMAP/SMTP.

**Feature:** `jmap`

Works with any JMAP-compliant server including [Stalwart](https://stalw.art/), [Fastmail](https://www.fastmail.com/), and [Cyrus](https://www.cyrusimap.org/).

**Environment Variables:**

| Variable | Required | Description |
|----------|----------|-------------|
| `JMAP_URL` | Yes | JMAP session URL (e.g., `https://jmap.example.com/session`) |
| `JMAP_USERNAME` | Conditional | Username for basic auth |
| `JMAP_PASSWORD` | Conditional | Password for basic auth |
| `JMAP_BEARER_TOKEN` | Conditional | Bearer token for OAuth2 (alternative to username/password) |

**Programmatic Configuration:**

```rust
use missive::providers::JmapMailer;

// Basic authentication
let mailer = JmapMailer::new("https://jmap.example.com")
    .credentials("username", "password")
    .build();

// Bearer token (OAuth2)
let mailer = JmapMailer::new("https://jmap.example.com")
    .bearer_token("your-oauth-token")
    .build();
```

**How It Works:**

JMAP email submission follows RFC 8621:

1. **Session discovery** - Fetches account info from `/.well-known/jmap`
2. **Identity lookup** - Gets sender identity via `Identity/get`
3. **Mailbox lookup** - Finds drafts mailbox via `Mailbox/get`
4. **Create email** - Creates email in drafts via `Email/set`
5. **Submit** - Sends via `EmailSubmission/set` with `onSuccessDestroyEmail`

Emails must belong to at least one mailbox per the JMAP spec. The provider uses the drafts mailbox and automatically deletes the draft after successful submission.

**Local Testing:**

See [JMAP Testing Guide](./jmap-testing.md) for setting up a local Stalwart server with Docker.

---

## Proton Bridge

[Proton Mail](https://proton.me/mail) via the local [Proton Bridge](https://proton.me/mail/bridge) application.

**Feature:** `protonbridge`

**Prerequisites:**

1. Install and configure [Proton Mail Bridge](https://proton.me/mail/bridge)
2. Sign in to your Proton account in the Bridge app
3. Note the bridge password (not your Proton password)

**Environment Variables:**

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `PROTONBRIDGE_USERNAME` | Yes | - | Your Proton email address |
| `PROTONBRIDGE_PASSWORD` | Yes | - | Bridge-generated password (from Bridge app) |
| `PROTONBRIDGE_HOST` | No | `127.0.0.1` | Bridge hostname |
| `PROTONBRIDGE_PORT` | No | `1025` | Bridge SMTP port |

**Programmatic Configuration:**

```rust
use missive::providers::ProtonBridgeMailer;

// Default: localhost:1025
let mailer = ProtonBridgeMailer::new("user@proton.me", "bridge-password")
    .build();

// Custom host/port
let mailer = ProtonBridgeMailer::new("user@proton.me", "bridge-password")
    .host("192.168.1.100")
    .port(1026)
    .build();
```

**Technical Details:**

Proton Bridge is a thin wrapper around SMTP configured for the local Bridge:
- Connects to `127.0.0.1:1025` by default
- Uses PLAIN authentication (Bridge handles encryption to Proton servers)
- No TLS required (local connection only)

The Bridge application must be running for email delivery to work.

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
use missive::preview::PreviewServer;

let mailer = LocalMailer::new();
let storage = mailer.storage();
missive::configure(mailer);

PreviewServer::new("127.0.0.1:3025", storage)
    .expect("Failed to start preview server")
    .spawn();
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
