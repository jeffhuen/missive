# Mailbox Preview

View sent emails in your browser during development. Emails are stored in memory and displayed via a web UI.

## Requirements

The preview UI requires `LocalMailer`, which stores emails in `MemoryStorage`.

| Mailer | Storage | Works with Preview |
|--------|---------|-------------------|
| `LocalMailer` | `MemoryStorage` | Yes |
| `LoggerMailer` | None (console only) | No |
| `ResendMailer`, etc. | Sends to provider | No |

## Quick Start

Choose the integration that fits your setup:

| Feature | Use Case | Dependency |
|---------|----------|------------|
| `preview` | Standalone server (recommended) | `tiny_http` |
| `preview-axum` | Embed in Axum app | `axum` |
| `preview-actix` | Embed in Actix app | `actix-web` |

```toml
# Standalone server (simplest - no framework required)
missive = { version = "0.4", features = ["preview"] }

# Embed in Axum app
missive = { version = "0.4", features = ["preview-axum"] }

# Embed in Actix app
missive = { version = "0.4", features = ["preview-actix"] }

# Development bundle (local + standalone preview)
missive = { version = "0.4", features = ["dev"] }
```

---

## Standalone Server (Recommended)

The simplest option. Runs a lightweight HTTP server on a separate port - no framework integration needed.

### How It Works

The standalone preview server and your mailer share the same in-memory storage:

```
Your App                          Preview Server
   │                                    │
   ▼                                    ▼
LocalMailer ──► MemoryStorage ◄── PreviewServer
   │               (shared)             │
   ▼                                    ▼
missive::deliver()              http://127.0.0.1:3025
```

When you send an email with `LocalMailer`, it's stored in `MemoryStorage`. The preview server reads from the same storage to display emails in the browser.

### Environment Setup

Add to your `.env` file:

```bash
# Use LocalMailer (stores emails in memory)
EMAIL_PROVIDER=local
EMAIL_FROM=noreply@example.com
```

### Complete Example

Here's a full working example with an async web app:

```rust
use axum::{Router, routing::get};
use missive::{Email, deliver};

#[tokio::main]
async fn main() {
    // 1. Initialize mailer from environment (EMAIL_PROVIDER=local)
    missive::init().expect("Failed to initialize mailer");

    // 2. Start preview server if using local provider
    if let Some(storage) = missive::local_storage() {
        missive::preview::PreviewServer::new("127.0.0.1:3025", storage)
            .expect("Failed to start preview server")
            .spawn();
        
        println!("Preview UI at http://127.0.0.1:3025");
    }

    // 3. Your app - emails sent here appear in the preview UI
    let app = Router::new()
        .route("/", get(|| async { "Hello" }))
        .route("/send", get(send_test_email));

    println!("App at http://127.0.0.1:3000");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn send_test_email() -> &'static str {
    let email = Email::new()
        .to("user@example.com")
        .subject("Test Email")
        .text_body("This will appear in the preview UI!");

    match deliver(&email).await {
        Ok(_) => "Email sent! Check http://127.0.0.1:3025",
        Err(e) => {
            eprintln!("Failed: {}", e);
            "Failed to send"
        }
    }
}
```

Now:
1. Visit `http://127.0.0.1:3000/send` to send a test email
2. Visit `http://127.0.0.1:3025` to see it in the preview UI

### Minimal Example

If you just want the preview server without environment config:

```rust
use missive::providers::LocalMailer;
use missive::preview::PreviewServer;

fn main() {
    // Create mailer and get its storage
    let mailer = LocalMailer::new();
    let storage = mailer.storage();
    
    // Configure as global mailer
    missive::configure(mailer);

    // Start preview server (fire-and-forget)
    PreviewServer::new("127.0.0.1:3025", storage)
        .expect("Failed to start preview server")
        .spawn();

    println!("Preview UI at http://127.0.0.1:3025");
    
    // Your app continues...
}
```

### Blocking Mode

For a dedicated preview server binary (blocks forever):

```rust
use missive::preview::serve;

fn main() -> std::io::Result<()> {
    let mailer = missive::providers::LocalMailer::new();
    missive::configure(mailer);

    let storage = missive::local_storage().unwrap();
    
    println!("Preview server at http://127.0.0.1:3025");
    serve("127.0.0.1:3025", storage)  // Blocks forever
}
```

### With CSP Nonces

If your app requires Content Security Policy nonces:

```rust
use missive::preview::{PreviewServer, PreviewConfig};

let config = PreviewConfig {
    script_nonce: Some("abc123".to_string()),
    style_nonce: Some("def456".to_string()),
};

PreviewServer::with_config("127.0.0.1:3025", storage, config)?
    .spawn();
```

### Key Points

| Concept | Explanation |
|---------|-------------|
| **Shared storage** | `LocalMailer` and `PreviewServer` must use the same `MemoryStorage` instance |
| **Fire-and-forget** | `spawn()` starts a background thread - no handle to manage |
| **Port separation** | Preview runs on a different port (e.g., 3025) from your app (e.g., 3000) |
| **Development only** | Don't run the preview server in production |

---

## Axum Integration

Embed the preview UI into your existing Axum application at a route like `/dev/mailbox`.

### Basic Setup

