//! Actix-web adapter for mailbox preview.

use std::sync::Arc;

use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;

use crate::storage::MemoryStorage;

use super::core::{self, EmailListResponse, PreviewConfig};

/// Shared state for routes.
#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<MemoryStorage>,
    pub config: PreviewConfig,
}

/// Configure routes on an Actix scope.
pub fn configure(cfg: &mut web::ServiceConfig, state: AppState) {
    cfg.app_data(web::Data::new(state))
        .route("/", web::get().to(index))
        .route("/json", web::get().to(list_json))
        .route("/{id}", web::get().to(view_email))
        .route("/{id}/html", web::get().to(email_html))
        .route(
            "/{id}/attachments/{idx}",
            web::get().to(download_attachment),
        )
        .route("/clear", web::post().to(clear_all));
}

/// Query params for CSP nonce override.
#[derive(Debug, Deserialize, Default)]
pub struct IndexQuery {
    script_nonce: Option<String>,
    style_nonce: Option<String>,
}

/// GET / - Render the mailbox UI.
async fn index(state: web::Data<AppState>, query: web::Query<IndexQuery>) -> impl Responder {
    let emails = core::list_emails(&state.storage);
    let script_nonce = query
        .script_nonce
        .clone()
        .or(state.config.script_nonce.clone());
    let style_nonce = query
        .style_nonce
        .clone()
        .or(state.config.style_nonce.clone());
    let html = core::render_index(&emails, script_nonce, style_nonce);
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

/// GET /json - Return all emails as JSON.
async fn list_json(state: web::Data<AppState>) -> impl Responder {
    let emails = core::list_emails(&state.storage);
    HttpResponse::Ok().json(EmailListResponse { data: emails })
}

/// GET /{id} - View a single email as JSON.
async fn view_email(state: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let id = path.into_inner();
    match core::get_email(&state.storage, &id) {
        Some(email) => HttpResponse::Ok().json(email),
        None => HttpResponse::NotFound().finish(),
    }
}

/// GET /{id}/html - Return raw HTML body for iframe embedding.
async fn email_html(state: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let id = path.into_inner();
    match core::get_email_html(&state.storage, &id) {
        Some(html) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html),
        None => HttpResponse::NotFound().finish(),
    }
}

/// GET /{id}/attachments/{idx} - Download an attachment.
async fn download_attachment(
    state: web::Data<AppState>,
    path: web::Path<(String, usize)>,
) -> impl Responder {
    let (id, idx) = path.into_inner();
    match core::get_attachment(&state.storage, &id, idx) {
        Some(att) => HttpResponse::Ok()
            .content_type(att.content_type)
            .insert_header((
                "Content-Disposition",
                format!("attachment; filename=\"{}\"", att.filename),
            ))
            .body(att.data),
        None => HttpResponse::NotFound().finish(),
    }
}

/// POST /clear - Delete all emails.
async fn clear_all(state: web::Data<AppState>) -> impl Responder {
    core::clear_emails(&state.storage);
    HttpResponse::NoContent().finish()
}
