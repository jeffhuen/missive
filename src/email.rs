//! Email struct with builder pattern.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::address::{Address, ToAddress};
use crate::attachment::Attachment;

/// An email message.
///
/// Use the builder pattern to construct emails:
///
/// ```
/// use missive::Email;
///
/// let email = Email::new()
///     .from("sender@example.com")
///     .to("recipient@example.com")
///     .subject("Hello!")
///     .text_body("Plain text content")
///     .html_body("<h1>HTML content</h1>");
/// ```
///
/// ## Fields
///
/// - `from`, `to`, `cc`, `bcc` - Addresses
/// - `reply_to` - Reply-to addresses (supports multiple)
/// - `subject`, `text_body`, `html_body` - Content
/// - `attachments` - File attachments
/// - `headers` - Custom email headers
/// - `assigns` - Template variables (for use with templating systems)
/// - `private` - Private storage for libraries/frameworks
/// - `provider_options` - Provider-specific options (tags, templates, etc.)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Email {
    /// Sender address
    pub from: Option<Address>,
    /// Primary recipients
    pub to: Vec<Address>,
    /// Carbon copy recipients
    pub cc: Vec<Address>,
    /// Blind carbon copy recipients
    pub bcc: Vec<Address>,
    /// Reply-to addresses (supports multiple)
    pub reply_to: Vec<Address>,
    /// Email subject line
    pub subject: String,
    /// Plain text body
    pub text_body: Option<String>,
    /// HTML body
    pub html_body: Option<String>,
    /// File attachments
    pub attachments: Vec<Attachment>,
    /// Custom email headers
    pub headers: HashMap<String, String>,
    /// Template variables for use with templating systems.
    pub assigns: HashMap<String, serde_json::Value>,
    /// Private storage for libraries/frameworks (e.g., template paths, metadata).
    pub private: HashMap<String, serde_json::Value>,
    /// Provider-specific options (e.g., tracking, tags, templates)
    pub provider_options: HashMap<String, serde_json::Value>,
}

impl Email {
    /// Create a new empty email.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the sender address.
    ///
    /// Accepts anything that implements `ToAddress`:
    /// - `"email@example.com"` - just email
    /// - `("Name", "email@example.com")` - name and email
    /// - Custom types that implement `ToAddress`
    pub fn from(mut self, addr: impl ToAddress) -> Self {
        self.from = Some(addr.to_address());
        self
    }

    /// Add a recipient.
    ///
    /// Can be called multiple times to add multiple recipients.
    /// Accepts anything that implements `ToAddress`.
    pub fn to(mut self, addr: impl ToAddress) -> Self {
        self.to.push(addr.to_address());
        self
    }

    /// Replace all recipients.
    pub fn put_to(mut self, addrs: Vec<Address>) -> Self {
        self.to = addrs;
        self
    }

    /// Add a CC recipient.
    /// Accepts anything that implements `ToAddress`.
    pub fn cc(mut self, addr: impl ToAddress) -> Self {
        self.cc.push(addr.to_address());
        self
    }

    /// Replace all CC recipients.
    pub fn put_cc(mut self, addrs: Vec<Address>) -> Self {
        self.cc = addrs;
        self
    }

    /// Add a BCC recipient.
    /// Accepts anything that implements `ToAddress`.
    pub fn bcc(mut self, addr: impl ToAddress) -> Self {
        self.bcc.push(addr.to_address());
        self
    }

    /// Replace all BCC recipients.
    pub fn put_bcc(mut self, addrs: Vec<Address>) -> Self {
        self.bcc = addrs;
        self
    }

    /// Add a reply-to address.
    ///
    /// Can be called multiple times to add multiple reply-to addresses.
    /// Accepts anything that implements `ToAddress`.
    pub fn reply_to(mut self, addr: impl ToAddress) -> Self {
        self.reply_to.push(addr.to_address());
        self
    }

    /// Replace all reply-to addresses.
    pub fn put_reply_to(mut self, addrs: Vec<Address>) -> Self {
        self.reply_to = addrs;
        self
    }

