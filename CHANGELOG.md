# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `Notation` (`Scientific`, `Engineering`, `Standard`, `Letters`, `Logarithm`) with
  `Decimal::to_notation(notation, places)` and the allocation-free `Decimal::display(..)`
  adapter, plus `notation::standard_abbreviation` / `letters_abbreviation` for custom layouts.
  Illion names follow the Antimatter Dimensions scheme; exponents past `1e9` print as `e` +
  logarithm.
- Rounding that returns a `Decimal`: `round_to_places` / `floor_to_places` / `ceil_to_places` /
  `trunc_to_places` (negative places allowed) and `round_to_significant` (works at layer 1 by
  rounding the mantissa). Also `fract` and `is_integer`.
- Integer interop: `TryFrom<Decimal>` and `TryFrom<&Decimal>` for every primitive integer
  (exact; `ArithmeticErrorKind::NotInteger` / `Overflow`), and `to_i32/u32/i64/u64/i128/u128/usize_saturating`.
- `ArithmeticErrorKind` is now `#[non_exhaustive]` with two new variants, `Overflow` and
  `NotInteger`.
- `proptest` feature: `Arbitrary for Decimal` with `DecimalParams` (zero / negative /
  fractional / infinite / max layer) and the presets `finite_decimal`, `any_decimal`,
  `positive_decimal`, `integer_decimal`, `layer0_decimal` in `break_eternity::strategy`.
- `serde`: binary formats now get the `(sign, layer, mag)` components instead of a string
  (17 bytes in bincode, no parsing on load); human-readable formats keep the string.
  Deserializing from JSON also accepts plain numbers and `[sign, layer, mag]` arrays, and
  parse errors name the offending input. `serde_components` and `serde_string` are
  `#[serde(with)]` adapters that pin one representation. Infinity in component form is
  `(±1, i64::MAX, 0.0)` so JSON can carry it.
- `JsDecimal` gained `toNotation`, `roundToPlaces`, `roundToSignificant`, and `isInteger`.
- `no_std` support. The new `std` feature is on by default; disable it and enable `libm` to
  build for targets without a standard library (`alloc` is still required). The powers-of-ten
  table is now a static array instead of a lazily initialised `Vec`, which also removes an
  atomic load from `to_number()` on the layer-0 fast path.

## [0.5.0] - 2026-09-20

A correctness release. Everything below was found by an audit of the 0.4.0 surface against
`break_eternity.js` 2.1.3 and a set of edge-case probes; the parity fixture was regenerated
against the pinned upstream release and now covers 12,414 cases across 60 operations with a
zero-mismatch gate (see `tests/parity.rs` for the handful of documented, deliberate
divergences).

### Breaking

- **Infinity is canonical.** `Decimal::inf()` / `neg_inf()` are now `(±1, layer > MAX_SAFE_LAYER, +inf)`.
  Previously `neg_inf()` was `(-1, 0, -inf)`, `-inf()` did not equal `neg_inf()`, infinity
  sorted *below* any layer-1 value, `neg_inf().to_number()` was `+inf`, and `neg_inf().abs()`
  and `inf() * 2` panicked when displayed. `layer()` of an infinity is now a large sentinel.
- `inf - inf`, `inf + -inf`, `inf * 0`, `inf / inf`, and any remainder involving an infinity
  are `ArithmeticErrorKind::Undefined` from the `checked_*` methods (and panics from the
  operators). They used to return `inf`, `0`, or an unprintable value.
- Any operation whose layer would exceed `MAX_SAFE_LAYER` (9e15) now saturates to infinity.
  `10.tetrate(1e19)` used to return a value with layer `i64::MAX - 2`.
- `pow` no longer treats bases or exponents within `1e-10` of `1` as exactly `1`.
  `(1 + 1e-11)^1e15` was `1.00000000001`; it is now about `10^4343`. Same fix in `mul`'s
  reciprocal shortcut and `log`'s base-1 check.
- `pow` with a negative base and a non-integer exponent now panics (`checked_pow` returns
  `NegativeBase`) instead of returning the positive magnitude. `cbrt(-8)` is `-2` and
  `root` of a negative number is real for odd integer degrees, matching upstream 2.1.1.
