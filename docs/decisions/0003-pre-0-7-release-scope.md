# Decision 0003: Pre-0.7 Release Scope

## Status

Accepted for 0.7.0.

## Context

Before releasing 0.7.0, four external inputs were reviewed:

- Rust 1.96.0 release notes: https://releases.rs/docs/1.96.0/
- GitHub issue #1: remove deprecated `MailerExt::validate()`
- GitHub issue #2: WASM support
- GitHub issue #3 and pull request #4: `EMAIL_PROVIDER=mailjet`
- Swoosh Mailjet adapter: https://github.com/swoosh/swoosh

The crate currently documents Rust 1.75+ as its minimum supported Rust version.
Rust 1.96.0 adds useful APIs such as `assert_matches!`, but adopting them in
library or test code would raise the toolchain required for contributors and CI.

WASM support requires coordinated target-specific changes across global state,
environment configuration, filesystem-backed attachments, reqwest features,
randomness/time dependencies, provider feature compatibility, and async trait
`Send` bounds. That is larger than a safe release-blocking patch.

Mailjet already has a provider implementation, but the environment-driven
configuration layer did not expose it. Swoosh's Mailjet adapter confirms the
expected integration shape: Mailjet API v3.1, `/send`, basic auth from API key
and secret, a `Messages` JSON payload, and provider options including templates,
custom IDs, event payloads, tracking toggles, and URL tags.

## Decision

- Do not adopt Rust 1.96-only APIs in 0.7.0. Keep the current MSRV posture.
- Do not make WASM support a 0.7.0 blocker. Treat it as a dedicated follow-up
  feature tracked by GitHub issue #2.
- Remove the deprecated `MailerExt::validate()` API in 0.7.0.
- Port the Mailjet provider-detection fix from PR #4 into the current typed
  `MailerConfig` architecture instead of merging the old conflicting patch.
- Extend Mailjet provider options with `track_opens`, `track_clicks`, and
  `url_tags` to align with the Swoosh adapter option surface.

## Consequences

0.7.0 contains the small breaking API removal that was already deprecated, and
fixes a real provider integration bug without changing the MSRV or taking on the
larger WASM compatibility project.
