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

use crate::email::Email;
use crate::error::MailError;
use crate::mailer::{DeliveryResult, Mailer};

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
pub trait Interceptor: Send + Sync {
    /// Transform an email before delivery.
    ///
    /// Return `Ok(email)` to continue with the (possibly modified) email.
    /// Return `Err(...)` to block the email from being sent.
    fn intercept(&self, email: Email) -> Result<Email, MailError>;
}

/// Blanket implementation for closures.
impl<F> Interceptor for F
where
    F: Fn(Email) -> Result<Email, MailError> + Send + Sync,
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
}

impl<M: Clone, I: Clone> Clone for WithInterceptor<M, I> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            interceptor: self.interceptor.clone(),
        }
    }
}

impl<M, I> WithInterceptor<M, I> {
    /// Create a new interceptor wrapper.
    pub(crate) fn new(inner: M, interceptor: I) -> Self {
        Self { inner, interceptor }
    }
}

#[async_trait]
impl<M, I> Mailer for WithInterceptor<M, I>
where
    M: Mailer,
    I: Interceptor,
{
    async fn deliver(&self, email: &Email) -> Result<DeliveryResult, MailError> {
        let email = self.interceptor.intercept(email.clone())?;
        self.inner.deliver(&email).await
    }

    async fn deliver_many(&self, emails: &[Email]) -> Result<Vec<DeliveryResult>, MailError> {
        let intercepted: Result<Vec<Email>, MailError> = emails
            .iter()
            .map(|e| self.interceptor.intercept(e.clone()))
            .collect();
        self.inner.deliver_many(&intercepted?).await
    }

    fn validate_batch(&self, emails: &[Email]) -> Result<(), MailError> {
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
}
