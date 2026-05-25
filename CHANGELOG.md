# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned

- Module split of the single-file `src/lib.rs` (~2,530 LOC) into focused submodules.
- Edition bump to 2021, declared MSRV (`rust-version`).
- Dependency modernization: `lazy_static` → `std::sync::OnceLock`, `custom_error` → `thiserror`, removal of unused `num-derive` / `num-traits` / `pad`.
- API redesign for `0.2.0`: `pub(crate)` field visibility, exact `PartialEq` with `±0` canonicalization, `Result`-based fallible arithmetic (`checked_*` methods), removal of `Decimal::nan()` from the public API.
- Godot 4 support via `gdext`; existing `godot` feature deprecated in favor of `godot4`.
- WASM build target via a new `wasm` feature.
- Expanded test suite: Hash/Eq contract tests, `proptest` properties, JS-parity fixture tests, `Display` ↔ `TryFrom<&str>` roundtrips.

## [0.1.1] - Unreleased

### Fixed

- `repository` and `homepage` in `Cargo.toml` pointed to a non-existent GitHub repo (`break-eternity-rust`); now correctly resolve to `break-eternity-rs`.
- Malformed crates.io badge in `README.md` is now a clickable link with proper alt text.

### Changed

- `authors` field in `Cargo.toml` reduced to GitHub handles only — no personal names or email addresses.
- `categories` updated from `"mathematics"` to the canonical `"science::mathematics"` crates.io slug.
- `LICENSE` updated to credit `Patashu` (original JavaScript author) and remove the personal name; copyright lines now read by GitHub handle / pseudonym.

### Added

- `documentation` field in `Cargo.toml` linking to docs.rs.
- `[package.metadata.docs.rs]` block enabling all-features docs builds.
- `docs.rs` and MIT license badges in `README.md`.
- `## Installation` section in `README.md`.
- `CHANGELOG.md` (this file).
- `SECURITY.md` with vulnerability reporting policy.
- `CONTRIBUTING.md` with branch model and PR workflow.
- `.github/workflows/ci.yml` — fmt, clippy, test (stable + beta × Linux/macOS/Windows), MSRV check, wasm32 build.
- `.github/workflows/audit.yml` — daily `cargo audit`.
- `.github/dependabot.yml` — weekly cargo + github-actions updates.
- `Patashu` attribution line in `LICENSE`.
- Standard Rust entries added to `.gitignore` (`*.rs.bk`, `*.pdb`, `.idea/`, `.DS_Store`).

## [0.1.0] - 2026-05-25

### Added

- Initial fork of [cozyGalvinism/break-eternity](https://github.com/cozyGalvinism/break-eternity) (itself a port of [Patashu/break_eternity.js](https://github.com/Patashu/break_eternity.js)) published to crates.io as `break-eternity-rs`.
- Public getters and setters for `Decimal` components.

### Notes

- **Known issue (fixed in 0.1.1)**: published `repository` and `homepage` URLs link to a non-existent GitHub repo.
- **Known issue (fixed in 0.1.1)**: published `authors` field exposed a personal email address.

[Unreleased]: https://github.com/MaddisonM79/break-eternity-rs/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/MaddisonM79/break-eternity-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/MaddisonM79/break-eternity-rs/releases/tag/v0.1.0
