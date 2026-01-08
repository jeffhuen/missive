//! Shared logic for mailbox preview.
//!
//! Framework-agnostic types and rendering functions used by both Axum and Actix adapters.

use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;

use crate::attachment::AttachmentType;
use crate::storage::{MemoryStorage, Storage, StoredEmail};

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for CSP nonces and base path.
#[derive(Clone, Default)]
pub struct PreviewConfig {
    /// Script CSP nonce (for inline scripts)
    pub script_nonce: Option<String>,
    /// Style CSP nonce (for inline styles)
    pub style_nonce: Option<String>,
}

// ============================================================================
// Response Types
// ============================================================================

/// Email list item for JSON API.
#[derive(Serialize)]
pub struct EmailListItem {
    pub id: String,
    pub subject: String,
    pub from: Option<String>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub reply_to: Option<String>,
    pub sent_at: Option<String>,
    pub text_body: Option<String>,
    pub html_body: Option<String>,
    pub headers: HashMap<String, String>,
    pub provider_options: Vec<ProviderOption>,
    pub attachments: Vec<AttachmentInfo>,
}

/// Provider option key-value pair.
#[derive(Serialize)]
pub struct ProviderOption {
    pub key: String,
    pub value: String,
}

/// Attachment metadata.
#[derive(Serialize)]
pub struct AttachmentInfo {
    pub index: usize,
    pub filename: String,
    pub content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(rename = "type")]
    pub disposition: String,
    pub headers: HashMap<String, String>,
    pub size: usize,
}

impl From<&StoredEmail> for EmailListItem {
    fn from(stored: &StoredEmail) -> Self {
        let email = &stored.email;

        Self {
            id: stored.id.clone(),
            subject: email.subject.clone(),
            from: email.from.as_ref().map(|a| a.formatted_rfc5322()),
            to: email.to.iter().map(|a| a.formatted_rfc5322()).collect(),
            cc: email.cc.iter().map(|a| a.formatted_rfc5322()).collect(),
            bcc: email.bcc.iter().map(|a| a.formatted_rfc5322()).collect(),
            reply_to: email.reply_to.first().map(|a| a.formatted_rfc5322()),
            sent_at: Some(stored.sent_at.to_rfc3339()),
            text_body: email.text_body.clone(),
            html_body: email.html_body.clone(),
            headers: email.headers.clone(),
            provider_options: email
                .provider_options
                .iter()
                .map(|(k, v)| ProviderOption {
                    key: k.clone(),
                    value: format!("{}", v),
                })
                .collect(),
            attachments: email
                .attachments
                .iter()
                .enumerate()
                .map(|(i, a)| AttachmentInfo {
                    index: i,
                    filename: a.filename.clone(),
                    content_type: a.content_type.clone(),
                    path: a.path.clone(),
                    disposition: match a.disposition {
                        AttachmentType::Inline => "inline".to_string(),
                        AttachmentType::Attachment => "attachment".to_string(),
                    },
                    headers: a.headers.iter().cloned().collect(),
                    size: a.size(),
                })
                .collect(),
        }
    }
}

/// Wrapper for JSON list response.
#[derive(Serialize)]
pub struct EmailListResponse {
    pub data: Vec<EmailListItem>,
}

// ============================================================================
// Service Functions
// ============================================================================

/// Get all emails as EmailListItems.
pub fn list_emails(storage: &Arc<MemoryStorage>) -> Vec<EmailListItem> {
    storage.all().iter().map(EmailListItem::from).collect()
}

/// Get a single email by ID.
pub fn get_email(storage: &Arc<MemoryStorage>, id: &str) -> Option<EmailListItem> {
    storage.get(id).map(|e| EmailListItem::from(&e))
}

/// Get raw HTML body for an email, with CID references replaced.
pub fn get_email_html(storage: &Arc<MemoryStorage>, id: &str) -> Option<String> {
    let stored = storage.get(id)?;
    let html = stored.email.html_body.clone()?;
    Some(replace_cid_references(&html, id, &stored.email.attachments))
}

/// Get attachment data and metadata.
pub struct AttachmentData {
    pub data: Vec<u8>,
    pub filename: String,
    pub content_type: String,
}

/// Get attachment by email ID and index.
pub fn get_attachment(
    storage: &Arc<MemoryStorage>,
    id: &str,
    idx: usize,
) -> Option<AttachmentData> {
    let stored = storage.get(id)?;
    let attachment = stored.email.attachments.get(idx)?;
    let data = attachment.get_data().ok()?;

    Some(AttachmentData {
        data,
        filename: attachment.filename.clone(),
        content_type: attachment.content_type.clone(),
    })
}

