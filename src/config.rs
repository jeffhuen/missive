//! Explicit environment configuration helpers.
#![cfg_attr(
    not(any(
        feature = "smtp",
        feature = "resend",
        feature = "unsent",
        feature = "postmark",
        feature = "sendgrid",
        feature = "brevo",
        feature = "mailgun",
        feature = "amazon_ses",
        feature = "mailtrap",
        feature = "socketlabs",
        feature = "gmail",
        feature = "protonbridge",
        feature = "jmap",
        feature = "local"
    )),
    allow(dead_code, unused_variables)
)]

use std::env;
use std::sync::Arc;

use crate::address::Address;
use crate::client::EmailClient;
use crate::error::MailError;
use crate::mailer::Mailer;
use crate::providers;

/// Typed configuration for building a mailer from environment-like input.
#[derive(Debug, Clone)]
pub enum MailerConfig {
    #[cfg(feature = "smtp")]
    Smtp(SmtpConfig),
    #[cfg(feature = "resend")]
    Resend(ResendConfig),
    #[cfg(feature = "unsent")]
    Unsent(ApiKeyConfig),
    #[cfg(feature = "postmark")]
    Postmark(ApiKeyConfig),
    #[cfg(feature = "sendgrid")]
    SendGrid(ApiKeyConfig),
    #[cfg(feature = "brevo")]
    Brevo(ApiKeyConfig),
    #[cfg(feature = "mailgun")]
    Mailgun(MailgunConfig),
    #[cfg(feature = "amazon_ses")]
    AmazonSes(AmazonSesConfig),
    #[cfg(feature = "mailtrap")]
    Mailtrap(MailtrapConfig),
    #[cfg(feature = "socketlabs")]
    SocketLabs(SocketLabsConfig),
    #[cfg(feature = "gmail")]
    Gmail(ApiKeyConfig),
    #[cfg(feature = "protonbridge")]
    ProtonBridge(ProtonBridgeConfig),
    #[cfg(feature = "jmap")]
    Jmap(JmapConfig),
    #[cfg(feature = "local")]
    Local,
    Logger {
        full: bool,
    },
}

impl MailerConfig {
    /// Build mailer configuration from process environment variables.
    ///
    /// If `EMAIL_PROVIDER` is not set, auto-detection checks providers in this
    /// order when their feature is enabled: Resend, SendGrid, Postmark, Unsent,
    /// Brevo, Mailgun, Amazon SES, Mailtrap, SocketLabs, Gmail, Proton Bridge,
    /// JMAP, SMTP, Local.
    pub fn from_env() -> Result<Self, MailError> {
        Self::from_env_with(|key| env::var(key).ok())
    }

    /// Build mailer configuration from a testable environment source.
    pub fn from_env_with<F>(mut get: F) -> Result<Self, MailError>
    where
        F: FnMut(&str) -> Option<String>,
    {
        Self::from_env_with_ref(&mut get)
    }

    pub(crate) fn from_env_with_ref<F>(get: &mut F) -> Result<Self, MailError>
    where
        F: FnMut(&str) -> Option<String> + ?Sized,
    {
        let provider = match optional_var(get, "EMAIL_PROVIDER") {
            Some(provider) => provider.to_lowercase(),
            None => detect_provider_with(get)
                .ok_or_else(|| {
                    MailError::Configuration(
                        "EMAIL_PROVIDER not set and could not auto-detect. Set EMAIL_PROVIDER or ensure an API key is configured."
                            .into(),
                    )
                })?
                .to_string(),
        };

        Self::for_provider_from_env(&provider, get)
    }

