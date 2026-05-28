//! JMAP provider for sending emails via JMAP-compliant servers.
//!
//! This is a minimal JMAP client implementation for email submission only.
//! It works with any JMAP-compliant server including:
//!
//! - [Stalwart Mail Server](https://stalw.art/)
//! - [Fastmail](https://www.fastmail.com/)
//! - [Cyrus IMAP](https://www.cyrusimap.org/)
//!
//! # How It Works
//!
//! JMAP (JSON Meta Application Protocol) is a modern, stateless alternative
//! to IMAP/SMTP that uses JSON over HTTP. Sending an email requires:
//!
//! 1. Session discovery (GET `/.well-known/jmap`)
//! 2. Fetch the drafts mailbox ID (`Mailbox/get`)
//! 3. Create email in drafts (`Email/set`)
//! 4. Submit for delivery (`EmailSubmission/set`)
//!
//! This provider handles all steps in a single `deliver()` call.
//!
//! # JMAP Submission Workflow
//!
//! Per [RFC 8621 Section 4](https://www.rfc-editor.org/rfc/rfc8621#section-4),
//! emails in JMAP must belong to at least one mailbox at all times. This
//! provider follows the standard submission pattern:
//!
//! 1. Create the email in the user's drafts mailbox
//! 2. Submit via `EmailSubmission/set` with `onSuccessDestroyEmail`
//! 3. The server automatically deletes the draft after successful delivery
//!
//! This ensures spec compliance across all JMAP servers.
//!
//! # Example
//!
//! ```rust,ignore
//! use missive::providers::JmapMailer;
//!
//! // Basic auth
//! let mailer = JmapMailer::new("https://jmap.example.com")
//!     .credentials("username", "password")
//!     .build();
//!
//! // Bearer token (OAuth2)
//! let mailer = JmapMailer::new("https://jmap.example.com")
//!     .bearer_token("your-oauth-token")
//!     .build();
//! ```
//!
//! # Environment Variables
//!
//! ```bash
//! EMAIL_PROVIDER=jmap
//! JMAP_URL=https://jmap.example.com
//! JMAP_USERNAME=your-username
//! JMAP_PASSWORD=your-password
//! # Or use bearer token instead:
//! # JMAP_BEARER_TOKEN=your-oauth-token
//! ```

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::address::Address;
use crate::email::{Email, PreparedEmail};
use crate::error::MailError;
use crate::mailer::{DeliveryResult, Mailer};

fn jmap_address(addr: &Address) -> Result<Value, MailError> {
    Ok(json!({
        "name": addr.name.clone(),
        "email": addr.to_ascii()?,
    }))
}

/// JMAP email provider.
///
/// A minimal JMAP client for email submission. Works with any
/// JMAP-compliant server (Stalwart, Fastmail, Cyrus, etc.).
pub struct JmapMailer {
    session_url: String,
    auth: JmapAuth,
    client: Client,
    /// Cached session data (API URL, account ID, identity ID)
    session: JmapSessionCache,
}

#[derive(Clone)]
enum JmapAuth {
    Basic { username: String, password: String },
    Bearer { token: String },
}

#[derive(Clone)]
struct JmapSession {
    api_url: String,
    account_id: String,
    identity_id: Option<String>,
    drafts_mailbox_id: Option<String>,
}

impl JmapMailer {
    /// Create a new JMAP mailer builder.
    ///
    /// The URL should be either:
    /// - The JMAP session URL directly (e.g., `https://jmap.example.com/session`)
    /// - The server base URL (will append `/.well-known/jmap`)
    #[allow(clippy::new_ret_no_self)]
    pub fn new(url: &str) -> JmapBuilder {
        // Normalize URL to session endpoint
        let session_url = if url.ends_with("/session") || url.contains("/.well-known/jmap") {
            url.to_string()
        } else {
            format!("{}/.well-known/jmap", url.trim_end_matches('/'))
        };

        JmapBuilder {
            session_url,
            auth: None,
            client: None,
            test_session: None,
        }
    }

