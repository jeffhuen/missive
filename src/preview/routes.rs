//! Route handlers for the mailbox preview.
//!
//! Provides a web UI for viewing emails captured by `LocalMailer`.
//!
//! ## Features
//!
//! - CSP nonce support for Content Security Policy compliance
//! - Full JSON API with private/provider_options/headers
//! - Path-based attachment lazy loading
//! - RFC 5322 compliant recipient rendering

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

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

/// Shared state for routes.
#[derive(Clone)]
struct AppState {
    storage: Arc<MemoryStorage>,
    config: PreviewConfig,
}

/// Create the mailbox router with default config.
pub fn create_router(storage: Arc<MemoryStorage>) -> Router {
    create_router_with_config(storage, PreviewConfig::default())
}

/// Create the mailbox router with CSP nonce configuration.
///
/// ```rust,ignore
/// use missive::preview::{create_router_with_config, PreviewConfig};
///
/// let config = PreviewConfig {
///     script_nonce: Some("abc123".to_string()),
///     style_nonce: Some("def456".to_string()),
/// };
/// let router = create_router_with_config(storage, config);
/// ```
pub fn create_router_with_config(storage: Arc<MemoryStorage>, config: PreviewConfig) -> Router {
    let state = AppState { storage, config };

    Router::new()
        .route("/", get(index))
        .route("/json", get(list_json))
        .route("/{id}", get(view_email))
        .route("/{id}/html", get(email_html))
        .route("/{id}/attachments/{idx}", get(download_attachment))
        .route("/clear", post(clear_all))
        .with_state(state)
}

// ============================================================================
// Response Types
// ============================================================================

/// Email list item for the sidebar.
#[derive(Serialize)]
struct EmailListItem {
    id: String,
    subject: String,
    from: Option<String>,
    to: Vec<String>,
    cc: Vec<String>,
    bcc: Vec<String>,
    reply_to: Option<String>,
    sent_at: Option<String>,
    text_body: Option<String>,
    html_body: Option<String>,
    headers: HashMap<String, String>,
    provider_options: Vec<ProviderOption>,
    attachments: Vec<AttachmentInfo>,
}

/// Provider option key-value pair.
#[derive(Serialize)]
struct ProviderOption {
    key: String,
    value: String,
}

/// Attachment metadata.
#[derive(Serialize)]
struct AttachmentInfo {
    index: usize,
    filename: String,
    content_type: String,
    /// File path for lazy attachments
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    /// Attachment disposition type
    #[serde(rename = "type")]
    disposition: String,
    /// Custom headers on the attachment
    headers: HashMap<String, String>,
    /// Size in bytes (0 for lazy attachments)
    size: usize,
}

impl From<&StoredEmail> for EmailListItem {
    fn from(stored: &StoredEmail) -> Self {
        let email = &stored.email;

        Self {
            id: stored.id.clone(),
            subject: email.subject.clone(),
            // Use RFC 5322 formatting for proper escaping
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
struct EmailListResponse {
    data: Vec<EmailListItem>,
}

// ============================================================================
// Handlers
// ============================================================================

/// Query params for CSP nonce override.
#[derive(Debug, Deserialize, Default)]
struct IndexQuery {
    /// Override script nonce via query param
    script_nonce: Option<String>,
    /// Override style nonce via query param
    style_nonce: Option<String>,
}

/// GET / - Render the mailbox UI.
///
/// Always renders the HTML UI. JavaScript auto-selects the first email if present.
async fn index(
    State(state): State<AppState>,
    Query(query): Query<IndexQuery>,
) -> Html<String> {
    let emails = state.storage.all();
    let email_items: Vec<EmailListItem> = emails.iter().map(EmailListItem::from).collect();

    // Merge query param nonces with config nonces (query takes precedence)
    let script_nonce = query.script_nonce.or(state.config.script_nonce.clone());
    let style_nonce = query.style_nonce.or(state.config.style_nonce.clone());

    Html(render_index(&email_items, script_nonce, style_nonce))
}

/// GET /json - Return all emails as JSON.
///
/// Returns `{data: [...]}` with full email details including:
/// - private fields (via sent_at)
/// - provider_options
/// - headers
/// - attachment metadata (path, type, headers)
async fn list_json(State(state): State<AppState>) -> Json<EmailListResponse> {
    let emails: Vec<EmailListItem> = state.storage.all().iter().map(EmailListItem::from).collect();
    Json(EmailListResponse { data: emails })
}

/// GET /:id - View a single email as JSON.
async fn view_email(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<EmailListItem>, StatusCode> {
    state
        .storage
        .get(&id)
        .map(|e| Json(EmailListItem::from(&e)))
        .ok_or(StatusCode::NOT_FOUND)
}

/// GET /:id/html - Return raw HTML body for iframe embedding.
///
/// Replaces inline image CID references (cid:filename) with attachment URLs.
async fn email_html(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Html<String>, StatusCode> {
    let stored = state.storage.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    let html = stored.email.html_body.clone().ok_or(StatusCode::NOT_FOUND)?;

    // Replace CID references with attachment URLs
    let html = replace_cid_references(&html, &id, &stored.email.attachments);

    Ok(Html(html))
}

/// Replace inline image CID references with attachment URLs.
///
/// Converts `cid:logo.png` or `cid:content-id` to `{id}/attachments/{index}`.
fn replace_cid_references(
    html: &str,
    email_id: &str,
    attachments: &[crate::attachment::Attachment],
) -> String {
    use regex::Regex;

    // Match "cid:something" in src attributes
    let re = Regex::new(r#""cid:([^"]*)""#).unwrap();

    re.replace_all(html, |caps: &regex::Captures| {
        let cid = &caps[1];

        // Find attachment by content_id or filename
        if let Some((idx, _)) = attachments.iter().enumerate().find(|(_, att)| {
            att.content_id.as_deref() == Some(cid) || att.filename == cid
        }) {
            format!("\"{}/attachments/{}\"", email_id, idx)
        } else {
            // Keep original if not found
            caps[0].to_string()
        }
    })
    .to_string()
}

/// GET /:id/attachments/:idx - Download an attachment.
///
/// Supports both eager (data) and lazy (path) attachments.
async fn download_attachment(
    State(state): State<AppState>,
    Path((id, idx)): Path<(String, usize)>,
) -> Result<Response, StatusCode> {
    let stored = state.storage.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    let attachment = stored
        .email
        .attachments
        .get(idx)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get data - handles both eager (data) and lazy (path) attachments
    let data = attachment
        .get_data()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = (
        [
            (header::CONTENT_TYPE, attachment.content_type.clone()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", attachment.filename),
            ),
        ],
        data,
    );

    Ok(response.into_response())
}

/// POST /clear - Delete all emails.
async fn clear_all(State(state): State<AppState>) -> StatusCode {
    state.storage.clear();
    StatusCode::NO_CONTENT
}

// ============================================================================
// Template Rendering
// ============================================================================

fn render_index(
    emails: &[EmailListItem],
    script_nonce: Option<String>,
    style_nonce: Option<String>,
) -> String {
    let css = include_str!("../../templates/preview/styles.css");
    let js = include_str!("../../templates/preview/script.js");

    // Build nonce attributes for CSP compliance
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

    // Icons for theme toggle
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
