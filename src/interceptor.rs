//! Email interceptors for modifying or blocking emails before delivery.
//!
//! Interceptors sit between your code and the mailer, transforming every email
//! that passes through. Use them to add headers, redirect recipients, or block
//! emails based on custom logic.
//!
//! # Example
//!
//! ```rust,ignore
//! use missive::providers::LocalMailer;
//! use missive::InterceptorExt;
//!
//! let mailer = LocalMailer::new()
//!     .with_interceptor(|email| {
//!         Ok(email.header("X-Custom", "value"))
//!     });
//! ```

use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::address::Address;
use crate::email::{Email, PreparedEmail};
use crate::error::MailError;
use crate::mailer::{DeliveryResult, Mailer};

static NEXT_INTERCEPTOR_ID: AtomicUsize = AtomicUsize::new(1);

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
#[doc(hidden)]
pub trait InterceptorThreadSafety: Send + Sync {}

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
impl<T: Send + Sync> InterceptorThreadSafety for T {}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
#[doc(hidden)]
pub trait InterceptorThreadSafety {}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl<T> InterceptorThreadSafety for T {}

/// A trait for intercepting and transforming emails before delivery.
///
/// Interceptors can modify the email or block it entirely by returning an error.
///
/// # Implementing Interceptor
///
/// For simple cases, use a closure:
///
/// ```rust,ignore
/// mailer.with_interceptor(|email| Ok(email.header("X-Foo", "bar")))
/// ```
///
/// For complex logic, implement the trait on a struct:
///
/// ```rust,ignore
/// struct TenantBranding { tenant_id: String }
///
/// impl Interceptor for TenantBranding {
///     fn intercept(&self, email: Email) -> Result<Email, MailError> {
///         Ok(email.header("X-Tenant-ID", &self.tenant_id))
///     }
/// }
/// ```
pub trait Interceptor: InterceptorThreadSafety {
    /// Transform an email before delivery.
    ///
    /// Return `Ok(email)` to continue with the (possibly modified) email.
    /// Return `Err(...)` to block the email from being sent.
    fn intercept(&self, email: Email) -> Result<Email, MailError>;
}

/// Blanket implementation for closures.
impl<F> Interceptor for F
where
    F: Fn(Email) -> Result<Email, MailError> + InterceptorThreadSafety,
{
    fn intercept(&self, email: Email) -> Result<Email, MailError> {
        (self)(email)
    }
}

/// A mailer wrapper that applies an interceptor before delivery.
///
/// Created by [`InterceptorExt::with_interceptor`].
#[derive(Debug)]
pub struct WithInterceptor<M, I> {
    inner: M,
    interceptor: I,
    marker_id: usize,
}

impl<M: Clone, I: Clone> Clone for WithInterceptor<M, I> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            interceptor: self.interceptor.clone(),
            marker_id: self.marker_id,
        }
    }
}

