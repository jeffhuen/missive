//! Error type behavior tests.

use missive::{Attachment, MailError};
use serde_json::Value;
use std::error::Error as StdError;
use std::fs;

#[test]
fn json_errors_preserve_source_chain() {
    let serde_error = serde_json::from_str::<Value>("{").unwrap_err();
    let error = MailError::from(serde_error);

    let source = StdError::source(&error).expect("json source should be preserved");
    assert!(source.is::<serde_json::Error>());
}

#[test]
fn attachment_read_errors_preserve_io_source_chain() {
    let path = std::env::temp_dir().join(format!(
        "missive-unreadable-attachment-dir-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir(&path).unwrap();

    let error = Attachment::from_path(&path).unwrap_err();
    fs::remove_dir(&path).unwrap();

    let source = StdError::source(&error).expect("io source should be preserved");
    assert!(source.is::<std::io::Error>());
}
