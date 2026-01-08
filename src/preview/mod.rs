//! Mailbox preview UI for development.
//!
//! Provides a web UI that displays emails stored in `MemoryStorage`.
//!
//! ## Features
//!
//! - CSP nonce support for Content Security Policy compliance
//! - Full JSON API with private/provider_options/headers
//! - Path-based attachment lazy loading
//! - RFC 5322 compliant recipient rendering
//!
//! # Example (Axum)
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
//! ```
//!
//! # Example (Actix)
//!
//! ```rust,ignore
//! use missive::providers::LocalMailer;
//! use missive::preview::{actix_configure, ActixAppState, PreviewConfig};
//! use actix_web::{App, web};
//!
//! let mailer = LocalMailer::new();
//! let storage = mailer.storage();
//! let state = ActixAppState { storage, config: PreviewConfig::default() };
//!
//! let app = App::new()
//!     .service(web::scope("/dev/mailbox").configure(|cfg| actix_configure(cfg, state)));
//! ```

mod core;

#[cfg(feature = "preview-axum")]
mod axum_routes;

#[cfg(feature = "preview-actix")]
mod actix_routes;

#[cfg(feature = "preview-axum")]
use std::sync::Arc;

#[cfg(feature = "preview-axum")]
use crate::storage::MemoryStorage;

// Re-export configuration type
pub use core::PreviewConfig;

// ============================================================================
// Axum Support
// ============================================================================

#[cfg(feature = "preview-axum")]
pub use axum::Router;

/// Re-exports for testing (Axum).
#[cfg(feature = "preview-axum")]
pub mod reexports {
    pub use axum::body::Body;
    pub use axum::http::{Request, StatusCode};
}

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
#[cfg(feature = "preview-axum")]
pub fn mailbox_router(storage: Arc<MemoryStorage>) -> Router {
    axum_routes::create_router(storage)
}

/// Create a mailbox router with CSP nonce configuration.
#[cfg(feature = "preview-axum")]
pub fn mailbox_router_with_config(storage: Arc<MemoryStorage>, config: PreviewConfig) -> Router {
    axum_routes::create_router_with_config(storage, config)
}

// ============================================================================
// Actix Support
// ============================================================================

#[cfg(feature = "preview-actix")]
pub use actix_routes::AppState as ActixAppState;

/// Configure Actix routes on a scope.
///
/// ## Example
///
/// ```rust,ignore
/// use missive::preview::{actix_configure, ActixAppState, PreviewConfig};
/// use actix_web::{App, web};
///
/// let state = ActixAppState {
///     storage: mailer.storage(),
///     config: PreviewConfig::default(),
/// };
///
/// App::new()
///     .service(web::scope("/mailbox").configure(|cfg| actix_configure(cfg, state)));
/// ```
#[cfg(feature = "preview-actix")]
pub fn actix_configure(cfg: &mut actix_web::web::ServiceConfig, state: ActixAppState) {
    actix_routes::configure(cfg, state)
}
