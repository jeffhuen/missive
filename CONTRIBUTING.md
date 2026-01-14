# Contributing to Missive

Thanks for thinking about contributing to Missive. Please review this document
and our [Code of Conduct](CODE_OF_CONDUCT.md) to help us keep this project
welcoming to all.

## Opening Issues

We classify bugs as any unexpected behaviour based on the code in the project,
and we appreciate it when users take the time to
[create an issue](https://github.com/jeffhuen/missive/issues).

Please include as much detail as you can in bug reports:
- Rust version (`rustc --version`)
- Missive version and features enabled
- Which provider you're having trouble with
- Any custom configuration
- Minimal reproduction if possible

If you're thinking about adding a feature, check the Issues first—someone else
may have started already, or it might be something we've intentionally decided
not to implement.

## Submitting Pull Requests

Whether you're fixing a bug or proposing a feature, follow this guide:

1. [Fork this repository](https://github.com/jeffhuen/missive/fork) and clone it:

   ```bash
   git clone https://github.com/YOUR_USERNAME/missive
   ```

2. Create a topic branch for your changes:

   ```bash
   git checkout -b fix-some-bug
   ```

3. Commit a failing test for the bug:

   ```bash
   git commit -am "Add failing test that demonstrates the bug"
   ```

4. Commit a fix that makes the test pass:

   ```bash
   git commit -am "Fix the bug"
   ```

5. Run the tests and checks:

   ```bash
   cargo fmt
   cargo clippy --features full
   cargo test --features full
   ```

6. If everything looks good, push to your fork:

   ```bash
   git push origin fix-some-bug
   ```

7. [Submit a pull request](https://help.github.com/articles/creating-a-pull-request)

## Adding a New Provider

If you're adding support for a new email provider:

1. Create `src/providers/your_provider.rs`
2. Add a feature flag in `Cargo.toml`
3. Add the module export in `src/providers/mod.rs`
4. Add environment variable detection in `src/lib.rs`
5. Create tests in `tests/adapters/your_provider_test.rs`
6. Add documentation in `docs/providers.md`
7. Update `CHANGELOG.md`

Look at existing providers (e.g., `resend.rs`, `socketlabs.rs`) for reference.

## Style Guidelines

We follow standard Rust conventions. When in doubt, look at existing code.

### General Rules

- Run `cargo fmt` before committing
- Run `cargo clippy --features full` and address warnings
- Keep functions focused and reasonably sized
- Add doc comments for public APIs
- Write tests for new functionality

### Commit Messages

- Use present tense ("Add feature" not "Added feature")
- Keep the first line under 72 characters
- Reference issues when relevant ("Fix #123")
