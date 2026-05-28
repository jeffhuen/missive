//! Email attachments with support for inline and regular attachments.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

use crate::error::MailError;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
fn attachment_io_error(path: impl Into<String>, source: std::io::Error) -> MailError {
    let path = path.into();
    if source.kind() == std::io::ErrorKind::NotFound {
        MailError::AttachmentFileNotFound(path)
    } else {
        MailError::AttachmentReadError { path, source }
    }
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
fn path_attachments_unsupported() -> MailError {
    MailError::UnsupportedFeature(
        "path-based attachments are not supported on wasm32-unknown-unknown; use Attachment::from_bytes".into(),
    )
}

mod shared_bytes_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::sync::Arc;

    pub(super) fn serialize<S>(data: &Arc<[u8]>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        data.as_ref().serialize(serializer)
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Arc<[u8]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = Vec::<u8>::deserialize(deserializer)?;
        Ok(Arc::from(data))
    }
}

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
    pub(crate) filename: String,
    /// MIME content type (e.g., "application/pdf", "image/png")
    pub(crate) content_type: String,
    /// Shared raw attachment data (empty if using path-based lazy loading)
    #[serde(with = "shared_bytes_serde")]
    pub(crate) data: Arc<[u8]>,
    /// File path for lazy loading.
    /// If set, data will be read from this path when needed.
    #[serde(default)]
    pub(crate) path: Option<String>,
    /// Whether this is an inline or regular attachment
    pub(crate) disposition: AttachmentType,
    /// Content-ID for inline attachments (used as cid: reference)
    pub(crate) content_id: Option<String>,
    /// Custom headers for the attachment
    #[serde(default)]
    pub(crate) headers: Vec<(String, String)>,
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
            data: Arc::from(data),
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
        #[cfg(all(target_family = "wasm", target_os = "unknown"))]
        {
            let _ = path;
            Err(path_attachments_unsupported())
        }

        #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
        {
            let path = path.as_ref();
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("attachment")
                .to_string();

            let data = std::fs::read(path)
                .map_err(|source| attachment_io_error(path.display().to_string(), source))?;

            let content_type = mime_guess::from_path(path)
                .first_or_octet_stream()
                .to_string();

            Ok(Self {
                filename,
                content_type,
                data: Arc::from(data),
                path: None, // Data is already loaded
                disposition: AttachmentType::Attachment,
                content_id: None,
                headers: Vec::new(),
            })
        }
    }

    /// Create a new attachment from a file path (lazy loading).
    ///
    /// This defers reading the file until delivery time. Useful for large files
    /// or when the file may be updated between email construction and sending.
    ///
    /// The file will be read when `get_data()` is called.
    pub fn from_path_lazy(path: impl AsRef<Path>) -> Result<Self, MailError> {
        #[cfg(all(target_family = "wasm", target_os = "unknown"))]
        {
            let _ = path;
            Err(path_attachments_unsupported())
        }

        #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
        {
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
                data: Arc::from(Vec::new()), // Empty - will be loaded lazily
                path: Some(path_string),
                disposition: AttachmentType::Attachment,
                content_id: None,
                headers: Vec::new(),
            })
        }
    }

    /// Set the content type explicitly.
    #[must_use = "content_type returns a modified attachment; chain or assign the returned value"]
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = content_type.into();
        self
    }

    /// Set as inline attachment (for embedding in HTML).
    #[must_use = "inline returns a modified attachment; chain or assign the returned value"]
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
    #[must_use = "content_id returns a modified attachment; chain or assign the returned value"]
    pub fn content_id(mut self, cid: impl Into<String>) -> Self {
        self.content_id = Some(cid.into());
        self
    }

    /// Add a custom header to the attachment.
    #[must_use = "header returns a modified attachment; chain or assign the returned value"]
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Borrow the attachment filename.
    pub fn filename(&self) -> &str {
        &self.filename
    }

    /// Borrow the attachment MIME type.
    pub fn mime_type(&self) -> &str {
        &self.content_type
    }

    /// Borrow eagerly loaded attachment bytes.
    pub fn data(&self) -> &[u8] {
        self.data.as_ref()
    }

    /// Borrow the lazy-loading path, if this is a path-based attachment.
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// Return the attachment disposition.
    pub fn disposition(&self) -> AttachmentType {
        self.disposition
    }

    /// Borrow the inline Content-ID, if set.
    pub fn inline_content_id(&self) -> Option<&str> {
        self.content_id.as_deref()
    }

    /// Borrow custom attachment headers.
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
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
    pub fn get_data(&self) -> Result<Vec<u8>, MailError> {
        if let Some(ref path) = self.path {
            read_attachment_path(path)
        } else {
            Ok(self.data.to_vec())
        }
    }

    /// Get the attachment data without blocking the async executor.
    #[cfg(feature = "_async_attachment_io")]
    pub(crate) async fn get_data_async(&self) -> Result<Vec<u8>, MailError> {
        if let Some(ref path) = self.path {
            read_attachment_path_async(path).await
        } else {
            Ok(self.data.to_vec())
        }
    }

    /// Get the attachment data as base64-encoded string.
    ///
    /// For path-based attachments, reads and encodes the file.
    pub fn base64_data(&self) -> Result<String, MailError> {
        use base64::Engine;
        let data = self.get_data()?;
        Ok(base64::engine::general_purpose::STANDARD.encode(&data))
    }

    /// Get the attachment data as base64 without blocking the async executor.
    #[cfg(feature = "_async_attachment_io")]
    pub(crate) async fn base64_data_async(&self) -> Result<String, MailError> {
        use base64::Engine;
        let data = self.get_data_async().await?;
        Ok(base64::engine::general_purpose::STANDARD.encode(&data))
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
            attachment_path_size(path)
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

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
fn read_attachment_path(path: &str) -> Result<Vec<u8>, MailError> {
    std::fs::read(path).map_err(|source| attachment_io_error(path.to_string(), source))
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
fn read_attachment_path(_path: &str) -> Result<Vec<u8>, MailError> {
    Err(path_attachments_unsupported())
}

#[cfg(all(
    feature = "_async_attachment_io",
    not(all(target_family = "wasm", target_os = "unknown"))
))]
async fn read_attachment_path_async(path: &str) -> Result<Vec<u8>, MailError> {
    tokio::fs::read(path)
        .await
        .map_err(|source| attachment_io_error(path.to_string(), source))
}