/// Clear all emails from storage.
pub fn clear_emails(storage: &Arc<MemoryStorage>) {
    storage.clear();
}

// ============================================================================
// HTML Rendering
// ============================================================================

/// Replace inline image CID references with attachment URLs.
fn replace_cid_references(
    html: &str,
    email_id: &str,
    attachments: &[crate::attachment::Attachment],
) -> String {
    use regex::Regex;

    let re = Regex::new(r#""cid:([^"]*)""#).unwrap();

    re.replace_all(html, |caps: &regex::Captures| {
        let cid = &caps[1];

        if let Some((idx, _)) = attachments
            .iter()
            .enumerate()
            .find(|(_, att)| att.content_id.as_deref() == Some(cid) || att.filename == cid)
        {
            format!("\"{}/attachments/{}\"", email_id, idx)
        } else {
            caps[0].to_string()
        }
    })
    .to_string()
}

/// Render the index HTML page.
pub fn render_index(
    emails: &[EmailListItem],
    script_nonce: Option<String>,
    style_nonce: Option<String>,
) -> String {
    let css = include_str!("../../templates/preview/styles.css");
    let js = include_str!("../../templates/preview/script.js");

    let style_nonce_attr = style_nonce
        .as_ref()
        .map(|n| format!(" nonce=\"{}\"", html_escape(n)))
        .unwrap_or_default();
    let script_nonce_attr = script_nonce
        .as_ref()
        .map(|n| format!(" nonce=\"{}\"", html_escape(n)))
        .unwrap_or_default();

    let email_items: String = emails
        .iter()
        .map(|e| {
            format!(
                r#"<div class="email-item" data-id="{id}" onclick="selectEmail('{id}')">
                    <div class="email-item-from">{from}</div>
                    <div class="email-item-subject">{subject}</div>
                </div>"#,
                id = e.id,
                from = html_escape(e.from.as_deref().unwrap_or("(no sender)")),
                subject = html_escape(&e.subject),
            )
        })
        .collect();

    let empty_state = if emails.is_empty() {
        r#"<div class="empty-state">
            <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round">
                <path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z"></path>
                <polyline points="22,6 12,13 2,6"></polyline>
            </svg>
            <h2>No emails yet</h2>
            <p>Emails sent via LocalMailer will appear here</p>
        </div>"#
    } else {
        ""
    };

    let plural = if emails.len() == 1 { "" } else { "s" };

    let sun_icon = r#"<svg id="sun-icon" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="display:none"><circle cx="12" cy="12" r="5"></circle><line x1="12" y1="1" x2="12" y2="3"></line><line x1="12" y1="21" x2="12" y2="23"></line><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"></line><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"></line><line x1="1" y1="12" x2="3" y2="12"></line><line x1="21" y1="12" x2="23" y2="12"></line><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"></line></svg>"#;
    let moon_icon = r#"<svg id="moon-icon" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path></svg>"#;

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Mailbox Preview - Missive</title>
    <style{style_nonce_attr}>{css}</style>
</head>
<body>
    <div class="container">
        <aside class="sidebar">
            <div class="sidebar-header">
                <h1>Mailbox</h1>
                <div class="sidebar-meta">
                    <span class="email-count">{count} message{plural}</span>
                    <div class="header-actions">
                        <button class="theme-toggle" onclick="toggleTheme()" title="Toggle theme">
                            {sun_icon}
                            {moon_icon}
                        </button>
                    </div>
                </div>
            </div>

            {empty_state}

            <div class="email-list">
                {email_items}
            </div>

            <div class="sidebar-footer">
                <button class="btn-clear" onclick="clearAll()">Empty mailbox</button>
            </div>
        </aside>

        <main class="main-content">
            <div class="email-view" id="email-view">
                <div class="no-selection">
                    <p>Select an email to view</p>
                </div>
            </div>
        </main>
    </div>

    <script{script_nonce_attr}>
    {js}
    </script>
</body>
</html>"##,
        css = css,
        js = js,
        count = emails.len(),
        plural = plural,
        email_items = email_items,
        empty_state = empty_state,
        sun_icon = sun_icon,
        moon_icon = moon_icon,
        style_nonce_attr = style_nonce_attr,
        script_nonce_attr = script_nonce_attr,
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
