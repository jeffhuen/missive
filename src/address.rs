//! Email address type with optional display name.

use crate::error::MailError;
use email_address::EmailAddress;
use serde::{Deserialize, Serialize};
use std::fmt;

/// An email address with an optional display name.
///
/// # Examples
///
/// ```
/// use missive::Address;
///
/// // From email string
/// let addr: Address = "user@example.com".into();
/// assert_eq!(addr.email, "user@example.com");
/// assert_eq!(addr.name, None);
///
/// // From tuple (name, email)
/// let addr: Address = ("Alice", "alice@example.com").into();
/// assert_eq!(addr.email, "alice@example.com");
/// assert_eq!(addr.name, Some("Alice".to_string()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
    /// Optional display name (e.g., "Alice Smith")
    pub name: Option<String>,
    /// Email address (e.g., "alice@example.com")
    pub email: String,
}

impl Address {
    /// Create a new address with just an email.
    ///
    /// This performs a basic sanity check (non-empty, contains @) and logs
    /// a warning if the email looks invalid. For strict validation, use
    /// [`Address::parse`] instead.
    pub fn new(email: impl Into<String>) -> Self {
        let email = email.into();

        // Basic sanity check - log warning for obviously invalid emails
        if !Self::basic_sanity_check(&email) {
            tracing::warn!(
                email = %email,
                "Creating address with potentially invalid email. Use Address::parse() for strict validation."
            );
        }

        Self { name: None, email }
    }

    /// Create a new address with a name and email.
    ///
    /// This performs a basic sanity check (non-empty, contains @) and logs
    /// a warning if the email looks invalid. For strict validation, use
    /// [`Address::parse_with_name`] instead.
    pub fn with_name(name: impl Into<String>, email: impl Into<String>) -> Self {
        let email = email.into();

        // Basic sanity check - log warning for obviously invalid emails
        if !Self::basic_sanity_check(&email) {
            tracing::warn!(
                email = %email,
                "Creating address with potentially invalid email. Use Address::parse_with_name() for strict validation."
            );
        }

        Self {
            name: Some(name.into()),
            email,
        }
    }

    /// Perform basic sanity check on an email address.
    ///
    /// Returns true if the email passes basic checks (non-empty, contains @).
    /// This is NOT a full validation - use `Address::parse()` for that.
    fn basic_sanity_check(email: &str) -> bool {
        !email.is_empty() && email.contains('@')
    }

    /// Set the display name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Parse and validate an email address.
    ///
    /// Uses RFC 5321/5322 compliant validation. Returns an error if the
    /// email address is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use missive::Address;
    ///
    /// // Valid address
    /// let addr = Address::parse("user@example.com").unwrap();
    /// assert_eq!(addr.email, "user@example.com");
    ///
    /// // Invalid address
    /// assert!(Address::parse("not-an-email").is_err());
    /// assert!(Address::parse("").is_err());
    /// ```
    pub fn parse(email: &str) -> Result<Self, MailError> {
        // Validate using email_address crate
        if !EmailAddress::is_valid(email) {
            return Err(MailError::InvalidAddress(format!(
                "'{}' is not a valid email address",
                email
            )));
        }

        Ok(Self {
            name: None,
            email: email.to_string(),
        })
    }

