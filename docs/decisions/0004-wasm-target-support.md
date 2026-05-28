# Decision 0004: WASM Target Support

## Status

Accepted for 0.7.0.

## Context

GitHub issue #2 asks for WASM support, especially for worker-style runtimes.
The relevant target is `wasm32-unknown-unknown`.

Rust's target documentation describes this target as minimal: it has `std`, but
many OS-backed APIs are unavailable or return errors, including filesystem APIs.
It recommends identifying this target with:

```rust
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
```

Rust 1.96 also changes WebAssembly linking behavior by no longer silently
allowing undefined symbols. That pushes crate authors toward explicit platform
bindings and target-aware dependencies instead of relying on linker fallbacks.

References:

- https://doc.rust-lang.org/stable/rustc/platform-support/wasm32-unknown-unknown.html
- https://blog.rust-lang.org/2026/04/04/changes-to-webassembly-targets-and-handling-undefined-symbols/
- https://github.com/jeffhuen/missive/issues/2

## Decision

- Expose a no-op `wasm` marker feature so users can opt in with
  `features = ["wasm", "resend"]`.
- Keep the actual platform wiring target-specific rather than feature-specific;
  Cargo selects the WASM dependency graph from `--target wasm32-unknown-unknown`.
- Keep native dependency features native: reqwest uses `rustls-tls` and
  `multipart` only off WASM, and Tokio filesystem support is native-only.
- Enable WASM-compatible randomness/time behavior through target-specific
  `uuid`, `chrono`, and `web-time` dependencies.
- Use `#[async_trait(?Send)]` only on `wasm32-unknown-unknown`, while preserving
  `Send + Sync` mailer bounds on native targets.
- Keep the compatibility global mailer, but use thread-local `RefCell` storage
  on WASM instead of a process-global lock.
- Treat path-backed attachments as unsupported on WASM. Byte-backed attachments
  work everywhere.
- Gate native-only features with a clear compile error: `smtp`, `gmail`,
  `protonbridge`, `mailgun`, `preview`, `preview-axum`, and `preview-actix`.
- Replace Amazon SES signing's `ring` dependency with pure-Rust `hmac` + `sha2`
  so SES can compile on WASM.
- Add CI coverage for the core crate and the supported WASM provider set.

## Consequences

Users can compile Missive for WASM with the `wasm` marker feature and a
supported provider feature:

```toml
missive = { version = "0.7.0", default-features = false, features = ["wasm", "resend"] }
```

The `wasm` feature is intentionally a marker. It makes the public Cargo API
clear, but it does not try to override Cargo's target resolution.

WASM users should prefer explicit providers and `EmailClient::new(...)`.
`EmailClient::from_env_with(...)` remains available for runtimes that expose
environment-like bindings, but `EmailClient::from_env()` and global
auto-configuration are native-only because `wasm32-unknown-unknown` has no
process environment.

Mailgun may become WASM-compatible later with manual multipart construction or
a reqwest WASM multipart path, but it is intentionally native-only for this
release.
