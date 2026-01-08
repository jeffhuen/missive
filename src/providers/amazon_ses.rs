//! Amazon Simple Email Service (SES) API provider.
//!
//! For reference: [Amazon SES API docs](https://docs.aws.amazon.com/ses/latest/APIReference/Welcome.html)
//!
//! This adapter uses the SES SendRawEmail action and generates SMTP-style MIME messages.
//! It implements AWS Signature v4 for authentication.
//!
//! # Example
//!
//! ```rust,ignore
//! use missive::providers::AmazonSesMailer;
//!
//! let mailer = AmazonSesMailer::new("us-east-1", "AKIAIOSFODNN7EXAMPLE", "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY");
//! ```
//!
//! ## Configuration
//!
//! * `region` - AWS region (e.g., "us-east-1", "eu-west-1")
//! * `access_key` - IAM access key ID
//! * `secret` - IAM secret access key
//!
//! ## Provider Options
//!
//! ```rust,ignore
//! let email = Email::new()
//!     .from("sender@example.com")
//!     .to("recipient@example.com")
//!     .subject("Hello")
//!     .provider_option("tags", vec![
//!         json!({"name": "campaign", "value": "welcome"}),
//!         json!({"name": "env", "value": "production"})
//!     ])
//!     .provider_option("configuration_set_name", "my-config-set")
//!     .provider_option("security_token", "session-token-for-iam-role");
//! ```
//!
//! ## Provider Options Reference
//!
//! * `tags` (list[{name, value}]) - Message tags for tracking
//! * `configuration_set_name` (string) - SES configuration set name
//! * `security_token` (string) - Temporary security token for IAM roles
//!
//! ## IAM Role Authentication
//!
//! When using IAM roles (e.g., on EC2 or ECS), fetch temporary credentials and pass
//! the security token via provider options:
//!
//! ```rust,ignore
//! let email = Email::new()
//!     // ...
//!     .provider_option("security_token", temporary_session_token);
//! ```

use async_trait::async_trait;
use base64::Engine;
use chrono::{DateTime, Utc};
use reqwest::Client;
use ring::hmac;
use sha2::{Digest, Sha256};

use crate::email::Email;
use crate::error::MailError;
use crate::mailer::{DeliveryResult, Mailer};

const SERVICE_NAME: &str = "ses";
const ACTION: &str = "SendRawEmail";
const VERSION: &str = "2010-12-01";
const ENCODING: &str = "AWS4-HMAC-SHA256";

/// Amazon SES API email provider.
pub struct AmazonSesMailer {
    region: String,
    access_key: String,
    secret: String,
    host: Option<String>,
    client: Client,
    // Optional config
    ses_source: Option<String>,
    ses_source_arn: Option<String>,
    ses_from_arn: Option<String>,
    ses_return_path_arn: Option<String>,
}

impl AmazonSesMailer {
    /// Create a new Amazon SES mailer.
    pub fn new(
        region: impl Into<String>,
        access_key: impl Into<String>,
        secret: impl Into<String>,
    ) -> Self {
        Self {
            region: region.into(),
            access_key: access_key.into(),
            secret: secret.into(),
            host: None,
            client: Client::new(),
            ses_source: None,
            ses_source_arn: None,
            ses_from_arn: None,
            ses_return_path_arn: None,
        }
    }

    /// Create with a custom reqwest client.
    pub fn with_client(
        region: impl Into<String>,
        access_key: impl Into<String>,
        secret: impl Into<String>,
        client: Client,
    ) -> Self {
        Self {
            region: region.into(),
            access_key: access_key.into(),
            secret: secret.into(),
            host: None,
            client,
            ses_source: None,
            ses_source_arn: None,
            ses_from_arn: None,
            ses_return_path_arn: None,
        }
    }

    /// Set a custom host (for testing or VPC endpoints).
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    /// Set the SES Source parameter.
    pub fn ses_source(mut self, source: impl Into<String>) -> Self {
        self.ses_source = Some(source.into());
        self
    }

