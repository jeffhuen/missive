//! # Missive
//!
//! Compose, deliver, and test emails in Rust. Plug and play.
//!
//! ## Quick Start
//!
//! Set environment variables:
//! ```bash
//! EMAIL_PROVIDER=resend
//! RESEND_API_KEY=re_xxxxx
//! EMAIL_FROM=noreply@example.com
//! EMAIL_FROM_NAME=My App
//! ```
//!
//! Send emails from anywhere:
//! ```rust,ignore
//! use missive::{Email, deliver};
//!
//! let email = Email::new()
//!     .to("user@example.com")
//!     .subject("Welcome!")
//!     .text_body("Hello");
//!
//! deliver(&email).await?;
//! ```
//!
//! That's it. No configuration code needed.
//!
//! ## Per-Call Mailer Override
//!
//! ```rust,ignore
//! use missive::{Email, deliver_with};
//! use missive::providers::ResendMailer;
//!
//! let mailer = ResendMailer::new("different_api_key");
//! deliver_with(&email, &mailer).await?;
//! ```
//!
//! ## Environment Variables
//!
//! | Variable | Description |
//! |----------|-------------|
//! | `EMAIL_PROVIDER` | `smtp`, `resend`, `unsent`, `postmark`, `sendgrid`, `brevo`, `mailgun`, `amazon_ses`, `logger`, `logger_full` |
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

/// The version of the missive crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

mod address;
mod attachment;
mod email;
mod error;
mod mailer;

pub mod providers;

#[cfg(feature = "local")]
mod storage;

#[cfg(feature = "local")]
pub mod testing;

#[cfg(any(feature = "preview-axum", feature = "preview-actix"))]
pub mod preview;

#[cfg(feature = "templates")]
mod template;
#[cfg(feature = "templates")]
pub use template::{EmailTemplate, EmailTemplateExt};

use parking_lot::RwLock;
use std::env;
use std::sync::Arc;

#[cfg(feature = "metrics")]
use std::time::Instant;

// Re-exports
pub use address::{Address, ToAddress};
pub use attachment::{Attachment, AttachmentType};
pub use email::Email;
pub use error::MailError;
pub use mailer::{DeliveryResult, Mailer, MailerExt};

#[cfg(feature = "local")]
pub use storage::{MemoryStorage, Storage, StoredEmail};

// ============================================================================
// Global Mailer Configuration
// ============================================================================

/// Global mailer - swappable for testing
static MAILER: RwLock<Option<Arc<dyn Mailer>>> = RwLock::new(None);

/// Global shared storage for LocalMailer (used by preview UI).
#[cfg(feature = "local")]
static LOCAL_STORAGE: std::sync::OnceLock<Arc<MemoryStorage>> = std::sync::OnceLock::new();

/// Get the shared storage for the LocalMailer.
///
/// Use this to mount the preview UI when using `EMAIL_PROVIDER=local`.
///
/// ```rust,ignore
/// use missive::local_storage;
/// use missive::preview::mailbox_router;
///
/// if let Some(storage) = local_storage() {
///     app = app.nest("/dev/mailbox", mailbox_router(storage));
/// }
/// ```
#[cfg(feature = "local")]
pub fn local_storage() -> Option<Arc<MemoryStorage>> {
    LOCAL_STORAGE.get().cloned()
}

/// Get the default from address from environment.
pub fn default_from() -> Option<Address> {
    let email = env::var("EMAIL_FROM").ok()?;
    match env::var("EMAIL_FROM_NAME").ok() {
        Some(name) => Some(Address::with_name(name, email)),
        None => Some(Address::new(email)),
    }
}