#[cfg(all(
    feature = "_async_attachment_io",
    target_family = "wasm",
    target_os = "unknown"
))]
async fn read_attachment_path_async(_path: &str) -> Result<Vec<u8>, MailError> {
    Err(path_attachments_unsupported())
}

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
fn attachment_path_size(path: &str) -> Result<usize, MailError> {
    let metadata =
        std::fs::metadata(path).map_err(|source| attachment_io_error(path.to_string(), source))?;
    Ok(metadata.len() as usize)
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
fn attachment_path_size(_path: &str) -> Result<usize, MailError> {
    Err(path_attachments_unsupported())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_bytes() {
        let attachment = Attachment::from_bytes("test.txt", b"Hello".to_vec());
        assert_eq!(attachment.filename, "test.txt");
        assert_eq!(attachment.content_type, "text/plain");
        assert_eq!(attachment.data.as_ref(), b"Hello");
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
        assert_eq!(attachment.base64_data().unwrap(), "SGVsbG8=");
    }

    #[cfg(all(
        feature = "_async_attachment_io",
        not(all(target_family = "wasm", target_os = "unknown"))
    ))]
    #[tokio::test]
    async fn test_base64_data_async_reads_lazy_file() {
        let path = std::env::temp_dir().join(format!(
            "missive-lazy-attachment-async-{}.txt",
            std::process::id()
        ));
        std::fs::write(&path, b"async").unwrap();

        let attachment = Attachment::from_path_lazy(&path).unwrap();

        assert_eq!(attachment.base64_data_async().await.unwrap(), "YXN5bmM=");

        std::fs::remove_file(&path).unwrap();
    }
}