    /// Set the SES SourceArn parameter.
    pub fn ses_source_arn(mut self, arn: impl Into<String>) -> Self {
        self.ses_source_arn = Some(arn.into());
        self
    }

    /// Set the SES FromArn parameter.
    pub fn ses_from_arn(mut self, arn: impl Into<String>) -> Self {
        self.ses_from_arn = Some(arn.into());
        self
    }

    /// Set the SES ReturnPathArn parameter.
    pub fn ses_return_path_arn(mut self, arn: impl Into<String>) -> Self {
        self.ses_return_path_arn = Some(arn.into());
        self
    }

    fn base_url(&self) -> String {
        match &self.host {
            Some(host) => host.clone(),
            None => format!("https://email.{}.amazonaws.com", self.region),
        }
    }

    fn host_header(&self) -> String {
        format!("email.{}.amazonaws.com", self.region)
    }

    fn build_body(&self, email: &Email) -> Result<String, MailError> {
        let raw_message = build_mime_message(email)?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&raw_message);
        let url_encoded = urlencoding::encode(&encoded);

        let mut params = vec![
            ("Action".to_string(), ACTION.to_string()),
            ("Version".to_string(), VERSION.to_string()),
            ("RawMessage.Data".to_string(), url_encoded.into_owned()),
        ];

        // Optional SES parameters
        if let Some(ref source) = self.ses_source {
            params.push(("Source".to_string(), source.clone()));
        }
        if let Some(ref source_arn) = self.ses_source_arn {
            params.push(("SourceArn".to_string(), source_arn.clone()));
        }
        if let Some(ref from_arn) = self.ses_from_arn {
            params.push(("FromArn".to_string(), from_arn.clone()));
        }
        if let Some(ref return_path_arn) = self.ses_return_path_arn {
            params.push(("ReturnPathArn".to_string(), return_path_arn.clone()));
        }

        // Provider options: configuration_set_name
        if let Some(config_set) = email.provider_options.get("configuration_set_name") {
            if let Some(name) = config_set.as_str() {
                params.push(("ConfigurationSetName".to_string(), name.to_string()));
            }
        }

        // Provider options: tags
        if let Some(tags) = email.provider_options.get("tags") {
            if let Some(arr) = tags.as_array() {
                for (i, tag) in arr.iter().enumerate() {
                    let index = i + 1;
                    if let (Some(name), Some(value)) = (
                        tag.get("name").and_then(|v| v.as_str()),
                        tag.get("value").and_then(|v| v.as_str()),
                    ) {
                        params.push((format!("Tags.member.{}.Name", index), name.to_string()));
                        params.push((format!("Tags.member.{}.Value", index), value.to_string()));
                    }
                }
            }
        }

