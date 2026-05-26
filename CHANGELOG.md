# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Breaking

- All tetration-family methods (`tetrate`, `checked_tetrate`, `iteratedexp`, `checked_iteratedexp`, `iteratedlog`, `checked_iteratedlog`, `layer_add`, `layer_add_10`, `pentate`, `checked_pentate`, `slog`, `checked_slog`) now take a `mode: TetrationMode` argument. `TetrationMode` is exported from the crate root. `TetrationMode::Analytic` (the default) matches `break_eternity.js`'s default critical-section interpolation; `TetrationMode::Linear` preserves the older closed-form approximation. Existing call sites must add the mode argument — typically `TetrationMode::Analytic` for JS-equivalent behavior.

### Added

- `TetrationMode` enum (`Analytic`, `Linear`; defaults to `Analytic`).
- Critical-section interpolation tables (`CRITICAL_HEADERS`, `CRITICAL_SLOG_VALUES`, `CRITICAL_TETR_VALUES`) and the `critical_section` interpolator, ported verbatim from `break_eternity.js@2.1.3`.
- JS-equivalent 100-iteration step-halving refinement loop wrapping `slog_internal`.
- Analytic fractional-height path for `tetrate` (bases ≤ 10): uses `tetrate_critical` instead of `base.pow(fract_height)`.
- Mode propagates through the full `tetrate` → `iteratedlog` → `layer_add_10` → `layer_add` → `slog` recursion chain so the analytic surface reaches every operation.
- Regression suite: analytic slog values vs JS reference, analytic tetrate values vs JS reference, analytic slog ↔ analytic tetrate round-trip to 1e-8, and locks against modes accidentally collapsing into one path.
- Parity fixture: new `tetrate2_5` and `tetrate2_5_linear` ops exercise fractional-height tetrate.

### Fixed

- Several math correctness bugs found by the parity audit:
  - `arithmetic`: additive-inverse collapse used an epsilon tolerance and swallowed legitimate small residuals (e.g., `1 - (1 - 1e-11)` returned 0). Now exact equality.
  - `sqrt`: layer-1 values with negative mag (tiny positives like 1e-20) returned NaN. Now stays at layer 1 with `mag/2`.
  - `atanh`: operator precedence bug — `(x+1)/(1-x).ln()/2` instead of `((1+x)/(1-x)).ln()/2`.
  - `f_gamma` (Stirling series): wrong shift direction (`num += 1.0` should be `num -= 1.0`), wrong pole-detection strict-equality, and four asymptotic coefficient signs flipped.
  - `gamma` layer-0 Stirling path (`x ≥ 24`): missing `-1/(360·t³)` term caused worst-case rel_err ~3e-7. Now matches scipy to ~1e-14.
  - `pow10`: off-by-epsilon at `10^-1` (returned 1.0 instead of 0.1). Also added `±Inf` short-circuits.
  - `exp` at layer 1: sign distribution bug — `exp(huge-negative)` returned huge-negative instead of tiny-positive.
  - `factorial` at layer 1: grouping bug in Stirling formula.
  - `layer_add_10`: ported four JS bugfixes that were missing. `(-3).layer_add_10(1)` now returns `0.001` (was `-1000`); `0.layer_add_10(-2)` now returns `-Infinity` (was an unnormalized sentinel that panicked when displayed); a "very smol" tower (mag < 0, layer > 0) is zeroed before layer-bumping; a sign=0 result with mag=0, layer≥1 is collapsed to layer-1, mag=1 before final normalize.
  - `tetrate(base, +Infinity, payload)`: the lambertw shortcut now branches on base. For `base > 1.444` returns `Infinity` (tower diverges) rather than panicking with `lambertw OutOfDomain`. For `base < 0.066` returns NaN (oscillates without converging). The convergence-zone path `[0.066, 1.444]` still uses lambertw.

### Known gaps (tracked separately)

- Fractional-height tetrate of bases in the convergence zone `(0, 1.444]` uses a special JS code path the Rust port hasn't implemented. Affected inputs are skipped in the parity fixture; tracked in a follow-up issue.

## [0.2.1] - 2026-05-25

### Changed

- Full README rewrite for clarity and scanability. Reorganized into Quick Start, Features table, Constants table, seven Operations tables (arithmetic, powers and roots, logarithms, tetration, special functions, trig and hyperbolics, comparison and clamping), six worked examples (five idle/incremental, one generic factorial), MSRV, Contributing, Acknowledgements (now crediting cozyGalvinism), and License sections.
- Replaced the inherited "Note to bugs" and "Afterword" sections (carried over from the original Rust port) with a short, professional Contributing section.
- README is now the crate-level documentation. `src/lib.rs` uses `#![doc = include_str!("../README.md")]`, so the README becomes the docs.rs landing page in place of the previous thin crate doc-comment. Every Rust block in the README is doctested by `cargo test --doc`.

## [0.2.0] - 2026-05-25

### Breaking

- The `godot` Cargo feature is replaced by two explicit alternatives: `godot3` (transitional, on gdnative 0.11; will be removed in 0.3.0) and `godot4` (active, on gdext 0.5). Users were previously on `godot` via `features = ["godot"]`; migrate to `features = ["godot3"]` if you're on Godot 3, or `features = ["godot4"]` if you're on Godot 4. The transitional `godot3` feature exists only to give downstream code one release cycle to migrate.
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

- `tests/proptest_arithmetic.rs`: property tests for arithmetic identities
  and commutativity via proptest. Properties: Display/parse round-trip,
  addition commutativity, additive identity (`a + 0 == a`), multiplication
  commutativity, multiplicative identity (`a * 1 ≈ a`), multiplication by
  zero, self-subtraction (`a - a == 0`), self-division (`a / a ≈ 1`),
  negation involution (`-(-a) == a`), `abs` sign non-negativity, and
  `ln`/`exp` round-trip for layer-0 positives.
