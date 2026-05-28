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
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::{Digest, Sha256};

use crate::address::{encode_rfc2047_phrase, validate_header_component};
use crate::email::{Email, PreparedEmail};
use crate::error::MailError;
use crate::mailer::{DeliveryResult, Mailer};

const SERVICE_NAME: &str = "ses";
const ACTION: &str = "SendRawEmail";
const VERSION: &str = "2010-12-01";
const ENCODING: &str = "AWS4-HMAC-SHA256";
type HmacSha256 = Hmac<Sha256>;

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

    async fn build_body(&self, email: &Email) -> Result<String, MailError> {
        let raw_message = build_mime_message(email).await?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&raw_message);

        let mut params = vec![
            ("Action".to_string(), ACTION.to_string()),
            ("Version".to_string(), VERSION.to_string()),
            ("RawMessage.Data".to_string(), encoded),
        ];

        for (index, destination) in email.all_recipients().into_iter().enumerate() {
            params.push((
                format!("Destinations.member.{}", index + 1),
                destination.to_ascii()?,
            ));
        }

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
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
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
            (
                "Content-Type".to_string(),
                "application/x-www-form-urlencoded".to_string(),
            ),
            ("Host".to_string(), host.clone()),
            ("X-Amz-Date".to_string(), amz_date_str.clone()),
            ("Content-Length".to_string(), body.len().to_string()),
        ];

        // Add security token if present
        if let Some(token) = security_token {
            headers.push(("X-Amz-Security-Token".to_string(), token.to_string()));
        }

        // Sort headers for canonical request
        headers.sort_by_key(|(name, _)| name.to_lowercase());

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
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC-SHA256 accepts keys of any size");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
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

fn encoded_header_value(field: &str, value: &str) -> Result<String, MailError> {
    validate_header_component(field, value)?;

    if value.is_ascii() {
        Ok(value.to_string())
    } else {
        Ok(encode_rfc2047_phrase(value))
    }
}

fn validate_header_name(name: &str) -> Result<(), MailError> {
    if name.is_empty() || !name.bytes().all(|b| matches!(b, b'!'..=b'~') && b != b':') {
        return Err(MailError::BuildError(format!(
            "Invalid header name: {name}"
        )));
    }

    Ok(())
}

fn push_header(message: &mut String, name: &str, value: &str) -> Result<(), MailError> {
    validate_header_name(name)?;
    let value = encoded_header_value(name, value)?;
    message.push_str(&format!("{name}: {value}\r\n"));
    Ok(())
}

