# Mailbox Preview

View sent emails in your browser during development. Emails are stored in memory and displayed via a web UI.

## Requirements

The preview UI requires `LocalMailer`, which stores emails in `MemoryStorage`.

| Mailer | Storage | Works with Preview |
|--------|---------|-------------------|
| `LocalMailer` | `MemoryStorage` | ✅ Yes |
| `LoggerMailer` | None (console only) | ❌ No |
| `ResendMailer`, etc. | Sends to provider | ❌ No |

## How It Works

The preview feature provides an **Axum router** that you mount in your application. Missive is a library, so it can't automatically add routes - you control where the preview UI lives.

## Setup

Enable the `preview` feature (or `dev` which includes it):

```toml
[dependencies]
missive = { version = "0.1", features = ["preview"] }
# or
missive = { version = "0.1", features = ["dev"] }
```

## Mounting the Preview UI

You must manually add the preview router to your Axum application:

```rust
use axum::Router;
use missive::providers::LocalMailer;
use missive::preview::mailbox_router;

#[tokio::main]
async fn main() {
    // Create local mailer and get shared storage
    let mailer = LocalMailer::new();
    let storage = mailer.storage();

    // Configure as global mailer
    missive::configure(mailer);

    // Mount preview UI at your chosen path
    let app = Router::new()
        .nest("/dev/mailbox", mailbox_router(storage))
        .route("/", axum::routing::get(|| async { "Hello" }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

Visit `http://localhost:3000/dev/mailbox` to see sent emails.

## Using with Auto-Configured Mailer

If you're using environment-based configuration (`EMAIL_PROVIDER=local`), get the shared storage via `local_storage()`:

```rust
use missive::local_storage;
use missive::preview::mailbox_router;

#[tokio::main]
async fn main() {
    // Initialize from environment (EMAIL_PROVIDER=local)
    missive::init().expect("Failed to initialize mailer");

    let mut app = Router::new()
        .route("/", get(home));

    // Mount preview if local storage is available
    if let Some(storage) = local_storage() {
        app = app.nest("/dev/mailbox", mailbox_router(storage));
    }

    // ...
}
```

## Features

- **Email list** - View all sent emails with sender, recipient, subject
- **HTML preview** - Rendered HTML body with inline image support
- **Plain text view** - View text body
- **Headers** - Inspect all email headers
- **Attachments** - Download attachments
- **Delete** - Remove individual emails or clear all
- **JSON API** - Programmatic access to mailbox

## Development-Only Mounting

Only mount in development builds:

```rust
fn build_router(storage: Arc<MemoryStorage>) -> Router {
    let mut app = Router::new()
        .route("/", get(home));

    #[cfg(debug_assertions)]
    {
        app = app.nest("/dev/mailbox", mailbox_router(storage));
    }

    app
}
```

Or using environment variables:

```rust
if std::env::var("ENABLE_MAILBOX_PREVIEW").is_ok() {
    if let Some(storage) = missive::local_storage() {
        app = app.nest("/dev/mailbox", mailbox_router(storage));
    }
}
```

## JSON API

The preview UI exposes a JSON API for programmatic access:

```bash
# List all emails
curl http://localhost:3000/dev/mailbox/json

# Get specific email
curl http://localhost:3000/dev/mailbox/{id}

# Get HTML body (for iframe embedding)
curl http://localhost:3000/dev/mailbox/{id}/html

# Download attachment
curl http://localhost:3000/dev/mailbox/{id}/attachments/{index}

# Clear all emails
curl -X POST http://localhost:3000/dev/mailbox/clear
```

## CSP Nonce Support

If your application uses Content Security Policy, pass nonces for inline scripts/styles:

```rust
use missive::preview::{mailbox_router_with_config, PreviewConfig};

let config = PreviewConfig {
    script_nonce: Some("abc123".to_string()),
    style_nonce: Some("def456".to_string()),
};

let router = mailbox_router_with_config(storage, config);
```

Nonces can also be passed via query parameters:

```
http://localhost:3000/dev/mailbox?script_nonce=abc123&style_nonce=def456
```

## Shared Storage

The `LocalMailer` and preview UI share storage via `Arc`:

```rust
use std::sync::Arc;
use missive::storage::MemoryStorage;
use missive::providers::LocalMailer;

// Option 1: Get storage from mailer
let mailer = LocalMailer::new();
let storage = mailer.storage();

// Option 2: Create shared storage first
let storage = MemoryStorage::shared();
let mailer = LocalMailer::with_storage(Arc::clone(&storage));

// Both approaches work - storage is shared
```

## Storage Limits

By default, `MemoryStorage` keeps all emails. For long-running dev servers:

```rust
// Periodically clear old emails
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(3600));
    loop {
        interval.tick().await;
        if storage.count() > 100 {
            storage.clear();
        }
    }
});
```

## Production Warning

The mailbox preview is for development only. In production:

1. Use a real email provider (`resend`, `sendgrid`, etc.)
2. Don't mount the preview routes
3. Consider disabling the `preview` feature entirely

```rust
// Only compile preview code in dev builds
#[cfg(debug_assertions)]
use missive::preview::mailbox_router;
```