/// Auto-detect provider based on enabled features and available API keys.
fn detect_provider() -> Option<&'static str> {
    // Check API keys first (explicit configuration)
    #[cfg(feature = "resend")]
    if env::var("RESEND_API_KEY").is_ok() {
        return Some("resend");
    }
    #[cfg(feature = "sendgrid")]
    if env::var("SENDGRID_API_KEY").is_ok() {
        return Some("sendgrid");
    }
    #[cfg(feature = "postmark")]
    if env::var("POSTMARK_API_KEY").is_ok() {
        return Some("postmark");
    }
    #[cfg(feature = "unsent")]
    if env::var("UNSENT_API_KEY").is_ok() {
        return Some("unsent");
    }
    #[cfg(feature = "brevo")]
    if env::var("BREVO_API_KEY").is_ok() {
        return Some("brevo");
    }
    #[cfg(feature = "mailgun")]
    if env::var("MAILGUN_API_KEY").is_ok() && env::var("MAILGUN_DOMAIN").is_ok() {
        return Some("mailgun");
    }
    #[cfg(feature = "amazon_ses")]
    if env::var("AWS_ACCESS_KEY_ID").is_ok()
        && env::var("AWS_SECRET_ACCESS_KEY").is_ok()
        && env::var("AWS_REGION").is_ok()
    {
        return Some("amazon_ses");
    }
    #[cfg(feature = "mailtrap")]
    if env::var("MAILTRAP_API_KEY").is_ok() {
        return Some("mailtrap");
    }
    #[cfg(feature = "smtp")]
    if env::var("SMTP_HOST").is_ok() {
        return Some("smtp");
    }
    #[cfg(feature = "local")]
    {
        return Some("local");
    }
    #[allow(unreachable_code)]
    None
}

