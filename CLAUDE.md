# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Operating Principles

Adapted from the concepts in
[`multica-ai/andrej-karpathy-skills`](https://github.com/multica-ai/andrej-karpathy-skills/blob/main/CLAUDE.md):

- **Think before coding**: State important assumptions, surface ambiguity, and ask when the request or risk is unclear. If a simpler or safer path exists, say so before implementing.
- **Prefer simple solutions**: Write the minimum code that solves the actual request. Avoid speculative features, single-use abstractions, and configurability that was not asked for.
- **Make surgical changes**: Touch only the files and behavior needed for the task. Match existing style, avoid unrelated refactors, and clean up only unused code introduced by your own changes.
- **Work against verifiable goals**: For non-trivial work, define what success means, choose the checks that prove it, and loop until those checks pass.
- **Keep every changed line accountable**: Each edit should trace back to the user request, the Beads issue, or a necessary verification/cleanup step.

## Build and Test

**All `cargo` commands require `--features full`** — tests, clippy, and most builds fail or skip coverage without it (e.g. `cargo test --features full`, `cargo clippy --features full`).

## Feature Flags

This crate uses Cargo features for conditional compilation. Use `--features full` to enable everything. Key feature groups:

- **Providers**: `smtp`, `resend`, `sendgrid`, `postmark`, `brevo`, `mailgun`, `amazon_ses`, `mailtrap`, `unsent`
- **Development**: `local` (in-memory storage + test assertions), `preview` (web UI), `dev` (both)
- **Extras**: `templates` (Askama integration), `metrics` (Prometheus counters/histograms)
- **Bundles**: `full` (all providers + templates), `dev` (local + preview)

Internal features prefixed with `_` (`_http`, `_aws_sig`) are shared dependencies and not meant for direct use.

## Architecture

For detailed architectural decisions and rationale, see `docs/architecture.md`.

**Dynamic dispatch with `async_trait`**: The `Mailer` trait uses `#[async_trait]` instead of native async traits to enable `Arc<dyn Mailer>`, allowing runtime provider selection from environment variables. The heap allocation cost (~10ns) is negligible vs. network I/O (50-500ms). See `src/mailer.rs`.

**Wrapper pattern for extensions**: Features like interceptors use the wrapper/decorator pattern — a struct that holds an inner `Mailer` and implements `Mailer` itself — enabling composition without modifying core types. See `src/interceptor.rs`.

**Global mailer**: `deliver(&email)` uses a global mailer auto-configured from env vars; `deliver_with(&email, &mailer)` uses a specific instance; `configure(mailer)` sets the global manually. See `src/lib.rs`.

## Environment Variables

The library auto-configures from environment:
- `EMAIL_PROVIDER` - Which provider to use (auto-detected if not set)
- `EMAIL_FROM` / `EMAIL_FROM_NAME` - Default sender
- Provider-specific: `RESEND_API_KEY`, `SENDGRID_API_KEY`, `SMTP_HOST`, etc.


<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:7510c1e2 -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

**Architecture in one line:** issues live in a local Dolt DB; sync uses `refs/dolt/data` on your git remote; `.beads/issues.jsonl` is a passive export. See https://github.com/gastownhall/beads/blob/main/docs/SYNC_CONCEPTS.md for details and anti-patterns.

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
<!-- END BEADS INTEGRATION -->

## graphify

This project has a knowledge graph at graphify-out/ with god nodes, community structure, and cross-file relationships.

Rules:
- For codebase questions, first run `graphify query "<question>"` when graphify-out/graph.json exists. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts. These return a scoped subgraph, usually much smaller than GRAPH_REPORT.md or raw grep output.
- If graphify-out/wiki/index.md exists, use it for broad navigation instead of raw source browsing.
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).
