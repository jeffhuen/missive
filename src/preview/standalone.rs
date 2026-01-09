//! Standalone preview server using tiny_http.
//!
//! A lightweight, zero-framework email preview server for development.
//!
//! # Example
//!
//! ```rust,ignore
//! use missive::providers::LocalMailer;
//! use missive::preview::serve;
//!
//! let mailer = LocalMailer::new();
//! let storage = mailer.storage();
//!
//! // Blocking - runs until error or shutdown
//! serve("127.0.0.1:3025", storage)?;
//! ```

use std::io;
use std::sync::Arc;
use std::thread;

use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

use crate::storage::MemoryStorage;

use super::core::{self, EmailListResponse, PreviewConfig};

// ============================================================================
// Public API
// ============================================================================

/// Start a blocking preview server at the given address.
///
/// This function blocks forever, handling requests until an error occurs.
///
/// # Example
///
/// ```rust,ignore
/// use missive::providers::LocalMailer;
/// use missive::preview::serve;
///
/// let mailer = LocalMailer::new();
/// serve("127.0.0.1:3025", mailer.storage())?;
/// ```
pub fn serve(addr: &str, storage: Arc<MemoryStorage>) -> io::Result<()> {
    PreviewServer::new(addr, storage)?.run()
}

/// A standalone preview server with lifecycle control.
///
/// Use this when you need to run the server in a background thread
/// or need more control over the server lifecycle.
///
/// # Example
///
/// ```rust,ignore
/// use missive::providers::LocalMailer;
/// use missive::preview::PreviewServer;
///
/// let mailer = LocalMailer::new();
/// let server = PreviewServer::new("127.0.0.1:3025", mailer.storage())?;
///
/// // Run in background
/// let handle = server.spawn();
///
/// // ... do other work ...
///
/// // Shutdown when done
/// handle.shutdown();
/// ```
pub struct PreviewServer {
    server: Server,
    storage: Arc<MemoryStorage>,
    config: PreviewConfig,
}

impl PreviewServer {
    /// Create a new preview server bound to the given address.
    pub fn new(addr: &str, storage: Arc<MemoryStorage>) -> io::Result<Self> {
        Self::with_config(addr, storage, PreviewConfig::default())
    }

    /// Create a new preview server with CSP nonce configuration.
    pub fn with_config(
        addr: &str,
        storage: Arc<MemoryStorage>,
        config: PreviewConfig,
    ) -> io::Result<Self> {
        let server = Server::http(addr).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(Self {
            server,
            storage,
            config,
        })
    }

    /// Run the server, blocking the current thread.
    ///
    /// This method handles requests until an error occurs.
    pub fn run(self) -> io::Result<()> {
        run_server(&self.server, &self.storage, &self.config)
    }

    /// Spawn the server in a background thread.
    ///
    /// The server runs until the process exits. This is fire-and-forget -
    /// no handle is returned because dev preview servers typically run
    /// for the lifetime of the application.
    pub fn spawn(self) {
        thread::spawn(move || {
            let _ = run_server(&self.server, &self.storage, &self.config);
        });
    }
}

// ============================================================================
// Server Implementation
// ============================================================================

fn run_server(
    server: &Server,
    storage: &Arc<MemoryStorage>,
    config: &PreviewConfig,
) -> io::Result<()> {
    loop {
        let request = match server.recv() {
            Ok(req) => req,
            Err(e) => return Err(io::Error::new(io::ErrorKind::Other, e)),
        };

        handle_request(request, storage, config);
    }
}

fn handle_request(request: Request, storage: &Arc<MemoryStorage>, config: &PreviewConfig) {
    let method = request.method().clone();
    let path = request.url().to_string();

    // Parse query string for CSP nonce overrides
    let (path, query) = parse_path_and_query(&path);

    let response = match (&method, path) {
        (Method::Get, "/") => handle_index(storage, config, &query),
        (Method::Get, "/json") => handle_list_json(storage),
        (Method::Post, "/clear") => handle_clear(storage),
        (Method::Get, p) => handle_dynamic_route(p, storage),
        _ => not_found(),
    };

    let _ = request.respond(response);
}

fn handle_dynamic_route(path: &str, storage: &Arc<MemoryStorage>) -> Response<io::Cursor<Vec<u8>>> {
    // Strip leading slash
    let path = path.strip_prefix('/').unwrap_or(path);

    // Check for /{uuid}/html
    if let Some(id) = path.strip_suffix("/html") {
        if is_uuid(id) {
            return handle_email_html(id, storage);
        }
    }

    // Check for /{uuid}/attachments/{idx}
    if let Some((id, rest)) = path.split_once("/attachments/") {
        if is_uuid(id) {
            if let Ok(idx) = rest.parse::<usize>() {
                return handle_attachment(id, idx, storage);
            }
        }
    }

    // Check for /{uuid} (single email)
    if is_uuid(path) {
        return handle_view_email(path, storage);
    }

    not_found()
}

// ============================================================================
// Route Handlers
// ============================================================================

