# v0.7 Migration Notes

This document tracks breaking API migrations introduced during the v0.7 cleanup.

The goal of the v0.7 line is to make missive a more idiomatic Rust library:
explicit client ownership, typed configuration, validated builders, reliable
attachment errors, and preserved error source chains.

## Breaking Changes At A Glance

| Change | Why | Replacement |
| --- | --- | --- |
| Process-global delivery is no longer the primary API | Avoids hidden mutable global state, cross-test contamination, and awkward dependency injection | Own an `EmailClient<M>` and pass it through app state |
| Environment auto-detection moved behind explicit constructors | Makes environment reads visible at startup and easier to test | Use `EmailClient::from_env()` or `MailerConfig::from_env()` |
| `Email`, `Address`, and `Attachment` fields are private | Prevents invalid states and preserves layout flexibility for future releases | Use builder methods and accessors |
| `MailerExt::validate()` is removed | It rejected valid emails that rely on the `EMAIL_FROM` default sender fallback | Use `Email::is_valid()` for a quick check, or rely on delivery validation |
| Provider-specific options are moving to typed helpers | Avoids typo-prone string keys and runtime JSON type mistakes | Use provider extension traits such as `ResendEmailExt`; keep `.provider_option(...)` only as an escape hatch |
| Attachment read/encoding errors are returned | Prevents silently sending empty or corrupted attachments | Handle `MailError::AttachmentReadError` from delivery or encoding |
| Lazy attachment reads are offloaded in async providers | Avoids blocking async executor threads with file I/O | No call-site change for delivery; direct callers can keep sync helpers |
| `MailError` preserves source errors and is no longer clone-oriented | Keeps timeout, transport, HTTP, SMTP, and I/O context available to callers | Inspect `Error::source()` or match source-carrying variants |

## Delivery Clients Instead Of Process-Global Delivery

The compatibility facade remains available during the transition, but
application code should own a client.

Before:

```rust
use missive::{Email, deliver};

let email = Email::new()
    .to("user@example.com")
    .subject("Welcome")
    .text_body("Thanks for signing up.");

deliver(&email).await?;
```

After:

```rust
use missive::{Email, EmailClient};
use missive::providers::ResendMailer;

let mailer = ResendMailer::new(std::env::var("RESEND_API_KEY")?);
let client = EmailClient::new(mailer)
    .with_default_from("noreply@example.com");

let email = Email::new()
    .to("user@example.com")
    .subject("Welcome")
    .text_body("Thanks for signing up.");

client.deliver(email).await?;
```

The client is `Clone` when the underlying mailer is cloneable, so web
applications can put it in Axum, Actix, or other framework state instead of
reconfiguring a global mailer.

## Explicit Environment Configuration

Environment-based setup is still supported, but call it directly instead of
letting the first delivery perform hidden configuration.

Before:

```rust
missive::init()?;
missive::deliver(&email).await?;
```

After:

```rust
use missive::EmailClient;

let client = EmailClient::from_env()?;
client.deliver(email).await?;
```

For startup validation without constructing the client in one step, use
`MailerConfig`:

```rust
use missive::{EmailClient, MailerConfig};

let config = MailerConfig::from_env()?;
let client = EmailClient::new(config.into_mailer()?)
    .with_default_from("noreply@example.com");
```

This keeps provider selection, missing environment variables, and default sender
handling visible and testable.

## Provider Options

Raw JSON provider options still exist for advanced or newly released provider
features, but typed helpers should be the default when available.

Before:

```rust
use missive::Email;
use serde_json::json;

let email = Email::new()
    .to("user@example.com")
    .subject("Welcome")
    .provider_option("tags", json!([{"name": "category", "value": "welcome"}]))
    .provider_option("scheduled_at", "2026-06-01T12:00:00Z");
```

After:

```rust
use chrono::{Duration, Utc};
use missive::Email;
use missive::providers::ResendEmailExt;

let email = Email::new()
    .to("user@example.com")
    .subject("Welcome")
    .resend_tag("category", "welcome")
    .resend_scheduled_at(Utc::now() + Duration::hours(1));
```

Typed helpers make option names and value types compiler-checked where provider
support exists. Continue to use `.provider_option(...)` for provider features
that missive has not typed yet.

## Field Accessors

`Email`, `Address`, and `Attachment` fields are no longer part of the downstream
public API. Construct values with the existing builder methods and read values
through accessors.

```rust
let email = Email::new()
    .from(("Alice", "alice@example.com"))
    .to("bob@example.com")
    .subject("Welcome");

assert_eq!(email.from_address().unwrap().email(), "alice@example.com");
assert_eq!(email.to_addresses()[0].email(), "bob@example.com");
assert_eq!(email.subject_line(), "Welcome");
```

