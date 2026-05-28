//! Error types for missive.

use thiserror::Error;

/// Errors that can occur when sending emails.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum MailError {
    /// Email provider is not configured.
    #[error("email provider not configured")]
    NotConfigured,

    /// Configuration error (missing env var, invalid value, etc.)
    #[error("configuration error: {0}")]
    Configuration(String),

    /// Missing required field (e.g., from address).
    #[error("missing required field: {0}")]
    MissingField(&'static str),

    /// Invalid email address format.
    #[error("invalid email address: {0}")]
    InvalidAddress(String),

    /// Error reading or processing attachment (generic).
    #[error("attachment error: {0}")]
    AttachmentError(String),

    /// Attachment has no content (neither data nor path provided).
    #[error("attachment has no content: {0}")]
    AttachmentMissingContent(String),

    /// Attachment file not found.
    #[error("attachment file not found: {0}")]
    AttachmentFileNotFound(String),

    /// Failed to read attachment file.
    #[error("failed to read attachment {path}: {source}")]
    AttachmentReadError {
        /// Attachment path that failed to read.
        path: String,
        /// Underlying IO error.
        #[source]
        source: std::io::Error,
    },

    /// Error building the email message.
    #[error("build error: {0}")]
    BuildError(String),

    /// Error sending the email.
    #[error("send error: {0}")]
    SendError(String),

    /// Unsupported feature for this adapter.
    #[error("unsupported feature: {0}")]
    UnsupportedFeature(String),

    /// Provider-specific error with details.
    #[error("provider error ({provider}): {message}")]
    ProviderError {
        provider: &'static str,
        message: String,
        /// Optional HTTP status code
        status: Option<u16>,
    },

    /// HTTP request failed.
    #[cfg(feature = "_http")]
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Template rendering error.
    #[error("template error: {0}")]
    TemplateError(String),

    /// Template rendering failed with a source error.
    #[cfg(feature = "templates")]
    #[error("template error: {0}")]
    TemplateRenderError(#[from] askama::Error),

    /// Email message construction failed with a lettre source error.
    #[cfg(all(
        feature = "smtp",
        not(all(target_family = "wasm", target_os = "unknown"))
    ))]
    #[error("build error: {0}")]
    LettreBuildError(#[from] lettre::error::Error),

    /// SMTP transport failed with a lettre source error.
    #[cfg(all(
        feature = "smtp",
        not(all(target_family = "wasm", target_os = "unknown"))
    ))]
    #[error("send error: {0}")]
    SmtpError(#[from] lettre::transport::smtp::Error),

    /// Lettre address parsing failed.
    #[cfg(all(
        feature = "smtp",
        not(all(target_family = "wasm", target_os = "unknown"))
    ))]
    #[error("invalid email address: {0}")]
    LettreAddressError(#[from] lettre::address::AddressError),

    /// Generic internal error.
    #[error("internal error: {0}")]
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

    /// Stable classification string for logs, metrics, and retry policies.
    pub fn kind(&self) -> &'static str {
        match self {
            MailError::NotConfigured => "not_configured",
            MailError::Configuration(_) => "configuration",
            MailError::MissingField(_) => "missing_field",
            MailError::InvalidAddress(_) => "invalid_address",
            MailError::AttachmentError(_) => "attachment_error",
            MailError::AttachmentMissingContent(_) => "attachment_missing_content",
            MailError::AttachmentFileNotFound(_) => "attachment_file_not_found",
            MailError::AttachmentReadError { .. } => "attachment_read_error",
            MailError::BuildError(_) => "build_error",
            MailError::SendError(_) => "send_error",
            MailError::UnsupportedFeature(_) => "unsupported_feature",
            MailError::ProviderError { .. } => "provider_error",
            #[cfg(feature = "_http")]
            MailError::HttpError(_) => "http_error",
            MailError::JsonError(_) => "json_error",
            MailError::TemplateError(_) => "template_error",
            #[cfg(feature = "templates")]
            MailError::TemplateRenderError(_) => "template_error",
            #[cfg(all(
                feature = "smtp",
                not(all(target_family = "wasm", target_os = "unknown"))
            ))]
            MailError::LettreBuildError(_) => "build_error",
            #[cfg(all(
                feature = "smtp",
                not(all(target_family = "wasm", target_os = "unknown"))
            ))]
            MailError::SmtpError(_) => "send_error",
            #[cfg(all(
                feature = "smtp",
                not(all(target_family = "wasm", target_os = "unknown"))
            ))]
            MailError::LettreAddressError(_) => "invalid_address",
            MailError::Internal(_) => "internal",
        }
    }

    /// HTTP status reported by a provider error, when available.
    pub fn provider_status(&self) -> Option<u16> {
        match self {
            MailError::ProviderError { status, .. } => *status,
            _ => None,
        }
    }
}
