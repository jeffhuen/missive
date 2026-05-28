//! Email struct with builder pattern.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Deref;

use crate::address::{Address, ToAddress};
use crate::attachment::Attachment;
use crate::error::MailError;

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
    pub(crate) from: Option<Address>,
    /// Primary recipients
    pub(crate) to: Vec<Address>,
    /// Carbon copy recipients
    pub(crate) cc: Vec<Address>,
    /// Blind carbon copy recipients
    pub(crate) bcc: Vec<Address>,
    /// Reply-to addresses (supports multiple)
    pub(crate) reply_to: Vec<Address>,
    /// Email subject line
    pub(crate) subject: String,
    /// Plain text body
    pub(crate) text_body: Option<String>,
    /// HTML body
    pub(crate) html_body: Option<String>,
    /// File attachments
    pub(crate) attachments: Vec<Attachment>,
    /// Custom email headers
    pub(crate) headers: HashMap<String, String>,
    /// Template variables for use with templating systems.
    pub(crate) assigns: HashMap<String, serde_json::Value>,
    /// Private storage for libraries/frameworks (e.g., template paths, metadata).
    pub(crate) private: HashMap<String, serde_json::Value>,
    /// Provider-specific options (e.g., tracking, tags, templates)
    pub(crate) provider_options: HashMap<String, serde_json::Value>,
}

/// An email that has passed Missive's shared delivery validation.
///
/// Provider implementations receive `PreparedEmail` values so direct provider
/// calls, `EmailClient`, and compatibility facades all pass through Missive's
/// shared or provider-specific validation before adapter serialization runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedEmail {
    email: Email,
}

impl PreparedEmail {
    /// Validate an email and convert it into a prepared message.
    pub fn new(email: Email) -> Result<Self, MailError> {
        Self::with_default_from(email, None)
    }

    /// Apply an optional default sender, then validate the email.
    pub fn with_default_from(
        mut email: Email,
        default_from: Option<Address>,
    ) -> Result<Self, MailError> {
        if email.from.is_none() {
            email.from = default_from;
        }

        validate_required_fields(&email)?;
        Ok(Self { email })
    }

    /// Borrow the underlying email.
    pub fn as_email(&self) -> &Email {
        &self.email
    }

    /// Consume the prepared wrapper and return the underlying email.
    pub fn into_inner(self) -> Email {
        self.email
    }

    #[cfg_attr(not(feature = "sendgrid"), allow(dead_code))]
    pub(crate) fn from_validated(email: Email) -> Self {
        Self { email }
    }
}

impl TryFrom<Email> for PreparedEmail {
    type Error = MailError;

    fn try_from(email: Email) -> Result<Self, Self::Error> {
        Self::new(email)
    }
}

impl Deref for PreparedEmail {
    type Target = Email;

    fn deref(&self) -> &Self::Target {
        self.as_email()
    }
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

    /// Set a raw provider-specific option.
    ///
    /// Prefer provider-specific typed extension traits such as
    /// `ResendEmailExt` where available. This method is an advanced escape
    /// hatch for custom providers or provider features that do not have typed
    /// helpers yet.
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

    /// Borrow the sender address.
    pub fn from_address(&self) -> Option<&Address> {
        self.from.as_ref()
    }

    /// Borrow primary recipients.
    pub fn to_addresses(&self) -> &[Address] {
        &self.to
    }

    /// Borrow CC recipients.
    pub fn cc_addresses(&self) -> &[Address] {
        &self.cc
    }

    /// Borrow BCC recipients.
    pub fn bcc_addresses(&self) -> &[Address] {
        &self.bcc
    }

    /// Borrow Reply-To addresses.
    pub fn reply_to_addresses(&self) -> &[Address] {
        &self.reply_to
    }

    /// Borrow the subject line.
    pub fn subject_line(&self) -> &str {
        &self.subject
    }

    /// Borrow the plain-text body.
    pub fn text_body_content(&self) -> Option<&str> {
        self.text_body.as_deref()
    }

    /// Borrow the HTML body.
    pub fn html_body_content(&self) -> Option<&str> {
        self.html_body.as_deref()
    }

    /// Borrow attachments.
    pub fn attachments(&self) -> &[Attachment] {
        &self.attachments
    }

    /// Borrow custom email headers.
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    /// Borrow template assigns.
    pub fn assigns(&self) -> &HashMap<String, serde_json::Value> {
        &self.assigns
    }

    /// Borrow private framework metadata.
    pub fn private(&self) -> &HashMap<String, serde_json::Value> {
        &self.private
    }

    /// Borrow raw provider-specific options.
    pub fn provider_options(&self) -> &HashMap<String, serde_json::Value> {
        &self.provider_options
    }

    /// Check if the email has all required fields for sending.
    pub fn is_valid(&self) -> bool {
        validate_required_fields(self).is_ok()
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

pub(crate) fn validate_required_fields(email: &Email) -> Result<(), MailError> {
    if email.from.is_none() {
        return Err(MailError::MissingField("from"));
    }
    if email.to.is_empty() {
        return Err(MailError::MissingField("to"));
    }
    Ok(())
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