    fn for_provider_from_env<F>(provider: &str, get: &mut F) -> Result<Self, MailError>
    where
        F: FnMut(&str) -> Option<String> + ?Sized,
    {
        match provider {
            #[cfg(feature = "smtp")]
            "smtp" => Ok(Self::Smtp(SmtpConfig::from_env_with_ref(get)?)),
            #[cfg(not(feature = "smtp"))]
            "smtp" => Err(feature_disabled("smtp")),

            #[cfg(feature = "resend")]
            "resend" => Ok(Self::Resend(ResendConfig::from_env_with_ref(get)?)),
            #[cfg(not(feature = "resend"))]
            "resend" => Err(feature_disabled("resend")),

            #[cfg(feature = "unsent")]
            "unsent" => Ok(Self::Unsent(ApiKeyConfig::from_env_with_ref(
                get,
                "UNSENT_API_KEY",
            )?)),
            #[cfg(not(feature = "unsent"))]
            "unsent" => Err(feature_disabled("unsent")),

            #[cfg(feature = "postmark")]
            "postmark" => Ok(Self::Postmark(ApiKeyConfig::from_env_with_ref(
                get,
                "POSTMARK_API_KEY",
            )?)),
            #[cfg(not(feature = "postmark"))]
            "postmark" => Err(feature_disabled("postmark")),

            #[cfg(feature = "sendgrid")]
            "sendgrid" => Ok(Self::SendGrid(ApiKeyConfig::from_env_with_ref(
                get,
                "SENDGRID_API_KEY",
            )?)),
            #[cfg(not(feature = "sendgrid"))]
            "sendgrid" => Err(feature_disabled("sendgrid")),

            #[cfg(feature = "brevo")]
            "brevo" => Ok(Self::Brevo(ApiKeyConfig::from_env_with_ref(
                get,
                "BREVO_API_KEY",
            )?)),
            #[cfg(not(feature = "brevo"))]
            "brevo" => Err(feature_disabled("brevo")),

            #[cfg(feature = "mailgun")]
            "mailgun" => Ok(Self::Mailgun(MailgunConfig::from_env_with_ref(get)?)),
            #[cfg(not(feature = "mailgun"))]
            "mailgun" => Err(feature_disabled("mailgun")),

            #[cfg(feature = "amazon_ses")]
            "amazon_ses" => Ok(Self::AmazonSes(AmazonSesConfig::from_env_with_ref(get)?)),
            #[cfg(not(feature = "amazon_ses"))]
            "amazon_ses" => Err(feature_disabled("amazon_ses")),

            #[cfg(feature = "mailtrap")]
            "mailtrap" => Ok(Self::Mailtrap(MailtrapConfig::from_env_with_ref(get)?)),
            #[cfg(not(feature = "mailtrap"))]
            "mailtrap" => Err(feature_disabled("mailtrap")),

            #[cfg(feature = "socketlabs")]
            "socketlabs" => Ok(Self::SocketLabs(SocketLabsConfig::from_env_with_ref(get)?)),
            #[cfg(not(feature = "socketlabs"))]
            "socketlabs" => Err(feature_disabled("socketlabs")),

            #[cfg(feature = "gmail")]
            "gmail" => Ok(Self::Gmail(ApiKeyConfig::from_env_with_ref(
                get,
                "GMAIL_ACCESS_TOKEN",
            )?)),
            #[cfg(not(feature = "gmail"))]
            "gmail" => Err(feature_disabled("gmail")),

            #[cfg(feature = "protonbridge")]
            "protonbridge" => Ok(Self::ProtonBridge(ProtonBridgeConfig::from_env_with_ref(
                get,
            )?)),
            #[cfg(not(feature = "protonbridge"))]
            "protonbridge" => Err(feature_disabled("protonbridge")),

            #[cfg(feature = "jmap")]
            "jmap" => Ok(Self::Jmap(JmapConfig::from_env_with_ref(get)?)),
            #[cfg(not(feature = "jmap"))]
            "jmap" => Err(feature_disabled("jmap")),

            #[cfg(feature = "local")]
            "local" => Ok(Self::Local),
            #[cfg(not(feature = "local"))]
            "local" => Err(feature_disabled("local")),

            "logger" => Ok(Self::Logger { full: false }),
            "logger_full" => Ok(Self::Logger { full: true }),

            _ => Err(MailError::Configuration(format!(
                "Unknown EMAIL_PROVIDER: {provider}. Valid providers are: smtp, resend, unsent, postmark, sendgrid, brevo, mailgun, amazon_ses, mailtrap, socketlabs, gmail, protonbridge, jmap, local, logger, logger_full"
            ))),
        }
    }

