//! # Missive
//!
//! Compose, deliver, and test emails in Rust. Plug and play.
//!
//! ## Quick Start
//!
//! Create a client and pass it through your application state:
//! ```rust,ignore
//! use missive::{Email, EmailClient};
//! use missive::providers::ResendMailer;
//!
//! let mailer = ResendMailer::new(std::env::var("RESEND_API_KEY")?);
//! let client = EmailClient::new(mailer)
//!     .with_default_from("noreply@example.com");
//!
//! let email = Email::new()
//!     .to("user@example.com")
//!     .subject("Welcome!")
//!     .text_body("Hello");
//!
//! client.deliver(email).await?;
//! ```
//!
//! The legacy `deliver(&email)` global facade remains available for small apps
//! and compatibility, but `EmailClient` is the primary API.
//! For explicit environment loading, use `EmailClient::from_env()`.
//!
//! ## Multiple Clients
//!
//! ```rust,ignore
//! use missive::{Email, EmailClient};
//! use missive::providers::ResendMailer;
//!
//! let transactional = EmailClient::new(ResendMailer::new("transactional_key"))
//!     .with_default_from("receipts@example.com");
//! let marketing = EmailClient::new(ResendMailer::new("marketing_key"))
//!     .with_default_from("news@example.com");
//!
//! transactional.deliver(receipt_email).await?;
//! marketing.deliver(newsletter_email).await?;
//! ```
//!
//! ## Environment Variables
//!
//! | Variable | Description |
//! |----------|-------------|
//! | `EMAIL_PROVIDER` | `smtp`, `resend`, `unsent`, `postmark`, `sendgrid`, `brevo`, `mailgun`, `amazon_ses`, `mailtrap`, `mailjet`, `socketlabs`, `gmail`, `protonbridge`, `jmap`, `logger`, `logger_full` |
//! | `EMAIL_FROM` | Default sender email |
//! | `EMAIL_FROM_NAME` | Default sender name |
//! | `SMTP_HOST` | SMTP server host |
//! | `SMTP_PORT` | SMTP server port (default: 587) |
//! | `SMTP_USERNAME` | SMTP username |
//! | `SMTP_PASSWORD` | SMTP password |
//! | `RESEND_API_KEY` | Resend API key |
//! | `UNSENT_API_KEY` | Unsent API key |
//! | `POSTMARK_API_KEY` | Postmark API key |
//! | `SENDGRID_API_KEY` | SendGrid API key |
//! | `BREVO_API_KEY` | Brevo API key |
//! | `MAILGUN_API_KEY` | Mailgun API key |
//! | `MAILGUN_DOMAIN` | Mailgun sending domain |
//! | `AWS_REGION` | AWS region for SES |
//! | `AWS_ACCESS_KEY_ID` | AWS access key |
//! | `AWS_SECRET_ACCESS_KEY` | AWS secret key |
//! | `MAILTRAP_API_KEY` | Mailtrap API key |
//! | `MAILTRAP_SANDBOX_INBOX_ID` | Mailtrap sandbox inbox ID (optional) |
//! | `MAILJET_API_KEY` | Mailjet API key |
//! | `MAILJET_SECRET_KEY` | Mailjet secret key |
//! | `SOCKETLABS_SERVER_ID` | SocketLabs server ID |
//! | `SOCKETLABS_API_KEY` | SocketLabs API key |
//! | `GMAIL_ACCESS_TOKEN` | Gmail OAuth2 access token |
//! | `PROTONBRIDGE_USERNAME` | Proton Bridge SMTP username |
//! | `PROTONBRIDGE_PASSWORD` | Proton Bridge SMTP password |
//! | `PROTONBRIDGE_HOST` | Proton Bridge host (default: 127.0.0.1) |
//! | `PROTONBRIDGE_PORT` | Proton Bridge port (default: 1025) |
//! | `JMAP_URL` | JMAP server URL |
//! | `JMAP_USERNAME` | JMAP username (for basic auth) |
//! | `JMAP_PASSWORD` | JMAP password (for basic auth) |
//! | `JMAP_BEARER_TOKEN` | JMAP bearer token (for OAuth2) |
//!
//! ## Feature Flags
//!
//! - `smtp` - SMTP provider via lettre
//! - `resend` - Resend API provider
//! - `unsent` - Unsent API provider
//! - `postmark` - Postmark API provider
//! - `sendgrid` - SendGrid API provider
//! - `brevo` - Brevo API provider (formerly Sendinblue)
//! - `mailgun` - Mailgun API provider
//! - `amazon_ses` - Amazon SES API provider
//! - `mailtrap` - Mailtrap API provider (testing/staging)
//! - `mailjet` - Mailjet API provider
//! - `socketlabs` - SocketLabs Injection API provider
//! - `gmail` - Gmail API provider (OAuth2)
//! - `protonbridge` - Proton Bridge provider (local SMTP)
//! - `jmap` - JMAP protocol provider (Stalwart, Fastmail, etc.)
//! - `local` - LocalMailer for development and testing
//! - `preview` - Mailbox preview web UI
//! - `metrics` - Prometheus-style metrics (counters/histograms)
//! - `dev` - Enables local and preview
//!
//! ## Metrics
//!
//! Enable `features = ["metrics"]` to emit Prometheus-style metrics:
//!
//! | Metric | Type | Labels | Description |
//! |--------|------|--------|-------------|
//! | `missive_emails_total` | Counter | provider, status | Total emails sent |
//! | `missive_delivery_duration_seconds` | Histogram | provider | Delivery duration |
//! | `missive_batch_total` | Counter | provider, status | Total batch operations |
//! | `missive_batch_size` | Histogram | provider | Emails per batch |
//!
//! Install a recorder (e.g., `metrics-exporter-prometheus`) in your app to collect them.