- `tests/parity.rs` + `tests/fixtures/parity.json`: parity tests against
  `break_eternity.js@2.1.3`. The fixture covers 1,966 cases across unary
  (abs, neg, sqrt, ln, log10, log2, exp, gamma, factorial, recip, tetrate,
  slog) and binary (add, sub, mul, div, pow) operations at layer 0–4.
  Generated by `tests/fixtures/generate.mjs`. Pass rate: ~97.9% (2.1%
  known divergences — see below).
- `serde` (derive) and `proptest` added to `[dev-dependencies]`.

### Known parity divergences (from break_eternity.js)

- **gamma / factorial** (~1.0%): The Stirling-series implementation in the
  Rust port produces severely incorrect values for non-integer and small
  arguments (e.g. `gamma(0.5)` gives `10^214` instead of `1.77`). This is
  a pre-existing bug in the ported implementation, not introduced by Phase 6.
- **slog** (~0.7%): The Rust `slog` implementation uses a different formula
  for the fractional part than the JS version, producing a systematic offset
  of roughly `log10(log10(x))` from the expected result.
- **pow edge cases** (3 cases): `a^(-1)` returns `1.0` instead of `1/a` for
  certain layer-0 inputs (e.g. `10^(-1) = 1.0` instead of `0.1`). Pre-existing
  precision/rounding bug.
- **exp sign for large negative layer-1 input** (1 case): `exp(-1e1000)`
  returns a positive layer-2 value instead of the expected negative one.
- **`5e-324` subnormal inputs** (excluded from fixture): The JS implementation
  applies a precision correction (`+3.27e-16`) to the IEEE 754 minimum subnormal
  mantissa that the Rust port does not replicate. These 143 cases were excluded
  from the fixture to avoid systematic false failures on a JS-specific artifact.

- `wasm` feature — exposes `Decimal` to JavaScript via `wasm-bindgen` as a `JsDecimal` class with string-based constructor, arithmetic, and comparison methods.
- `godot4` feature — `GodotConvert`/`FromGodot`/`ToGodot` implementations for the [`godot`](https://crates.io/crates/godot) crate (gdext, Godot 4). `Decimal` round-trips through `GString` via `Display`/`TryFrom<&str>`.
- `godot3` feature — **deprecated** — `FromVariant`/`ToVariant` implementations for [`gdnative`](https://crates.io/crates/gdnative) (Godot 3). Will be removed in 0.3.0.
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
- `repository` and `homepage` in `Cargo.toml` pointed to a non-existent GitHub repo (`break-eternity-rust`); now correctly resolve to `break-eternity-rs`.
- Malformed crates.io badge in `README.md` is now a clickable link with proper alt text.

### Project hygiene

- Edition bump to 2021, MSRV declared as `rust-version = "1.94"`. (The MSRV is set by the optional `godot 0.5.3` / `gdext` dependency, which cargo's resolver picks up even when the `godot4` feature is disabled. The core crate alone — without godot4 — could compile on 1.70+, but cargo's resolution model does not allow declaring a lower MSRV than the highest-required dep in the manifest.)
- Dependency modernization: `lazy_static` → `std::sync::OnceLock`, `custom_error` → `thiserror`, removed unused `num-derive` / `num-traits` / `pad`.
- `authors` field in `Cargo.toml` reduced to GitHub handles only — no personal names or email addresses.
- `src/lib.rs` (~2,530 LOC) split into focused submodules: `arithmetic`, `constants`, `decimal`, `error`, `format`, `parse`, `serde_impl`, `tetration`, `transcendental`, `utils`.
- `documentation` field in `Cargo.toml` linking to docs.rs and a `[package.metadata.docs.rs]` block.
- `docs.rs` and MIT license badges in `README.md`.
- `## Installation` section in `README.md`.
- `CHANGELOG.md` (this file), `SECURITY.md`, `CONTRIBUTING.md`.
- `.github/workflows/ci.yml` — fmt, clippy, test (stable + beta × Linux/macOS/Windows), MSRV check, wasm32 build.
- `.github/workflows/audit.yml` — daily `cargo audit`.
- `.github/dependabot.yml` — weekly cargo + github-actions updates.
- `Patashu` attribution line in `LICENSE`.
- Standard Rust entries added to `.gitignore` (`*.rs.bk`, `*.pdb`, `.idea/`, `.DS_Store`).
- Crate-wide lint gating: `#![warn(clippy::pedantic)]`, `#![deny(unsafe_op_in_unsafe_fn)]`, with a tailored allow list for numeric-library idioms.
- `pub const COMPARE_EPSILON: f64 = 1e-10` hoisted to `constants.rs`; all magic `1e-10` literals in arithmetic and parse logic now reference it.

## [0.1.0] - 2026-05-25

### Added

- Initial fork of [cozyGalvinism/break-eternity](https://github.com/cozyGalvinism/break-eternity) (itself a port of [Patashu/break_eternity.js](https://github.com/Patashu/break_eternity.js)) published to crates.io as `break-eternity-rs`.
- Public getters and setters for `Decimal` components.

### Notes

- **Known issue (fixed in 0.2.0)**: published `repository` and `homepage` URLs link to a non-existent GitHub repo.
- **Known issue (fixed in 0.2.0)**: published `authors` field exposed a personal email address.

[Unreleased]: https://github.com/MaddisonM79/break-eternity-rs/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/MaddisonM79/break-eternity-rs/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/MaddisonM79/break-eternity-rs/releases/tag/v0.1.0
