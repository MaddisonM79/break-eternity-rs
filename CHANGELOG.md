# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned

- Godot 4 support via `gdext`; existing `godot` feature deprecated in favor of `godot4`.
- WASM build target via a new `wasm` feature.
- Expanded test suite: `proptest` property tests, JS-parity fixture tests.

## [0.2.0-rc.1] - Unreleased

### Breaking

- `Decimal` fields `sign`, `layer`, and `mag` are now `pub(crate)`. External code must use the public accessor methods (`sign()`, `layer()`, `mag()`) instead of direct field access or struct-literal construction.
- `set_sign`, `set_layer`, `set_mag` removed from the public API. These methods allowed callers to bypass normalization and violate the struct invariant.
- `from_components_no_normalize` renamed to `from_components_unchecked` and made `pub(crate)`. It was never safe to call from outside the crate.
- `set_from_mantissa_exponent_no_normalize` was a misnomer (always normalized). Removed from public; an internal `pub(crate)` equivalent exists.
- `PartialEq` changed from approximate comparison (`|Δmag| < 1e-10`) to exact bit-comparison. Code relying on equality of nearly-equal `Decimal` values must switch to `approx_eq`.
- `Ord` / `cmpabs` now return `std::cmp::Ordering` instead of `i8`.
- `From<f64>` and `From<f32>` replaced with `TryFrom<f64>` and `TryFrom<f32>`. Non-finite inputs (NaN, ±Infinity) now return `Err(ArithmeticError { kind: Undefined })`.
- `Decimal::nan()` removed from the public API. Use `TryFrom<f64>` for fallible construction; for sentinels inside the crate, `nan_sentinel()` is `pub(crate)`.
- `Decimal::is_nan()` removed from the public API; replaced internally by `has_nan_mag()` (`pub(crate)`).
- `from_number` deprecated; use `from_finite` or `TryFrom<f64>`.
- `from_mantissa_exponent_no_normalize` deprecated; use `from_mantissa_exponent` (always normalizes; the `no_normalize` name was always a misnomer).
- `iteratedexp` deprecated; use `tetrate` (identical behavior).
- `eq_tolerance` deprecated; use `approx_eq`.
- `to_fixed` (free function in `format` module) parameter `places: i32` changed to `places: usize`.
- Parsing `"NaN"` now returns `Err(ParseError)` instead of a NaN-sentinel `Decimal`. `"Infinity"` and `"-Infinity"` still parse successfully.

### Added

- `Decimal::from_finite(x: f64) -> Decimal` — infallible constructor; debug-asserts finiteness.
- `TryFrom<f64>` and `TryFrom<f32>` for `Decimal` — explicit fallible conversion, returns `Err(ArithmeticError { kind: Undefined })` for NaN/infinite inputs.
- `Decimal::approx_eq(&self, other: &Self, tolerance: f64) -> bool` — tolerance-based equality (the semantics of the old `PartialEq`).
- `ArithmeticError` and `ArithmeticErrorKind` — structured error type for arithmetic failures; `BreakEternityError::Arithmetic` variant wraps it.
- `ArithmeticErrorKind` variants: `Undefined`, `DivisionByZero`, `NegativeBase`, `OutOfDomain`, `IterationDiverged`.
- `Decimal::checked_add`, `checked_sub`, `checked_mul`, `checked_div`, `checked_rem`, `checked_pow` — fallible arithmetic returning `Result<Decimal, ArithmeticError>`.
- `Decimal::checked_ln`, `checked_log10`, `checked_log2`, `checked_log`, `checked_sqrt` — fallible transcendental functions.
- `Decimal::checked_lambertw`, `checked_gamma`, `checked_factorial` — fallible gamma/lambertw.
- `Decimal::checked_tetrate`, `checked_iteratedexp`, `checked_iteratedlog`, `checked_slog`, `checked_ssqrt`, `checked_pentate` — fallible tetration operations.
- Reference-combination operator overloads: `&Decimal op &Decimal`, `Decimal op &Decimal`, `&Decimal op Decimal` for Add, Sub, Mul, Div, Rem.
- `normalize()` now canonicalizes `-0.0` to `0.0` so that `Hash` and `Eq` are consistent.
- `Hash` implementation uses `(sign, layer, mag.to_bits())` — consistent with exact `Eq`.
- `parse.rs`: added `parse_f64` helper to consolidate error-mapped float parsing; parsing no longer propagates internal panics for non-matching formats.
- Parsing of `"eeeee<mag>"` (multi-`e` prefix format from `Display`) and `"(e^N)M"` (parenthesized large-layer format from `Display`) so round-trips work for all layer values.
- `serde_impl.rs`: module-level doc explaining the string-based serialization format.
- `tests/hash_eq_contract.rs`: `HashMap` lookup, `HashSet` dedup, `±0` canonicalization.
- `tests/checked_arithmetic.rs`: `checked_div` by zero, `checked_ln` of non-positive, `TryFrom<f64>` for NaN/∞, operator-overload panic verification, reference-op correctness.
- `tests/serde_roundtrip.rs` (feature = `serde`): round-trips for layers 0, 1, 2, 5, 100, and ±Infinity.