fn quoted_parameter(value: &str) -> Result<String, MailError> {
    validate_header_component("MIME parameter", value)?;
    Ok(value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn content_disposition(kind: &str, filename: &str) -> Result<String, MailError> {
    validate_header_component("attachment filename", filename)?;

    if filename.is_ascii() {
        Ok(format!(
            "Content-Disposition: {kind}; filename=\"{}\"\r\n",
            quoted_parameter(filename)?
        ))
    } else {
        Ok(format!(
            "Content-Disposition: {kind}; filename*=UTF-8''{}\r\n",
            urlencoding::encode(filename)
        ))
    }
}

fn content_id_header(content_id: &str) -> Result<String, MailError> {
    validate_header_component("attachment content ID", content_id)?;
    if content_id.contains('<') || content_id.contains('>') {
        return Err(MailError::BuildError(
            "attachment content ID must not contain angle brackets".into(),
        ));
    }

    Ok(format!("Content-ID: <{content_id}>\r\n"))
}

fn push_wrapped_base64(message: &mut String, encoded: &str) {
    if encoded.is_empty() {
        message.push_str("\r\n");
        return;
    }

    for chunk in encoded.as_bytes().chunks(76) {
        for byte in chunk {
            message.push(char::from(*byte));
        }
        message.push_str("\r\n");
    }
}

async fn push_inline_attachments(
    message: &mut String,
    related_boundary: &str,
    email: &Email,
) -> Result<(), MailError> {
    for attachment in email.attachments.iter().filter(|a| a.is_inline()) {
        message.push_str(&format!("--{}\r\n", related_boundary));
        validate_header_component("attachment content type", &attachment.content_type)?;
        message.push_str(&format!("Content-Type: {}\r\n", attachment.content_type));
        message.push_str("Content-Transfer-Encoding: base64\r\n");
        message.push_str(&content_disposition("inline", &attachment.filename)?);
        if let Some(ref cid) = attachment.content_id {
            message.push_str(&content_id_header(cid)?);
        }
        message.push_str("\r\n");
        push_wrapped_base64(message, &attachment.base64_data_async().await?);
    }

    message.push_str(&format!("--{}--\r\n", related_boundary));

    Ok(())
}

/// Build a MIME message from an Email.
async fn build_mime_message(email: &Email) -> Result<Vec<u8>, MailError> {
    let from = email.from.as_ref().ok_or(MailError::MissingField("from"))?;

    if email.to.is_empty() {
        return Err(MailError::MissingField("to"));
    }

    let mut message = String::new();
    let boundary = format!(
        "----=_Part_{}",
        uuid::Uuid::new_v4().to_string().replace("-", "")
    );

    // Headers
    message.push_str(&format!("From: {}\r\n", from.formatted_rfc5322_ascii()?));
    message.push_str(&format!(
        "To: {}\r\n",
        email
            .to
            .iter()
            .map(|a| a.formatted_rfc5322_ascii())
            .collect::<Result<Vec<_>, _>>()?
            .join(", ")
    ));

    if !email.cc.is_empty() {
        message.push_str(&format!(
            "Cc: {}\r\n",
            email
                .cc
                .iter()
                .map(|a| a.formatted_rfc5322_ascii())
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        ));
    }

    // BCC is NOT included in headers (that's the point of BCC)
    // But we need to include them as recipients in the RCPT TO command
    // SES handles this via the raw message destinations

    if let Some(reply_to) = email.reply_to.first() {
        message.push_str(&format!(
            "Reply-To: {}\r\n",
            reply_to.formatted_rfc5322_ascii()?
        ));
    }

    push_header(&mut message, "Subject", &email.subject)?;
    message.push_str("MIME-Version: 1.0\r\n");

    // Custom headers
    for (name, value) in &email.headers {
        push_header(&mut message, name, value)?;
    }

    // Determine content structure
    let text_body = email.text_body.as_deref();
    let html_body = email.html_body.as_deref();
    let has_attachments = !email.attachments.is_empty();
    let has_inline = email.attachments.iter().any(|a| a.is_inline());

    if !has_attachments {
        // Simple case: no attachments
        match (text_body, html_body) {
            (Some(text), Some(html)) => {
                // Multipart/alternative
                message.push_str(&format!(
                    "Content-Type: multipart/alternative; boundary=\"{}\"\r\n\r\n",
                    boundary
                ));

                // Text part
                message.push_str(&format!("--{}\r\n", boundary));
                message.push_str("Content-Type: text/plain; charset=utf-8\r\n");
                message.push_str("Content-Transfer-Encoding: quoted-printable\r\n\r\n");
                message.push_str(text);
                message.push_str("\r\n");

                // HTML part
                message.push_str(&format!("--{}\r\n", boundary));
                message.push_str("Content-Type: text/html; charset=utf-8\r\n");
                message.push_str("Content-Transfer-Encoding: quoted-printable\r\n\r\n");
                message.push_str(html);
                message.push_str("\r\n");

                message.push_str(&format!("--{}--\r\n", boundary));
            }
            (None, Some(html)) => {
                message.push_str("Content-Type: text/html; charset=utf-8\r\n");
                message.push_str("Content-Transfer-Encoding: quoted-printable\r\n\r\n");
                message.push_str(html);
            }
            (Some(text), None) => {
                message.push_str("Content-Type: text/plain; charset=utf-8\r\n");
                message.push_str("Content-Transfer-Encoding: quoted-printable\r\n\r\n");
                message.push_str(text);
            }
            (None, None) => {
                message.push_str("Content-Type: text/plain; charset=utf-8\r\n\r\n");
            }
        }
    } else {
        // Complex case: with attachments
        let mixed_boundary = format!(
            "----=_Mixed_{}",
            uuid::Uuid::new_v4().to_string().replace("-", "")
        );
        let alt_boundary = format!(
            "----=_Alt_{}",
            uuid::Uuid::new_v4().to_string().replace("-", "")
        );
        let related_boundary = format!(
            "----=_Related_{}",
            uuid::Uuid::new_v4().to_string().replace("-", "")
        );

        message.push_str(&format!(
            "Content-Type: multipart/mixed; boundary=\"{}\"\r\n\r\n",
            mixed_boundary
        ));

        // Body part
        message.push_str(&format!("--{}\r\n", mixed_boundary));

        match (text_body, html_body) {
            (Some(text), Some(html)) if has_inline => {
                // Use multipart/related for inline attachments
                message.push_str(&format!(
                    "Content-Type: multipart/related; boundary=\"{}\"\r\n\r\n",
                    related_boundary
                ));

                message.push_str(&format!("--{}\r\n", related_boundary));

                // Multipart/alternative inside related
                message.push_str(&format!(
                    "Content-Type: multipart/alternative; boundary=\"{}\"\r\n\r\n",
                    alt_boundary
                ));

                message.push_str(&format!("--{}\r\n", alt_boundary));
                message.push_str("Content-Type: text/plain; charset=utf-8\r\n\r\n");
                message.push_str(text);
                message.push_str("\r\n");

                message.push_str(&format!("--{}\r\n", alt_boundary));
                message.push_str("Content-Type: text/html; charset=utf-8\r\n\r\n");
                message.push_str(html);
                message.push_str("\r\n");

                message.push_str(&format!("--{}--\r\n", alt_boundary));
                push_inline_attachments(&mut message, &related_boundary, email).await?;
            }
            (None, Some(html)) if has_inline => {
                // Use multipart/related for inline attachments
                message.push_str(&format!(
                    "Content-Type: multipart/related; boundary=\"{}\"\r\n\r\n",
                    related_boundary
                ));

                message.push_str(&format!("--{}\r\n", related_boundary));
                message.push_str("Content-Type: text/html; charset=utf-8\r\n\r\n");
                message.push_str(html);
                message.push_str("\r\n");
                push_inline_attachments(&mut message, &related_boundary, email).await?;
            }
            (Some(text), None) if has_inline => {
                message.push_str(&format!(
                    "Content-Type: multipart/related; boundary=\"{}\"\r\n\r\n",
                    related_boundary
                ));

                message.push_str(&format!("--{}\r\n", related_boundary));
                message.push_str("Content-Type: text/plain; charset=utf-8\r\n\r\n");
                message.push_str(text);
                message.push_str("\r\n");
                push_inline_attachments(&mut message, &related_boundary, email).await?;
            }
            (None, None) if has_inline => {
                message.push_str(&format!(
                    "Content-Type: multipart/related; boundary=\"{}\"\r\n\r\n",
                    related_boundary
                ));

                message.push_str(&format!("--{}\r\n", related_boundary));
                message.push_str("Content-Type: text/plain; charset=utf-8\r\n\r\n");
                push_inline_attachments(&mut message, &related_boundary, email).await?;
            }
            (Some(text), Some(html)) => {
                // Multipart/alternative
                message.push_str(&format!(
                    "Content-Type: multipart/alternative; boundary=\"{}\"\r\n\r\n",
                    alt_boundary
                ));

                message.push_str(&format!("--{}\r\n", alt_boundary));
                message.push_str("Content-Type: text/plain; charset=utf-8\r\n\r\n");
                message.push_str(text);
                message.push_str("\r\n");

                message.push_str(&format!("--{}\r\n", alt_boundary));
                message.push_str("Content-Type: text/html; charset=utf-8\r\n\r\n");
                message.push_str(html);
                message.push_str("\r\n");

                message.push_str(&format!("--{}--\r\n", alt_boundary));
            }
            (None, Some(html)) => {
                message.push_str("Content-Type: text/html; charset=utf-8\r\n\r\n");
                message.push_str(html);
                message.push_str("\r\n");
            }
            (Some(text), None) => {
                message.push_str("Content-Type: text/plain; charset=utf-8\r\n\r\n");
                message.push_str(text);
                message.push_str("\r\n");
            }
            (None, None) => {
                message.push_str("Content-Type: text/plain; charset=utf-8\r\n\r\n");
            }
        }

        // Regular attachments
        for attachment in email.attachments.iter().filter(|a| !a.is_inline()) {
            message.push_str(&format!("--{}\r\n", mixed_boundary));
            validate_header_component("attachment content type", &attachment.content_type)?;
            message.push_str(&format!("Content-Type: {}\r\n", attachment.content_type));
            message.push_str("Content-Transfer-Encoding: base64\r\n");
            message.push_str(&content_disposition("attachment", &attachment.filename)?);
            message.push_str("\r\n");
            push_wrapped_base64(&mut message, &attachment.base64_data_async().await?);
        }

        message.push_str(&format!("--{}--\r\n", mixed_boundary));
    }

    Ok(message.into_bytes())
}

#[cfg_attr(
    all(target_family = "wasm", target_os = "unknown"),
    async_trait(?Send)
)]
#[cfg_attr(not(all(target_family = "wasm", target_os = "unknown")), async_trait)]
impl Mailer for AmazonSesMailer {
    async fn deliver_prepared(&self, email: &PreparedEmail) -> Result<DeliveryResult, MailError> {
        let body = self.build_body(email).await?;
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
            let error_code =
                extract_xml_value(&body, "Code").unwrap_or_else(|| "Unknown".to_string());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Attachment, Email};