impl<M, I> WithInterceptor<M, I> {
    /// Create a new interceptor wrapper.
    pub(crate) fn new(inner: M, interceptor: I) -> Self {
        Self {
            inner,
            interceptor,
            marker_id: NEXT_INTERCEPTOR_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    fn marker_id(&self) -> usize {
        self.marker_id
    }
}

#[cfg_attr(
    all(target_family = "wasm", target_os = "unknown"),
    async_trait(?Send)
)]
#[cfg_attr(not(all(target_family = "wasm", target_os = "unknown")), async_trait)]
impl<M, I> Mailer for WithInterceptor<M, I>
where
    M: Mailer,
    I: Interceptor,
{
    fn prepare_email(
        &self,
        mut email: Email,
        default_from: Option<Address>,
    ) -> Result<PreparedEmail, MailError> {
        let marker_id = self.marker_id();
        if !email.interceptor_applied(marker_id) {
            email = self.interceptor.intercept(email)?;
            email.mark_interceptor_applied(marker_id);
        }

        self.inner.prepare_email(email, default_from)
    }

    async fn deliver(&self, email: &Email) -> Result<DeliveryResult, MailError> {
        let email = self.interceptor.intercept(email.clone())?;
        self.inner.deliver(&email).await
    }

    async fn deliver_prepared(&self, email: &PreparedEmail) -> Result<DeliveryResult, MailError> {
        if email.interceptor_applied(self.marker_id()) {
            return self.inner.deliver_prepared(email).await;
        }

        let marker_id = self.marker_id();
        let mut email = self.interceptor.intercept(email.as_email().clone())?;
        email.mark_interceptor_applied(marker_id);
        let email = self.inner.prepare_email(email, None)?;
        self.inner.deliver_prepared(&email).await
    }

    async fn deliver_many(&self, emails: &[Email]) -> Result<Vec<DeliveryResult>, MailError> {
        let intercepted: Result<Vec<Email>, MailError> = emails
            .iter()
            .map(|e| self.interceptor.intercept(e.clone()))
            .collect();
        self.inner.deliver_many(&intercepted?).await
    }

    async fn deliver_many_prepared(
        &self,
        emails: &[PreparedEmail],
    ) -> Result<Vec<DeliveryResult>, MailError> {
        let marker_id = self.marker_id();
        if emails
            .iter()
            .all(|email| email.interceptor_applied(marker_id))
        {
            return self.inner.deliver_many_prepared(emails).await;
        }

        let emails = emails
            .iter()
            .map(|email| {
                if email.interceptor_applied(marker_id) {
                    return Ok(email.clone());
                }

                let mut email = self.interceptor.intercept(email.as_email().clone())?;
                email.mark_interceptor_applied(marker_id);
                self.inner.prepare_email(email, None)
            })
            .collect::<Result<Vec<_>, MailError>>()?;

        self.inner.deliver_many_prepared(&emails).await
    }

    fn validate_batch(&self, emails: &[PreparedEmail]) -> Result<(), MailError> {
        self.inner.validate_batch(emails)
    }

    fn provider_name(&self) -> &'static str {
        self.inner.provider_name()
    }

    fn validate_config(&self) -> Result<(), MailError> {
        self.inner.validate_config()
    }
}

/// Extension trait for adding interceptors to any mailer.
pub trait InterceptorExt: Mailer + Sized {
    /// Wrap this mailer with an interceptor.
    ///
    /// The interceptor will be called for every email before it is sent.
    /// Interceptors can modify the email or block it by returning an error.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use missive::providers::LocalMailer;
    /// use missive::interceptor::InterceptorExt;
    ///
    /// let mailer = LocalMailer::new()
    ///     .with_interceptor(|email| {
    ///         Ok(email.header("X-Debug", "true"))
    ///     });
    /// ```
    ///
    /// # Chaining
    ///
    /// Multiple interceptors can be chained:
    ///
    /// ```rust,ignore
    /// let mailer = LocalMailer::new()
    ///     .with_interceptor(add_tracking)
    ///     .with_interceptor(validate_recipients)
    ///     .with_interceptor(add_branding);
    /// ```
    fn with_interceptor<I>(self, interceptor: I) -> WithInterceptor<Self, I>
    where
        I: Interceptor,
    {
        WithInterceptor::new(self, interceptor)
    }
}

// Blanket implementation for all Mailers
impl<M: Mailer + Sized> InterceptorExt for M {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::EmailClient;
    use std::sync::{Arc, Mutex};

    struct AddHeader {
        name: &'static str,
        value: &'static str,
    }

    impl Interceptor for AddHeader {
        fn intercept(&self, email: Email) -> Result<Email, MailError> {
            Ok(email.header(self.name, self.value))
        }
    }

    #[test]
    fn test_closure_interceptor_compiles() {
        fn assert_interceptor<I: Interceptor>(_: I) {}

        let closure = |email: Email| -> Result<Email, MailError> { Ok(email) };
        assert_interceptor(closure);
    }

    #[test]
    fn test_struct_interceptor_compiles() {
        fn assert_interceptor<I: Interceptor>(_: I) {}

        let interceptor = AddHeader {
            name: "X-Test",
            value: "test",
        };
        assert_interceptor(interceptor);
    }

    #[derive(Clone, Default)]
    struct RecordingMailer {
        subjects: Arc<Mutex<Vec<String>>>,
    }

