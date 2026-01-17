//! Proton Bridge provider for sending emails through a local Proton Mail Bridge.
//!
//! This is a thin wrapper around the SMTP adapter, pre-configured to connect to
//! a locally running [Proton Bridge](https://proton.me/mail/bridge) instance.
//!
//! # How It Works
//!
//! Proton Bridge is a desktop application that runs a local SMTP/IMAP server,
//! acting as a bridge between standard email clients and Proton Mail's encrypted
//! infrastructure:
//!
//! ```text
//! Your App → localhost:1025 → Proton Bridge → Proton Mail servers
//! ```
//!
//! The Bridge handles:
//! - Authentication with Proton's servers
//! - PGP encryption (Proton's end-to-end encryption)
//! - Queuing and delivery to Proton Mail
//!
//! # Prerequisites
//!
//! 1. Install [Proton Bridge](https://proton.me/mail/bridge)
//! 2. Log in to your Proton Mail account in the Bridge app
//! 3. Get the SMTP credentials from Bridge (username/password)
//! 4. Keep Bridge running while sending emails
//!
//! # Example
//!
//! ```rust,ignore
//! use missive::providers::ProtonBridgeMailer;
//!
//! // Using Bridge credentials from the desktop app
//! let mailer = ProtonBridgeMailer::new("your-bridge-username", "your-bridge-password");
//!
//! // Custom port (default is 1025)
//! let mailer = ProtonBridgeMailer::new("username", "password")
//!     .port(1026);
//! ```
//!
//! # Environment Variables
//!
//! ```bash
//! EMAIL_PROVIDER=protonbridge
//! PROTONBRIDGE_USERNAME=your-bridge-username
//! PROTONBRIDGE_PASSWORD=your-bridge-password
//! PROTONBRIDGE_PORT=1025  # optional, defaults to 1025
//! PROTONBRIDGE_HOST=127.0.0.1  # optional, defaults to 127.0.0.1
//! ```
//!
//! # Security Notes
//!
//! - No TLS is used for the localhost connection (the Bridge handles encryption)
//! - The Bridge credentials are separate from your Proton Mail password
//! - Keep the Bridge running for emails to be delivered

use async_trait::async_trait;

use crate::email::Email;
use crate::error::MailError;
use crate::mailer::{DeliveryResult, Mailer};
use crate::providers::SmtpMailer;

/// Proton Bridge email provider.
///
/// A thin wrapper around [`SmtpMailer`] pre-configured for Proton Bridge's
/// local SMTP server (default: `127.0.0.1:1025`, no TLS).
///
/// See [Proton Bridge](https://proton.me/mail/bridge) for setup instructions.
pub struct ProtonBridgeMailer {
    inner: SmtpMailer,
}

impl ProtonBridgeMailer {
    /// Create a new Proton Bridge mailer with the given credentials.
    ///
    /// The credentials are provided by the Proton Bridge desktop app after
    /// logging in with your Proton Mail account.
    ///
    /// Uses default host `127.0.0.1` and port `1025`.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(username: &str, password: &str) -> ProtonBridgeBuilder {
        ProtonBridgeBuilder {
            host: "127.0.0.1".to_string(),
            port: 1025,
            username: username.to_string(),
            password: password.to_string(),
        }
    }
}

/// Builder for ProtonBridgeMailer.
pub struct ProtonBridgeBuilder {
    host: String,
    port: u16,
    username: String,
    password: String,
}

impl ProtonBridgeBuilder {
    /// Set the Bridge host (default: `127.0.0.1`).
    ///
    /// Only change this if Bridge is running on a different machine.
    pub fn host(mut self, host: &str) -> Self {
        self.host = host.to_string();
        self
    }

    /// Set the Bridge SMTP port (default: `1025`).
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Build the ProtonBridgeMailer.
    pub fn build(self) -> ProtonBridgeMailer {
        let inner = SmtpMailer::new(&self.host, self.port)
            .no_tls()
            .credentials(&self.username, &self.password)
            .build();

        ProtonBridgeMailer { inner }
    }
}

#[async_trait]
impl Mailer for ProtonBridgeMailer {
    async fn deliver(&self, email: &Email) -> Result<DeliveryResult, MailError> {
        self.inner.deliver(email).await
    }

    async fn deliver_many(&self, emails: &[Email]) -> Result<Vec<DeliveryResult>, MailError> {
        self.inner.deliver_many(emails).await
    }

    fn provider_name(&self) -> &'static str {
        "protonbridge"
    }
}
