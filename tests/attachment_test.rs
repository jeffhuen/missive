//! Attachment tests.
//!
//! Ported from Swoosh's attachment_test.exs

use missive::{Attachment, AttachmentType};

// ============================================================================
// Constructor Tests
// ============================================================================

#[test]
fn from_bytes_creates_attachment() {
    let attachment = Attachment::from_bytes("file.txt", b"content".to_vec());
    assert_eq!(attachment.filename, "file.txt");
    assert_eq!(attachment.content_type, "text/plain");
    assert_eq!(attachment.data, b"content");
}

#[test]
fn from_bytes_with_unknown_extension_uses_octet_stream() {
    let attachment = Attachment::from_bytes("unknown-file.xyz123", b"data".to_vec());
    assert_eq!(attachment.content_type, "application/octet-stream");
}

// ============================================================================
// Content Type Tests
// ============================================================================

#[test]
fn content_type_is_guessed_from_extension() {
    let pdf = Attachment::from_bytes("report.pdf", vec![]);
    assert_eq!(pdf.content_type, "application/pdf");

    let png = Attachment::from_bytes("image.png", vec![]);
    assert_eq!(png.content_type, "image/png");

    let jpg = Attachment::from_bytes("photo.jpg", vec![]);
    assert_eq!(jpg.content_type, "image/jpeg");

    let zip = Attachment::from_bytes("archive.zip", vec![]);
    assert_eq!(zip.content_type, "application/zip");
}

#[test]
fn content_type_can_be_overridden() {
    let attachment = Attachment::from_bytes("file.txt", vec![]).content_type("application/msword");
    assert_eq!(attachment.content_type, "application/msword");
}

// ============================================================================
// Inline Attachment Tests
// ============================================================================

#[test]
fn inline_sets_disposition_and_content_id() {
    let attachment = Attachment::from_bytes("file.png", vec![]).inline();
    assert_eq!(attachment.disposition, AttachmentType::Inline);
    assert_eq!(attachment.content_id, Some("file.png".to_string()));
}

#[test]
fn inline_with_custom_content_id() {
    let attachment = Attachment::from_bytes("file.png", vec![])
        .inline()
        .content_id("my-cid");
    assert_eq!(attachment.disposition, AttachmentType::Inline);
    assert_eq!(attachment.content_id, Some("my-cid".to_string()));
}

#[test]
fn regular_attachment_has_no_content_id() {
    let attachment = Attachment::from_bytes("file.png", vec![]);
    assert!(attachment.content_id.is_none());
}

#[test]
fn is_inline_returns_correct_value() {
    let regular = Attachment::from_bytes("file.png", vec![]);
    assert!(!regular.is_inline());

    let inline = Attachment::from_bytes("file.png", vec![]).inline();
    assert!(inline.is_inline());
}

// ============================================================================
// Custom Headers Tests
// ============================================================================

#[test]
fn header_adds_custom_header() {
    let attachment = Attachment::from_bytes("file.ics", vec![])
        .header("Content-Type", "text/calendar; method=\"REQUEST\"");

    assert_eq!(attachment.headers.len(), 1);
    assert_eq!(attachment.headers[0].0, "Content-Type");
    assert_eq!(attachment.headers[0].1, "text/calendar; method=\"REQUEST\"");
}

#[test]
fn header_adds_multiple_headers() {
    let attachment = Attachment::from_bytes("file.txt", vec![])
        .header("X-Custom-Header", "value1")
        .header("X-Another-Header", "value2");

    assert_eq!(attachment.headers.len(), 2);
}

// ============================================================================
// Data Access Tests
// ============================================================================

#[test]
fn get_data_returns_data() {
    let attachment = Attachment::from_bytes("test.txt", b"assemble".to_vec());
    assert_eq!(attachment.get_data().unwrap(), b"assemble");
}

#[test]
fn base64_data_returns_encoded_data() {
    let attachment = Attachment::from_bytes("test.txt", b"assemble".to_vec());
    // "assemble" in base64 is "YXNzZW1ibGU="
    assert_eq!(attachment.base64_data(), "YXNzZW1ibGU=");
}

#[test]
fn size_returns_data_length() {
    let attachment = Attachment::from_bytes("test.txt", b"hello".to_vec());
    assert_eq!(attachment.size(), 5);
}

// ============================================================================
// Email Integration Tests
// ============================================================================

#[test]
fn email_can_have_attachment() {
    use missive::Email;

    let attachment = Attachment::from_bytes("report.pdf", b"PDF content".to_vec());
    let email = Email::new()
        .from("sender@example.com")
        .to("recipient@example.com")
        .subject("Report attached")
        .attachment(attachment);

    assert_eq!(email.attachments.len(), 1);
    assert_eq!(email.attachments[0].filename, "report.pdf");
}

#[test]
fn email_can_have_multiple_attachments() {
    use missive::Email;

    let email = Email::new()
        .from("sender@example.com")
        .to("recipient@example.com")
        .subject("Files attached")
        .attachment(Attachment::from_bytes("file1.txt", b"one".to_vec()))
        .attachment(Attachment::from_bytes("file2.txt", b"two".to_vec()));

    assert_eq!(email.attachments.len(), 2);
    assert_eq!(email.attachments[0].filename, "file1.txt");
    assert_eq!(email.attachments[1].filename, "file2.txt");
}

#[test]
fn email_can_have_inline_attachment() {
    use missive::Email;

    let logo = Attachment::from_bytes("logo.png", vec![0x89, 0x50, 0x4E, 0x47])
        .inline()
        .content_id("company-logo");

    let email = Email::new()
        .from("sender@example.com")
        .to("recipient@example.com")
        .subject("Welcome")
        .html_body("<img src=\"cid:company-logo\">")
        .attachment(logo);

    assert_eq!(email.attachments.len(), 1);
    assert!(email.attachments[0].is_inline());
    assert_eq!(
        email.attachments[0].content_id,
        Some("company-logo".to_string())
    );
}