    /// Name of the provider selected by this config.
    pub fn provider_name(&self) -> &'static str {
        match self {
            #[cfg(feature = "smtp")]
            Self::Smtp(_) => "smtp",
            #[cfg(feature = "resend")]
            Self::Resend(_) => "resend",
            #[cfg(feature = "unsent")]
            Self::Unsent(_) => "unsent",
            #[cfg(feature = "postmark")]
            Self::Postmark(_) => "postmark",
            #[cfg(feature = "sendgrid")]
            Self::SendGrid(_) => "sendgrid",
            #[cfg(feature = "brevo")]
            Self::Brevo(_) => "brevo",
            #[cfg(feature = "mailgun")]
            Self::Mailgun(_) => "mailgun",
            #[cfg(feature = "amazon_ses")]
            Self::AmazonSes(_) => "amazon_ses",
            #[cfg(feature = "mailtrap")]
            Self::Mailtrap(_) => "mailtrap",
            #[cfg(feature = "socketlabs")]
            Self::SocketLabs(_) => "socketlabs",
            #[cfg(feature = "gmail")]
            Self::Gmail(_) => "gmail",
            #[cfg(feature = "protonbridge")]
            Self::ProtonBridge(_) => "protonbridge",
            #[cfg(feature = "jmap")]
            Self::Jmap(_) => "jmap",
            #[cfg(feature = "local")]
            Self::Local => "local",
            Self::Logger { full: false } => "logger",
            Self::Logger { full: true } => "logger_full",
        }
    }

    /// Build the configured mailer.
    pub fn into_mailer(self) -> Result<Arc<dyn Mailer>, MailError> {
        match self {
            #[cfg(feature = "smtp")]
            Self::Smtp(config) => Ok(Arc::new(config.into_mailer())),
            #[cfg(feature = "resend")]
            Self::Resend(config) => Ok(Arc::new(providers::ResendMailer::new(config.api_key))),
            #[cfg(feature = "unsent")]
            Self::Unsent(config) => Ok(Arc::new(providers::UnsentMailer::new(config.api_key))),
            #[cfg(feature = "postmark")]
            Self::Postmark(config) => Ok(Arc::new(providers::PostmarkMailer::new(config.api_key))),
            #[cfg(feature = "sendgrid")]
            Self::SendGrid(config) => Ok(Arc::new(providers::SendGridMailer::new(config.api_key))),
            #[cfg(feature = "brevo")]
            Self::Brevo(config) => Ok(Arc::new(providers::BrevoMailer::new(config.api_key))),
            #[cfg(feature = "mailgun")]
            Self::Mailgun(config) => Ok(Arc::new(config.into_mailer())),
            #[cfg(feature = "amazon_ses")]
            Self::AmazonSes(config) => Ok(Arc::new(providers::AmazonSesMailer::new(
                config.region,
                config.access_key,
                config.secret,
            ))),
            #[cfg(feature = "mailtrap")]
            Self::Mailtrap(config) => Ok(Arc::new(config.into_mailer())),
            #[cfg(feature = "socketlabs")]
            Self::SocketLabs(config) => Ok(Arc::new(providers::SocketLabsMailer::new(
                config.server_id,
                config.api_key,
            ))),
            #[cfg(feature = "gmail")]
            Self::Gmail(config) => Ok(Arc::new(providers::GmailMailer::new(config.api_key))),
            #[cfg(feature = "protonbridge")]
            Self::ProtonBridge(config) => Ok(Arc::new(config.into_mailer())),
            #[cfg(feature = "jmap")]
            Self::Jmap(config) => Ok(Arc::new(config.into_mailer())),
            #[cfg(feature = "local")]
            Self::Local => Ok(Arc::new(providers::LocalMailer::new())),
            Self::Logger { full: false } => Ok(Arc::new(providers::LoggerMailer::new())),
            Self::Logger { full: true } => Ok(Arc::new(providers::LoggerMailer::full())),
        }
    }
}

/// Configuration for providers that require one API key.
#[derive(Debug, Clone)]
pub struct ApiKeyConfig {
    pub api_key: String,
}

impl ApiKeyConfig {
    pub fn new(api_key_name: &'static str, api_key: impl Into<String>) -> Result<Self, MailError> {
        Ok(Self {
            api_key: required_value(api_key_name, api_key.into())?,
        })
    }

    fn from_env_with_ref<F>(get: &mut F, api_key_name: &'static str) -> Result<Self, MailError>
    where
        F: FnMut(&str) -> Option<String> + ?Sized,
    {
        Self::new(api_key_name, required_var(get, api_key_name)?)
    }
}

#[cfg(feature = "resend")]
#[derive(Debug, Clone)]
pub struct ResendConfig {
    pub api_key: String,
}

#[cfg(feature = "resend")]
impl ResendConfig {
    pub fn new(api_key: impl Into<String>) -> Result<Self, MailError> {
        Ok(Self {
            api_key: required_value("RESEND_API_KEY", api_key.into())?,
        })
    }