- `0` raised to a negative power is `DivisionByZero` (was `0`).
- Every `checked_*` method now maps the internal NaN sentinel to an error, and every
  panicking method panics instead of returning it. `zero().recip()`, `asin(2)`,
  `acosh(0.5)`, `checked_tetrate(2, -2)`, `checked_slog(x, 1)`, `checked_ssqrt(-1)`,
  `checked_pow(0, -1)` were all leaking NaN or panicking inconsistently.
- `gamma` / `factorial` at the poles (zero and negative integers) return `OutOfDomain`
  from the checked forms and panic from the plain forms; they used to panic in debug and
  return garbage in release.
- `checked_slog` takes the base as a `Decimal` (was `f64`).
- `lambertw()` returns `Decimal` and panics outside its domain like every other unchecked
  method; `checked_lambertw()` returns `Result<Decimal, ArithmeticError>`. The
  `BreakEternityError::LambertWError` and `IterationFailedConverging` variants are gone.
- `from_components` reduces `sign` to its signum and panics on a negative layer; it used to
  store `sign = 5` or `layer = -1` verbatim.
- `minimum()` is `f64::MIN_POSITIVE`; it used to be `(1, 0, f64::MIN)`, an invalid state that
  displayed as `-1.797e…NaN` and compared greater than zero.
- `floor` / `ceil` follow the mathematical definition: `floor(-2.5) == -3`,
  `ceil(-2.5) == -2`, `ceil(1e-20) == 1`, `floor(-1e-20) == -1` (all four were wrong).
- `exponent()` at layer 2 returned `|mag|^10` instead of `10^|mag|`; `set_mantissa` passed
  the layer where the exponent was meant. Both fixed.
- `to_fixed` / `to_precision` keep the sign at layer 0 (`(-5).to_fixed(2)` was `"5.00"`),
  print the exponent as an integer (`"1.50e100"`, not `"1.50e100.00"`), handle negative
  exponents (`0.00123.to_precision(2)` is `"0.0012"`, was `"0.0"`), and clamp `places` to
  `[1, 100]` instead of panicking on `0`.
- Trig functions with a domain (`asin`, `acos`, `acosh`, `atanh`) panic on out-of-range input
  and gained `checked_` forms; `asinh` is evaluated on `|x|` to avoid cancellation.
- `PartialOrd::partial_cmp` returns `None` when either side is the crate-internal NaN
  sentinel (never observable through the public API); `Ord` is unchanged.
- The deprecated `from_number`, `from_mantissa_exponent_no_normalize`, `eq_tolerance`,
  `iteratedexp` and `checked_iteratedexp` are removed.
- The `godot3` feature (gdnative 0.11, Godot 3) is removed as announced in 0.2.0.
- `ArithmeticError` is now `Copy + PartialEq + Eq + Hash` and has a `new` constructor.
- `layer_add` / `layer_add_10` panic on undefined results (they used to return NaN silently);
  `checked_layer_add` / `checked_layer_add_10` added.

### Added

- Ported from `break_eternity.js` 2.1.3:
  - The non-principal Lambert W branch: `lambertw_branch(LambertBranch::NonPrincipal)` and
    `checked_lambertw_branch`.
  - Tetration of bases in the convergence zone `(0, e^(1/e)]` at fractional heights, the
    `excess_slog` machinery behind `layer_add` on those bases, the `< 2` rescale of the
    critical-section table, the two-fixed-point logic for infinite heights, and the 10000
    iteration caps. Closes #18.
  - `linear_sroot(degree)`, `penta_log(base, mode)`, `linear_penta_root(degree)` and their
    `checked_` forms. `ssqrt` now goes through `linear_sroot(2)` (it used the pre-1.4 Lambert
    formula, which fails for `0.5 < x < 1`).
  - `InverseSearch`, the port of `increasingInverse`: numerically inverts any strictly
    monotone `Decimal -> Decimal` function across the whole representable range.
  - `pentate` uses the 2.1 algorithm (convergence and 2-cycle exits, negative heights via
    repeated `slog`, fractional heights via `penta_log`).
  - `rem_floored` / `checked_rem_floored` (floored modulo). Truncated `%` now uses the native
    `f64` remainder when both operands fit, so `1e20 % 7 == 2` (was `0`).
  - Tolerance comparisons `approx_ne`, `approx_lt`, `approx_le`, `approx_gt`, `approx_ge`,
    `cmp_tolerance`.
  - `pow_base` / `checked_pow_base`, `p_log10`, `checked_abs_log10`, `checked_ln_gamma`,
    `checked_recip`, `checked_root`, `layer_safe_max()`, `layer_safe_min()`,
    `MAX_SAFE_LAYER`, `TETRATION_CONVERGENCE_MAX`, `TETRATION_CONVERGENCE_MIN`.
  - Game helpers: `afford_geometric_series`, `sum_geometric_series`,
    `afford_arithmetic_series`, `sum_arithmetic_series`, `efficiency_of_purchase`, each with a
    `checked_` twin.