```rust
use axum::Router;
use missive::providers::LocalMailer;
use missive::preview::mailbox_router;

#[tokio::main]
async fn main() {
    let mailer = LocalMailer::new();
    let storage = mailer.storage();

    missive::configure(mailer);

    // Mount preview UI at /dev/mailbox
    let app = Router::new()
        .nest("/dev/mailbox", mailbox_router(storage))
        .route("/", axum::routing::get(|| async { "Hello" }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

Visit `http://localhost:3000/dev/mailbox` to see sent emails.

### Apps with Custom State

`mailbox_router()` returns `Router<()>`. If your app uses custom state, you need `.nest_service()` instead of `.nest()`:

```rust
// Your app has custom state
let app: Router<AppState> = Router::new()
    .route("/", get(home))
    .nest_service("/dev/mailbox", mailbox_router(storage));  // not .nest()
```

| Your App | Method | Why |
|----------|--------|-----|
| `Router<()>` | `.nest()` | Same state type |
| `Router<AppState>` | `.nest_service()` | Different state types - treats nested router as opaque service |

### Conditional Mounting

To mount the preview only when `LocalMailer` is configured (e.g., in development), you need to conditionally add the route. There are two Rust idioms for this:

**Mutable binding:**

```rust
let mut app = Router::new()
    .route("/health", get(health))
    .nest("/api", api::router());

if let Some(storage) = missive::local_storage() {
    app = app.nest_service("/dev/mailbox", mailbox_router(storage));
}

axum::serve(listener, app).await.unwrap();
```

**Shadowing (avoids `mut`):**

```rust
let app = Router::new()
    .route("/health", get(health))
    .nest("/api", api::router());

let app = if let Some(storage) = missive::local_storage() {
    app.nest_service("/dev/mailbox", mailbox_router(storage))
} else {
    app
};

axum::serve(listener, app).await.unwrap();
```

Both achieve the same result. Shadowing is often preferred in Rust because it avoids `mut` when you're just transforming a value once.

### With CSP Nonces

```rust
use missive::preview::{mailbox_router_with_config, PreviewConfig};

let config = PreviewConfig {
    script_nonce: Some("abc123".to_string()),
    style_nonce: Some("def456".to_string()),
};

let router = mailbox_router_with_config(storage, config);
```

---

## Actix Integration

Configure routes on an Actix scope:

```rust
use actix_web::{App, HttpServer, web};
use missive::providers::LocalMailer;
use missive::preview::{actix_configure, ActixAppState, PreviewConfig};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let mailer = LocalMailer::new();
    let storage = mailer.storage();

    missive::configure(mailer);

    let state = ActixAppState {
        storage,
        config: PreviewConfig::default(),
    };

    HttpServer::new(move || {
        let state = state.clone();
        App::new()
            .service(
                web::scope("/dev/mailbox")
                    .configure(|cfg| actix_configure(cfg, state))
            )
    })
    .bind("127.0.0.1:3000")?
    .run()
    .await
}
```

---

## Features

- **Email list** - View all sent emails with sender, recipient, subject
- **HTML preview** - Rendered HTML body with inline image support
- **Plain text view** - View text body
- **Headers** - Inspect all email headers
- **Attachments** - Download attachments
- **Delete** - Remove individual emails or clear all
- **Dark mode** - Toggle between light and dark themes
- **JSON API** - Programmatic access to mailbox

## Routes

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | HTML UI listing all emails |
| GET | `/json` | JSON API - list all emails |
| GET | `/{id}` | View single email as JSON |
| GET | `/{id}/html` | Raw HTML body (for iframe) |
| GET | `/{id}/attachments/{idx}` | Download attachment |
| POST | `/clear` | Delete all emails |

---

## JSON API

```bash
# List all emails
curl http://localhost:3025/json

# Get specific email
curl http://localhost:3025/{id}

# Get HTML body (for iframe embedding)
curl http://localhost:3025/{id}/html

# Download attachment
curl http://localhost:3025/{id}/attachments/{index}

# Clear all emails
curl -X POST http://localhost:3025/clear
```

---

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

---

## Development-Only Mounting

### Using Conditional Compilation

```rust
fn setup_app(storage: Arc<MemoryStorage>) -> Router {
    let app = Router::new()
        .route("/", get(home));

    #[cfg(debug_assertions)]
    let app = app.nest_service("/dev/mailbox", mailbox_router(storage));

    app
}
```

> **Note:** Use `.nest()` if your app is `Router<()>`, or `.nest_service()` if your app has custom state. See [Apps with Custom State](#apps-with-custom-state).

### Using Environment Variables

```rust
if std::env::var("ENABLE_MAILBOX_PREVIEW").is_ok() {
    if let Some(storage) = missive::local_storage() {
        // Axum: app = app.nest_service("/dev/mailbox", mailbox_router(storage));
        // Standalone:
        PreviewServer::new("127.0.0.1:3025", storage)?.spawn();
    }
}
```

---

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

---

## Production Warning

The mailbox preview is for development only. In production:

1. Use a real email provider (`resend`, `sendgrid`, etc.)
2. Don't mount the preview routes or start the preview server
3. Consider disabling the `preview` feature entirely

```rust
// Only compile preview code in dev builds
#[cfg(debug_assertions)]
{
    if let Some(storage) = missive::local_storage() {
        missive::preview::PreviewServer::new("127.0.0.1:3025", storage)?.spawn();
    }
}
```
