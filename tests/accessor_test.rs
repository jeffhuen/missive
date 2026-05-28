use missive::{Address, Attachment, AttachmentType, Email};

#[test]
fn address_exposes_read_only_accessors() {
    let address = Address::with_name("Alice", "alice@example.com");

    assert_eq!(address.email(), "alice@example.com");
    assert_eq!(address.display_name(), Some("Alice"));
}

#[test]
fn attachment_exposes_read_only_accessors() {
    let attachment = Attachment::from_bytes("report.pdf", b"PDF".to_vec())
        .content_type("application/pdf")
        .inline()
        .content_id("report")
        .header("X-Attachment-Id", "123");

    assert_eq!(attachment.filename(), "report.pdf");
    assert_eq!(attachment.mime_type(), "application/pdf");
    assert_eq!(attachment.data(), b"PDF");
    assert_eq!(attachment.path(), None);
    assert_eq!(attachment.disposition(), AttachmentType::Inline);
    assert_eq!(attachment.inline_content_id(), Some("report"));
    assert_eq!(
        attachment.headers(),
        &[("X-Attachment-Id".to_string(), "123".to_string())]
    );
}

#[test]
fn email_exposes_read_only_accessors() {
    let email = Email::new()
        .from(("Alice", "alice@example.com"))
        .to("bob@example.com")
        .cc("carol@example.com")
        .bcc("dave@example.com")
        .reply_to("reply@example.com")
        .subject("Welcome")
        .text_body("hello")
        .html_body("<p>hello</p>")
        .attachment(Attachment::from_bytes("report.pdf", b"PDF".to_vec()))
        .header("X-Test", "1")
        .assign("user_id", 42)
        .put_private("trace_id", "abc")
        .provider_option("tag", "welcome");

    assert_eq!(email.from_address().unwrap().email(), "alice@example.com");
    assert_eq!(email.to_addresses()[0].email(), "bob@example.com");
    assert_eq!(email.cc_addresses()[0].email(), "carol@example.com");
    assert_eq!(email.bcc_addresses()[0].email(), "dave@example.com");
    assert_eq!(email.reply_to_addresses()[0].email(), "reply@example.com");
    assert_eq!(email.subject_line(), "Welcome");
    assert_eq!(email.text_body_content(), Some("hello"));
    assert_eq!(email.html_body_content(), Some("<p>hello</p>"));
    assert_eq!(email.attachments()[0].filename(), "report.pdf");
    assert_eq!(email.headers().get("X-Test").map(String::as_str), Some("1"));
    assert_eq!(
        email.assigns().get("user_id").and_then(|v| v.as_i64()),
        Some(42)
    );
    assert_eq!(
        email.private().get("trace_id").and_then(|v| v.as_str()),
        Some("abc")
    );
    assert_eq!(
        email.provider_options().get("tag").and_then(|v| v.as_str()),
        Some("welcome")
    );
}
