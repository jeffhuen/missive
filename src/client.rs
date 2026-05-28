//! Instance-owned email delivery client.

use std::time::Instant;

use crate::address::{Address, ToAddress};
use crate::email::{Email, PreparedEmail};
use crate::error::MailError;
use crate::mailer::{DeliveryResult, Mailer};

/// Instance-owned email client.
///
/// `EmailClient` is the preferred API for applications that want explicit
/// configuration and dependency injection instead of process-global mailer
/// state.
#[derive(Clone)]
pub struct EmailClient<M> {
    mailer: M,
    default_from: Option<Address>,
}

impl<M> EmailClient<M> {
    /// Create a client from a concrete mailer.
    pub fn new(mailer: M) -> Self {
        Self {
            mailer,
            default_from: None,
        }
    }

    /// Configure the sender used when an email omits `from`.
    pub fn with_default_from(mut self, addr: impl ToAddress) -> Self {
        self.default_from = Some(addr.to_address());
        self
    }

    pub(crate) fn with_optional_default_from(mut self, addr: Option<Address>) -> Self {
        self.default_from = addr;
        self
    }

    /// Borrow the underlying mailer.
    pub fn mailer(&self) -> &M {
        &self.mailer
    }

    fn prepare_email(&self, email: Email) -> Result<PreparedEmail, MailError>
    where
        M: Mailer,
    {
        self.mailer.prepare_email(email, self.default_from.clone())
    }
}

impl<M: Mailer> EmailClient<M> {
    /// Deliver a single email through this client.
    pub async fn deliver(&self, email: Email) -> Result<DeliveryResult, MailError> {
        let email = self.prepare_email(email)?;
        deliver_prepared(&self.mailer, &email).await
    }

    /// Deliver multiple emails through this client.
    pub async fn deliver_many<I>(&self, emails: I) -> Result<Vec<DeliveryResult>, MailError>
    where
        I: IntoIterator<Item = Email>,
    {
        let emails = emails
            .into_iter()
            .map(|email| self.prepare_email(email))
            .collect::<Result<Vec<_>, _>>()?;

        self.mailer.validate_batch(&emails)?;

        let provider = self.mailer.provider_name();
        let count = emails.len();
        let span = tracing::info_span!("missive.deliver_many", provider = provider, count = count,);
        let _guard = span.enter();

        let start = Instant::now();
        let result = self.mailer.deliver_many_prepared(&emails).await;
        let duration = start.elapsed();
        let status = if result.is_ok() { "success" } else { "error" };

        #[cfg(feature = "metrics")]
        {
            metrics::counter!("missive_emails_total", "provider" => provider, "status" => status)
                .increment(count as u64);
            metrics::counter!("missive_batch_total", "provider" => provider, "status" => status)
                .increment(1);
            metrics::histogram!("missive_delivery_duration_seconds", "provider" => provider, "batch" => "true")
                .record(duration.as_secs_f64());
            metrics::histogram!("missive_batch_size", "provider" => provider).record(count as f64);
        }

        match &result {
            Ok(_) => tracing::info!(
                provider = provider,
                status = status,
                count = count,
                duration_ms = duration.as_millis() as u64,
                "Emails delivered",
            ),
            Err(e) => tracing::error!(
                provider = provider,
                status = status,
                count = count,
                duration_ms = duration.as_millis() as u64,
                error_kind = e.kind(),
                "Email batch delivery failed",
            ),
        }

        result
    }
}

async fn deliver_prepared<M: Mailer>(
    mailer: &M,
    email: &PreparedEmail,
) -> Result<DeliveryResult, MailError> {
    let provider = mailer.provider_name();
    let recipient_count = email.all_recipients().len();
    let attachment_count = email.attachments.len();

    let span = tracing::info_span!(
        "missive.deliver",
        provider = provider,
        recipient_count = recipient_count,
        attachment_count = attachment_count,
        status = tracing::field::Empty,
        duration_ms = tracing::field::Empty,
    );
    let _guard = span.enter();

    tracing::debug!("Delivering email");

    let start = Instant::now();
    let result = mailer.deliver_prepared(email).await;
    let duration = start.elapsed();
    let duration_ms = duration.as_millis() as u64;
    let status = if result.is_ok() { "success" } else { "error" };
    span.record("status", status);
    span.record("duration_ms", duration_ms);

    #[cfg(feature = "metrics")]
    {
        metrics::counter!("missive_emails_total", "provider" => provider, "status" => status)
            .increment(1);
        metrics::histogram!("missive_delivery_duration_seconds", "provider" => provider)
            .record(duration.as_secs_f64());
    }

    match &result {
        Ok(r) => {
            tracing::info!(
                provider = provider,
                status = status,
                recipient_count = recipient_count,
                attachment_count = attachment_count,
                duration_ms = duration_ms,
                "Email delivered",
            );
            tracing::debug!(message_id = %r.message_id, "Provider message id");
        }
        Err(e) => {
            tracing::error!(
                provider = provider,
                status = status,
                recipient_count = recipient_count,
                attachment_count = attachment_count,
                duration_ms = duration_ms,
                error_kind = e.kind(),
                "Email delivery failed",
            );
            tracing::debug!(error = %e, "Email delivery error details");
        }
    }

    result
}
