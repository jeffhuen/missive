//! Mailbox preview UI for development.
//!
//! Provides an Axum router that displays emails stored in `MemoryStorage`.
//!
//! ## Features
//!
//! - CSP nonce support for Content Security Policy compliance
//! - Full JSON API with private/provider_options/headers
//! - Path-based attachment lazy loading
//! - RFC 5322 compliant recipient rendering
//!
//! # Example
//!
//! ```rust,ignore
//! use missive::providers::LocalMailer;
//! use missive::preview::mailbox_router;
//! use axum::Router;
//!
//! let mailer = LocalMailer::new();
//! let storage = mailer.storage();
//!
//! let app = Router::new()
//!     .nest("/dev/mailbox", mailbox_router(storage));
//!
//! // Now emails sent via mailer appear at http://localhost:3000/dev/mailbox
//! ```
//!
//! ## With CSP Nonces
//!
//! ```rust,ignore
//! use missive::preview::{mailbox_router_with_config, PreviewConfig};
//!
//! let config = PreviewConfig {
//!     script_nonce: Some("abc123".to_string()),
//!     style_nonce: Some("def456".to_string()),
//! };
//! let router = mailbox_router_with_config(storage, config);
//! ```

mod routes;

use std::sync::Arc;

use axum::Router;

use crate::storage::MemoryStorage;

// Re-export configuration type
pub use routes::PreviewConfig;

/// Create an Axum router for the mailbox preview UI.
///
/// Mount this at a development route (e.g., `/dev/mailbox`).
///
/// ## Routes
///
/// | Method | Path | Description |
/// |--------|------|-------------|
/// | GET | `/` | HTML UI listing all emails |
/// | GET | `/json` | JSON API |
/// | GET | `/:id` | View single email as JSON |
/// | GET | `/:id/html` | Raw HTML body (for iframe) |
/// | GET | `/:id/attachments/:idx` | Download attachment |
/// | POST | `/clear` | Delete all emails |
///
/// ## JSON API
///
/// The `/json` endpoint returns JSON including:
/// - `headers` - Custom email headers
/// - `provider_options` - Provider-specific options
/// - `sent_at` - Timestamp (from private storage)
/// - Full attachment metadata (path, type, headers)
pub fn mailbox_router(storage: Arc<MemoryStorage>) -> Router {
    routes::create_router(storage)
}

/// Create a mailbox router with CSP nonce configuration.
///
/// This allows specifying nonces for inline scripts and styles to comply
/// with Content Security Policy.
///
/// Nonces can also be passed via query parameters (`?script_nonce=...&style_nonce=...`).
pub fn mailbox_router_with_config(storage: Arc<MemoryStorage>, config: PreviewConfig) -> Router {
    routes::create_router_with_config(storage, config)
}