    /// Parse and validate an email address with a display name.
    ///
    /// # Examples
    ///
    /// ```
    /// use missive::Address;
    ///
    /// let addr = Address::parse_with_name("Alice", "alice@example.com").unwrap();
    /// assert_eq!(addr.email, "alice@example.com");
    /// assert_eq!(addr.name, Some("Alice".to_string()));
    ///
    /// // Invalid email
    /// assert!(Address::parse_with_name("Alice", "not-valid").is_err());
    /// ```
    pub fn parse_with_name(name: &str, email: &str) -> Result<Self, MailError> {
        // Validate using email_address crate
        if !EmailAddress::is_valid(email) {
            return Err(MailError::InvalidAddress(format!(
                "'{}' is not a valid email address",
                email
            )));
        }

        Ok(Self {
            name: if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            },
            email: email.to_string(),
        })
    }

    /// Convert the domain part of the email address to ASCII (Punycode).
    ///
    /// This is useful for international domain names (IDN) that contain
    /// non-ASCII characters. The local part (before @) is preserved as-is.
    ///
    /// # Examples
    ///
    /// ```
    /// use missive::Address;
    ///
    /// // Japanese domain
    /// let addr = Address::new("user@例え.jp");
    /// assert_eq!(addr.to_ascii().unwrap(), "user@xn--r8jz45g.jp");
    ///
    /// // Already ASCII domain
    /// let addr = Address::new("user@example.com");
    /// assert_eq!(addr.to_ascii().unwrap(), "user@example.com");
    /// ```
    pub fn to_ascii(&self) -> Result<String, MailError> {
        let parts: Vec<&str> = self.email.splitn(2, '@').collect();
        if parts.len() != 2 {
            return Err(MailError::InvalidAddress(format!(
                "'{}' is missing @ symbol",
                self.email
            )));
        }

        let local_part = parts[0];
        let domain = parts[1];

        // Convert domain to ASCII using IDNA
        let ascii_domain = idna::domain_to_ascii(domain).map_err(|e| {
            MailError::InvalidAddress(format!(
                "Failed to convert domain '{}' to ASCII: {:?}",
                domain, e
            ))
        })?;

        Ok(format!("{}@{}", local_part, ascii_domain))
    }

    /// Format with ASCII-encoded domain (Punycode for IDN).
    ///
    /// Like `formatted()` but converts international domain names to ASCII.
    /// Use this when sending emails through SMTP or other protocols that
    /// require ASCII domain names.
    ///
    /// # Examples
    ///
    /// ```
    /// use missive::Address;
    ///
    /// let addr = Address::with_name("User", "user@例え.jp");
    /// assert_eq!(addr.formatted_ascii().unwrap(), "User <user@xn--r8jz45g.jp>");
    /// ```
    pub fn formatted_ascii(&self) -> Result<String, MailError> {
        let ascii_email = self.to_ascii()?;
        match &self.name {
            Some(name) if name.is_empty() => Ok(ascii_email),
            Some(name) => Ok(format!("{} <{}>", name, ascii_email)),
            None => Ok(ascii_email),
        }
    }

    /// Format according to RFC 5322 with ASCII-encoded domain.
    ///
    /// Combines RFC 5322 escaping with IDN/Punycode conversion.
    pub fn formatted_rfc5322_ascii(&self) -> Result<String, MailError> {
        let ascii_email = self.to_ascii()?;
        match &self.name {
            Some(name) if name.is_empty() => Ok(ascii_email),
            Some(name) => {
                // Escape backslashes first, then quotes
                let escaped = name.replace('\\', "\\\\").replace('"', "\\\"");
                Ok(format!("\"{}\" <{}>", escaped, ascii_email))
            }
            None => Ok(ascii_email),
        }
    }

    /// Format as "Name <email>" or just "email" if no name.
    ///
    /// For simple names without special characters, returns `Name <email>`.
    /// For names with special chars, use `formatted_rfc5322()` for proper quoting.
    pub fn formatted(&self) -> String {
        match &self.name {
            Some(name) if name.is_empty() => self.email.clone(),
            Some(name) => format!("{} <{}>", name, self.email),
            None => self.email.clone(),
        }
    }

    /// Format according to RFC 5322 with proper escaping.
    ///
    /// This method:
    /// - Escapes backslashes: `\` → `\\`
    /// - Escapes double quotes: `"` → `\"`
    /// - Wraps the name in double quotes: `"Name" <email>`
    ///
    /// This is the format that should be used in email headers.
    pub fn formatted_rfc5322(&self) -> String {
        match &self.name {
            Some(name) if name.is_empty() => self.email.clone(),
            Some(name) => {
                // Escape backslashes first, then quotes
                let escaped = name.replace('\\', "\\\\").replace('"', "\\\"");
                format!("\"{}\" <{}>", escaped, self.email)
            }
            None => self.email.clone(),
        }
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.formatted())
    }
}

// From &str - just email
impl From<&str> for Address {
    fn from(email: &str) -> Self {
        Self::new(email)
    }
}

// From String - just email
impl From<String> for Address {
    fn from(email: String) -> Self {
        Self::new(email)
    }
}

// From tuple (&str, &str) - (name, email)
impl From<(&str, &str)> for Address {
    fn from((name, email): (&str, &str)) -> Self {
        Self::with_name(name, email)
    }
}

// From tuple (String, String) - (name, email)
impl From<(String, String)> for Address {
    fn from((name, email): (String, String)) -> Self {
        Self::with_name(name, email)
    }
}

// From tuple (&str, String)
impl From<(&str, String)> for Address {
    fn from((name, email): (&str, String)) -> Self {
        Self::with_name(name, email)
    }
}

// From tuple (String, &str)
impl From<(String, &str)> for Address {
    fn from((name, email): (String, &str)) -> Self {
        Self::with_name(name, email)
    }
}

