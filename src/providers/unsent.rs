//! Unsent API provider.
//!
//! # Example
//!
//! ```rust,ignore
//! use missive::providers::UnsentMailer;
//!
//! let mailer = UnsentMailer::new("unsent_xxxxx");
//! ```

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::email::Email;
use crate::error::MailError;
use crate::mailer::{DeliveryResult, Mailer};

const UNSENT_API_URL: &str = "https://api.unsend.dev/v1";

/// Unsent API email provider.
pub struct UnsentMailer {
    api_key: String,
    client: Client,
    base_url: String,
}

impl UnsentMailer {
    /// Create a new Unsent mailer with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            client: Client::new(),
            base_url: UNSENT_API_URL.to_string(),
        }
    }

    /// Create with a custom reqwest client.
    pub fn with_client(api_key: impl Into<String>, client: Client) -> Self {
        Self {
            api_key: api_key.into(),
            client,
            base_url: UNSENT_API_URL.to_string(),
        }
    }

    /// Set a custom base URL (for testing).
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    fn build_request(&self, email: &Email) -> Result<UnsentRequest, MailError> {
        let from = email
            .from
            .as_ref()
            .ok_or(MailError::MissingField("from"))?;

        if email.to.is_empty() {
            return Err(MailError::MissingField("to"));
        }

        Ok(UnsentRequest {
            from: from.formatted(),
            to: email.to.iter().map(|a| a.formatted()).collect(),
            subject: email.subject.clone(),
            html: email.html_body.clone(),
            text: email.text_body.clone(),
            cc: if email.cc.is_empty() {
                None
            } else {
                Some(email.cc.iter().map(|a| a.formatted()).collect())
            },
            bcc: if email.bcc.is_empty() {
                None
            } else {
                Some(email.bcc.iter().map(|a| a.formatted()).collect())
            },
            reply_to: email.reply_to.first().map(|a| a.formatted()),
        })
    }
}

#[async_trait]
impl Mailer for UnsentMailer {
    async fn deliver(&self, email: &Email) -> Result<DeliveryResult, MailError> {
        let request = self.build_request(email)?;

        let url = format!("{}/emails", self.base_url);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("User-Agent", format!("missive/{}", crate::VERSION))
            .json(&request)
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            let result: UnsentResponse = response.json().await?;
            Ok(DeliveryResult::with_response(
                result.email_id,
                serde_json::json!({ "provider": "unsent" }),
            ))
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(MailError::provider_with_status(
                "unsent",
                error_text,
                status.as_u16(),
            ))
        }
    }

    fn provider_name(&self) -> &'static str {
        "unsent"
    }
}

// ============================================================================
// Unsent API Types
// ============================================================================

#[derive(Debug, Serialize)]
struct UnsentRequest {
    from: String,
    to: Vec<String>,
    subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cc: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bcc: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UnsentResponse {
    #[serde(rename = "emailId")]
    email_id: String,
}