    fn email() -> Email {
        Email::new()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Hello")
            .text_body("Hello")
    }

    #[tokio::test]
    async fn build_mime_rejects_subject_header_injection() {
        let email = email().subject("Hello\r\nBcc: attacker@example.com");
        let err = build_mime_message(&email).await.unwrap_err();

        assert!(matches!(err, MailError::BuildError(message) if message.contains("Subject")));
    }

    #[tokio::test]
    async fn build_mime_rejects_custom_header_injection() {
        let email = email().header("X-Test", "ok\r\nBcc: attacker@example.com");
        let err = build_mime_message(&email).await.unwrap_err();

        assert!(matches!(err, MailError::BuildError(message) if message.contains("X-Test")));
    }

    #[tokio::test]
    async fn build_mime_encodes_non_ascii_subject_and_display_name() {
        let email = email().from(("José", "sender@example.com")).subject("Olá");

        let raw = String::from_utf8(build_mime_message(&email).await.unwrap()).unwrap();

        assert!(raw.contains("From: =?UTF-8?B?Sm9zw6k=?= <sender@example.com>\r\n"));
        assert!(raw.contains("Subject: =?UTF-8?B?T2zDoQ==?=\r\n"));
    }

    #[tokio::test]
    async fn build_mime_rejects_attachment_filename_injection() {
        let attachment =
            Attachment::from_bytes("report.pdf\r\nContent-Type: text/html", b"hello".to_vec());
        let email = email().attachment(attachment);

        let err = build_mime_message(&email).await.unwrap_err();

        assert!(
            matches!(err, MailError::BuildError(message) if message.contains("attachment filename"))
        );
    }