    /// Fetch or return cached JMAP session.
    async fn get_session(&self) -> Result<JmapSession, MailError> {
        if let Some(session) = self.cached_session() {
            return Ok(session);
        }

        // Fetch session
        let session = self.fetch_session().await?;
        self.cache_session(session.clone());

        Ok(session)
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    fn cached_session(&self) -> Option<JmapSession> {
        self.session.read().clone()
    }

    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    fn cached_session(&self) -> Option<JmapSession> {
        read_session_cache(&self.session).clone()
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    fn cache_session(&self, session: JmapSession) {
        *self.session.write() = Some(session);
    }

    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    fn cache_session(&self, session: JmapSession) {
        *write_session_cache(&self.session) = Some(session);
    }

    /// Fetch JMAP session from server.
    async fn fetch_session(&self) -> Result<JmapSession, MailError> {
        let req = self.apply_auth(self.client.get(&self.session_url));
        let response = req.send().await?;

        if !response.status().is_success() {
            return Err(MailError::provider_with_status(
                "jmap",
                format!("Session discovery failed: {}", response.status()),
                response.status().as_u16(),
            ));
        }

        let session: JmapSessionResponse = response.json().await?;

        // Get the primary account ID
        let account_id = session
            .primary_accounts
            .get("urn:ietf:params:jmap:mail")
            .or_else(|| {
                session
                    .primary_accounts
                    .get("urn:ietf:params:jmap:submission")
            })
            .or_else(|| session.accounts.keys().next())
            .ok_or_else(|| MailError::Configuration("No JMAP mail account found".into()))?
            .clone();

        // Try to find an identity ID for submission
        let identity_id = None; // Will be fetched on first send if needed

        Ok(JmapSession {
            api_url: session.api_url,
            account_id,
            identity_id,
            drafts_mailbox_id: None, // Will be fetched on first send
        })
    }

    /// Apply authentication to a request.
    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.auth {
            JmapAuth::Basic { username, password } => req.basic_auth(username, Some(password)),
            JmapAuth::Bearer { token } => req.bearer_auth(token),
        }
    }

    /// Fetch identity ID if not cached.
    async fn get_identity_id(&self, session: &JmapSession) -> Result<String, MailError> {
        if let Some(ref id) = session.identity_id {
            return Ok(id.clone());
        }

        // Fetch identities from server
        let request = JmapRequest {
            using: vec![
                "urn:ietf:params:jmap:core".into(),
                "urn:ietf:params:jmap:submission".into(),
            ],
            method_calls: vec![(
                "Identity/get".into(),
                json!({
                    "accountId": session.account_id,
                }),
                "i0".into(),
            )],
        };

        let req = self.apply_auth(self.client.post(&session.api_url));
        let response = req
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            // Fall back to using account ID as identity
            return Ok(session.account_id.clone());
        }

        let jmap_response: JmapResponse = response.json().await?;

        // Extract identity ID from response
        for (method, result, _) in jmap_response.method_responses {
            if method == "Identity/get" {
                if let Some(list) = result.get("list").and_then(|l| l.as_array()) {
                    if let Some(first) = list.first() {
                        if let Some(id) = first.get("id").and_then(|i| i.as_str()) {
                            return Ok(id.to_string());
                        }
                    }
                }
            }
        }

        // Fall back to account ID
        Ok(session.account_id.clone())
    }

    /// Fetch drafts mailbox ID if not cached.
    ///
    /// Per RFC 8621, emails must belong to at least one mailbox. We use the
    /// drafts mailbox for outgoing emails, falling back to inbox if drafts
    /// doesn't exist. The email is automatically destroyed after successful
    /// submission via `onSuccessDestroyEmail`.
    async fn get_drafts_mailbox_id(&self, session: &JmapSession) -> Result<String, MailError> {
        if let Some(ref id) = session.drafts_mailbox_id {
            return Ok(id.clone());
        }

        // Fetch mailboxes from server
        let request = JmapRequest {
            using: vec![
                "urn:ietf:params:jmap:core".into(),
                "urn:ietf:params:jmap:mail".into(),
            ],
            method_calls: vec![(
                "Mailbox/get".into(),
                json!({
                    "accountId": session.account_id,
                }),
                "m0".into(),
            )],
        };

        let req = self.apply_auth(self.client.post(&session.api_url));
        let response = req
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(MailError::provider_with_status(
                "jmap",
                "Failed to fetch mailboxes",
                response.status().as_u16(),
            ));
        }

        let jmap_response: JmapResponse = response.json().await?;

