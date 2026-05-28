# Decision 0001: v0.7 Public API Shape

Status: accepted

Issue: `missive-wdl.1`

Date: 2026-05-28

## Context

The 0.6 API optimizes for quick setup:

```rust
let email = Email::new()
    .to("user@example.com")
    .subject("Welcome")
    .text_body("Hello");

missive::deliver(&email).await?;
```

That convenience comes from process-global mailer state, environment auto-detection
inside `deliver`, public data fields, `HashMap<String, serde_json::Value>`
extension points, and an object-safe `Mailer` trait implemented with
`async_trait`. Those choices made the initial library simple, but they also make
the crate harder to use as an idiomatic Rust dependency in larger applications.

The v0.7 refactor should make the explicit, typed, instance-owned path the
primary API while keeping a compatibility path for small applications and tests.

## Decision

### Primary API

The primary API for v0.7 is an instance-owned client:

```rust
let mailer = ResendMailer::new(ResendConfig::new(api_key)?);
let client = EmailClient::new(mailer)
    .with_default_from(Address::parse("noreply@example.com")?);

let email = Email::builder()
    .to("user@example.com")
    .subject("Welcome")
    .text_body("Hello")
    .build()?;

client.deliver(email).await?;
```

`EmailClient<M>` owns or shares the provider, default sender, interceptors,
telemetry policy, and other delivery configuration. Applications should pass the
client through normal dependency injection mechanisms such as Axum state,
Actix data, or an application-owned `Arc`.

Delivery should consume the email by value on the primary path:

```rust
impl<M: Mailer> EmailClient<M> {
    pub async fn deliver(&self, email: Email) -> Result<DeliveryResult, MailError>;
}
```

This makes clone costs explicit. A caller that needs to reuse a message can clone
it intentionally, while providers and interceptors can avoid deep cloning by
moving or sharing large attachment payloads internally.

### Email Construction And Validation

`Email`, `Address`, and `Attachment` fields should become private. Public
construction should happen through builders and accessor methods.

`Email` remains a draft message type that can be prepared by a client. A new
validated delivery model should separate provider input from user construction:

```rust
let prepared = client.prepare(email)?;
mailer.deliver(prepared).await?;
```

`PreparedEmail` is the provider-facing type. It includes defaults applied by the
client and guarantees required delivery invariants such as a sender and at least
one primary recipient. Providers should not need to repeat basic validation.

Address constructors should return `Result` when parsing or normalization can
fail. Builder methods that accept strings can either validate immediately or
defer errors into `build`, but invalid addresses should not silently enter a
prepared email.

### Provider Configuration

Provider constructors should accept typed configuration structs:

```rust
let config = ResendConfig {
    api_key,
    base_url: None,
    timeout: None,
};
let mailer = ResendMailer::new(config)?;
```

Environment loading should move out of the core delivery path and into explicit
helpers:

```rust
let config = ResendConfig::from_env()?;
let client = EmailClient::new(ResendMailer::new(config)?);
```

This keeps runtime configuration errors visible at startup and avoids hidden
global initialization during delivery.

### Provider-Specific Options

The stringly `provider_options` map should stop being the primary provider
extension mechanism. Provider crates/modules should expose typed option structs
and extension traits gated by their provider feature:

```rust
let email = Email::builder()
    .to("user@example.com")
    .subject("Welcome")
    .text_body("Hello")
    .resend_tag("category", "welcome")
    .resend_scheduled_at(send_at)
    .build()?;
```

Internally, provider-specific values can be stored in a private typed extension
bag keyed by concrete option type. Custom providers should be able to define
their own option type without depending on raw string keys. JSON maps may remain
as an escape hatch behind clearly named methods such as
`with_raw_provider_option`, but those methods should not be used in primary
examples.

Template data should likewise move toward typed input:

```rust
email.with_template_data(&welcome_context)?;
```

`serde_json::Value` can remain an internal representation for providers that
need JSON payloads, but callers should pass serializable Rust values where
possible.

### Provider Dispatch Strategy

Static dispatch is the default:

```rust
type ResendClient = EmailClient<ResendMailer>;
```

This path should avoid dynamic dispatch and avoid boxing futures when the
provider type is known at compile time.

Dynamic dispatch remains available, but it should be explicit:

```rust
let client: EmailClient<BoxMailer> = EmailClient::boxed(mailer);
```

The target shape is:

- A static provider trait for normal generic use.
- A boxed adapter type for runtime provider selection.
- Runtime provider selection only when the application asks for it.

The follow-up native-async-trait work should evaluate whether `Mailer` can move
to native async trait syntax or an explicit `impl Future + Send` return while
providing an object-safe boxed adapter for dynamic dispatch.

### Global Facade Policy

The process-global `deliver`, `deliver_with`, `configure`, and `reset` APIs are
compatibility sugar, not the primary v0.7 design.

They may remain behind a `global` or `compat-global` Cargo feature for one
transition cycle, but docs should lead with `EmailClient`. The global facade
should delegate to an internally owned `EmailClient` and should not introduce
behavior that the instance API cannot express.

Environment auto-detection should also move behind an explicit feature and
helper, such as `EmailClient::from_env()` or `GlobalMailer::init_from_env()`.
Calling `deliver` should not silently choose a provider unless the compatibility
feature documents that behavior.

### Migration Constraints

v0.7 is allowed to make breaking changes because the crate is still pre-1.0, but
the migration should be staged and mechanical:

- Keep existing builder-style method names where they remain valid.
- Add accessors before making fields private.
- Prefer deprecation warnings before removing global functions when feasible.
- Provide a migration guide with before and after examples for global delivery,
  provider configuration, provider options, attachments, and tests.
- Keep feature flags granular so users do not compile providers they do not use.
- Avoid adding a Tokio requirement to provider-neutral construction APIs.

## Consequences

This decision unblocks the v0.7 refactor sequence:

- `missive-wdl.5` should introduce `EmailClient` and make it the documented
  primary delivery path.
- `missive-wdl.6` should move environment detection into typed configuration
  helpers.
- `missive-wdl.7` should replace raw provider option maps with typed provider
  option APIs.
- `missive-wdl.8` and `missive-wdl.9` should encapsulate message fields and add
  the validated preparation model.
- `missive-wdl.14` should record the async dispatch decision and revisit the
  static plus boxed provider split only if profiling justifies the migration.

The existing 0.6 architecture notes remain useful as a description of the
current implementation, but this decision is the target architecture for the
breaking v0.7 cleanup.
