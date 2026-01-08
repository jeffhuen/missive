# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-01-07

Initial release.

### Added

#### Email Providers
- **SMTP** - Traditional SMTP delivery via lettre (`smtp` feature)
- **Resend** - Resend API integration (`resend` feature)
- **SendGrid** - SendGrid API integration (`sendgrid` feature)
- **Postmark** - Postmark API integration (`postmark` feature)
- **Unsent** - Unsent API integration (`unsent` feature)
- **LocalMailer** - In-memory storage for development and testing (`local` feature)
- **LoggerMailer** - Console logging without storage (always available)

#### Email Composition
- Fluent builder API for composing emails
- Support for HTML and plain text bodies
- Multiple recipients (to, cc, bcc)
- Reply-to addresses
- Custom headers
- Provider-specific options via `.provider_option()`

#### Attachments
- `Attachment::from_bytes()` - Create from in-memory data
- `Attachment::from_path()` - Eager file loading
- `Attachment::from_path_lazy()` - Lazy file loading at send time
- Inline attachments with Content-ID for HTML embedding
- Automatic MIME type detection

#### Email Validation
- `Address::parse()` - RFC 5321/5322 compliant validation
- `Address::parse_with_name()` - Validated name + email pairs
- `Address::to_ascii()` - IDN/Punycode encoding for international domains
- `ToAddress` trait for custom recipient types

#### Testing Support
- `LocalMailer` with in-memory storage for test assertions
- Assertion helpers: `assert_email_sent`, `assert_email_to`, `assert_email_subject`, etc.
- Regex assertions: `assert_email_subject_matches`, `assert_email_html_matches`
- Failure simulation with `set_failure()` / `clear_failure()`
- `flush_emails()` for atomic get-and-clear in multi-phase tests

#### Development Tools
- Mailbox preview web UI (`preview` feature)
- HTML and plain text email preview
- Attachment downloads
- JSON API for programmatic access
- CSP nonce support

#### Observability
- Tracing spans for all email deliveries
- Prometheus-style metrics (`metrics` feature)
  - `missive_emails_total` counter
  - `missive_delivery_duration_seconds` histogram
  - `missive_batch_total` counter
  - `missive_batch_size` histogram

#### Templates
- Askama template integration (`templates` feature)
- `EmailTemplate` trait for type-safe templates
- `.render_html()` and `.render_text()` methods

#### Configuration
- Zero-config setup via environment variables
- Auto-detection of provider from available API keys
- `EMAIL_PROVIDER` for explicit provider selection
- `EMAIL_FROM` / `EMAIL_FROM_NAME` for default sender
- `init()` for environment-based initialization
- `configure()` for programmatic setup

#### Infrastructure
- Batch sending with `deliver_many()`
- Batch validation via `Mailer::validate_batch()`
- User-Agent headers with version on all API requests
- Detailed error types for different failure modes