    /// Set the subject line.
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = subject.into();
        self
    }

    /// Set the plain text body.
    pub fn text_body(mut self, body: impl Into<String>) -> Self {
        self.text_body = Some(body.into());
        self
    }

    /// Set the HTML body.
    pub fn html_body(mut self, body: impl Into<String>) -> Self {
        self.html_body = Some(body.into());
        self
    }

    /// Add an attachment.
    pub fn attachment(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Add a custom header.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    /// Set a provider-specific option.
    ///
    /// These are passed to the adapter for provider-specific features
    /// (e.g., SendGrid categories, Postmark tags, Resend templates).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Email::new()
    ///     .provider_option("template_id", "welcome-email")
    ///     .provider_option("tags", vec!["signup", "welcome"])
    /// ```
    pub fn provider_option(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.provider_options.insert(key.into(), value.into());
        self
    }

    /// Store a template variable for use with templating systems.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Email::new()
    ///     .assign("username", "alice")
    ///     .assign("action_url", "https://example.com/verify")
    /// ```
    pub fn assign(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.assigns.insert(key.into(), value.into());
        self
    }

    /// Store a private value for frameworks/libraries.
    ///
    /// Reserved for framework use (e.g., template paths, metadata).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Email::new()
    ///     .put_private("template_path", "emails/welcome.html")
    ///     .put_private("sent_at", chrono::Utc::now().to_rfc3339())
    /// ```
    pub fn put_private(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.private.insert(key.into(), value.into());
        self
    }

    /// Check if the email has all required fields for sending.
    pub fn is_valid(&self) -> bool {
        self.from.is_some() && !self.to.is_empty()
    }

    /// Get all recipients (to + cc + bcc).
    pub fn all_recipients(&self) -> Vec<&Address> {
        self.to
            .iter()
            .chain(self.cc.iter())
            .chain(self.bcc.iter())
            .collect()
    }

    /// Check if the email has any attachments.
    pub fn has_attachments(&self) -> bool {
        !self.attachments.is_empty()
    }

    /// Get inline attachments only.
    pub fn inline_attachments(&self) -> Vec<&Attachment> {
        self.attachments.iter().filter(|a| a.is_inline()).collect()
    }

    /// Get regular (non-inline) attachments only.
    pub fn regular_attachments(&self) -> Vec<&Attachment> {
        self.attachments.iter().filter(|a| !a.is_inline()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder() {
        let email = Email::new()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Test")
            .text_body("Hello");

        assert_eq!(email.from.unwrap().email, "sender@example.com");
        assert_eq!(email.to.len(), 1);
        assert_eq!(email.to[0].email, "recipient@example.com");
        assert_eq!(email.subject, "Test");
        assert_eq!(email.text_body, Some("Hello".to_string()));
    }

    #[test]
    fn test_multiple_recipients() {
        let email = Email::new()
            .to("one@example.com")
            .to("two@example.com")
            .cc("cc@example.com")
            .bcc("bcc@example.com");

        assert_eq!(email.to.len(), 2);
        assert_eq!(email.cc.len(), 1);
        assert_eq!(email.bcc.len(), 1);
        assert_eq!(email.all_recipients().len(), 4);
    }

    #[test]
    fn test_with_name() {
        let email = Email::new().from(("Alice", "alice@example.com"));

        let from = email.from.unwrap();
        assert_eq!(from.email, "alice@example.com");
        assert_eq!(from.name, Some("Alice".to_string()));
    }

    #[test]
    fn test_is_valid() {
        let invalid = Email::new().to("recipient@example.com");
        assert!(!invalid.is_valid());

        let valid = Email::new()
            .from("sender@example.com")
            .to("recipient@example.com");
        assert!(valid.is_valid());
    }

    #[test]
    fn test_headers() {
        let email = Email::new()
            .header("X-Custom", "value")
            .header("X-Priority", "1");

        assert_eq!(email.headers.get("X-Custom"), Some(&"value".to_string()));
        assert_eq!(email.headers.get("X-Priority"), Some(&"1".to_string()));
    }

    #[test]
    fn test_provider_options() {
        let email = Email::new().provider_option("template_id", "welcome-email");

        assert_eq!(
            email.provider_options.get("template_id"),
            Some(&serde_json::json!("welcome-email"))
        );
    }

    #[test]
    fn test_to_address_trait() {
        struct User {
            name: String,
            email: String,
        }

        impl ToAddress for User {
            fn to_address(&self) -> Address {
                Address::with_name(&self.name, &self.email)
            }
        }

        let user = User {
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        };

        let email = Email::new().to(&user);
        assert_eq!(email.to[0].email, "alice@example.com");
        assert_eq!(email.to[0].name, Some("Alice".to_string()));
    }

    #[test]
    fn test_to_address_trait_all_methods() {
        struct Contact {
            name: String,
            email: String,
        }

        impl ToAddress for Contact {
            fn to_address(&self) -> Address {
                Address::with_name(&self.name, &self.email)
            }
        }

        let sender = Contact {
            name: "Sender".to_string(),
            email: "sender@example.com".to_string(),
        };
        let recipient = Contact {
            name: "Recipient".to_string(),
            email: "recipient@example.com".to_string(),
        };
        let cc_contact = Contact {
            name: "CC".to_string(),
            email: "cc@example.com".to_string(),
        };
        let bcc_contact = Contact {
            name: "BCC".to_string(),
            email: "bcc@example.com".to_string(),
        };
        let reply_contact = Contact {
            name: "Reply".to_string(),
            email: "reply@example.com".to_string(),
        };

        let email = Email::new()
            .from(&sender)
            .to(&recipient)
            .cc(&cc_contact)
            .bcc(&bcc_contact)
            .reply_to(&reply_contact);

        assert_eq!(email.from.as_ref().unwrap().email, "sender@example.com");
        assert_eq!(email.to[0].email, "recipient@example.com");
        assert_eq!(email.cc[0].email, "cc@example.com");
        assert_eq!(email.bcc[0].email, "bcc@example.com");
        assert_eq!(email.reply_to[0].email, "reply@example.com");
    }
}