Common replacements:

| Before | After |
| --- | --- |
| `address.email` | `address.email()` |
| `address.name.as_deref()` | `address.display_name()` |
| `email.from.as_ref()` | `email.from_address()` |
| `email.to` | `email.to_addresses()` |
| `email.cc` | `email.cc_addresses()` |
| `email.bcc` | `email.bcc_addresses()` |
| `email.reply_to` | `email.reply_to_addresses()` |
| `email.subject` | `email.subject_line()` |
| `email.text_body.as_deref()` | `email.text_body_content()` |
| `email.html_body.as_deref()` | `email.html_body_content()` |
| `email.attachments` | `email.attachments()` |
| `email.headers` | `email.headers()` |
| `email.assigns` | `email.assigns()` |
| `email.private` | `email.private()` |
| `email.provider_options` | `email.provider_options()` |
| `attachment.filename` | `attachment.filename()` |
| `attachment.content_type` | `attachment.mime_type()` |
| `attachment.data` | `attachment.data()` |
| `attachment.path.as_deref()` | `attachment.path()` |
| `attachment.disposition` | `attachment.disposition()` |
| `attachment.content_id.as_deref()` | `attachment.inline_content_id()` |
| `attachment.headers` | `attachment.headers()` |

Serde support is unchanged: the structs still derive `Serialize` and
`Deserialize`, and the serialized field names remain stable for existing JSON
payloads.

## Attachment Clone Semantics

Eager attachments now store their bytes in shared immutable backing storage.
`Attachment::from_bytes` still takes ownership of a `Vec<u8>`, and
`Attachment::data()` still returns a borrowed byte slice, but cloning an
`Attachment` or an `Email` containing attachments no longer copies the raw
attachment payload.

For compatibility, `Attachment::get_data()` continues to return an owned
`Vec<u8>`. Use `Attachment::data()` when borrowing eagerly loaded bytes is
sufficient.

## Lazy Attachment Reads In Async Providers

Lazy path-based attachments are still available through
`Attachment::from_path_lazy`. The synchronous `Attachment::get_data()` and
`Attachment::base64_data()` APIs remain synchronous for callers that use them
directly.

When an async provider feature such as `smtp` or any HTTP provider is enabled,
missive enables an internal Tokio-backed attachment I/O path and provider
delivery uses that path instead of reading files directly on the async worker
thread. The default feature set does not add Tokio solely for core attachment
construction or inspection.

Read failures now abort delivery instead of producing an empty attachment:

```rust
match client.deliver(email).await {
    Ok(result) => tracing::info!(message_id = %result.message_id, "email sent"),
    Err(missive::MailError::AttachmentReadError { path, source }) => {
        eprintln!("failed to read attachment {path}: {source}");
    }
    Err(error) => return Err(error),
}
```

Rationale: a successful delivery with an empty attachment is harder to recover
from than an explicit error returned to the caller.

## Error Source Chains

`MailError` now stores source errors for HTTP, SMTP, lettre build, template, and
attachment I/O failures where the backing library exposes them. Code that cloned
`MailError` values should instead store their string representation or wrap them
in an application-level `Arc` if sharing is required.

```rust
use std::error::Error;

if let Err(error) = client.deliver(email).await {
    if let Some(source) = error.source() {
        tracing::debug!(%source, "mail delivery source error");
    }
}
```

Rationale: preserving the source chain lets applications distinguish transport
timeouts, malformed messages, and file-system errors without parsing strings.

## Local Preview Storage

`local_storage()` no longer creates or returns a process-global mailbox. It is
kept only as a deprecated compatibility facade and returns `None`.

Create a `LocalMailer` explicitly and pass its storage to preview APIs:

```rust
use missive::providers::LocalMailer;
use missive::preview::PreviewServer;

let mailer = LocalMailer::new();
let storage = mailer.storage();
missive::configure(mailer);

PreviewServer::new("127.0.0.1:3025", storage)?.spawn();
```

## Provider Address And Header Serialization

Provider adapters now normalize outbound address domains with IDNA/Punycode
before serialization. Providers that accept mailbox strings use RFC 5322-safe
formatting, so display names with commas or quotes are quoted and escaped.
Providers that accept structured `{ email, name }` objects keep names separate
and punycode only the email domain.

SMTP and Gmail/RFC 2822 delivery now include valid custom headers through
lettre raw headers. Invalid custom header names fail message construction with
`MailError::BuildError` instead of being ignored.