/// Create mailer from environment variables.
fn create_mailer_from_env() -> Result<Arc<dyn Mailer>, MailError> {
    let provider = match env::var("EMAIL_PROVIDER") {
        Ok(p) => p.to_lowercase(),
        Err(_) => {
            // Auto-detect based on features and API keys
            match detect_provider() {
                Some(p) => {
                    tracing::debug!(provider = p, "Auto-detected email provider");
                    p.to_string()
                }
                None => {
                    return Err(MailError::Configuration(
                        "EMAIL_PROVIDER not set and could not auto-detect. \
                        Set EMAIL_PROVIDER or ensure an API key is configured."
                            .into(),
                    ));
                }
            }
        }
    };

    match provider.as_str() {
        #[cfg(feature = "smtp")]
        "smtp" => {
            let host = env::var("SMTP_HOST")
                .map_err(|_| MailError::Configuration("SMTP_HOST not set".into()))?;
            let port: u16 = env::var("SMTP_PORT")
                .unwrap_or_else(|_| "587".to_string())
                .parse()
                .unwrap_or(587);
            let username = env::var("SMTP_USERNAME").unwrap_or_default();
            let password = env::var("SMTP_PASSWORD").unwrap_or_default();

            let mailer = if username.is_empty() {
                providers::SmtpMailer::new(&host, port).build()
            } else {
                providers::SmtpMailer::new(&host, port)
                    .credentials(&username, &password)
                    .build()
            };
            Ok(Arc::new(mailer))
        }
        #[cfg(not(feature = "smtp"))]
        "smtp" => Err(MailError::Configuration(
            "EMAIL_PROVIDER=smtp but 'smtp' feature is not enabled. \
            Add `features = [\"smtp\"]` to Cargo.toml"
                .into(),
        )),

        #[cfg(feature = "resend")]
        "resend" => {
            let key = env::var("RESEND_API_KEY")
                .map_err(|_| MailError::Configuration("RESEND_API_KEY not set".into()))?;
            Ok(Arc::new(providers::ResendMailer::new(&key)))
        }
        #[cfg(not(feature = "resend"))]
        "resend" => Err(MailError::Configuration(
            "EMAIL_PROVIDER=resend but 'resend' feature is not enabled. \
            Add `features = [\"resend\"]` to Cargo.toml"
                .into(),
        )),

        #[cfg(feature = "unsent")]
        "unsent" => {
            let key = env::var("UNSENT_API_KEY")
                .map_err(|_| MailError::Configuration("UNSENT_API_KEY not set".into()))?;
            Ok(Arc::new(providers::UnsentMailer::new(&key)))
        }
        #[cfg(not(feature = "unsent"))]
        "unsent" => Err(MailError::Configuration(
            "EMAIL_PROVIDER=unsent but 'unsent' feature is not enabled. \
            Add `features = [\"unsent\"]` to Cargo.toml"
                .into(),
        )),

        #[cfg(feature = "postmark")]
        "postmark" => {
            let key = env::var("POSTMARK_API_KEY")
                .map_err(|_| MailError::Configuration("POSTMARK_API_KEY not set".into()))?;
            Ok(Arc::new(providers::PostmarkMailer::new(&key)))
        }
        #[cfg(not(feature = "postmark"))]
        "postmark" => Err(MailError::Configuration(
            "EMAIL_PROVIDER=postmark but 'postmark' feature is not enabled. \
            Add `features = [\"postmark\"]` to Cargo.toml"
                .into(),
        )),

        #[cfg(feature = "sendgrid")]
        "sendgrid" => {
            let key = env::var("SENDGRID_API_KEY")
                .map_err(|_| MailError::Configuration("SENDGRID_API_KEY not set".into()))?;
            Ok(Arc::new(providers::SendGridMailer::new(&key)))
        }
        #[cfg(not(feature = "sendgrid"))]
        "sendgrid" => Err(MailError::Configuration(
            "EMAIL_PROVIDER=sendgrid but 'sendgrid' feature is not enabled. \
            Add `features = [\"sendgrid\"]` to Cargo.toml"
                .into(),
        )),

        #[cfg(feature = "brevo")]
        "brevo" => {
            let key = env::var("BREVO_API_KEY")
                .map_err(|_| MailError::Configuration("BREVO_API_KEY not set".into()))?;
            Ok(Arc::new(providers::BrevoMailer::new(&key)))
        }
        #[cfg(not(feature = "brevo"))]
        "brevo" => Err(MailError::Configuration(
            "EMAIL_PROVIDER=brevo but 'brevo' feature is not enabled. \
            Add `features = [\"brevo\"]` to Cargo.toml"
                .into(),
        )),

        #[cfg(feature = "mailgun")]
        "mailgun" => {
            let key = env::var("MAILGUN_API_KEY")
                .map_err(|_| MailError::Configuration("MAILGUN_API_KEY not set".into()))?;
            let domain = env::var("MAILGUN_DOMAIN")
                .map_err(|_| MailError::Configuration("MAILGUN_DOMAIN not set".into()))?;
            let mut mailer = providers::MailgunMailer::new(&key, &domain);
            // Check for EU endpoint
            if let Ok(base_url) = env::var("MAILGUN_BASE_URL") {
                mailer = mailer.base_url(base_url);
            }
            Ok(Arc::new(mailer))
        }
        #[cfg(not(feature = "mailgun"))]
        "mailgun" => Err(MailError::Configuration(
            "EMAIL_PROVIDER=mailgun but 'mailgun' feature is not enabled. \
            Add `features = [\"mailgun\"]` to Cargo.toml"
                .into(),
        )),

        #[cfg(feature = "amazon_ses")]
        "amazon_ses" => {
            let region = env::var("AWS_REGION")
                .map_err(|_| MailError::Configuration("AWS_REGION not set".into()))?;
            let access_key = env::var("AWS_ACCESS_KEY_ID")
                .map_err(|_| MailError::Configuration("AWS_ACCESS_KEY_ID not set".into()))?;
            let secret = env::var("AWS_SECRET_ACCESS_KEY")
                .map_err(|_| MailError::Configuration("AWS_SECRET_ACCESS_KEY not set".into()))?;
            Ok(Arc::new(providers::AmazonSesMailer::new(region, access_key, secret)))
        }
        #[cfg(not(feature = "amazon_ses"))]
        "amazon_ses" => Err(MailError::Configuration(
            "EMAIL_PROVIDER=amazon_ses but 'amazon_ses' feature is not enabled. \
            Add `features = [\"amazon_ses\"]` to Cargo.toml"
                .into(),
        )),

        #[cfg(feature = "mailtrap")]
        "mailtrap" => {
            let key = env::var("MAILTRAP_API_KEY")
                .map_err(|_| MailError::Configuration("MAILTRAP_API_KEY not set".into()))?;
            let mut mailer = providers::MailtrapMailer::new(&key);
            // Check for sandbox mode
            if let Ok(inbox_id) = env::var("MAILTRAP_SANDBOX_INBOX_ID") {
                mailer = mailer.sandbox_inbox_id(inbox_id);
            }
            Ok(Arc::new(mailer))
        }
        #[cfg(not(feature = "mailtrap"))]
        "mailtrap" => Err(MailError::Configuration(
            "EMAIL_PROVIDER=mailtrap but 'mailtrap' feature is not enabled. \
            Add `features = [\"mailtrap\"]` to Cargo.toml"
                .into(),
        )),

        #[cfg(feature = "local")]
        "local" => {
            // Use global shared storage so preview UI can access emails
            let storage = LOCAL_STORAGE.get_or_init(MemoryStorage::shared);
            Ok(Arc::new(providers::LocalMailer::with_storage(Arc::clone(storage))))
        }
        #[cfg(not(feature = "local"))]
        "local" => Err(MailError::Configuration(
            "EMAIL_PROVIDER=local but 'local' feature is not enabled. \
            Add `features = [\"local\"]` to Cargo.toml"
                .into(),
        )),

        "logger" => Ok(Arc::new(providers::LoggerMailer::new())),
        "logger_full" => Ok(Arc::new(providers::LoggerMailer::full())),

        _ => Err(MailError::Configuration(format!(
            "Unknown EMAIL_PROVIDER: {}. Valid providers are: smtp, resend, unsent, postmark, sendgrid, brevo, mailgun, amazon_ses, mailtrap, local, logger, logger_full",
            provider
        ))),
    }
}