#[cfg(all(
    target_family = "wasm",
    target_os = "unknown",
    any(
        feature = "smtp",
        feature = "gmail",
        feature = "protonbridge",
        feature = "mailgun",
        feature = "preview",
        feature = "preview-axum",
        feature = "preview-actix"
    )
))]
compile_error!(
    "missive features smtp, gmail, protonbridge, mailgun, preview, preview-axum, and preview-actix are native-only on wasm32-unknown-unknown. Use HTTP JSON providers such as resend, postmark, sendgrid, brevo, amazon_ses, mailtrap, mailjet, socketlabs, or jmap."
);

/// The version of the missive crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

mod address;
mod attachment;
mod client;
mod config;
mod email;
mod error;
pub mod interceptor;
mod mailer;

pub mod providers;

#[cfg(feature = "local")]
mod storage;

#[cfg(feature = "local")]
pub mod testing;

#[cfg(any(
    all(
        feature = "preview",
        not(all(target_family = "wasm", target_os = "unknown"))
    ),
    all(
        feature = "preview-axum",
        not(all(target_family = "wasm", target_os = "unknown"))
    ),
    all(
        feature = "preview-actix",
        not(all(target_family = "wasm", target_os = "unknown"))
    )
))]
pub mod preview;

#[cfg(feature = "templates")]
mod template;
#[cfg(feature = "templates")]
pub use template::{EmailTemplate, EmailTemplateExt};

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use std::cell::RefCell;
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use std::env;
use std::sync::Arc;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use parking_lot::RwLock;

// Re-exports
pub use address::{Address, ToAddress};
pub use attachment::{Attachment, AttachmentType};
pub use client::EmailClient;
pub use config::*;
pub use email::{Email, PreparedEmail};
pub use error::MailError;
pub use interceptor::{Interceptor, InterceptorExt, WithInterceptor};
pub use mailer::{DeliveryResult, Mailer};

#[cfg(feature = "local")]
pub use storage::{MemoryStorage, Storage, StoredEmail};

// ============================================================================
// Global Mailer Configuration
// ============================================================================

