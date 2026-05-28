//! Email provider implementations.
//!
//! Each provider implements the [`Mailer`](crate::Mailer) trait.
//!
//! ## Available Providers
//!
//! | Provider | Feature Flag | Description |
//! |----------|-------------|-------------|
//! | [`SmtpMailer`] | `smtp` | SMTP via lettre |
//! | [`ResendMailer`] | `resend` | Resend API |
//! | [`UnsentMailer`] | `unsent` | Unsent API |
//! | [`PostmarkMailer`] | `postmark` | Postmark API |
//! | [`SendGridMailer`] | `sendgrid` | SendGrid API |
//! | [`BrevoMailer`] | `brevo` | Brevo API (formerly Sendinblue) |
//! | [`MailgunMailer`] | `mailgun` | Mailgun API |
//! | [`AmazonSesMailer`] | `amazon_ses` | Amazon SES API |
//! | [`MailtrapMailer`] | `mailtrap` | Mailtrap API (testing/staging) |
//! | [`MailjetMailer`] | `mailjet` | Mailjet API |
//! | [`SocketLabsMailer`] | `socketlabs` | SocketLabs Injection API |
//! | [`GmailMailer`] | `gmail` | Gmail API (OAuth2) |
//! | [`ProtonBridgeMailer`] | `protonbridge` | [Proton Bridge](https://proton.me/mail/bridge) (local SMTP) |
//! | [`JmapMailer`] | `jmap` | JMAP protocol (Stalwart, Fastmail, etc.) |
//! | [`LocalMailer`] | `local` | In-memory storage for dev/testing |
//! | [`LoggerMailer`] | (none) | Logs emails without storing |

#[cfg(all(
    feature = "smtp",
    not(all(target_family = "wasm", target_os = "unknown"))
))]
mod smtp;
#[cfg(all(
    feature = "smtp",
    not(all(target_family = "wasm", target_os = "unknown"))
))]
pub use smtp::{SmtpMailer, TlsMode};

#[cfg(feature = "resend")]
mod resend;
#[cfg(feature = "resend")]
pub use resend::{ResendEmailExt, ResendMailer, ResendTag};

#[cfg(feature = "unsent")]
mod unsent;
#[cfg(feature = "unsent")]
pub use unsent::UnsentMailer;

#[cfg(feature = "postmark")]
mod postmark;
#[cfg(feature = "postmark")]
pub use postmark::PostmarkMailer;

#[cfg(feature = "sendgrid")]
mod sendgrid;
#[cfg(feature = "sendgrid")]
pub use sendgrid::SendGridMailer;

#[cfg(feature = "brevo")]
mod brevo;
#[cfg(feature = "brevo")]
pub use brevo::BrevoMailer;

#[cfg(all(
    feature = "mailgun",
    not(all(target_family = "wasm", target_os = "unknown"))
))]
mod mailgun;
#[cfg(all(
    feature = "mailgun",
    not(all(target_family = "wasm", target_os = "unknown"))
))]
pub use mailgun::MailgunMailer;

#[cfg(feature = "amazon_ses")]
mod amazon_ses;
#[cfg(feature = "amazon_ses")]
pub use amazon_ses::AmazonSesMailer;

#[cfg(feature = "mailtrap")]
mod mailtrap;
#[cfg(feature = "mailtrap")]
pub use mailtrap::MailtrapMailer;

#[cfg(feature = "mailjet")]
mod mailjet;
#[cfg(feature = "mailjet")]
pub use mailjet::MailjetMailer;

#[cfg(feature = "socketlabs")]
mod socketlabs;
#[cfg(feature = "socketlabs")]
pub use socketlabs::SocketLabsMailer;

#[cfg(all(
    feature = "gmail",
    not(all(target_family = "wasm", target_os = "unknown"))
))]
mod gmail;
#[cfg(all(
    feature = "gmail",
    not(all(target_family = "wasm", target_os = "unknown"))
))]
pub use gmail::GmailMailer;

#[cfg(all(
    feature = "protonbridge",
    not(all(target_family = "wasm", target_os = "unknown"))
))]
mod protonbridge;
#[cfg(all(
    feature = "protonbridge",
    not(all(target_family = "wasm", target_os = "unknown"))
))]
pub use protonbridge::ProtonBridgeMailer;

#[cfg(feature = "jmap")]
mod jmap;
#[cfg(feature = "jmap")]
pub use jmap::JmapMailer;

#[cfg(feature = "local")]
mod local;
#[cfg(feature = "local")]
pub use local::LocalMailer;

mod logger;
pub use logger::LoggerMailer;
