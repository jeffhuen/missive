//! Email attachments with support for inline and regular attachments.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::MailError;

/// Type of attachment disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AttachmentType {
    /// Regular attachment (shown as downloadable file)
    #[default]
    Attachment,
    /// Inline attachment (embedded in HTML via cid:)
    Inline,
}

/// An email attachment.
///
/// Attachments can be created from bytes (eager) or from a file path (lazy).
/// Path-based attachments defer reading until delivery time.
///
/// # Examples
///
/// ```
/// use missive::Attachment;
///
/// // From bytes (eager - data loaded immediately)
/// let attachment = Attachment::from_bytes("report.pdf", b"PDF content".to_vec())
///     .content_type("application/pdf");
///
/// // Inline image for HTML emails
/// let png_bytes = vec![0x89, 0x50, 0x4E, 0x47]; // PNG header bytes
/// let logo = Attachment::from_bytes("logo.png", png_bytes)
///     .inline()
///     .content_id("company-logo");
/// // Reference in HTML: <img src="cid:company-logo">
/// ```
///
/// ```rust,ignore
/// // From path (lazy - file read at delivery time)
/// let attachment = Attachment::from_path_lazy("/path/to/large-file.pdf")?;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    /// Filename for the attachment
    pub filename: String,
    /// MIME content type (e.g., "application/pdf", "image/png")
    pub content_type: String,
    /// Raw attachment data (empty if using path-based lazy loading)
    pub data: Vec<u8>,
    /// File path for lazy loading.
    /// If set, data will be read from this path when needed.
    #[serde(default)]
    pub path: Option<String>,
    /// Whether this is an inline or regular attachment
    pub disposition: AttachmentType,
    /// Content-ID for inline attachments (used as cid: reference)
    pub content_id: Option<String>,
    /// Custom headers for the attachment
    #[serde(default)]
    pub headers: Vec<(String, String)>,
}

impl Attachment {
    /// Create a new attachment from raw bytes.
    ///
    /// Content type is guessed from the filename extension.
    pub fn from_bytes(filename: impl Into<String>, data: Vec<u8>) -> Self {
        let filename = filename.into();
        let content_type = mime_guess::from_path(&filename)
            .first_or_octet_stream()
            .to_string();

        Self {
            filename,
            content_type,
            data,
            path: None,
            disposition: AttachmentType::Attachment,
            content_id: None,
            headers: Vec::new(),
        }
    }

    /// Create a new attachment from a file path (eager loading).
    ///
    /// Reads the file immediately and guesses the content type from the extension.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, MailError> {
        let path = path.as_ref();
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("attachment")
            .to_string();