fn handle_index(
    storage: &Arc<MemoryStorage>,
    config: &PreviewConfig,
    query: &QueryParams,
) -> Response<io::Cursor<Vec<u8>>> {
    let emails = core::list_emails(storage);
    let script_nonce = query
        .get("script_nonce")
        .or(config.script_nonce.as_deref())
        .map(String::from);
    let style_nonce = query
        .get("style_nonce")
        .or(config.style_nonce.as_deref())
        .map(String::from);

    let html = core::render_index(&emails, script_nonce, style_nonce);
    html_response(html)
}

fn handle_list_json(storage: &Arc<MemoryStorage>) -> Response<io::Cursor<Vec<u8>>> {
    let emails = core::list_emails(storage);
    let response = EmailListResponse { data: emails };
    json_response(&response)
}

fn handle_view_email(id: &str, storage: &Arc<MemoryStorage>) -> Response<io::Cursor<Vec<u8>>> {
    match core::get_email(storage, id) {
        Some(email) => json_response(&email),
        None => not_found(),
    }
}

fn handle_email_html(id: &str, storage: &Arc<MemoryStorage>) -> Response<io::Cursor<Vec<u8>>> {
    match core::get_email_html(storage, id) {
        Some(html) => html_response(html),
        None => not_found(),
    }
}

fn handle_attachment(
    id: &str,
    idx: usize,
    storage: &Arc<MemoryStorage>,
) -> Response<io::Cursor<Vec<u8>>> {
    match core::get_attachment(storage, id, idx) {
        Some(att) => {
            let cursor = io::Cursor::new(att.data);
            let content_type =
                Header::from_bytes("Content-Type", att.content_type.as_bytes()).unwrap();
            let disposition = Header::from_bytes(
                "Content-Disposition",
                format!("attachment; filename=\"{}\"", att.filename).as_bytes(),
            )
            .unwrap();

            Response::from_data(cursor.into_inner())
                .with_header(content_type)
                .with_header(disposition)
        }
        None => not_found(),
    }
}

fn handle_clear(storage: &Arc<MemoryStorage>) -> Response<io::Cursor<Vec<u8>>> {
    core::clear_emails(storage);
    Response::from_data(Vec::new()).with_status_code(StatusCode(204))
}

// ============================================================================
// Response Helpers
// ============================================================================

fn html_response(body: String) -> Response<io::Cursor<Vec<u8>>> {
    let header = Header::from_bytes("Content-Type", "text/html; charset=utf-8").unwrap();
    Response::from_data(body.into_bytes()).with_header(header)
}

fn json_response<T: serde::Serialize>(data: &T) -> Response<io::Cursor<Vec<u8>>> {
    let body = serde_json::to_vec(data).unwrap_or_default();
    let header = Header::from_bytes("Content-Type", "application/json").unwrap();
    Response::from_data(body).with_header(header)
}

fn not_found() -> Response<io::Cursor<Vec<u8>>> {
    Response::from_data(Vec::new()).with_status_code(StatusCode(404))
}

// ============================================================================
// Utilities
// ============================================================================

/// Check if a string looks like a UUID (36 chars, correct format).
fn is_uuid(s: &str) -> bool {
    if s.len() != 36 {
        return false;
    }

    let bytes = s.as_bytes();

    // Check hyphens at positions 8, 13, 18, 23
    if bytes[8] != b'-' || bytes[13] != b'-' || bytes[18] != b'-' || bytes[23] != b'-' {
        return false;
    }

    // Check all other chars are hex
    s.chars().enumerate().all(|(i, c)| {
        if i == 8 || i == 13 || i == 18 || i == 23 {
            true // Already checked hyphens
        } else {
            c.is_ascii_hexdigit()
        }
    })
}

/// Simple query parameter storage.
struct QueryParams {
    params: Vec<(String, String)>,
}

impl QueryParams {
    fn get(&self, key: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

/// Parse path and query string from URL.
fn parse_path_and_query(url: &str) -> (&str, QueryParams) {
    let (path, query_str) = url.split_once('?').unwrap_or((url, ""));

    let params = query_str
        .split('&')
        .filter(|s| !s.is_empty())
        .filter_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            Some((k.to_string(), v.to_string()))
        })
        .collect();

    (path, QueryParams { params })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_uuid() {
        assert!(is_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(is_uuid("00000000-0000-0000-0000-000000000000"));
        assert!(is_uuid("ffffffff-ffff-ffff-ffff-ffffffffffff"));

        assert!(!is_uuid("not-a-uuid"));
        assert!(!is_uuid("550e8400-e29b-41d4-a716-44665544000")); // Too short
        assert!(!is_uuid("550e8400-e29b-41d4-a716-4466554400000")); // Too long
        assert!(!is_uuid("550e8400xe29b-41d4-a716-446655440000")); // Wrong separator
        assert!(!is_uuid("550g8400-e29b-41d4-a716-446655440000")); // Invalid hex char
    }

    #[test]
    fn test_parse_path_and_query() {
        let (path, query) = parse_path_and_query("/");
        assert_eq!(path, "/");
        assert!(query.get("foo").is_none());

        let (path, query) = parse_path_and_query("/?script_nonce=abc123");
        assert_eq!(path, "/");
        assert_eq!(query.get("script_nonce"), Some("abc123"));

        let (path, query) = parse_path_and_query("/?a=1&b=2");
        assert_eq!(path, "/");
        assert_eq!(query.get("a"), Some("1"));
        assert_eq!(query.get("b"), Some("2"));
    }
}