### Changed

- `Ord::cmp` rewritten to use `Ordering::then_with` instead of sign-multiplied `i8` arithmetic.
- `cmpabs` return type changed from `i8` to `std::cmp::Ordering`.
- Operator overloads (`Add`/`Sub`/`Mul`/`Div`/`Rem`) now delegate to `checked_*` and panic with a structured message on undefined results, analogous to integer overflow-panic in debug builds.
- `format::decimal_places` no longer uses format-and-reparse; now uses direct float arithmetic.
- `format::to_fixed` parameter changed from `i32` to `usize`.
- `parse.rs`: `"^"` / `"pt"` / `"p"` parsing branches are now non-fatal for non-matching inputs (fall through rather than propagating `ParseError`), fixing incorrect early-exit for formats like `"(e^100)15000000000"`.

### Fixed

- `Eq` contract: `PartialEq` was using approximate comparison while `Eq` was derived, violating the reflexivity contract for `Hash`. Both are now exact.
- `Hash` contract: derived `Hash` previously did not match the approximate `PartialEq`. Now both use `mag.to_bits()`.
- `set_from_mantissa_exponent_no_normalize` previously normalized despite its name — the behavior is now documented and the method is `pub(crate)`.
- `mantissa_with_decimal_places` and `magnitude_with_decimal_places` removed NaN sentinel checks (no publicly-reachable NaN Decimals after Phase 4).

## [0.1.1] - Unreleased

### Fixed

- `repository` and `homepage` in `Cargo.toml` pointed to a non-existent GitHub repo (`break-eternity-rust`); now correctly resolve to `break-eternity-rs`.
- Malformed crates.io badge in `README.md` is now a clickable link with proper alt text.

### Changed

- Edition bump to 2021, MSRV declared as `rust-version = "1.70"`.
- Dependency modernization: `lazy_static` → `std::sync::OnceLock`, `custom_error` → `thiserror`, removed unused `num-derive` / `num-traits` / `pad`.
- `authors` field in `Cargo.toml` reduced to GitHub handles only — no personal names or email addresses.
- `categories` updated from `"mathematics"` to the canonical `"science::mathematics"` crates.io slug.
- `src/lib.rs` split into focused submodules: `arithmetic`, `constants`, `decimal`, `error`, `format`, `parse`, `serde_impl`, `tetration`, `transcendental`, `utils`.

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

[Unreleased]: https://github.com/MaddisonM79/break-eternity-rs/compare/v0.2.0-rc.1...HEAD
[0.2.0-rc.1]: https://github.com/MaddisonM79/break-eternity-rs/compare/v0.1.1...v0.2.0-rc.1
[0.1.1]: https://github.com/MaddisonM79/break-eternity-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/MaddisonM79/break-eternity-rs/releases/tag/v0.1.0