/// Global mailer - swappable for testing.
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
static MAILER: RwLock<Option<Arc<dyn Mailer>>> = RwLock::new(None);

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
thread_local! {
    static MAILER: RefCell<Option<Arc<dyn Mailer>>> = RefCell::new(None);
}

/// Get legacy global storage for the LocalMailer.
///
/// The preview UI no longer uses process-global storage. Create a
/// [`providers::LocalMailer`], call [`providers::LocalMailer::storage`], and
/// pass that storage to the preview server or router explicitly.
///
/// ```rust,ignore
/// use missive::providers::LocalMailer;
/// use missive::preview::mailbox_router;
///
/// let mailer = LocalMailer::new();
/// let app = app.nest("/dev/mailbox", mailbox_router(mailer.storage()));
/// ```
#[cfg(feature = "local")]
#[deprecated(
    since = "0.7.0",
    note = "use LocalMailer::storage() and pass the storage to preview explicitly"
)]
pub fn local_storage() -> Option<Arc<MemoryStorage>> {
    None
}

/// Get the default from address from environment.
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
pub fn default_from() -> Option<Address> {
    let email = env::var("EMAIL_FROM").ok()?;
    match env::var("EMAIL_FROM_NAME").ok() {
        Some(name) => Some(Address::with_name(name, email)),
        None => Some(Address::new(email)),
    }
}

/// Get the default from address from environment.
///
/// `wasm32-unknown-unknown` has no process environment. Use
/// [`EmailClient::with_default_from`] or [`EmailClient::from_env_with`] to pass
/// values from JavaScript or worker bindings explicitly.
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub fn default_from() -> Option<Address> {
    None
}