    #[tokio::test]
    async fn build_mime_wraps_attachment_base64_lines() {
        let attachment = Attachment::from_bytes("report.txt", vec![b'a'; 80]);
        let email = email().attachment(attachment);

        let raw = String::from_utf8(build_mime_message(&email).await.unwrap()).unwrap();

        for line in raw.lines() {
            if line
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '+' || ch == '/' || ch == '=')
                && line.len() > 76
            {
                panic!("base64 line exceeded 76 characters: {line}");
            }
        }
    }

    #[tokio::test]
    async fn build_body_includes_explicit_destinations_for_bcc() {
        let mailer = AmazonSesMailer::new("us-east-1", "key", "secret");
        let email = email().cc("cc@example.com").bcc("blind@example.com");

        let body = mailer.build_body(&email).await.unwrap();

        assert!(body.contains("Destinations.member.1=recipient%40example.com"));
        assert!(body.contains("Destinations.member.2=cc%40example.com"));
        assert!(body.contains("Destinations.member.3=blind%40example.com"));
    }

    #[tokio::test]
    async fn build_mime_keeps_text_only_inline_attachments() {
        let attachment = Attachment::from_bytes("logo.png", b"image".to_vec())
            .inline()
            .content_id("logo");
        let email = email().attachment(attachment);

        let raw = String::from_utf8(build_mime_message(&email).await.unwrap()).unwrap();

        assert!(raw.contains("Content-Disposition: inline; filename=\"logo.png\"\r\n"));
        assert!(raw.contains("Content-ID: <logo>\r\n"));
        assert!(!raw.contains("Content-Disposition: attachment; filename=\"logo.png\"\r\n"));
    }

    #[tokio::test]
    async fn build_mime_keeps_empty_body_inline_attachments() {
        let attachment = Attachment::from_bytes("logo.png", b"image".to_vec())
            .inline()
            .content_id("logo");
        let email = Email::new()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Hello")
            .attachment(attachment);

        let raw = String::from_utf8(build_mime_message(&email).await.unwrap()).unwrap();

        assert!(raw.contains("Content-Disposition: inline; filename=\"logo.png\"\r\n"));
        assert!(raw.contains("Content-ID: <logo>\r\n"));
        assert!(!raw.contains("Content-Disposition: attachment; filename=\"logo.png\"\r\n"));
    }
}