        let data = std::fs::read(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                MailError::AttachmentFileNotFound(path.display().to_string())
            } else {
                MailError::AttachmentReadError(format!("{}: {}", path.display(), e))
            }
        })?;

        let content_type = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();

        Ok(Self {
            filename,
            content_type,
            data,
            path: None, // Data is already loaded
            disposition: AttachmentType::Attachment,
            content_id: None,
            headers: Vec::new(),
        })
    }

    /// Create a new attachment from a file path (lazy loading).
    ///
    /// This defers reading the file until delivery time. Useful for large files
    /// or when the file may be updated between email construction and sending.
    ///
    /// The file will be read when `get_data()` is called.
    pub fn from_path_lazy(path: impl AsRef<Path>) -> Result<Self, MailError> {
        let path_ref = path.as_ref();

        // Validate path exists
        if !path_ref.exists() {
            return Err(MailError::AttachmentFileNotFound(
                path_ref.display().to_string(),
            ));
        }

        let filename = path_ref
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("attachment")
            .to_string();

        let content_type = mime_guess::from_path(path_ref)
            .first_or_octet_stream()
            .to_string();

        let path_string = path_ref.to_string_lossy().to_string();

        Ok(Self {
            filename,
            content_type,
            data: Vec::new(), // Empty - will be loaded lazily
            path: Some(path_string),
            disposition: AttachmentType::Attachment,
            content_id: None,
            headers: Vec::new(),
        })
    }

    /// Set the content type explicitly.
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = content_type.into();
        self
    }

    /// Set as inline attachment (for embedding in HTML).
    pub fn inline(mut self) -> Self {
        self.disposition = AttachmentType::Inline;
        // Auto-generate content_id from filename if not set
        if self.content_id.is_none() {
            self.content_id = Some(self.filename.clone());
        }
        self
    }

    /// Set the Content-ID for inline attachments.
    ///
    /// This is used to reference the attachment in HTML: `<img src="cid:your-id">`
    pub fn content_id(mut self, cid: impl Into<String>) -> Self {
        self.content_id = Some(cid.into());
        self
    }

    /// Add a custom header to the attachment.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Get the attachment data, loading from path if necessary.
    ///
    /// For path-based attachments, this reads the file. For byte-based
    /// attachments, this returns a clone of the data.
    ///
    /// # Errors
    ///
    /// - `AttachmentFileNotFound` - File path doesn't exist
    /// - `AttachmentReadError` - Failed to read file
    /// - `AttachmentMissingContent` - No data and no path provided
    pub fn get_data(&self) -> Result<Vec<u8>, MailError> {
        if let Some(ref path) = self.path {
            // Lazy load from path
            std::fs::read(path).map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    MailError::AttachmentFileNotFound(path.clone())
                } else {
                    MailError::AttachmentReadError(format!("{}: {}", path, e))
                }
            })
        } else if self.data.is_empty() && self.path.is_none() {
            Err(MailError::AttachmentMissingContent(self.filename.clone()))
        } else {
            Ok(self.data.clone())
        }
    }

    /// Get the attachment data as base64-encoded string.
    ///
    /// For path-based attachments, reads and encodes the file.
    pub fn base64_data(&self) -> String {
        use base64::Engine;
        let data = self.get_data().unwrap_or_default();
        base64::engine::general_purpose::STANDARD.encode(&data)
    }

    /// Get the size in bytes.
    ///
    /// For path-based attachments, returns 0 (file not loaded yet).
    /// Use `get_size()` for accurate size of path-based attachments.
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Get the accurate size, loading from path if necessary.
    pub fn get_size(&self) -> Result<usize, MailError> {
        if let Some(ref path) = self.path {
            let metadata =
                std::fs::metadata(path).map_err(|e| MailError::AttachmentError(e.to_string()))?;
            Ok(metadata.len() as usize)
        } else {
            Ok(self.data.len())
        }
    }

    /// Check if this is a path-based (lazy) attachment.
    pub fn is_lazy(&self) -> bool {
        self.path.is_some()
    }

    /// Check if this is an inline attachment.
    pub fn is_inline(&self) -> bool {
        self.disposition == AttachmentType::Inline
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_bytes() {
        let attachment = Attachment::from_bytes("test.txt", b"Hello".to_vec());
        assert_eq!(attachment.filename, "test.txt");
        assert_eq!(attachment.content_type, "text/plain");
        assert_eq!(attachment.data, b"Hello");
        assert_eq!(attachment.disposition, AttachmentType::Attachment);
    }

    #[test]
    fn test_inline() {
        let attachment = Attachment::from_bytes("logo.png", vec![1, 2, 3]).inline();
        assert_eq!(attachment.disposition, AttachmentType::Inline);
        assert_eq!(attachment.content_id, Some("logo.png".to_string()));
    }

    #[test]
    fn test_content_id() {
        let attachment = Attachment::from_bytes("image.png", vec![])
            .inline()
            .content_id("my-logo");
        assert_eq!(attachment.content_id, Some("my-logo".to_string()));
    }

    #[test]
    fn test_mime_guess() {
        let pdf = Attachment::from_bytes("doc.pdf", vec![]);
        assert_eq!(pdf.content_type, "application/pdf");

        let png = Attachment::from_bytes("image.png", vec![]);
        assert_eq!(png.content_type, "image/png");

        let unknown = Attachment::from_bytes("file.unknown_ext_12345", vec![]);
        assert_eq!(unknown.content_type, "application/octet-stream");
    }

    #[test]
    fn test_base64() {
        let attachment = Attachment::from_bytes("test.txt", b"Hello".to_vec());
        assert_eq!(attachment.base64_data(), "SGVsbG8=");
    }
}
