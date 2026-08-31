# Changelog

## Unreleased

### Fixed

- Redacted provider credentials from configuration `Debug` output.
- Added a 30-second timeout to HTTP requests made by default provider clients.

## [0.7.0] - 2026-05-28

### Changed

- **Breaking:** Made `Email`, `Address`, and `Attachment` fields private. Use
  builder methods for construction and accessor methods such as
  `email.to_addresses()`, `email.subject_line()`, `address.email()`, and
  `attachment.data()` for reads.
- **Breaking:** Changed `Attachment::base64_data()` to return
  `Result<String, MailError>` and propagate attachment read failures during
  delivery instead of substituting empty content.
- **Breaking:** Changed `SmtpBuilder::build()` and
  `ProtonBridgeBuilder::build()` to return `Result<_, MailError>`.
- **Breaking:** Changed `MailError` to preserve source errors for HTTP, SMTP,
  lettre build, template, and attachment I/O failures. `MailError` no longer
  implements `Clone`.
- **Breaking:** Marked `MailError`, `MailerConfig`, and SMTP `TlsMode` as
  non-exhaustive so downstream matches need a wildcard arm.
- Preferred explicit `EmailClient<M>` ownership over process-global delivery
  configuration for application integration and dependency injection.
- Changed default delivery telemetry so recipient addresses and subjects are no
  longer recorded at info level.
- Changed provider address serialization to normalize internationalized domains
  with IDNA/Punycode where provider APIs require ASCII addresses.

### Added

- Added `EmailClient<M>` as the primary instance-owned delivery API.
- Added environment configuration through `EmailClient::from_env()`,
  `EmailClient::from_env_with(...)`, and `MailerConfig::from_env()`.
- Added typed Resend provider options through `ResendEmailExt`.
- Added Mailjet provider selection and credential auto-detection through
  `EMAIL_PROVIDER=mailjet`, `MAILJET_API_KEY`, and `MAILJET_SECRET_KEY`,
  fixing [#3](https://github.com/jeffhuen/missive/issues/3) and porting the
  intent of [#4](https://github.com/jeffhuen/missive/pull/4). Thanks
  [@emmiegit](https://github.com/emmiegit).
- Added `wasm32-unknown-unknown` support for core types, logger/local mailers,
  and HTTP JSON providers, addressing
  [#2](https://github.com/jeffhuen/missive/issues/2).
- Added a public `wasm` marker feature for explicit WASM builds, for example
  `missive = { features = ["wasm", "resend"] }`.

### Removed

- **Breaking:** Removed the deprecated `MailerExt::validate()` trait method. Use
  `Email::is_valid()` for a quick local check or delivery-time validation for
  default sender handling.
- **Breaking:** Removed process-global local preview storage behavior.
  `local_storage()` is retained only as a deprecated compatibility facade and
  returns `None`; create `LocalMailer` explicitly and pass `mailer.storage()` to
  preview APIs.

### Fixed

- Fixed lazy/path-backed attachments so missing files fail delivery instead of
  sending empty content.
- Fixed zero-byte attachments so intentionally empty files can be sent.
- Fixed SMTP and Gmail invalid attachment MIME fallback to use
  `application/octet-stream`.
- Fixed Amazon SES Bcc delivery and inline attachment handling.
- Fixed WASM compile support for supported providers and feature combinations.
- Fixed SMTP TLS configuration so `SMTP_TLS` accepts `starttls`, `tls`, or
  `none`, and rejects `opportunistic` rather than allowing silent downgrade.
- Fixed Amazon SES raw MIME generation for non-ASCII headers, attachment
  filenames, and CR/LF header-injection rejection.
- Fixed feature-bundle documentation for `full`, `dev`, preview, and WASM
  combinations.

## [0.6.2] - 2026-01-19

### Changed

- Deprecated `MailerExt::validate()` because it does not apply the
  `EMAIL_FROM` fallback. Use `Email::is_valid()` for a quick local check or
  delivery-time validation through `deliver()` and `deliver_with()`. Removed in
  0.7.0. ([#1](https://github.com/jeffhuen/missive/issues/1))

## [0.6.1] - 2026-01-17

### Fixed

- Fixed Clippy lint errors in the JMAP provider.
- Fixed formatting issues in the JMAP provider and test module ordering.

## [0.6.0] - 2026-01-17

### Added

- Added JMAP provider support through the `jmap` feature, including session
  discovery, basic authentication, bearer token authentication, and email
  submission.
- Added Proton Mail Bridge support through the `protonbridge` feature.
- Added a Docker-based JMAP testing guide in `docs/jmap-testing.md`.

## [0.5.0] - 2026-01-13

### Added

- Added Gmail API provider support through the `gmail` feature.
- Added SocketLabs Injection API provider support through the `socketlabs`
  feature.
- Added HTTP client architecture documentation.

## [0.4.0] - 2026-01-09

### Changed

- Changed the `preview` feature to use the standalone `tiny_http` preview
  server instead of aliasing to `preview-axum`.
- Changed the `dev` feature bundle to use the standalone preview server.

### Added

- Added standalone preview server support through the `preview` feature.
- Added `PreviewServer::new(addr, storage)`, `PreviewServer::spawn()`,
  `PreviewServer::run()`, and `serve(addr, storage)`.

## [0.3.0] - 2026-01-08

### Added

- Added interceptors for modifying or blocking emails before delivery.
- Added `Interceptor`, `InterceptorExt`, closure-based interceptors, and
  interceptor chaining.

## [0.2.0] - 2026-01-07

### Changed

- Refactored preview routing into shared core logic with framework-specific
  adapters.

### Added

- Added Actix-web support for mailbox preview UI through the `preview-actix`
  feature.
- Added `preview-axum` and `preview-actix` feature flags.
- Added `actix_configure()` and `ActixAppState`.

## [0.1.0] - 2025-01-07

### Added

- Added SMTP, Resend, SendGrid, Postmark, Unsent, local, and logger mailers.
- Added fluent email composition for HTML and plain text bodies, multiple
  recipients, reply-to addresses, custom headers, provider options, and
  attachments.
- Added eager and lazy attachment loading, inline attachments, Content-ID
  support, and MIME type detection.
- Added address parsing, address formatting, IDN/Punycode conversion, and the
  `ToAddress` trait.
- Added local testing helpers and in-memory mail storage.
- Added mailbox preview UI for development.