/// Create mailer from explicit environment configuration.
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
fn create_mailer_from_env() -> Result<Arc<dyn Mailer>, MailError> {
    MailerConfig::from_env()?.into_mailer()
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
fn create_mailer_from_env() -> Result<Arc<dyn Mailer>, MailError> {
    Err(MailError::UnsupportedFeature(
        "global environment auto-configuration is not supported on wasm32-unknown-unknown; use EmailClient::new, EmailClient::from_env_with, or configure_arc".into(),
    ))
}

/// Get or initialize the global mailer.
fn get_mailer() -> Result<Arc<dyn Mailer>, MailError> {
    if let Some(mailer) = configured_mailer() {
        return Ok(mailer);
    }

    let mailer = create_mailer_from_env()?;
    Ok(configure_if_missing(mailer))
}

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
fn configured_mailer() -> Option<Arc<dyn Mailer>> {
    let guard = MAILER.read();
    guard.as_ref().cloned()
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
fn configured_mailer() -> Option<Arc<dyn Mailer>> {
    MAILER.with(|slot| slot.borrow().as_ref().cloned())
}

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
fn configure_if_missing(mailer: Arc<dyn Mailer>) -> Arc<dyn Mailer> {
    let mut guard = MAILER.write();
    if guard.is_none() {
        *guard = Some(Arc::clone(&mailer));
    }

    guard.as_ref().unwrap().clone()
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
fn configure_if_missing(mailer: Arc<dyn Mailer>) -> Arc<dyn Mailer> {
    MAILER.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_none() {
            *slot = Some(Arc::clone(&mailer));
        }
        slot.as_ref().unwrap().clone()
    })
}

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
fn set_configured_mailer(mailer: Arc<dyn Mailer>) {
    let mut guard = MAILER.write();
    *guard = Some(mailer);
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
fn set_configured_mailer(mailer: Arc<dyn Mailer>) {
    MAILER.with(|slot| {
        *slot.borrow_mut() = Some(mailer);
    });
}

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
fn clear_configured_mailer() {
    let mut guard = MAILER.write();
    *guard = None;
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
fn clear_configured_mailer() {
    MAILER.with(|slot| {
        *slot.borrow_mut() = None;
    });
}

/// Check if email is configured (env vars are set and feature is enabled).
///
/// Returns `true` only if both:
/// 1. The required environment variables for the provider are set
/// 2. The corresponding feature flag is enabled
///
/// Supports auto-detection: if `EMAIL_PROVIDER` is not set, checks for
/// available API keys and enabled features.
///
/// Logs a warning if the provider is specified but the feature flag is not enabled.
pub fn is_configured() -> bool {
    MailerConfig::from_env().is_ok()
}

/// Initialize the mailer from environment variables.
///
/// Call this at startup if you need early initialization (e.g., for preview UI).
/// Returns Ok if successful, Err if configuration is invalid.
///
/// ```rust,ignore
/// // In main.rs
/// missive::init().ok(); // Ignore error if email not configured
/// ```
pub fn init() -> Result<(), MailError> {
    if !is_configured() {
        return Err(MailError::NotConfigured);
    }
    let _ = get_mailer()?;
    Ok(())
}

/// Deliver an email using the global mailer.
///
/// Auto-configures from environment variables on first call.
/// Validates required fields (`from`, `to`) before sending.
/// Adds default `from` address from `EMAIL_FROM` if not set on email.
///
/// ```rust,ignore
/// use missive::{Email, deliver};
///
/// let email = Email::new()
///     .to("user@example.com")
///     .subject("Hello!")
///     .text_body("Hi there");
///
/// deliver(&email).await?;
/// ```
pub async fn deliver(email: &Email) -> Result<DeliveryResult, MailError> {
    let mailer = get_mailer()?;
    EmailClient::new(mailer)
        .with_optional_default_from(default_from())
        .deliver(email.clone())
        .await
}

/// Deliver an email using a specific mailer (per-call override).
///
/// Useful for testing or sending via a different provider.
///
/// ```rust,ignore
/// use missive::{Email, deliver_with};
/// use missive::providers::ResendMailer;
///
/// let mailer = ResendMailer::new("different_api_key");
/// let email = Email::new()
///     .to("user@example.com")
///     .subject("Hello!");
///
/// deliver_with(&email, &mailer).await?;
/// ```
pub async fn deliver_with<M: Mailer>(
    email: &Email,
    mailer: &M,
) -> Result<DeliveryResult, MailError> {
    EmailClient::new(mailer)
        .with_optional_default_from(default_from())
        .deliver(email.clone())
        .await
}

/// Deliver multiple emails using the global mailer.
pub async fn deliver_many(emails: &[Email]) -> Result<Vec<DeliveryResult>, MailError> {
    let mailer = get_mailer()?;
    EmailClient::new(mailer)
        .with_optional_default_from(default_from())
        .deliver_many(emails.to_vec())
        .await
}

// ============================================================================
// Manual Configuration (for testing or custom setups)
// ============================================================================

/// Manually configure the global mailer.
///
/// Sets a single global mailer used by `deliver()`.
/// Can be called multiple times - later calls replace the previous mailer.
///
/// ```rust,ignore
/// use missive::{configure, providers::LocalMailer};
///
/// configure(LocalMailer::new());
/// ```
pub fn configure<M: Mailer + 'static>(mailer: M) {
    set_configured_mailer(Arc::new(mailer));
}

/// Configure with an Arc'd mailer.
pub fn configure_arc(mailer: Arc<dyn Mailer>) {
    set_configured_mailer(mailer);
}

/// Reset the global mailer (useful for tests).
///
/// After calling this, the next `deliver()` will re-initialize from env vars.
pub fn reset() {
    clear_configured_mailer();
}

/// Get a reference to the configured mailer (if initialized).
pub fn mailer() -> Option<Arc<dyn Mailer>> {
    configured_mailer()
}

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::Address;
    pub use crate::Attachment;
    pub use crate::DeliveryResult;
    pub use crate::Email;
    pub use crate::EmailClient;
    pub use crate::MailError;
    pub use crate::Mailer;
    pub use crate::PreparedEmail;
    pub use crate::ToAddress;
    pub use crate::{default_from, deliver, deliver_many, deliver_with, is_configured};

    #[cfg(feature = "local")]
    pub use crate::Storage;
}
