# Decision 0002: Mailer Async Dispatch Strategy

Status: accepted

Issue: `missive-wdl.14`

Date: 2026-05-28

## Context

`Mailer` currently uses `#[async_trait]`:

```rust
#[async_trait]
pub trait Mailer: Send + Sync {
    async fn deliver_prepared(&self, email: &PreparedEmail) -> Result<DeliveryResult, MailError>;
}
```

The macro boxes returned futures so the trait remains object-safe. That object
safety is used by environment-based provider selection, the compatibility global
facade, interceptors, and `EmailClient<Arc<dyn Mailer>>`.

Native async functions in traits remove the macro for static dispatch, but they
are not object-safe. A trait with native `async fn` cannot be used directly as
`dyn Mailer`, so runtime provider selection would need a separate boxed adapter
trait or wrapper.

## Decision

For v0.7, keep `Mailer` as the object-safe provider trait implemented with
`async_trait`.

`EmailClient<M>` remains the primary application API:

```rust
let client = EmailClient::new(ResendMailer::new(config)?);
client.deliver(email).await?;
```

Runtime provider selection remains explicit through a trait object:

```rust
let client: EmailClient<Arc<dyn Mailer>> = EmailClient::from_env()?;
```

Do not split the public provider trait into native-async and boxed-object
variants during the v0.7 cleanup. The library can revisit that split later if
benchmarks show the boxed future allocation matters for a real workload.

## Tradeoffs

### Object Safety

Keeping `async_trait` preserves a single provider trait that works for both
concrete providers and runtime-selected providers. This keeps custom provider
implementations simple and avoids forcing downstream applications to learn two
traits.

Native async traits would improve the static-dispatch shape, but they would
break `Arc<dyn Mailer>` and require a second object-safe adapter. That adapter
would still box futures, so runtime dispatch would not become allocation-free.

### Dispatch Modes

Concrete clients such as `EmailClient<ResendMailer>` avoid trait-object dynamic
dispatch, but provider method futures are still boxed by `async_trait`.

Object clients such as `EmailClient<Arc<dyn Mailer>>` pay both dynamic dispatch
and boxed future costs. This is acceptable for v0.7 because delivery providers
are I/O-bound and network latency dominates a small allocation.

### Migration Cost

Moving to native async traits now would require touching every provider,
interceptor, compatibility facade, config loader, and custom-provider example.
It would also introduce a new boxed adapter type and migration guidance before
the rest of the v0.7 API changes have settled.

The current cleanup has higher-value work: typed configuration, explicit
clients, validated prepared emails, attachment correctness, and safer provider
serialization.

## Future Option

A later release can introduce a split design if there is measured demand:

- `StaticMailer` with native async trait methods for concrete provider use.
- `BoxMailer` or `DynMailer` as an object-safe adapter for runtime provider
  selection.
- `EmailClient<M>` generic over either concrete static providers or the boxed
  adapter.

That future change should be benchmark-driven and should include migration
examples for custom providers.

## Consequences

`async_trait` remains a core dependency for v0.7.

The public docs should describe `EmailClient<M>` as the primary API and
`Arc<dyn Mailer>` as the explicit runtime-dispatch path, not as a hidden default.

This decision supersedes the zero-boxing aspiration in Decision 0001 for the
v0.7 cleanup window. The target remains idiomatic and explicit ownership first;
removing boxed futures is deferred until it has evidence and a lower-migration
design.