    pub fn from_env() -> Result<Self, MailError> {
        let mut get = |key: &str| env::var(key).ok();
        Self::from_env_with_ref(&mut get)
    }

    pub fn from_env_with<F>(mut get: F) -> Result<Self, MailError>
    where
        F: FnMut(&str) -> Option<String>,
    {
        Self::from_env_with_ref(&mut get)
    }

    fn from_env_with_ref<F>(get: &mut F) -> Result<Self, MailError>
    where
        F: FnMut(&str) -> Option<String> + ?Sized,
    {
        Self::new(required_var(get, "RESEND_API_KEY")?)
    }
}

#[cfg(feature = "smtp")]
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[cfg(feature = "smtp")]
impl SmtpConfig {
    pub fn new(host: impl Into<String>, port: u16) -> Result<Self, MailError> {
        Ok(Self {
            host: required_value("SMTP_HOST", host.into())?,
            port,
            username: None,
            password: None,
        })
    }

    fn from_env_with_ref<F>(get: &mut F) -> Result<Self, MailError>
    where
        F: FnMut(&str) -> Option<String> + ?Sized,
    {
        let mut config = Self::new(
            required_var(get, "SMTP_HOST")?,
            optional_port(get, "SMTP_PORT", 587)?,
        )?;
        config.username = optional_var(get, "SMTP_USERNAME").filter(|value| !value.is_empty());
        config.password = optional_var(get, "SMTP_PASSWORD").filter(|value| !value.is_empty());
        Ok(config)
    }

    fn into_mailer(self) -> providers::SmtpMailer {
        let mut builder = providers::SmtpMailer::new(&self.host, self.port);
        if let Some(username) = self.username {
            let password = self.password.unwrap_or_default();
            builder = builder.credentials(&username, &password);
        }
        builder.build()
    }
}

#[cfg(feature = "mailgun")]
#[derive(Debug, Clone)]
pub struct MailgunConfig {
    pub api_key: String,
    pub domain: String,
    pub base_url: Option<String>,
}

#[cfg(feature = "mailgun")]
impl MailgunConfig {
    fn from_env_with_ref<F>(get: &mut F) -> Result<Self, MailError>
    where
        F: FnMut(&str) -> Option<String> + ?Sized,
    {
        Ok(Self {
            api_key: required_var(get, "MAILGUN_API_KEY")?,
            domain: required_var(get, "MAILGUN_DOMAIN")?,
            base_url: optional_var(get, "MAILGUN_BASE_URL"),
        })
    }

    fn into_mailer(self) -> providers::MailgunMailer {
        let mailer = providers::MailgunMailer::new(self.api_key, self.domain);
        match self.base_url {
            Some(base_url) => mailer.base_url(base_url),
            None => mailer,
        }
    }
}

#[cfg(feature = "amazon_ses")]
#[derive(Debug, Clone)]
pub struct AmazonSesConfig {
    pub region: String,
    pub access_key: String,
    pub secret: String,
}

#[cfg(feature = "amazon_ses")]
impl AmazonSesConfig {
    fn from_env_with_ref<F>(get: &mut F) -> Result<Self, MailError>
    where
        F: FnMut(&str) -> Option<String> + ?Sized,
    {
        Ok(Self {
            region: required_var(get, "AWS_REGION")?,
            access_key: required_var(get, "AWS_ACCESS_KEY_ID")?,
            secret: required_var(get, "AWS_SECRET_ACCESS_KEY")?,
        })
    }
}

#[cfg(feature = "mailtrap")]
#[derive(Debug, Clone)]
pub struct MailtrapConfig {
    pub api_key: String,
    pub sandbox_inbox_id: Option<String>,
}

#[cfg(feature = "mailtrap")]
impl MailtrapConfig {
    fn from_env_with_ref<F>(get: &mut F) -> Result<Self, MailError>
    where
        F: FnMut(&str) -> Option<String> + ?Sized,
    {
        Ok(Self {
            api_key: required_var(get, "MAILTRAP_API_KEY")?,
            sandbox_inbox_id: optional_var(get, "MAILTRAP_SANDBOX_INBOX_ID"),
        })
    }