/// Trait for types that can be converted to an email address.
///
/// Implement this trait for your custom types to use them directly
/// in email builder methods.
///
/// # Example
///
/// ```rust
/// use missive::{Address, ToAddress};
///
/// struct User {
///     name: String,
///     email: String,
/// }
///
/// impl ToAddress for User {
///     fn to_address(&self) -> Address {
///         Address::with_name(&self.name, &self.email)
///     }
/// }
///
/// // Now you can use User directly:
/// // let email = Email::new().to(&user);
/// ```
pub trait ToAddress {
    fn to_address(&self) -> Address;
}

// Blanket implementation for references to types that implement ToAddress
impl<T: ToAddress + ?Sized> ToAddress for &T {
    fn to_address(&self) -> Address {
        (*self).to_address()
    }
}

// Implement for Address itself
impl ToAddress for Address {
    fn to_address(&self) -> Address {
        self.clone()
    }
}

// Implement for string types
impl ToAddress for str {
    fn to_address(&self) -> Address {
        Address::new(self)
    }
}

impl ToAddress for String {
    fn to_address(&self) -> Address {
        Address::new(self)
    }
}

// Implement for tuples (name, email)
impl<N: AsRef<str>, E: AsRef<str>> ToAddress for (N, E) {
    fn to_address(&self) -> Address {
        Address::with_name(self.0.as_ref(), self.1.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        let addr: Address = "test@example.com".into();
        assert_eq!(addr.email, "test@example.com");
        assert_eq!(addr.name, None);
    }

    #[test]
    fn test_from_tuple() {
        let addr: Address = ("Alice", "alice@example.com").into();
        assert_eq!(addr.email, "alice@example.com");
        assert_eq!(addr.name, Some("Alice".to_string()));
    }

    #[test]
    fn test_formatted() {
        let addr = Address::new("test@example.com");
        assert_eq!(addr.formatted(), "test@example.com");

        let addr = Address::with_name("Alice", "alice@example.com");
        assert_eq!(addr.formatted(), "Alice <alice@example.com>");
    }

    #[test]
    fn test_display() {
        let addr = Address::with_name("Bob", "bob@example.com");
        assert_eq!(format!("{}", addr), "Bob <bob@example.com>");
    }

    #[test]
    fn test_formatted_rfc5322() {
        // Simple name
        let addr = Address::with_name("Alice", "alice@example.com");
        assert_eq!(addr.formatted_rfc5322(), "\"Alice\" <alice@example.com>");

        // Name with quotes
        let addr = Address::with_name("Alice \"Ali\" Smith", "alice@example.com");
        assert_eq!(
            addr.formatted_rfc5322(),
            "\"Alice \\\"Ali\\\" Smith\" <alice@example.com>"
        );

        // Name with backslash
        let addr = Address::with_name("Alice\\Bob", "alice@example.com");
        assert_eq!(
            addr.formatted_rfc5322(),
            "\"Alice\\\\Bob\" <alice@example.com>"
        );

        // Empty name
        let addr = Address::with_name("", "alice@example.com");
        assert_eq!(addr.formatted_rfc5322(), "alice@example.com");

        // No name
        let addr = Address::new("alice@example.com");
        assert_eq!(addr.formatted_rfc5322(), "alice@example.com");
    }

    // ========================================================================
    // Tests for Address::parse() - validated parsing
    // ========================================================================

    #[test]
    fn test_parse_valid_email() {
        let addr = Address::parse("user@example.com").unwrap();
        assert_eq!(addr.email, "user@example.com");
        assert_eq!(addr.name, None);
    }

    #[test]
    fn test_parse_valid_email_with_subdomain() {
        let addr = Address::parse("user@mail.example.com").unwrap();
        assert_eq!(addr.email, "user@mail.example.com");
    }

    #[test]
    fn test_parse_valid_email_with_plus() {
        let addr = Address::parse("user+tag@example.com").unwrap();
        assert_eq!(addr.email, "user+tag@example.com");
    }

    #[test]
    fn test_parse_valid_email_with_dots() {
        let addr = Address::parse("user.name@example.com").unwrap();
        assert_eq!(addr.email, "user.name@example.com");
    }

    #[test]
    fn test_parse_invalid_empty() {
        let result = Address::parse("");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, MailError::InvalidAddress(msg) if msg.contains("not a valid email"))
        );
    }

    #[test]
    fn test_parse_invalid_no_at() {
        let result = Address::parse("userexample.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_no_domain() {
        let result = Address::parse("user@");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_no_local() {
        let result = Address::parse("@example.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_multiple_at() {
        let result = Address::parse("user@@example.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_spaces() {
        let result = Address::parse("user @example.com");
        assert!(result.is_err());
    }

    // ========================================================================
    // Tests for Address::parse_with_name()
    // ========================================================================

    #[test]
    fn test_parse_with_name_valid() {
        let addr = Address::parse_with_name("Alice Smith", "alice@example.com").unwrap();
        assert_eq!(addr.email, "alice@example.com");
        assert_eq!(addr.name, Some("Alice Smith".to_string()));
    }

    #[test]
    fn test_parse_with_name_empty_name() {
        let addr = Address::parse_with_name("", "alice@example.com").unwrap();
        assert_eq!(addr.email, "alice@example.com");
        assert_eq!(addr.name, None); // Empty name becomes None
    }

    #[test]
    fn test_parse_with_name_invalid_email() {
        let result = Address::parse_with_name("Alice", "not-valid");
        assert!(result.is_err());
    }

    // ========================================================================
    // Tests for to_ascii() - IDN/Punycode conversion
    // ========================================================================

    #[test]
    fn test_to_ascii_already_ascii() {
        let addr = Address::new("user@example.com");
        assert_eq!(addr.to_ascii().unwrap(), "user@example.com");
    }

    #[test]
    fn test_to_ascii_japanese_domain() {
        // 例え.jp -> xn--r8jz45g.jp
        let addr = Address::new("user@例え.jp");
        assert_eq!(addr.to_ascii().unwrap(), "user@xn--r8jz45g.jp");
    }

    #[test]
    fn test_to_ascii_german_umlaut() {
        // muller.de -> xn--mller-kva.de
        let addr = Address::new("user@müller.de");
        assert_eq!(addr.to_ascii().unwrap(), "user@xn--mller-kva.de");
    }

    #[test]
    fn test_to_ascii_chinese_domain() {
        // Test Chinese domain
        let addr = Address::new("user@中文.com");
        assert_eq!(addr.to_ascii().unwrap(), "user@xn--fiq228c.com");
    }

    #[test]
    fn test_to_ascii_preserves_local_part() {
        // Local part should be preserved as-is, even with special chars
        let addr = Address::new("user+tag@例え.jp");
        assert_eq!(addr.to_ascii().unwrap(), "user+tag@xn--r8jz45g.jp");
    }

    #[test]
    fn test_to_ascii_no_at_symbol() {
        let addr = Address::new("no-at-symbol");
        let result = addr.to_ascii();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, MailError::InvalidAddress(msg) if msg.contains("missing @")));
    }

    // ========================================================================
    // Tests for formatted_ascii()
    // ========================================================================

    #[test]
    fn test_formatted_ascii_no_name() {
        let addr = Address::new("user@例え.jp");
        assert_eq!(addr.formatted_ascii().unwrap(), "user@xn--r8jz45g.jp");
    }

    #[test]
    fn test_formatted_ascii_with_name() {
        let addr = Address::with_name("User", "user@例え.jp");
        assert_eq!(
            addr.formatted_ascii().unwrap(),
            "User <user@xn--r8jz45g.jp>"
        );
    }

    #[test]
    fn test_formatted_ascii_empty_name() {
        let addr = Address::with_name("", "user@例え.jp");
        assert_eq!(addr.formatted_ascii().unwrap(), "user@xn--r8jz45g.jp");
    }

    // ========================================================================
    // Tests for formatted_rfc5322_ascii()
    // ========================================================================

    #[test]
    fn test_formatted_rfc5322_ascii_no_name() {
        let addr = Address::new("user@例え.jp");
        assert_eq!(
            addr.formatted_rfc5322_ascii().unwrap(),
            "user@xn--r8jz45g.jp"
        );
    }

    #[test]
    fn test_formatted_rfc5322_ascii_with_name() {
        let addr = Address::with_name("User Name", "user@例え.jp");
        assert_eq!(
            addr.formatted_rfc5322_ascii().unwrap(),
            "\"User Name\" <user@xn--r8jz45g.jp>"
        );
    }

    #[test]
    fn test_formatted_rfc5322_ascii_escapes_quotes() {
        let addr = Address::with_name("User \"Nick\" Name", "user@例え.jp");
        assert_eq!(
            addr.formatted_rfc5322_ascii().unwrap(),
            "\"User \\\"Nick\\\" Name\" <user@xn--r8jz45g.jp>"
        );
    }

    // ========================================================================
    // Tests for basic_sanity_check
    // ========================================================================

    #[test]
    fn test_basic_sanity_check_valid() {
        assert!(Address::basic_sanity_check("user@example.com"));
        assert!(Address::basic_sanity_check("a@b"));
    }

    #[test]
    fn test_basic_sanity_check_empty() {
        assert!(!Address::basic_sanity_check(""));
    }

    #[test]
    fn test_basic_sanity_check_no_at() {
        assert!(!Address::basic_sanity_check("userexample.com"));
    }
}
