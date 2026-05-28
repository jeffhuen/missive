# v0.7 Migration Notes

This document tracks breaking API migrations introduced during the v0.7 cleanup.
It is intentionally incremental and will be expanded as the refactor continues.

## Field Accessors

`Email`, `Address`, and `Attachment` fields are no longer part of the downstream
public API. Construct values with the existing builder methods and read values
through accessors.

```rust
let email = Email::new()
    .from(("Alice", "alice@example.com"))
    .to("bob@example.com")
    .subject("Welcome");

assert_eq!(email.from_address().unwrap().email(), "alice@example.com");
assert_eq!(email.to_addresses()[0].email(), "bob@example.com");
assert_eq!(email.subject_line(), "Welcome");
```

Common replacements:

| Before | After |
| --- | --- |
| `address.email` | `address.email()` |
| `address.name.as_deref()` | `address.display_name()` |
| `email.from.as_ref()` | `email.from_address()` |
| `email.to` | `email.to_addresses()` |
| `email.cc` | `email.cc_addresses()` |
| `email.bcc` | `email.bcc_addresses()` |
| `email.reply_to` | `email.reply_to_addresses()` |
| `email.subject` | `email.subject_line()` |
| `email.text_body.as_deref()` | `email.text_body_content()` |
| `email.html_body.as_deref()` | `email.html_body_content()` |
| `email.attachments` | `email.attachments()` |
| `email.headers` | `email.headers()` |
| `email.assigns` | `email.assigns()` |
| `email.private` | `email.private()` |
| `email.provider_options` | `email.provider_options()` |
| `attachment.filename` | `attachment.filename()` |
| `attachment.content_type` | `attachment.mime_type()` |
| `attachment.data` | `attachment.data()` |
| `attachment.path.as_deref()` | `attachment.path()` |
| `attachment.disposition` | `attachment.disposition()` |
| `attachment.content_id.as_deref()` | `attachment.inline_content_id()` |
| `attachment.headers` | `attachment.headers()` |

Serde support is unchanged: the structs still derive `Serialize` and
`Deserialize`, and the serialized field names remain stable for existing JSON
payloads.

## Attachment Clone Semantics

Eager attachments now store their bytes in shared immutable backing storage.
`Attachment::from_bytes` still takes ownership of a `Vec<u8>`, and
`Attachment::data()` still returns a borrowed byte slice, but cloning an
`Attachment` or an `Email` containing attachments no longer copies the raw
attachment payload.

For compatibility, `Attachment::get_data()` continues to return an owned
`Vec<u8>`. Use `Attachment::data()` when borrowing eagerly loaded bytes is
sufficient.
