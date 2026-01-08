# Interceptors

Interceptors allow you to modify or block emails before they are sent to a provider.

## Overview

An interceptor sits between your code and the mailer, transforming every email that passes through:

```
Email → Interceptor → Mailer → Provider
```

## Use Cases

### Redirect emails in development

Prevent accidentally emailing real users during development:

```rust
use missive::{Address, Email, InterceptorExt};
use missive::providers::ResendMailer;

let mailer = ResendMailer::new(api_key)
    .with_interceptor(|email: Email| {
        Ok(email
            .put_to(vec![Address::new("[email protected]")])
            .put_cc(vec![])
            .put_bcc(vec![]))
    });

// All emails now go to the test address
mailer.deliver(&email).await?;
```

### Add tracking headers

Inject correlation IDs or debug info into every email:

```rust
let mailer = ResendMailer::new(api_key)
    .with_interceptor(|email: Email| {
        Ok(email.header("X-Request-ID", get_request_id()))
    });
```

### Block emails to certain domains

Prevent sending to competitors or restricted addresses:

```rust
use missive::MailError;

let mailer = ResendMailer::new(api_key)
    .with_interceptor(|email: Email| {
        for recipient in &email.to {
            if recipient.email.ends_with("@competitor.com") {
                return Err(MailError::SendError(
                    "Cannot send to competitor.com".into()
                ));
            }
        }
        Ok(email)
    });
```

### Multi-tenant branding

Automatically add tenant-specific footers:

```rust
struct TenantBranding {
    tenant_id: String,
    footer_html: String,
}

impl Interceptor for TenantBranding {
    fn intercept(&self, email: Email) -> Result<Email, MailError> {
        let html = email.html_body
            .as_ref()
            .map(|h| format!("{}\n{}", h, self.footer_html));
        
        Ok(email
            .header("X-Tenant-ID", &self.tenant_id)
            .html_body(html.unwrap_or_default()))
    }
}

let mailer = ResendMailer::new(api_key)
    .with_interceptor(TenantBranding {
        tenant_id: "acme".into(),
        footer_html: "<p>Sent by Acme Corp</p>".into(),
    });
```

## API Reference

### `Interceptor` trait

```rust
pub trait Interceptor: Send + Sync {
    /// Transform an email before delivery.
    ///
    /// Return `Ok(email)` to continue with the (possibly modified) email.
    /// Return `Err(...)` to block the email from being sent.
    fn intercept(&self, email: Email) -> Result<Email, MailError>;
}
```

### `InterceptorExt::with_interceptor`

```rust
pub trait InterceptorExt: Mailer + Sized {
    /// Wrap this mailer with an interceptor.
    fn with_interceptor<I: Interceptor>(self, interceptor: I) -> WithInterceptor<Self, I>;
}
```

### Closure support

Any closure matching `Fn(Email) -> Result<Email, MailError>` automatically implements `Interceptor`:

```rust
mailer.with_interceptor(|email| Ok(email.header("X-Foo", "bar")))
```

## Chaining Interceptors

Multiple interceptors can be chained:

```rust
let mailer = ResendMailer::new(api_key)
    .with_interceptor(AddTrackingHeaders)
    .with_interceptor(ValidateRecipients)
    .with_interceptor(AddTenantFooter);
```

All interceptors run before delivery. The email passes through each one.

## Best Practices

### Keep interceptors independent

Each interceptor should do one thing and not depend on other interceptors. If you find yourself writing an interceptor that checks for a header added by another interceptor, consolidate them into one:

```rust
// Bad: coupled interceptors
.with_interceptor(|e| Ok(e.header("X-Priority", "high")))
.with_interceptor(|e| {
    // Depends on previous interceptor's header
    if e.headers.get("X-Priority") == Some(&"high".into()) {
        Ok(e.header("X-Route", "fast"))
    } else {
        Ok(e)
    }
})

// Good: single interceptor with related logic
.with_interceptor(|e| {
    Ok(e.header("X-Priority", "high")
       .header("X-Route", "fast"))
})
```

### Don't rely on execution order

Interceptors are independent transformations. The library does not guarantee a specific execution order. If your logic requires ordering, it should be in a single interceptor:

```rust
// Bad: order-dependent logic split across interceptors
.with_interceptor(redirect_to_test)  // Must run before block check?
.with_interceptor(block_production)  // Confusing interaction

// Good: clear, self-contained logic
.with_interceptor(|email| {
    if is_development() {
        Ok(email.put_to(vec![test_address()]))
    } else if is_blocked_domain(&email) {
        Err(MailError::SendError("Blocked".into()))
    } else {
        Ok(email)
    }
})
```

### One concern per interceptor

Good interceptors are focused and reusable:

- `AddRequestId` - adds X-Request-ID header
- `BlockCompetitors` - prevents sending to competitor domains  
- `DevRedirect` - redirects all emails in development
- `TenantBranding` - adds tenant-specific headers/footers

Each handles one cross-cutting concern independently.

## Interaction with Observers

Interceptors run **before** delivery. Observers run **after** delivery.

```
Email
  → Interceptor (can modify/block)
    → Mailer.deliver()
      → Observer (can only observe result)
```

If an interceptor returns `Err(...)`, the email is not sent and observers are not called.

## When NOT to use Interceptors

For simple cases, just modify the email before calling `deliver()`:

```rust
// This is fine for one-off modifications
let email = email.header("X-Campaign", "welcome");
deliver(&email).await?;
```

Use interceptors when:
- You want the behavior to apply to **all** emails through a mailer
- You're building reusable components
- You have multiple call sites and want consistent behavior