        // Sort params and encode
        params.sort_by(|a, b| a.0.cmp(&b.0));
        let body = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        Ok(body)
    }

    fn sign_request(
        &self,
        body: &str,
        date_time: DateTime<Utc>,
        security_token: Option<&str>,
    ) -> Vec<(String, String)> {
        let host = self.host_header();
        let amz_date_str = amz_datetime(&date_time);
        let date = amz_date(&date_time);

        // Build headers map
        let mut headers = vec![
            ("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string()),
            ("Host".to_string(), host.clone()),
            ("X-Amz-Date".to_string(), amz_date_str.clone()),
            ("Content-Length".to_string(), body.len().to_string()),
        ];

        // Add security token if present
        if let Some(token) = security_token {
            headers.push(("X-Amz-Security-Token".to_string(), token.to_string()));
        }

        // Sort headers for canonical request
        headers.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

        // Build signed headers list
        let signed_headers = headers
            .iter()
            .map(|(k, _)| k.to_lowercase())
            .collect::<Vec<_>>()
            .join(";");

        // Build canonical headers string
        let canonical_headers = headers
            .iter()
            .map(|(k, v)| format!("{}:{}", k.to_lowercase(), v))
            .collect::<Vec<_>>()
            .join("\n");

        // Hash the body
        let body_hash = hex_sha256(body.as_bytes());

        // Build canonical request
        let canonical_request = format!(
            "POST\n/\n\n{}\n\n{}\n{}",
            canonical_headers, signed_headers, body_hash
        );

        let request_hash = hex_sha256(canonical_request.as_bytes());

        // Build string to sign
        let credential_scope = format!("{}/{}/{}/aws4_request", date, self.region, SERVICE_NAME);
        let string_to_sign = format!(
            "{}\n{}\n{}\n{}",
            ENCODING, amz_date_str, credential_scope, request_hash
        );

        // Generate signature
        let signature = self.generate_signature(&string_to_sign, &date_time);

        // Build authorization header
        let authorization = format!(
            "{} Credential={}/{}, SignedHeaders={}, Signature={}",
            ENCODING, self.access_key, credential_scope, signed_headers, signature
        );

        headers.push(("Authorization".to_string(), authorization));

        headers
    }

    fn generate_signature(&self, string_to_sign: &str, date_time: &DateTime<Utc>) -> String {
        let date = amz_date(date_time);

        // AWS4 + secret
        let k_secret = format!("AWS4{}", self.secret);

        // Sign date
        let k_date = hmac_sha256(k_secret.as_bytes(), date.as_bytes());

        // Sign region
        let k_region = hmac_sha256(&k_date, self.region.as_bytes());

        // Sign service
        let k_service = hmac_sha256(&k_region, SERVICE_NAME.as_bytes());

        // Sign "aws4_request"
        let k_signing = hmac_sha256(&k_service, b"aws4_request");

        // Sign the string to sign
        let signature = hmac_sha256(&k_signing, string_to_sign.as_bytes());

        hex::encode(signature)
    }
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let key = hmac::Key::new(hmac::HMAC_SHA256, key);
    hmac::sign(&key, data).as_ref().to_vec()
}

fn hex_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn amz_date(dt: &DateTime<Utc>) -> String {
    dt.format("%Y%m%d").to_string()
}

fn amz_datetime(dt: &DateTime<Utc>) -> String {
    dt.format("%Y%m%dT%H%M%SZ").to_string()
}