    fn into_mailer(self) -> providers::MailtrapMailer {
        let mailer = providers::MailtrapMailer::new(self.api_key);
        match self.sandbox_inbox_id {
            Some(inbox_id) => mailer.sandbox_inbox_id(inbox_id),
            None => mailer,
        }
    }
}

#[cfg(feature = "socketlabs")]
#[derive(Debug, Clone)]
pub struct SocketLabsConfig {
    pub server_id: String,
    pub api_key: String,
}

#[cfg(feature = "socketlabs")]
impl SocketLabsConfig {
    fn from_env_with_ref<F>(get: &mut F) -> Result<Self, MailError>
    where
        F: FnMut(&str) -> Option<String> + ?Sized,
    {
        Ok(Self {
            server_id: required_var(get, "SOCKETLABS_SERVER_ID")?,
            api_key: required_var(get, "SOCKETLABS_API_KEY")?,
        })
    }
}

#[cfg(feature = "protonbridge")]
#[derive(Debug, Clone)]
pub struct ProtonBridgeConfig {
    pub username: String,
    pub password: String,
    pub host: Option<String>,
    pub port: Option<u16>,
}

#[cfg(feature = "protonbridge")]
impl ProtonBridgeConfig {
    fn from_env_with_ref<F>(get: &mut F) -> Result<Self, MailError>
    where
        F: FnMut(&str) -> Option<String> + ?Sized,
    {
        Ok(Self {
            username: required_var(get, "PROTONBRIDGE_USERNAME")?,
            password: required_var(get, "PROTONBRIDGE_PASSWORD")?,
            host: optional_var(get, "PROTONBRIDGE_HOST"),
            port: optional_var(get, "PROTONBRIDGE_PORT")
                .map(|port| parse_port("PROTONBRIDGE_PORT", &port))
                .transpose()?,
        })
    }

    fn into_mailer(self) -> providers::ProtonBridgeMailer {
        let mut builder = providers::ProtonBridgeMailer::new(&self.username, &self.password);
        if let Some(host) = self.host {
            builder = builder.host(&host);
        }
        if let Some(port) = self.port {
            builder = builder.port(port);
        }
        builder.build()
    }
}

#[cfg(feature = "jmap")]
#[derive(Debug, Clone)]
pub struct JmapConfig {
    pub url: String,
    pub auth: JmapAuthConfig,
}

#[cfg(feature = "jmap")]
#[derive(Debug, Clone)]
pub enum JmapAuthConfig {
    BearerToken(String),
    Basic { username: String, password: String },
}

#[cfg(feature = "jmap")]
impl JmapConfig {
    fn from_env_with_ref<F>(get: &mut F) -> Result<Self, MailError>
    where
        F: FnMut(&str) -> Option<String> + ?Sized,
    {
        let url = required_var(get, "JMAP_URL")?;
        let auth = match optional_var(get, "JMAP_BEARER_TOKEN") {
            Some(token) if !token.is_empty() => JmapAuthConfig::BearerToken(token),
            _ => JmapAuthConfig::Basic {
                username: required_var(get, "JMAP_USERNAME")?,
                password: required_var(get, "JMAP_PASSWORD")?,
            },
        };
        Ok(Self { url, auth })
    }

    fn into_mailer(self) -> providers::JmapMailer {
        let builder = providers::JmapMailer::new(&self.url);
        match self.auth {
            JmapAuthConfig::BearerToken(token) => builder.bearer_token(&token).build(),
            JmapAuthConfig::Basic { username, password } => {
                builder.credentials(&username, &password).build()
            }
        }
    }
}

impl EmailClient<Arc<dyn Mailer>> {
    /// Build an explicit client from process environment variables.
    pub fn from_env() -> Result<Self, MailError> {
        Self::from_env_with(|key| env::var(key).ok())
    }

