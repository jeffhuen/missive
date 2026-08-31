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

#[cfg(feature = "_http")]
const DEFAULT_HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

#[cfg(feature = "_http")]
#[derive(Clone)]
struct HttpClient {
    inner: reqwest::Client,
    timeout: Option<std::time::Duration>,
}

#[cfg(feature = "_http")]
impl HttpClient {
    #[cfg(any(feature = "gmail", feature = "jmap", test))]
    fn get<U: reqwest::IntoUrl>(&self, url: U) -> reqwest::RequestBuilder {
        self.with_timeout(self.inner.get(url))
    }

    fn post<U: reqwest::IntoUrl>(&self, url: U) -> reqwest::RequestBuilder {
        self.with_timeout(self.inner.post(url))
    }

    fn with_timeout(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match self.timeout {
            Some(timeout) => request.timeout(timeout),
            None => request,
        }
    }
}

#[cfg(feature = "_http")]
impl From<reqwest::Client> for HttpClient {
    fn from(inner: reqwest::Client) -> Self {
        Self {
            inner,
            timeout: None,
        }
    }
}

#[cfg(feature = "_http")]
fn default_http_client() -> HttpClient {
    HttpClient {
        inner: reqwest::Client::new(),
        timeout: Some(DEFAULT_HTTP_TIMEOUT),
    }
}

#[cfg(all(test, feature = "_http"))]
mod tests {
    use super::{default_http_client, HttpClient, DEFAULT_HTTP_TIMEOUT};

    #[test]
    fn default_http_client_sets_a_request_timeout() {
        let request = default_http_client()
            .get("https://example.com")
            .build()
            .unwrap();
        assert_eq!(request.timeout(), Some(&DEFAULT_HTTP_TIMEOUT));

        let custom = HttpClient::from(reqwest::Client::new());
        let request = custom.get("https://example.com").build().unwrap();
        assert_eq!(request.timeout(), None);
    }
}

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
