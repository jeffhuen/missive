//! Error types for missive.

use thiserror::Error;

/// Errors that can occur when sending emails.
#[derive(Debug, Clone, Error)]
pub enum MailError {
    /// Email provider is not configured.
    #[error("Email provider not configured")]
    NotConfigured,

    /// Configuration error (missing env var, invalid value, etc.)
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Missing required field (e.g., from address).
    #[error("Missing required field: {0}")]
    MissingField(&'static str),

    /// Invalid email address format.
    #[error("Invalid email address: {0}")]
    InvalidAddress(String),

    /// Error reading or processing attachment (generic).
    #[error("Attachment error: {0}")]
    AttachmentError(String),

    /// Attachment has no content (neither data nor path provided).
    #[error("Attachment has no content: {0}")]
    AttachmentMissingContent(String),

    /// Attachment file not found.
    #[error("Attachment file not found: {0}")]
    AttachmentFileNotFound(String),

    /// Failed to read attachment file.
    #[error("Failed to read attachment: {0}")]
    AttachmentReadError(String),

    /// Error building the email message.
    #[error("Build error: {0}")]
    BuildError(String),

    /// Error sending the email.
    #[error("Send error: {0}")]
    SendError(String),

    /// Unsupported feature for this adapter.
    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),

    /// Provider-specific error with details.
    #[error("Provider error ({provider}): {message}")]
    ProviderError {
        provider: &'static str,
        message: String,
        /// Optional HTTP status code
        status: Option<u16>,
    },

    /// HTTP request failed.
    #[error("HTTP error: {0}")]
    HttpError(String),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    JsonError(String),

    /// Template rendering error.
    #[error("Template error: {0}")]
    TemplateError(String),

    /// Generic internal error.
    #[error("Internal error: {0}")]
    Internal(String),
}

impl MailError {
    /// Create a provider-specific error.
    pub fn provider(provider: &'static str, message: impl Into<String>) -> Self {
        Self::ProviderError {
            provider,
            message: message.into(),
            status: None,
        }
    }

    /// Create a provider error with HTTP status.
    pub fn provider_with_status(
        provider: &'static str,
        message: impl Into<String>,
        status: u16,
    ) -> Self {
        Self::ProviderError {
            provider,
            message: message.into(),
            status: Some(status),
        }
    }
}

#[cfg(feature = "_http")]
impl From<reqwest::Error> for MailError {
    fn from(err: reqwest::Error) -> Self {
        Self::HttpError(err.to_string())
    }
}

impl From<serde_json::Error> for MailError {
    fn from(err: serde_json::Error) -> Self {
        Self::JsonError(err.to_string())
    }
}

#[cfg(feature = "smtp")]
impl From<lettre::error::Error> for MailError {
    fn from(err: lettre::error::Error) -> Self {
        Self::SendError(err.to_string())
    }
}

#[cfg(feature = "smtp")]
impl From<lettre::transport::smtp::Error> for MailError {
    fn from(err: lettre::transport::smtp::Error) -> Self {
        Self::SendError(err.to_string())
    }
}

#[cfg(feature = "smtp")]
impl From<lettre::address::AddressError> for MailError {
    fn from(err: lettre::address::AddressError) -> Self {
        Self::InvalidAddress(err.to_string())
    }
}
