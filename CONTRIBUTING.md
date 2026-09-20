# Contributing to break-eternity-rs

Thanks for taking the time to contribute! This document describes the workflow for filing issues, sending pull requests, and the local tooling expectations. For how the crate is structured internally (the raw / checked / panicking layering, the NaN sentinel rules, and the parity policy) read [`docs/DESIGN.md`](docs/DESIGN.md) first.

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
- Release tags (`v0.5.0`, etc.) are cut from `develop` once a release is staged. Pushing a `v*` tag runs the release workflow, which verifies the tag matches `Cargo.toml`, publishes to crates.io, and creates the GitHub release from the matching `CHANGELOG.md` section.

### Before you push

```sh
cargo fmt --all
cargo clippy --features serde,wasm --all-targets -- -D warnings
cargo test --features serde
cargo build --no-default-features
```

The `godot4` feature needs `libclang` for the gdext build script; CI builds it on Linux, so you only need it locally if you touch `src/godot_impl.rs`:

```sh
cargo clippy --all-features --all-targets -- -D warnings
```

If your change might affect WASM users:

```sh
cargo build --target wasm32-unknown-unknown --features wasm
# with wasm-bindgen-cli installed (matching the wasm-bindgen version in Cargo.lock):
cargo test --target wasm32-unknown-unknown --features wasm --test wasm
```

If your change touches the public API, please update `CHANGELOG.md` under `[Unreleased]`. Follow the [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/) format already established there. CI runs `cargo semver-checks`, so a breaking change also needs the version bumped (pre-1.0, that means the minor version).

### Parity fixture

`tests/parity.rs` replays `tests/fixtures/parity.json`, which is generated from a pinned `break_eternity.js` release, and fails on any mismatch that is not an explicitly listed deliberate divergence. If you change numeric behaviour:

```sh
cd tests/fixtures
npm install break_eternity.js@2.1.3   # the pinned version; generate.mjs refuses others
node generate.mjs                     # takes several minutes (slog is slow in JS)
cd ../.. && cargo test --features serde --test parity -- --nocapture
```

Add new operations to `generate.mjs` *and* to the `evaluate` match in `parity.rs`. If the Rust answer is deliberately different from JS (and better), add a rule to `deliberate_divergence` with the reason rather than loosening the tolerance.

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

- **Correctness**: numeric ports of `break_eternity.js` functions should be checked against the JS reference behavior. Include test vectors when possible (the parity fixture is the easiest place).
- **Soundness**: any change that affects `PartialEq` / `Eq` / `Hash` / `Ord` semantics on `Decimal` requires explicit contract tests. `checked_*` methods must never return the internal NaN sentinel; the panicking forms must never return it either. New operations go in the tables in `tests/contract.rs`, which enforces this mechanically.
- **No panics on input**: the parser is fuzzed by proptest and must never panic on any string. Arithmetic on any two finite `Decimal`s must not panic except through the documented operator-overload convention.
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
cargo build --features godot4    # needs libclang
cargo build --features wasm --target wasm32-unknown-unknown
cargo bench                      # criterion benchmarks in benches/ops.rs
cargo run --example idle_loop
```

## License

By contributing, you agree that your contributions are licensed under the [MIT License](LICENSE) used by this project.