/// Build a MIME message from an Email.
fn build_mime_message(email: &Email) -> Result<Vec<u8>, MailError> {
    let from = email
        .from
        .as_ref()
        .ok_or(MailError::MissingField("from"))?;

    if email.to.is_empty() {
        return Err(MailError::MissingField("to"));
    }

    let mut message = String::new();
    let boundary = format!("----=_Part_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));

    // Headers
    message.push_str(&format!("From: {}\r\n", from.formatted()));
    message.push_str(&format!(
        "To: {}\r\n",
        email
            .to
            .iter()
            .map(|a| a.formatted())
            .collect::<Vec<_>>()
            .join(", ")
    ));

    if !email.cc.is_empty() {
        message.push_str(&format!(
            "Cc: {}\r\n",
            email
                .cc
                .iter()
                .map(|a| a.formatted())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // BCC is NOT included in headers (that's the point of BCC)
    // But we need to include them as recipients in the RCPT TO command
    // SES handles this via the raw message destinations

    if let Some(reply_to) = email.reply_to.first() {
        message.push_str(&format!("Reply-To: {}\r\n", reply_to.formatted()));
    }

    message.push_str(&format!("Subject: {}\r\n", email.subject));
    message.push_str("MIME-Version: 1.0\r\n");

    // Custom headers
    for (name, value) in &email.headers {
        message.push_str(&format!("{}: {}\r\n", name, value));
    }

    // Determine content structure
    let has_text = email.text_body.is_some();
    let has_html = email.html_body.is_some();
    let has_attachments = !email.attachments.is_empty();
    let has_inline = email.attachments.iter().any(|a| a.is_inline());

    if !has_attachments {
        // Simple case: no attachments
        if has_text && has_html {
            // Multipart/alternative
            message.push_str(&format!(
                "Content-Type: multipart/alternative; boundary=\"{}\"\r\n\r\n",
                boundary
            ));

            // Text part
            message.push_str(&format!("--{}\r\n", boundary));
            message.push_str("Content-Type: text/plain; charset=utf-8\r\n");
            message.push_str("Content-Transfer-Encoding: quoted-printable\r\n\r\n");
            message.push_str(email.text_body.as_ref().unwrap());
            message.push_str("\r\n");

            // HTML part
            message.push_str(&format!("--{}\r\n", boundary));
            message.push_str("Content-Type: text/html; charset=utf-8\r\n");
            message.push_str("Content-Transfer-Encoding: quoted-printable\r\n\r\n");
            message.push_str(email.html_body.as_ref().unwrap());
            message.push_str("\r\n");

            message.push_str(&format!("--{}--\r\n", boundary));
        } else if has_html {
            message.push_str("Content-Type: text/html; charset=utf-8\r\n");
            message.push_str("Content-Transfer-Encoding: quoted-printable\r\n\r\n");
            message.push_str(email.html_body.as_ref().unwrap());
        } else if has_text {
            message.push_str("Content-Type: text/plain; charset=utf-8\r\n");
            message.push_str("Content-Transfer-Encoding: quoted-printable\r\n\r\n");
            message.push_str(email.text_body.as_ref().unwrap());
        } else {
            message.push_str("Content-Type: text/plain; charset=utf-8\r\n\r\n");
        }
    } else {
        // Complex case: with attachments
        let mixed_boundary = format!("----=_Mixed_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
        let alt_boundary = format!("----=_Alt_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
        let related_boundary = format!("----=_Related_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));

        message.push_str(&format!(
            "Content-Type: multipart/mixed; boundary=\"{}\"\r\n\r\n",
            mixed_boundary
        ));

        // Body part
        message.push_str(&format!("--{}\r\n", mixed_boundary));

        if has_inline && has_html {
            // Use multipart/related for inline attachments
            message.push_str(&format!(
                "Content-Type: multipart/related; boundary=\"{}\"\r\n\r\n",
                related_boundary
            ));

            message.push_str(&format!("--{}\r\n", related_boundary));

            if has_text {
                // Multipart/alternative inside related
                message.push_str(&format!(
                    "Content-Type: multipart/alternative; boundary=\"{}\"\r\n\r\n",
                    alt_boundary
                ));

                message.push_str(&format!("--{}\r\n", alt_boundary));
                message.push_str("Content-Type: text/plain; charset=utf-8\r\n\r\n");
                message.push_str(email.text_body.as_ref().unwrap());
                message.push_str("\r\n");

                message.push_str(&format!("--{}\r\n", alt_boundary));
                message.push_str("Content-Type: text/html; charset=utf-8\r\n\r\n");
                message.push_str(email.html_body.as_ref().unwrap());
                message.push_str("\r\n");

                message.push_str(&format!("--{}--\r\n", alt_boundary));
            } else {
                message.push_str("Content-Type: text/html; charset=utf-8\r\n\r\n");
                message.push_str(email.html_body.as_ref().unwrap());
                message.push_str("\r\n");
            }

            // Inline attachments
            for attachment in email.attachments.iter().filter(|a| a.is_inline()) {
                message.push_str(&format!("--{}\r\n", related_boundary));
                message.push_str(&format!("Content-Type: {}\r\n", attachment.content_type));
                message.push_str("Content-Transfer-Encoding: base64\r\n");
                message.push_str(&format!(
                    "Content-Disposition: inline; filename=\"{}\"\r\n",
                    attachment.filename
                ));
                if let Some(ref cid) = attachment.content_id {
                    message.push_str(&format!("Content-ID: <{}>\r\n", cid));
                }
                message.push_str("\r\n");
                message.push_str(&attachment.base64_data());
                message.push_str("\r\n");
            }

            message.push_str(&format!("--{}--\r\n", related_boundary));
        } else if has_text && has_html {
            // Multipart/alternative
            message.push_str(&format!(
                "Content-Type: multipart/alternative; boundary=\"{}\"\r\n\r\n",
                alt_boundary
            ));

            message.push_str(&format!("--{}\r\n", alt_boundary));
            message.push_str("Content-Type: text/plain; charset=utf-8\r\n\r\n");
            message.push_str(email.text_body.as_ref().unwrap());
            message.push_str("\r\n");

            message.push_str(&format!("--{}\r\n", alt_boundary));
            message.push_str("Content-Type: text/html; charset=utf-8\r\n\r\n");
            message.push_str(email.html_body.as_ref().unwrap());
            message.push_str("\r\n");

            message.push_str(&format!("--{}--\r\n", alt_boundary));
        } else if has_html {
            message.push_str("Content-Type: text/html; charset=utf-8\r\n\r\n");
            message.push_str(email.html_body.as_ref().unwrap());
            message.push_str("\r\n");
        } else if has_text {
            message.push_str("Content-Type: text/plain; charset=utf-8\r\n\r\n");
            message.push_str(email.text_body.as_ref().unwrap());
            message.push_str("\r\n");
        }

        // Regular attachments
        for attachment in email.attachments.iter().filter(|a| !a.is_inline()) {
            message.push_str(&format!("--{}\r\n", mixed_boundary));
            message.push_str(&format!("Content-Type: {}\r\n", attachment.content_type));
            message.push_str("Content-Transfer-Encoding: base64\r\n");
            message.push_str(&format!(
                "Content-Disposition: attachment; filename=\"{}\"\r\n",
                attachment.filename
            ));
            message.push_str("\r\n");
            message.push_str(&attachment.base64_data());
            message.push_str("\r\n");
        }

        message.push_str(&format!("--{}--\r\n", mixed_boundary));
    }

    Ok(message.into_bytes())
}

#[async_trait]
impl Mailer for AmazonSesMailer {
    async fn deliver(&self, email: &Email) -> Result<DeliveryResult, MailError> {
        let body = self.build_body(email)?;
        let date_time = Utc::now();

        // Get security token from provider options
        let security_token = email
            .provider_options
            .get("security_token")
            .and_then(|v| v.as_str());

        let headers = self.sign_request(&body, date_time, security_token);
        let url = self.base_url();

        let mut request = self.client.post(&url);
        for (name, value) in headers {
            request = request.header(&name, &value);
        }
        request = request.header("User-Agent", format!("missive/{}", crate::VERSION));
        request = request.body(body);

        let response = request.send().await?;
        let status = response.status();
        let body = response.text().await?;

        if status.is_success() {
            // Parse XML response
            let message_id = extract_xml_value(&body, "MessageId").unwrap_or_default();
            let request_id = extract_xml_value(&body, "RequestId").unwrap_or_default();

            Ok(DeliveryResult::with_response(
                message_id,
                serde_json::json!({
                    "provider": "amazon_ses",
                    "request_id": request_id,
                }),
            ))
        } else {
            // Parse error XML
            let error_code = extract_xml_value(&body, "Code").unwrap_or_else(|| "Unknown".to_string());
            let error_message =
                extract_xml_value(&body, "Message").unwrap_or_else(|| "Unknown error".to_string());

            Err(MailError::provider_with_status(
                "amazon_ses",
                format!("[{}] {}", error_code, error_message),
                status.as_u16(),
            ))
        }
    }

    fn provider_name(&self) -> &'static str {
        "amazon_ses"
    }
}

/// Simple XML value extractor (avoids XML parsing dependency).
fn extract_xml_value(xml: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{}>", tag);
    let end_tag = format!("</{}>", tag);

    let start = xml.find(&start_tag)? + start_tag.len();
    let end = xml[start..].find(&end_tag)? + start;

    Some(xml[start..end].to_string())
}
