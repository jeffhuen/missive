#![cfg(feature = "full")]

use missive::{EmailClient, MailerConfig, ResendConfig};
use std::collections::HashMap;

fn env(pairs: &[(&str, &str)]) -> impl FnMut(&str) -> Option<String> {
    let vars: HashMap<String, String> = pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect();

    move |key| vars.get(key).cloned()
}

#[test]
fn resend_config_validates_required_api_key() {
    let error = ResendConfig::new("").unwrap_err();

    assert!(error.to_string().contains("RESEND_API_KEY"));
}

#[test]
fn mailer_config_from_env_prefers_explicit_provider() {
    let config = MailerConfig::from_env_with(env(&[
        ("EMAIL_PROVIDER", "sendgrid"),
        ("RESEND_API_KEY", "re_123"),
        ("SENDGRID_API_KEY", "sg_123"),
    ]))
    .unwrap();

    assert_eq!(config.provider_name(), "sendgrid");
}

#[test]
fn mailer_config_from_env_documents_auto_detection_order() {
    let config = MailerConfig::from_env_with(env(&[
        ("RESEND_API_KEY", "re_123"),
        ("SENDGRID_API_KEY", "sg_123"),
    ]))
    .unwrap();

    assert_eq!(config.provider_name(), "resend");
}

#[test]
fn mailer_config_from_env_accepts_explicit_mailjet_provider() {
    let config = MailerConfig::from_env_with(env(&[
        ("EMAIL_PROVIDER", "mailjet"),
        ("MAILJET_API_KEY", "mj_api"),
        ("MAILJET_SECRET_KEY", "mj_secret"),
    ]))
    .unwrap();

    assert_eq!(config.provider_name(), "mailjet");
}

#[test]
fn mailer_config_from_env_auto_detects_mailjet_credentials() {
    let config = MailerConfig::from_env_with(env(&[
        ("MAILJET_API_KEY", "mj_api"),
        ("MAILJET_SECRET_KEY", "mj_secret"),
    ]))
    .unwrap();

    assert_eq!(config.provider_name(), "mailjet");
}

#[test]
fn mailer_config_from_env_requires_mailjet_secret_key() {
    let error = MailerConfig::from_env_with(env(&[
        ("EMAIL_PROVIDER", "mailjet"),
        ("MAILJET_API_KEY", "mj_api"),
    ]))
    .unwrap_err();

    assert!(error.to_string().contains("MAILJET_SECRET_KEY"));
}

#[test]
fn email_client_from_env_with_is_testable_without_process_env_mutation() {
    let client = EmailClient::from_env_with(env(&[
        ("EMAIL_PROVIDER", "local"),
        ("EMAIL_FROM", "noreply@example.com"),
    ]))
    .unwrap();

    assert_eq!(client.mailer().provider_name(), "local");
}

#[test]
fn local_env_mailer_does_not_publish_global_preview_storage() {
    let _mailer = MailerConfig::Local.into_mailer().unwrap();

    #[allow(deprecated)]
    let storage = missive::local_storage();
    assert!(storage.is_none());
}
