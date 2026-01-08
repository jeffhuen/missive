//! Axum adapter for mailbox preview.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

use crate::storage::MemoryStorage;

use super::core::{
    self, AttachmentData, EmailListItem, EmailListResponse, PreviewConfig,
};

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

/// Query params for CSP nonce override.
#[derive(Debug, Deserialize, Default)]
struct IndexQuery {
    script_nonce: Option<String>,
    style_nonce: Option<String>,
}

/// GET / - Render the mailbox UI.
async fn index(
    State(state): State<AppState>,
    Query(query): Query<IndexQuery>,
) -> Html<String> {
    let emails = core::list_emails(&state.storage);
    let script_nonce = query.script_nonce.or(state.config.script_nonce.clone());
    let style_nonce = query.style_nonce.or(state.config.style_nonce.clone());
    Html(core::render_index(&emails, script_nonce, style_nonce))
}

/// GET /json - Return all emails as JSON.
async fn list_json(State(state): State<AppState>) -> Json<EmailListResponse> {
    let emails = core::list_emails(&state.storage);
    Json(EmailListResponse { data: emails })
}

/// GET /:id - View a single email as JSON.
async fn view_email(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<EmailListItem>, StatusCode> {
    core::get_email(&state.storage, &id)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// GET /:id/html - Return raw HTML body for iframe embedding.
async fn email_html(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Html<String>, StatusCode> {
    core::get_email_html(&state.storage, &id)
        .map(Html)
        .ok_or(StatusCode::NOT_FOUND)
}

/// GET /:id/attachments/:idx - Download an attachment.
async fn download_attachment(
    State(state): State<AppState>,
    Path((id, idx)): Path<(String, usize)>,
) -> Result<Response, StatusCode> {
    let AttachmentData { data, filename, content_type } =
        core::get_attachment(&state.storage, &id, idx)
            .ok_or(StatusCode::NOT_FOUND)?;

    let response = (
        [
            (header::CONTENT_TYPE, content_type),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        data,
    );

    Ok(response.into_response())
}

/// POST /clear - Delete all emails.
async fn clear_all(State(state): State<AppState>) -> StatusCode {
    core::clear_emails(&state.storage);
    StatusCode::NO_CONTENT
}