/// Get or initialize the global mailer.
fn get_mailer() -> Result<Arc<dyn Mailer>, MailError> {
    // Fast path: already configured
    {
        let guard = MAILER.read();
        if let Some(ref mailer) = *guard {
            return Ok(Arc::clone(mailer));
        }
    }

    // Slow path: need to configure
    let mailer = create_mailer_from_env()?;
    let mut guard = MAILER.write();

    // Double-check after acquiring write lock
    if guard.is_none() {
        *guard = Some(Arc::clone(&mailer));
    }

    Ok(guard.as_ref().unwrap().clone())
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
    let provider = match env::var("EMAIL_PROVIDER") {
        Ok(p) => p,
        Err(_) => {
            // Auto-detect
            match detect_provider() {
                Some(p) => p.to_string(),
                None => return false,
            }
        }
    };
    match provider.to_lowercase().as_str() {
        #[cfg(feature = "smtp")]
        "smtp" => env::var("SMTP_HOST").is_ok(),
        #[cfg(not(feature = "smtp"))]
        "smtp" => {
            tracing::warn!(
                "EMAIL_PROVIDER=smtp but 'smtp' feature is not enabled. \
                Add `features = [\"smtp\"]` to Cargo.toml"
            );
            false
        }

        #[cfg(feature = "resend")]
        "resend" => env::var("RESEND_API_KEY").is_ok(),
        #[cfg(not(feature = "resend"))]
        "resend" => {
            tracing::warn!(
                "EMAIL_PROVIDER=resend but 'resend' feature is not enabled. \
                Add `features = [\"resend\"]` to Cargo.toml"
            );
            false
        }

        #[cfg(feature = "unsent")]
        "unsent" => env::var("UNSENT_API_KEY").is_ok(),
        #[cfg(not(feature = "unsent"))]
        "unsent" => {
            tracing::warn!(
                "EMAIL_PROVIDER=unsent but 'unsent' feature is not enabled. \
                Add `features = [\"unsent\"]` to Cargo.toml"
            );
            false
        }

        #[cfg(feature = "postmark")]
        "postmark" => env::var("POSTMARK_API_KEY").is_ok(),
        #[cfg(not(feature = "postmark"))]
        "postmark" => {
            tracing::warn!(
                "EMAIL_PROVIDER=postmark but 'postmark' feature is not enabled. \
                Add `features = [\"postmark\"]` to Cargo.toml"
            );
            false
        }

        #[cfg(feature = "sendgrid")]
        "sendgrid" => env::var("SENDGRID_API_KEY").is_ok(),
        #[cfg(not(feature = "sendgrid"))]
        "sendgrid" => {
            tracing::warn!(
                "EMAIL_PROVIDER=sendgrid but 'sendgrid' feature is not enabled. \
                Add `features = [\"sendgrid\"]` to Cargo.toml"
            );
            false
        }

        #[cfg(feature = "brevo")]
        "brevo" => env::var("BREVO_API_KEY").is_ok(),
        #[cfg(not(feature = "brevo"))]
        "brevo" => {
            tracing::warn!(
                "EMAIL_PROVIDER=brevo but 'brevo' feature is not enabled. \
                Add `features = [\"brevo\"]` to Cargo.toml"
            );
            false
        }

        #[cfg(feature = "mailgun")]
        "mailgun" => env::var("MAILGUN_API_KEY").is_ok() && env::var("MAILGUN_DOMAIN").is_ok(),
        #[cfg(not(feature = "mailgun"))]
        "mailgun" => {
            tracing::warn!(
                "EMAIL_PROVIDER=mailgun but 'mailgun' feature is not enabled. \
                Add `features = [\"mailgun\"]` to Cargo.toml"
            );
            false
        }

        #[cfg(feature = "amazon_ses")]
        "amazon_ses" => {
            env::var("AWS_REGION").is_ok()
                && env::var("AWS_ACCESS_KEY_ID").is_ok()
                && env::var("AWS_SECRET_ACCESS_KEY").is_ok()
        }
        #[cfg(not(feature = "amazon_ses"))]
        "amazon_ses" => {
            tracing::warn!(
                "EMAIL_PROVIDER=amazon_ses but 'amazon_ses' feature is not enabled. \
                Add `features = [\"amazon_ses\"]` to Cargo.toml"
            );
            false
        }

        #[cfg(feature = "mailtrap")]
        "mailtrap" => env::var("MAILTRAP_API_KEY").is_ok(),
        #[cfg(not(feature = "mailtrap"))]
        "mailtrap" => {
            tracing::warn!(
                "EMAIL_PROVIDER=mailtrap but 'mailtrap' feature is not enabled. \
                Add `features = [\"mailtrap\"]` to Cargo.toml"
            );
            false
        }

        #[cfg(feature = "local")]
        "local" => true,
        #[cfg(not(feature = "local"))]
        "local" => {
            tracing::warn!(
                "EMAIL_PROVIDER=local but 'local' feature is not enabled. \
                Add `features = [\"local\"]` to Cargo.toml"
            );
            false
        }

        "logger" | "logger_full" => true,

        _ => false,
    }
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

/// Validate an email has required fields.
fn validate(email: &Email) -> Result<(), MailError> {
    if email.from.is_none() && default_from().is_none() {
        return Err(MailError::MissingField("from"));
    }
    if email.to.is_empty() {
        return Err(MailError::MissingField("to"));
    }
    Ok(())
}

/// Prepare email by adding default from address if needed.
fn prepare_email(email: &Email) -> Email {
    if email.from.is_none() {
        if let Some(from) = default_from() {
            let mut e = email.clone();
            e.from = Some(from);
            return e;
        }
    }
    email.clone()
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
    // Validate required fields early
    validate(email)?;

    let mailer = get_mailer()?;
    let provider = mailer.provider_name();
    let email = prepare_email(email);

    // Emit telemetry span
    let span = tracing::info_span!(
        "missive.deliver",
        provider = provider,
        to = ?email.to.iter().map(|a| &a.email).collect::<Vec<_>>(),
        subject = %email.subject,
    );
    let _guard = span.enter();

    tracing::debug!("Delivering email");

    #[cfg(feature = "metrics")]
    let start = Instant::now();

    let result = mailer.deliver(&email).await;

    // Record metrics
    #[cfg(feature = "metrics")]
    {
        let duration = start.elapsed().as_secs_f64();
        let status = if result.is_ok() { "success" } else { "error" };
        metrics::counter!("missive_emails_total", "provider" => provider, "status" => status)
            .increment(1);
        metrics::histogram!("missive_delivery_duration_seconds", "provider" => provider)
            .record(duration);
    }

    match &result {
        Ok(r) => tracing::info!(message_id = %r.message_id, "Email delivered"),
        Err(e) => tracing::error!(error = %e, "Email delivery failed"),
    }

    result
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
    // Validate required fields early
    validate(email)?;

    let provider = mailer.provider_name();
    let email = prepare_email(email);

    // Emit telemetry span
    let span = tracing::info_span!(
        "missive.deliver",
        provider = provider,
        to = ?email.to.iter().map(|a| &a.email).collect::<Vec<_>>(),
        subject = %email.subject,
    );
    let _guard = span.enter();

    tracing::debug!("Delivering email");

    #[cfg(feature = "metrics")]
    let start = Instant::now();

    let result = mailer.deliver(&email).await;

    // Record metrics
    #[cfg(feature = "metrics")]
    {
        let duration = start.elapsed().as_secs_f64();
        let status = if result.is_ok() { "success" } else { "error" };
        metrics::counter!("missive_emails_total", "provider" => provider, "status" => status)
            .increment(1);
        metrics::histogram!("missive_delivery_duration_seconds", "provider" => provider)
            .record(duration);
    }

    match &result {
        Ok(r) => tracing::info!(message_id = %r.message_id, "Email delivered"),
        Err(e) => tracing::error!(error = %e, "Email delivery failed"),
    }

    result
}

/// Deliver multiple emails using the global mailer.
pub async fn deliver_many(emails: &[Email]) -> Result<Vec<DeliveryResult>, MailError> {
    // Validate all emails first
    for email in emails {
        validate(email)?;
    }

    let mailer = get_mailer()?;
    let provider = mailer.provider_name();
    let count = emails.len();
    let emails: Vec<Email> = emails.iter().map(prepare_email).collect();

    let span = tracing::info_span!("missive.deliver_many", provider = provider, count = count,);
    let _guard = span.enter();

    #[cfg(feature = "metrics")]
    let start = Instant::now();

    let result = mailer.deliver_many(&emails).await;

    // Record metrics
    #[cfg(feature = "metrics")]
    {
        let duration = start.elapsed().as_secs_f64();
        let status = if result.is_ok() { "success" } else { "error" };
        metrics::counter!("missive_emails_total", "provider" => provider, "status" => status)
            .increment(count as u64);
        metrics::counter!("missive_batch_total", "provider" => provider, "status" => status)
            .increment(1);
        metrics::histogram!("missive_delivery_duration_seconds", "provider" => provider, "batch" => "true").record(duration);
        metrics::histogram!("missive_batch_size", "provider" => provider).record(count as f64);
    }

    result
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
    let mut guard = MAILER.write();
    *guard = Some(Arc::new(mailer));
}

/// Configure with an Arc'd mailer.
pub fn configure_arc(mailer: Arc<dyn Mailer>) {
    let mut guard = MAILER.write();
    *guard = Some(mailer);
}

/// Reset the global mailer (useful for tests).
///
/// After calling this, the next `deliver()` will re-initialize from env vars.
pub fn reset() {
    let mut guard = MAILER.write();
    *guard = None;
}

/// Get a reference to the configured mailer (if initialized).
pub fn mailer() -> Option<Arc<dyn Mailer>> {
    let guard = MAILER.read();
    guard.as_ref().cloned()
}

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::Address;
    pub use crate::Attachment;
    pub use crate::DeliveryResult;
    pub use crate::Email;
    pub use crate::MailError;
    pub use crate::Mailer;
    pub use crate::ToAddress;
    pub use crate::{default_from, deliver, deliver_many, deliver_with, is_configured};

    #[cfg(feature = "local")]
    pub use crate::Storage;
}
