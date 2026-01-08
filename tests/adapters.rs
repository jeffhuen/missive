//! Adapter integration tests.
//!
//! Tests for email provider adapters, ported from Swoosh's adapter tests.

#[path = "adapters/amazon_ses_test.rs"]
mod amazon_ses_test;
#[path = "adapters/brevo_test.rs"]
mod brevo_test;
#[path = "adapters/local_test.rs"]
mod local_test;
#[path = "adapters/logger_test.rs"]
mod logger_test;
#[path = "adapters/mailgun_test.rs"]
mod mailgun_test;
#[path = "adapters/mailjet_test.rs"]
mod mailjet_test;
#[path = "adapters/mailtrap_test.rs"]
mod mailtrap_test;
#[path = "adapters/postmark_test.rs"]
mod postmark_test;
#[path = "adapters/resend_test.rs"]
mod resend_test;
#[path = "adapters/sendgrid_test.rs"]
mod sendgrid_test;
#[path = "adapters/unsent_test.rs"]
mod unsent_test;