- Parser: `eX` with an empty mantissa (`"e3"`), spaced `"N PT M"` / `"N PT (M)"`, the
  `X^^N;P` and `X^^^N;P` payload forms, the `XfN` / `fN` shorthand, `(e^N)X` with negative or
  fractional `N`, `inf` / `-inf`, `+5`, and the 2.1.2 subnormal guard. `"(e^-1)5"` used to
  panic. New `Decimal::from_string_with_mode` threads a `TetrationMode` into literal
  tetration, `TryFrom<String>` is implemented, and
  `BreakEternityError::ParseUndefined` distinguishes "valid syntax, undefined value" from
  malformed input. The parser is fuzzed by proptest and never panics.
- `to_exponential(places)`.
- `is_finite`, `is_infinite`, `is_zero`, `is_positive`, `is_negative`.
- Exact `powf` fast path for layer-0 `pow` (`2^10 == 1024`, `10^-1 == 0.1`) and exact
  factorials/gamma for integers up to 171.
- `Sum` / `Product` for `Decimal` and `&Decimal`, `Neg for &Decimal`, `PartialEq` /
  `PartialOrd` between `Decimal` and every primitive numeric type, `From<i128 / u128 / isize /
  usize>`, `AddAssign` and friends with `&Decimal`.
- `wasm`: `JsDecimal` now exposes formatting, component getters, rounding, the log/pow/root
  family, gamma, Lambert W, tetration, `slog`, `ssqrt`, `pentate`, comparisons, clamping, and
  the game helpers, all throwing on undefined results. Tests in `tests/wasm.rs` run under
  `wasm-bindgen-test` in CI.
- `tests/contract.rs`: property tests that every `checked_*` method never panics and never
  returns NaN, every plain method panics exactly when its checked twin errs and otherwise
  agrees with it, and every returned value is normalized and survives a `Display` round-trip.
- `docs/DESIGN.md` describing the representation, the raw / checked / panicking layering, the
  NaN-sentinel comparison rules, and the parity policy.
- Criterion benchmarks (`cargo bench`) and two runnable examples (`idle_loop`, `formatting`).
- CI: `wasm32` test job, `cargo semver-checks`, `cargo deny`, pinned action bumps; issue
  and PR templates; `CODEOWNERS`.

### Fixed

- Trig precision policy documented (closes #15): `sin` / `cos` / `tan` return `0` at
  layer ≥ 1 because the phase is unresolvable in `f64`.
- README, `SECURITY.md`, `CONTRIBUTING.md`, and `Cargo.toml` no longer reference removed
  features, stale version pins, or the "0.3.0" removal date for `godot3`; the changelog
  footer links cover every release.

## [0.4.0] - 2026-06-03

### Added

- `Decimal::from_string(&str) -> Result<Decimal, BreakEternityError>` as the canonical method-style entry point for string parsing.
- `impl FromStr for Decimal`, enabling `"…".parse::<Decimal>()` and the standard `str::FromStr` workflow.

These join the existing `TryFrom<&str>` impl; all three delegate to the same parser, which mirrors `fromStringInternal` from `break_eternity.js` (plain, scientific past f64 range, `eN`/`eeN`/`(e^N)M`, `10^N`/`10^^N`/`10^^^N`, `pt`/`p` tetrate shorthands, `Infinity`/`-Infinity`).

## [0.3.0] - 2026-05-25

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

> Historical: every item in this list was fixed by 0.3.0, and 0.5.0 regenerated the fixture with
> a zero-mismatch gate. Kept for the record.

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

[Unreleased]: https://github.com/MaddisonM79/break-eternity-rs/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/MaddisonM79/break-eternity-rs/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/MaddisonM79/break-eternity-rs/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/MaddisonM79/break-eternity-rs/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/MaddisonM79/break-eternity-rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/MaddisonM79/break-eternity-rs/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/MaddisonM79/break-eternity-rs/releases/tag/v0.1.0
