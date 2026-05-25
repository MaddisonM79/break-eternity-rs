# Contributing to break-eternity-rs

Thanks for taking the time to contribute! This document describes the workflow for filing issues, sending pull requests, and the local tooling expectations.

## Reporting Bugs

- Search [open and closed issues](https://github.com/MaddisonM79/break-eternity-rs/issues?q=is%3Aissue) first.
- Open a new issue with: the crate version, your Rust toolchain (`rustc --version`), a minimal reproduction, the expected behavior, and what actually happens. Numeric bugs benefit enormously from a known-correct comparison value (e.g., the matching output from `break_eternity.js`).
- For security issues, please follow [`SECURITY.md`](SECURITY.md) instead of opening a public issue.

## Suggesting Features

Open an issue prefixed with `feat:` and describe:

- the use case (what game / system you're building),
- the proposed API (function signature, behavior on edge cases),
- whether it would parallel a feature in upstream [`break_eternity.js`](https://github.com/Patashu/break_eternity.js) (this crate tries to preserve JS parity where reasonable).

## Pull Requests

### Branches

- The default branch is **`develop`**. All PRs target `develop`.
- Release tags (`v0.1.1`, etc.) are cut from `develop` once a release is staged.

### Before you push

```sh
cargo fmt --all
cargo clippy --all-features --all-targets -- -D warnings
cargo test --all-features
cargo build --no-default-features
```

If your change might affect WASM users:

```sh
cargo build --target wasm32-unknown-unknown
```

If your change touches the public API, please update `CHANGELOG.md` under `[Unreleased]`. Follow the [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/) format already established there.

### Commit messages

Conventional Commits are encouraged but not enforced. A typical prefix set:

- `feat:` new public API
- `fix:` bug fix
- `perf:` performance improvement
- `refactor:` internal change, no behavior diff
- `test:` test-only change
- `docs:` documentation
- `chore:` build / CI / tooling

Co-author trailers from AI tools (e.g., `Co-Authored-By: Claude`) **should not** be added to commits or PR descriptions.

### What gets reviewed

- **Correctness**: numeric ports of `break_eternity.js` functions should be checked against the JS reference behavior. Include test vectors when possible.
- **Soundness**: any change that affects `PartialEq` / `Eq` / `Hash` / `Ord` semantics on `Decimal` requires explicit contract tests.
- **API surface**: new public items need rustdoc with at least one usage example. The crate is `#![warn(missing_docs)]`.
- **No new `unsafe`** without a written justification.

## Local Development

Standard cargo workflow:

```sh
git clone https://github.com/MaddisonM79/break-eternity-rs.git
cd break-eternity-rs
cargo test
```

To exercise the optional features:

```sh
cargo test --features serde
cargo test --features godot   # Godot 3 (gdnative); will be renamed to godot4 in a future release
```

## License

By contributing, you agree that your contributions are licensed under the [MIT License](LICENSE) used by this project.