    impl RecordingMailer {
        fn subjects(&self) -> Vec<String> {
            self.subjects
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone()
        }
    }

    #[cfg_attr(
        all(target_family = "wasm", target_os = "unknown"),
        async_trait(?Send)
    )]
    #[cfg_attr(not(all(target_family = "wasm", target_os = "unknown")), async_trait)]
    impl Mailer for RecordingMailer {
        async fn deliver_prepared(
            &self,
            email: &PreparedEmail,
        ) -> Result<DeliveryResult, MailError> {
            self.subjects
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(email.as_email().subject_line().to_string());
            Ok(DeliveryResult::new("recorded"))
        }
    }

    #[tokio::test]
    async fn email_client_applies_interceptor_once() {
        let recorder = RecordingMailer::default();
        let mailer = recorder.clone().with_interceptor(|email: Email| {
            let subject = format!("{}!", email.subject_line());
            Ok(email.subject(subject))
        });
        let email = Email::new()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Hi");

        EmailClient::new(mailer).deliver(email).await.unwrap();

        assert_eq!(recorder.subjects(), vec!["Hi!"]);
    }

    #[tokio::test]
    async fn email_client_deliver_many_applies_interceptor_once_per_email() {
        let recorder = RecordingMailer::default();
        let mailer = recorder.clone().with_interceptor(|email: Email| {
            let subject = format!("{}!", email.subject_line());
            Ok(email.subject(subject))
        });
        let emails = [
            Email::new()
                .from("sender@example.com")
                .to("one@example.com")
                .subject("One"),
            Email::new()
                .from("sender@example.com")
                .to("two@example.com")
                .subject("Two"),
        ];

        EmailClient::new(mailer).deliver_many(emails).await.unwrap();

        assert_eq!(recorder.subjects(), vec!["One!", "Two!"]);
    }

    #[tokio::test]
    async fn direct_deliver_prepared_applies_interceptor_once() {
        let recorder = RecordingMailer::default();
        let mailer = recorder.clone().with_interceptor(|email: Email| {
            let subject = format!("{}!", email.subject_line());
            Ok(email.subject(subject))
        });
        let prepared = PreparedEmail::new(
            Email::new()
                .from("sender@example.com")
                .to("recipient@example.com")
                .subject("Hi"),
        )
        .unwrap();

        mailer.deliver_prepared(&prepared).await.unwrap();

        assert_eq!(recorder.subjects(), vec!["Hi!"]);
    }

    #[tokio::test]
    async fn direct_deliver_many_prepared_applies_interceptor_once_per_email() {
        let recorder = RecordingMailer::default();
        let mailer = recorder.clone().with_interceptor(|email: Email| {
            let subject = format!("{}!", email.subject_line());
            Ok(email.subject(subject))
        });
        let emails = [
            PreparedEmail::new(
                Email::new()
                    .from("sender@example.com")
                    .to("one@example.com")
                    .subject("One"),
            )
            .unwrap(),
            PreparedEmail::new(
                Email::new()
                    .from("sender@example.com")
                    .to("two@example.com")
                    .subject("Two"),
            )
            .unwrap(),
        ];

        mailer.deliver_many_prepared(&emails).await.unwrap();

        assert_eq!(recorder.subjects(), vec!["One!", "Two!"]);
    }

    #[tokio::test]
    async fn prepared_email_from_different_interceptor_stack_is_intercepted() {
        let recorder = RecordingMailer::default();
        let prepare_mailer = RecordingMailer::default().with_interceptor(|email: Email| {
            let subject = format!("{}A", email.subject_line());
            Ok(email.subject(subject))
        });
        let deliver_mailer = recorder.clone().with_interceptor(|email: Email| {
            let subject = format!("{}B", email.subject_line());
            Ok(email.subject(subject))
        });
        let email = Email::new()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Hi");
        let prepared = prepare_mailer.prepare_email(email, None).unwrap();

        deliver_mailer.deliver_prepared(&prepared).await.unwrap();

        assert_eq!(recorder.subjects(), vec!["HiAB"]);
    }
}
