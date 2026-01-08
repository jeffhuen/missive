//! Askama template integration for emails.
//!
//! Provides a trait for creating emails from Askama templates.
//!
//! # Example
//!
//! ```rust,ignore
//! use askama::Template;
//! use missive::{Email, EmailTemplate, Address};
//!
//! #[derive(Template)]
//! #[template(path = "emails/welcome.html")]
//! struct WelcomeEmail {
//!     user_name: String,
//!     to_email: String,
//! }
//!
//! impl EmailTemplate for WelcomeEmail {
//!     fn subject(&self) -> String {
//!         format!("Welcome, {}!", self.user_name)
//!     }
//!
//!     fn to(&self) -> Address {
//!         self.to_email.as_str().into()
//!     }
//! }
//!
//! // Convert to Email
//! let email: Email = WelcomeEmail {
//!     user_name: "Alice".to_string(),
//!     to_email: "alice@example.com".to_string(),
//! }.into_email()?;
//!
//! mailer.deliver(&email).await?;
//! ```

use askama::Template;

use crate::address::Address;
use crate::email::Email;
use crate::error::MailError;

/// Trait for email templates.
///
/// Implement this trait on your Askama templates to easily convert them to `Email`.
pub trait EmailTemplate: Template {
    /// The email subject line.
    fn subject(&self) -> String;

    /// The primary recipient.
    fn to(&self) -> Address;

    /// Optional sender address.
    /// Override this to set a specific from address.
    fn from(&self) -> Option<Address> {
        None
    }

    /// Optional reply-to address.
    fn reply_to(&self) -> Option<Address> {
        None
    }

    /// Optional CC recipients.
    fn cc(&self) -> Vec<Address> {
        Vec::new()
    }

    /// Optional BCC recipients.
    fn bcc(&self) -> Vec<Address> {
        Vec::new()
    }

    /// Convert this template into an `Email`.
    ///
    /// The template is rendered as the HTML body.
    fn into_email(self) -> Result<Email, MailError>
    where
        Self: Sized,
    {
        let html = self
            .render()
            .map_err(|e| MailError::TemplateError(e.to_string()))?;

        let mut email = Email::new().subject(&self.subject()).to(self.to()).html_body(&html);

        if let Some(from) = self.from() {
            email = email.from(from);
        }

        if let Some(reply_to) = self.reply_to() {
            email = email.reply_to(reply_to);
        }

        for cc in self.cc() {
            email = email.cc(cc);
        }

        for bcc in self.bcc() {
            email = email.bcc(bcc);
        }

        Ok(email)
    }
}

/// Extension trait for rendering templates with a text fallback.
pub trait EmailTemplateExt: EmailTemplate {
    /// Convert to email with both HTML and plain text bodies.
    ///
    /// The `text_body` parameter provides the plain text version.
    fn into_email_with_text(self, text_body: &str) -> Result<Email, MailError>
    where
        Self: Sized,
    {
        self.into_email().map(|e| e.text_body(text_body))
    }
}

// Blanket implementation
impl<T: EmailTemplate> EmailTemplateExt for T {}
