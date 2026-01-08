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
//! | [`LocalMailer`] | `local` | In-memory storage for dev/testing |
//! | [`LoggerMailer`] | (none) | Logs emails without storing |

#[cfg(feature = "smtp")]
mod smtp;
#[cfg(feature = "smtp")]
pub use smtp::SmtpMailer;

#[cfg(feature = "resend")]
mod resend;
#[cfg(feature = "resend")]
pub use resend::ResendMailer;

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

#[cfg(feature = "mailgun")]
mod mailgun;
#[cfg(feature = "mailgun")]
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

#[cfg(feature = "local")]
mod local;
#[cfg(feature = "local")]
pub use local::LocalMailer;

mod logger;
pub use logger::LoggerMailer;