    /// Build an explicit client from a testable environment source.
    pub fn from_env_with<F>(mut get: F) -> Result<Self, MailError>
    where
        F: FnMut(&str) -> Option<String>,
    {
        let default_from = default_from_with(&mut get);
        let mailer = MailerConfig::from_env_with_ref(&mut get)?.into_mailer()?;
        Ok(EmailClient::new(mailer).with_optional_default_from(default_from))
    }
}

pub(crate) fn default_from_with<F>(get: &mut F) -> Option<Address>
where
    F: FnMut(&str) -> Option<String> + ?Sized,
{
    let email = optional_var(get, "EMAIL_FROM")?;
    let name = optional_var(get, "EMAIL_FROM_NAME");
    Some(match name {
        Some(name) => Address::with_name(name, email),
        None => Address::new(email),
    })
}

fn detect_provider_with<F>(get: &mut F) -> Option<&'static str>
where
    F: FnMut(&str) -> Option<String> + ?Sized,
{
    #[cfg(feature = "resend")]
    if optional_var(get, "RESEND_API_KEY").is_some() {
        return Some("resend");
    }
    #[cfg(feature = "sendgrid")]
    if optional_var(get, "SENDGRID_API_KEY").is_some() {
        return Some("sendgrid");
    }
    #[cfg(feature = "postmark")]
    if optional_var(get, "POSTMARK_API_KEY").is_some() {
        return Some("postmark");
    }
    #[cfg(feature = "unsent")]
    if optional_var(get, "UNSENT_API_KEY").is_some() {
        return Some("unsent");
    }
    #[cfg(feature = "brevo")]
    if optional_var(get, "BREVO_API_KEY").is_some() {
        return Some("brevo");
    }
    #[cfg(feature = "mailgun")]
    if optional_var(get, "MAILGUN_API_KEY").is_some()
        && optional_var(get, "MAILGUN_DOMAIN").is_some()
    {
        return Some("mailgun");
    }
    #[cfg(feature = "amazon_ses")]
    if optional_var(get, "AWS_ACCESS_KEY_ID").is_some()
        && optional_var(get, "AWS_SECRET_ACCESS_KEY").is_some()
        && optional_var(get, "AWS_REGION").is_some()
    {
        return Some("amazon_ses");
    }
    #[cfg(feature = "mailtrap")]
    if optional_var(get, "MAILTRAP_API_KEY").is_some() {
        return Some("mailtrap");
    }
    #[cfg(feature = "socketlabs")]
    if optional_var(get, "SOCKETLABS_API_KEY").is_some()
        && optional_var(get, "SOCKETLABS_SERVER_ID").is_some()
    {
        return Some("socketlabs");
    }
    #[cfg(feature = "gmail")]
    if optional_var(get, "GMAIL_ACCESS_TOKEN").is_some() {
        return Some("gmail");
    }
    #[cfg(feature = "protonbridge")]
    if optional_var(get, "PROTONBRIDGE_USERNAME").is_some()
        && optional_var(get, "PROTONBRIDGE_PASSWORD").is_some()
    {
        return Some("protonbridge");
    }
    #[cfg(feature = "jmap")]
    if optional_var(get, "JMAP_URL").is_some()
        && (optional_var(get, "JMAP_BEARER_TOKEN").is_some()
            || (optional_var(get, "JMAP_USERNAME").is_some()
                && optional_var(get, "JMAP_PASSWORD").is_some()))
    {
        return Some("jmap");
    }
    #[cfg(feature = "smtp")]
    if optional_var(get, "SMTP_HOST").is_some() {
        return Some("smtp");
    }
    #[cfg(feature = "local")]
    {
        return Some("local");
    }
    #[allow(unreachable_code)]
    None
}

fn optional_var<F>(get: &mut F, name: &str) -> Option<String>
where
    F: FnMut(&str) -> Option<String> + ?Sized,
{
    get(name).filter(|value| !value.is_empty())
}

fn required_var<F>(get: &mut F, name: &'static str) -> Result<String, MailError>
where
    F: FnMut(&str) -> Option<String> + ?Sized,
{
    optional_var(get, name).ok_or_else(|| MailError::Configuration(format!("{name} not set")))
}

fn required_value(name: &'static str, value: String) -> Result<String, MailError> {
    if value.is_empty() {
        Err(MailError::Configuration(format!("{name} not set")))
    } else {
        Ok(value)
    }
}

fn optional_port<F>(get: &mut F, name: &'static str, default: u16) -> Result<u16, MailError>
where
    F: FnMut(&str) -> Option<String> + ?Sized,
{
    match optional_var(get, name) {
        Some(value) => parse_port(name, &value),
        None => Ok(default),
    }
}

fn parse_port(name: &'static str, value: &str) -> Result<u16, MailError> {
    value.parse::<u16>().map_err(|source| {
        MailError::Configuration(format!("{name} must be a valid port: {source}"))
    })
}

#[allow(dead_code)]
fn feature_disabled(provider: &'static str) -> MailError {
    MailError::Configuration(format!(
        "EMAIL_PROVIDER={provider} but '{provider}' feature is not enabled. Add `features = [\"{provider}\"]` to Cargo.toml"
    ))
}