        // Find drafts mailbox (role = "drafts") or fall back to first mailbox
        for (method, result, _) in jmap_response.method_responses {
            if method == "Mailbox/get" {
                if let Some(list) = result.get("list").and_then(|l| l.as_array()) {
                    // First try to find drafts
                    for mailbox in list {
                        if mailbox.get("role").and_then(|r| r.as_str()) == Some("drafts") {
                            if let Some(id) = mailbox.get("id").and_then(|i| i.as_str()) {
                                return Ok(id.to_string());
                            }
                        }
                    }
                    // Fall back to inbox
                    for mailbox in list {
                        if mailbox.get("role").and_then(|r| r.as_str()) == Some("inbox") {
                            if let Some(id) = mailbox.get("id").and_then(|i| i.as_str()) {
                                return Ok(id.to_string());
                            }
                        }
                    }
                    // Fall back to first mailbox
                    if let Some(first) = list.first() {
                        if let Some(id) = first.get("id").and_then(|i| i.as_str()) {
                            return Ok(id.to_string());
                        }
                    }
                }
            }
        }

        Err(MailError::Configuration("No mailboxes found".into()))
    }

    /// Build the JMAP Email object from our Email struct.
    async fn build_email_object(
        &self,
        email: &Email,
        mailbox_id: &str,
    ) -> Result<Value, MailError> {
        let from = email.from.as_ref().ok_or(MailError::MissingField("from"))?;

        if email.to.is_empty() {
            return Err(MailError::MissingField("to"));
        }

        // Build address objects
        let from_addrs: Vec<Value> = vec![jmap_address(from)?];

        let to_addrs: Vec<Value> = email
            .to
            .iter()
            .map(jmap_address)
            .collect::<Result<_, _>>()?;

        let cc_addrs: Option<Vec<Value>> = if email.cc.is_empty() {
            None
        } else {
            Some(
                email
                    .cc
                    .iter()
                    .map(jmap_address)
                    .collect::<Result<_, _>>()?,
            )
        };

        let bcc_addrs: Option<Vec<Value>> = if email.bcc.is_empty() {
            None
        } else {
            Some(
                email
                    .bcc
                    .iter()
                    .map(jmap_address)
                    .collect::<Result<_, _>>()?,
            )
        };

        let reply_to: Option<Vec<Value>> = if email.reply_to.is_empty() {
            None
        } else {
            Some(
                email
                    .reply_to
                    .iter()
                    .map(jmap_address)
                    .collect::<Result<_, _>>()?,
            )
        };

        // Build body parts
        let mut body_values: HashMap<String, Value> = HashMap::new();
        let mut text_body: Vec<Value> = vec![];
        let mut html_body: Vec<Value> = vec![];

        if let Some(ref text) = email.text_body {
            body_values.insert(
                "text".into(),
                json!({
                    "value": text,
                    "isEncodingProblem": false,
                    "isTruncated": false,
                }),
            );
            text_body.push(json!({
                "partId": "text",
                "type": "text/plain",
            }));
        }

        if let Some(ref html) = email.html_body {
            body_values.insert(
                "html".into(),
                json!({
                    "value": html,
                    "isEncodingProblem": false,
                    "isTruncated": false,
                }),
            );
            html_body.push(json!({
                "partId": "html",
                "type": "text/html",
            }));
        }

        // Build attachments
        let attachments: Option<Vec<Value>> = if email.attachments.is_empty() {
            None
        } else {
            let mut attachments = Vec::with_capacity(email.attachments.len());
            for (i, a) in email.attachments.iter().enumerate() {
                let part_id = format!("att{}", i);
                body_values.insert(
                    part_id.clone(),
                    json!({
                        "value": a.base64_data_async().await?,
                        "isEncodingProblem": false,
                        "isTruncated": false,
                    }),
                );
                let mut att = json!({
                    "partId": part_id,
                    "type": a.content_type,
                    "name": a.filename,
                    "disposition": if a.is_inline() { "inline" } else { "attachment" },
                });
                if let Some(ref cid) = a.content_id {
                    att["cid"] = json!(cid);
                }
                attachments.push(att);
            }
            Some(attachments)
        };

        // Build custom headers
        let headers: Option<Vec<Value>> = if email.headers.is_empty() {
            None
        } else {
            Some(
                email
                    .headers
                    .iter()
                    .map(|(k, v)| {
                        json!({
                            "name": k,
                            "value": v,
                        })
                    })
                    .collect(),
            )
        };

        let mut email_obj = json!({
            "mailboxIds": { mailbox_id: true },
            "from": from_addrs,
            "to": to_addrs,
            "subject": email.subject,
            "bodyValues": body_values,
        });

        // Add optional fields
        if !text_body.is_empty() {
            email_obj["textBody"] = json!(text_body);
        }
        if !html_body.is_empty() {
            email_obj["htmlBody"] = json!(html_body);
        }
        if let Some(cc) = cc_addrs {
            email_obj["cc"] = json!(cc);
        }
        if let Some(bcc) = bcc_addrs {
            email_obj["bcc"] = json!(bcc);
        }
        if let Some(rt) = reply_to {
            email_obj["replyTo"] = json!(rt);
        }
        if let Some(atts) = attachments {
            email_obj["attachments"] = json!(atts);
        }
        if let Some(hdrs) = headers {
            email_obj["headers"] = json!(hdrs);
        }

        // Mark for sending without saving to mailbox
        email_obj["keywords"] = json!({ "$draft": true });

        Ok(email_obj)
    }
}

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
type JmapSessionCache = parking_lot::RwLock<Option<JmapSession>>;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
type JmapSessionCache = std::sync::RwLock<Option<JmapSession>>;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
fn session_cache(session: Option<JmapSession>) -> JmapSessionCache {
    parking_lot::RwLock::new(session)
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
fn session_cache(session: Option<JmapSession>) -> JmapSessionCache {
    std::sync::RwLock::new(session)
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
fn read_session_cache<T>(lock: &std::sync::RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
fn write_session_cache<T>(lock: &std::sync::RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Builder for JmapMailer.
pub struct JmapBuilder {
    session_url: String,
    auth: Option<JmapAuth>,
    client: Option<Client>,
    /// Pre-configured session for testing (bypasses session discovery)
    test_session: Option<(String, String, Option<String>)>, // (api_url, account_id, drafts_mailbox_id)
}

impl JmapBuilder {
    /// Set basic authentication credentials.
    pub fn credentials(mut self, username: &str, password: &str) -> Self {
        self.auth = Some(JmapAuth::Basic {
            username: username.to_string(),
            password: password.to_string(),
        });
        self
    }

    /// Set bearer token authentication (OAuth2).
    pub fn bearer_token(mut self, token: &str) -> Self {
        self.auth = Some(JmapAuth::Bearer {
            token: token.to_string(),
        });
        self
    }

    /// Use a custom reqwest client.
    pub fn client(mut self, client: Client) -> Self {
        self.client = Some(client);
        self
    }

    /// Set a pre-configured session for testing (bypasses session discovery).
    ///
    /// This is useful for unit tests where you want to mock the JMAP API
    /// without needing to mock the session discovery endpoint.
    #[doc(hidden)]
    pub fn test_session(mut self, api_url: &str, account_id: &str) -> Self {
        self.test_session = Some((api_url.to_string(), account_id.to_string(), None));
        self
    }

    /// Set a pre-configured session with drafts mailbox for testing.
    #[doc(hidden)]
    pub fn test_session_with_mailbox(
        mut self,
        api_url: &str,
        account_id: &str,
        drafts_mailbox_id: &str,
    ) -> Self {
        self.test_session = Some((
            api_url.to_string(),
            account_id.to_string(),
            Some(drafts_mailbox_id.to_string()),
        ));
        self
    }

    /// Build the JmapMailer.
    pub fn build(self) -> JmapMailer {
        // If test_session is provided, pre-populate the session cache
        let session = self
            .test_session
            .map(|(api_url, account_id, drafts_mailbox_id)| JmapSession {
                api_url,
                account_id,
                identity_id: Some("default".to_string()),
                drafts_mailbox_id,
            });

        JmapMailer {
            session_url: self.session_url,
            auth: self.auth.unwrap_or(JmapAuth::Basic {
                username: String::new(),
                password: String::new(),
            }),
            client: self.client.unwrap_or_default(),
            session: session_cache(session),
        }
    }
}

#[cfg_attr(
    all(target_family = "wasm", target_os = "unknown"),
    async_trait(?Send)
)]
#[cfg_attr(not(all(target_family = "wasm", target_os = "unknown")), async_trait)]
impl Mailer for JmapMailer {
    async fn deliver_prepared(&self, email: &PreparedEmail) -> Result<DeliveryResult, MailError> {
        // Get session info
        let session = self.get_session().await?;
        let identity_id = self.get_identity_id(&session).await?;
        let mailbox_id = self.get_drafts_mailbox_id(&session).await?;

        // Build email object
        let email_obj = self.build_email_object(email, &mailbox_id).await?;

        // Build JMAP request with Email/set and EmailSubmission/set
        let request = JmapRequest {
            using: vec![
                "urn:ietf:params:jmap:core".into(),
                "urn:ietf:params:jmap:mail".into(),
                "urn:ietf:params:jmap:submission".into(),
            ],
            method_calls: vec![
                // First create the email
                (
                    "Email/set".into(),
                    json!({
                        "accountId": session.account_id,
                        "create": {
                            "draft": email_obj,
                        },
                    }),
                    "e0".into(),
                ),
                // Then submit it for delivery
                (
                    "EmailSubmission/set".into(),
                    json!({
                        "accountId": session.account_id,
                        "create": {
                            "sub": {
                                "emailId": "#draft",
                                "identityId": identity_id,
                            },
                        },
                        "onSuccessDestroyEmail": ["#sub"],
                    }),
                    "s0".into(),
                ),
            ],
        };

        // Send request
        let req = self.apply_auth(self.client.post(&session.api_url));
        let response = req
            .header("Content-Type", "application/json")
            .header("User-Agent", format!("missive/{}", crate::VERSION))
            .json(&request)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(MailError::provider_with_status(
                "jmap",
                format!("JMAP request failed: {}", error_text),
                status.as_u16(),
            ));
        }

        let jmap_response: JmapResponse = response.json().await?;

        // Check for errors in method responses
        for (method, result, _) in &jmap_response.method_responses {
            if method == "error" {
                let error_type = result
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("unknown");
                let description = result
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("Unknown error");
                return Err(MailError::ProviderError {
                    provider: "jmap",
                    message: format!("{}: {}", error_type, description),
                    status: None,
                });
            }

            // Check for Email/set or EmailSubmission/set errors
            if method == "Email/set" || method == "EmailSubmission/set" {
                if let Some(not_created) = result.get("notCreated") {
                    if let Some(obj) = not_created.as_object() {
                        if let Some((_, error)) = obj.into_iter().next() {
                            let error_type = error
                                .get("type")
                                .and_then(|t| t.as_str())
                                .unwrap_or("unknown");
                            let description = error
                                .get("description")
                                .and_then(|d| d.as_str())
                                .unwrap_or("Creation failed");
                            return Err(MailError::ProviderError {
                                provider: "jmap",
                                message: format!("{}: {}", error_type, description),
                                status: None,
                            });
                        }
                    }
                }
            }
        }

        // Extract submission ID from response
        let mut submission_id = uuid::Uuid::new_v4().to_string();
        for (method, result, _) in &jmap_response.method_responses {
            if method == "EmailSubmission/set" {
                if let Some(created) = result.get("created") {
                    if let Some(sub) = created.get("sub") {
                        if let Some(id) = sub.get("id").and_then(|i| i.as_str()) {
                            submission_id = id.to_string();
                        }
                    }
                }
            }
        }

        Ok(DeliveryResult::with_response(
            submission_id,
            json!({ "provider": "jmap" }),
        ))
    }

    fn provider_name(&self) -> &'static str {
        "jmap"
    }
}

// ============================================================================
// JMAP Protocol Types
// ============================================================================

#[derive(Debug, Serialize)]
struct JmapRequest {
    using: Vec<String>,
    #[serde(rename = "methodCalls")]
    method_calls: Vec<(String, Value, String)>,
}

#[derive(Debug, Deserialize)]
struct JmapResponse {
    #[serde(rename = "methodResponses")]
    method_responses: Vec<(String, Value, String)>,
}

#[derive(Debug, Deserialize)]
struct JmapSessionResponse {
    #[serde(rename = "apiUrl")]
    api_url: String,
    accounts: HashMap<String, Value>,
    #[serde(rename = "primaryAccounts", default)]
    primary_accounts: HashMap<String, String>,
}
