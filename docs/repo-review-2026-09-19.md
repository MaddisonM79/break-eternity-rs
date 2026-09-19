# break-eternity-rs review: gaps, bugs, and drift

> **Status (2026-09-20):** every item below was addressed in the 0.5.0 release on the
> `develop` branch, except the `no_std` port and the `Decimal` size reduction, which were left
> as future work. See `CHANGELOG.md` for the itemised list. This document is kept as the
> record of what was found and why.

Reviewed at `develop` @ `8055226` (v0.4.0, published 2026-06-03). Compared against
`break_eternity.js` 2.1.3 (upstream `master` is identical to the 2.1.3 tag; nothing newer
exists) and `cozyGalvinism/break-eternity` 0.2.0 (the fork parent).

Method: read every source file, ran `fmt`/`clippy`/`test`/`doc`/`publish --dry-run`
locally (all clean, parity 2002/2002), then wrote throwaway probe tests against edge
cases the parity fixture does not cover. Every value quoted below was observed on this
commit. Upstream and parent API inventories were pulled from their sources.

## 1. Correctness bugs (verified)

Ordered by blast radius for an incremental game.

### High

| # | What | Repro on 0.4.0 | Expected | Where |
|---|------|----------------|----------|-------|
| B1 | `pow` uses `COMPARE_EPSILON` to detect base == 1 and exponent == 1, so anything within 1e-10 of 1 short-circuits. Compounding multipliers near 1 are the bread and butter of idle games. Same bug class as the additive-inverse fix in 0.3.0. | `from_finite(1.0 + 1e-11).pow(1e15)` = `1.00000000001` (true value about `10^4343`). `7.pow(1 + 1e-11)` = `7`. | Exact equality like JS. | `transcendental.rs:324-333` |
| B2 | Infinity has no canonical representation, so every operation on it is wrong in a different way. `neg_inf()` is `(-1, 0, -inf)` but `-inf()` is `(-1, 0, +inf)`, and they compare unequal. `normalize()` never handles non-finite `mag`. | `inf() > 1e1000` is `false`. `1e1000.max(inf())` = `1e1000`. `neg_inf() < -1e1000` is `false`. `neg_inf().to_number()` = `+inf`. `neg_inf().abs()` and `inf() * 2` **panic on Display** (index overflow in `power_of_10`). `inf().floor()`, `inf().ln()`, `from_components(1, 0, INFINITY)` display `NaNeinf`. `inf().ln().recip()` displays `ee-inf`. `inf().slog()` = `-31.07`. `checked_sub(inf, inf)` = `Ok(inf)`, `checked_div(inf, inf)` = `Ok(0)`, `checked_mul(inf, 0)` = `Ok(0)`. | JS 2.x: sign carries direction, `layer = mag = +inf`; `cmp`, `Display`, `to_number` special-case it; `inf - inf`, `inf * 0`, `inf / inf` are NaN. `error.rs` already documents `Undefined` for `+inf + -inf` but never emits it. | `decimal.rs:411-416`, `cmpabs`, `normalize`, `format.rs` |
| B3 | Layer arithmetic silently saturates instead of overflowing to infinity. `height as i64` and `layer + n` clamp at `i64::MAX`. | `ten().tetrate(1e19)` and `ten().tetrate(1e300)` both = `(e^9223372036854775805)10000000000`. `ten().checked_pentate(2.5)` same. `ten().tetrate(1e16)` = layer `9999999999999998`, above JS's `dLayerSafeMax` of 9e15. | Overflow to `inf()` or `Err(ArithmeticErrorKind::Overflow)` past layer 9e15. | `tetration.rs:112-125` |
| B4 | Gamma poles panic in debug and return garbage in release. `from_finite(-inf)` trips the debug assert. | `checked_gamma(0)`, `checked_gamma(-1)`, `checked_gamma(-2)`, `checked_factorial(-1)` all panic. | `Err(OutOfDomain)`. | `transcendental.rs:427-505` |
| B5 | Negative base with non-integer exponent silently returns the positive magnitude in the unchecked path. `cbrt` and `root` inherit it. | `from(-8).cbrt()` = `+2`. `from(-2).checked_tetrate(2.5)` = `Ok(6.72)`. | JS 2.1.1: `cbrt(-8)` = `-2`, odd integer roots of negatives are negative, everything else NaN. `checked_pow` already errors correctly; `pow` and `checked_tetrate` do not. | `transcendental.rs:335-339`, `decimal.rs:671-680` |

### Medium

| # | What | Repro | Where |
|---|------|-------|-------|
| B6 | `to_fixed` and `to_precision` drop the sign at layer 0 (they format `self.mag`, not `sign * mag`). | `from(-5).to_fixed(2)` = `"5.00"`, `.to_precision(3)` = `"5.00"`. | `format.rs:38-41` |
| B7 | `to_precision` casts a negative exponent to `usize`, which saturates to 0. | `from_finite(0.00123).to_precision(2)` = `"0.0"` (want `"0.0012"`). | `format.rs:57` |
| B8 | `to_precision(x, 0)` panics with subtract-with-overflow. | any value, `places = 0` | `format.rs:53` |
| B9 | Exponent gets decimal places in fixed/precision output at layer >= 1. | `"1.5e100".to_fixed(2)` = `"1.50e100.00"`; `"ee100.5".to_string_with_decimal_places(2)` = `"ee100.50"`. JS gives `1.50e100` / `ee100.5`. | `format.rs:88-125` |
| B10 | `minimum()` is `(1, 0, f64::MIN)`, an invalid state. README says "smallest safe positive magnitude". | `minimum()` displays `-1.7976931348623157eNaN` and `minimum() > zero()` is `true`. | `decimal.rs:426` |
| B11 | `ceil`/`floor` of sub-1 magnitudes at layer >= 1 return 0. JS 1.4 fixed this. | `from_finite(1e-20).ceil()` = `0` (want 1). `from_finite(-1e-20).floor()` = `0` (want -1). | `decimal.rs:444-468` |
| B12 | The parser panics on a negative `(e^N)` layer. `SECURITY.md` puts save-file parsing of untrusted input in scope, so a panic here is a policy violation, not just a bug. Fractional layers are silently truncated. | `"(e^-1)5".parse()` panics. `"(e^1.5)5"` = `100000` (JS: `tetrate(10, 1.5, 5)`). | `parse.rs`, `(e^N)M` branch |
| B13 | `checked_*` leaks NaN or panics. | `checked_tetrate(2, -2)` = `Ok(mag: NaN)`. `zero().recip()` = NaN sentinel (there is no `checked_recip`). `checked_slog(x, base 1)` panics. `checked_ssqrt(-1)` panics. `checked_pow(0, -1)` = `Ok(0)`. `acosh(0.5)` = `-0.69` (domain is x >= 1). `asin(2)` displays `NaN`; no checked trig exists. | `tetration.rs`, `decimal.rs` |
| B14 | Modulo loses precision because it always goes through `lhs - floor(lhs / rhs) * rhs`. JS uses f64 `%` when both operands fit. | `"1e20" % 7` = `0` (true answer 2). | `arithmetic.rs:195-208` |
| B15 | `from_components` does not validate `sign` or `layer`, so the `unreachable!("sign invariant violated")` in `Ord::cmp` is reachable from safe public code. | `from_components(5, 0, 5.0)` keeps sign 5; `from_components(1, -1, 5.0)` keeps layer -1. | `decimal.rs:115` |

### Low / parity nits

- B16 `ssqrt` still uses the pre-1.4 `ln(x) / W(ln x)` formula. JS >= 1.4 uses `linear_sroot(2)`. `checked_ssqrt(0.5)` errors where JS returns a value.
- B17 `2^10` = `1024.0000000000002`, `(-2)^3` = `-7.999999999999999`. Identical to JS, but a layer-0 `powf` fast path would make the ubiquitous `base * ratio^n` cost formula exact. Worth doing even though it is a deliberate divergence.
- B18 `pentate` bails after 10 iterations (old JS). 2.1 uses a 10000 cap with convergence and 2-cycle exits.
- B19 README lists `neg` as a method; only the `Neg` trait impl exists, so `x.neg()` needs `use std::ops::Neg`.
- B20 `1e15 - 0.5` = `999999999999999.5` and `9.5e15 + 1` = `9.499999999999996e15`: expected precision loss at the layer boundary, same as JS. Documented here only so nobody files it as a bug.

## 2. Missing relative to break_eternity.js 2.1.3

Upstream is frozen at 2.1.3 (2025-12-07). The port's 2.1.3 pin is current. These are the
items 2.1.3 has that the port does not.

**Math**
- `lambertw(principal = false)`: the W₋₁ branch on both the f64 and Decimal paths. This is the prerequisite for issue #18 (convergence-zone tetrate), for `excess_slog`, and for the small-base branch of `layer_add`. Without it, `tetrate(base, +inf)` only ever finds the lower fixed point.
- Convergence-zone tetrate for bases in (0, 1.4447] with fractional height (open #18). Sized: (a) the branch itself with its `linear`/`height > 10000` gating and the odd-height 2-cycle fix-up, (b) the geometric-mean fractional start for base < 1, (c) `excess_slog` with its three ranges plus `layer_add`'s small-base path, (d) W₋₁, (e) `slog_internal`'s `infTower` checks. The parity generator currently skips these inputs (`skipTetrFrac`).
- `linear_sroot(degree)`, `penta_log`, `linear_penta_root`, `increasing_inverse` (all 2.1.0).
- `mod(value, floored)`: floored modulo variant.
- `pow_base`, `p_log10`.
- `normalize()` rule: `mag == -inf && layer > 0` collapses to zero (JS rule 1). Related to B2.
- 2.1.1 `root`/`cbrt` of negatives (B5).

**Comparison**
- `cmp_tolerance`, `neq_tolerance`, `lt_tolerance`, `lte_tolerance`, `gt_tolerance`, `gte_tolerance`. Only `approx_eq` exists.

**Constants**
- `dLayerSafeMax`/`dLayerSafeMin` (layer 9e15) and `dLayerMax`/`dLayerMin` (2.1.0). Useful as the overflow boundary for B3.
- `dNumberMin` = 5e-324. The port's `minimum()` is broken (B10).
- `dNaN` is intentionally absent. Keep it that way.

**Parser (`fromString`)**. The README's accepted-format table promises several of these and they fail today.
- `eX` with empty mantissa: `"e3"` errors (README says `eX === 10^X`; `"ee3"` works).
- `N PT X` / `N PT (X)` with spaces: `"3 PT 2"` errors; `"3PT2"` works. Trim the split parts.
- `X^^N;Y` and `X^^^N;Y` payload forms: `"10^^3;2"` errors.
- `XFN` / `FN` format (1.4.x).
- `(e^N)X` with negative or fractional N routes to `tetrate(10, N, X)` in JS (B12).
- `linearhyper4` parameter: the port hardcodes `Analytic` inside the parser.
- Already fine: 2.1.2 subnormal guard (`"2.47e-324"` parses correctly, better than pre-2.1.2 JS), 2.1.3 `Infinity`/`-Infinity`, comma stripping. Rejecting `NaN` and returning `Err` on garbage (JS returns 0) are deliberate improvements; keep them.
- Skip: the `fromStringCache` LRU. It exists to avoid JS allocation churn and buys nothing in Rust.

**Formatting**
- `to_exponential(places)`. `LowerExp`/`UpperExp` cover most of it but the JS-named entry point is absent.
- `toJSON`/`valueOf`: covered by `Display` and `serde`.

**Game helpers**. For a crate whose pitch is incremental games, this is the most visible gap.
- `afford_geometric_series`, `sum_geometric_series`, `afford_arithmetic_series`, `sum_arithmetic_series`, `efficiency_of_purchase` (plus the `_core` Decimal-typed forms).
- `randomDecimalForTesting`: worth turning into a `proptest` strategy exposed behind a `proptest` feature or kept in `tests/`.

**Rust-idiom gaps** (not upstream items, but expected of a numeric crate)
- `Sum`/`Product` impls, `From<Decimal> for f64` or `TryFrom`, `PartialEq<f64>`/`PartialOrd<f64>`, `From<i128 / u128 / isize / usize>`.
- `checked_recip`, `checked_asin`/`acos`/`acosh`/`atanh`.
- `no_std` + `alloc` + `libm`: moderate effort, would unlock bare wasm and embedded targets. Not urgent.
- `Decimal` is 24 bytes (`i8` + `i64` + `f64` with padding). Packing `layer` into `i32` would make it 16 bytes. Perf note only; measure first (there are no benches).

## 3. Parent crate: cozyGalvinism/break-eternity

Nothing to pull. Latest is 0.2.0 (2024-05-20), 15 commits in its whole history, no activity
in 16 months, no CI, no changelog, tracks 2021-era JS. The fork already carries everything it
has (the 0.2.0 getters/setters included) except items removed on purpose: `set_sign`/
`set_layer`/`set_mag`, `nan()`, `from_components_no_normalize`, and the `godot` (gdnative)
feature. The parent's tolerant `PartialEq` alongside bitwise `Hash` was a contract violation
that 0.2.0 of this fork fixed. Consider asking cozyGalvinism to point their README and
crates.io description at this fork.

## 4. Documentation and metadata drift

- **README**: install snippets pin `0.2` and `0.3`; `godot3` row says "scheduled for removal in 0.3.0"; `neg` listed as a method (B19); string-format table promises `eX`, `N PT X`, `N PT (X)`, `X^^N;Y`, `X^^^N;Y` (all fail, see section 2); `minimum()` described as smallest positive (B10); "Creating a Decimal" does not mention `from_string`/`FromStr` from 0.4.0; no trig precision note (open #15).
- **CHANGELOG**: link footer only has 0.2.0 and 0.1.0, and `[Unreleased]` compares `v0.2.0...HEAD`. The 0.2.0 "Known parity divergences" block still reads as current ("pass rate ~97.9%"); it is now 2002/2002. A one-line note under 0.3.0 or 0.4.0 that the fixture is fully green would stop readers assuming gamma is still broken.
- **Cargo.toml**: `godot3` comment says "will be removed in 0.3.0". `rust-version = "1.94"` is still correct: `godot` 0.5.5 (2026-08-09) also requires 1.94.
- **SECURITY.md**: supported-versions table says `0.1.x`.
- **CONTRIBUTING.md**: `cargo test --features godot # Godot 3 (gdnative); will be renamed to godot4` refers to a feature that no longer exists; example tags are `v0.1.1`.
- The parity docs say `npm install break_eternity.js` (unpinned). Pin `break_eternity.js@2.1.3` so a regenerated fixture is reproducible.

## 5. CI, tests, and repo hygiene

- **Four Dependabot PRs open since June to August**: #24 (`crates-io-auth-action` 1.0.5), #27 (`actions/checkout` 7.0.1), #28 (`dtolnay/rust-toolchain`), #29 (`rust-cache` 2.9.2). `checkout` v7 tightens `pull_request_target` defaults, which this repo does not use. All four are safe to merge.
- **Feature code has zero tests.** CI only *builds* `wasm`, `godot3`, `godot4`; nothing exercises `wasm.rs`, `godot_impl.rs`, or `gdnative_impl.rs`. Add a `wasm-bindgen-test` job (node) and, if gdext allows `GString` without a live engine, a round-trip test for the Godot conversions.
- **Parity fixture is narrow and its gate is loose.** The threshold is "fewer than 5% failures" while reality is 0/2002; tighten to zero with an explicit allowlist so regressions fail. The generator covers no `mod`, no `pow` with negative or fractional exponents, no tetrate with payload, no pentate, no trig, no floor/ceil/round, no `toFixed`/`toPrecision`/`toString`, no infinities. B1, B5, B6, B7, B9, B11, B14 would all have been caught by extending `generate.mjs`.
- **Property tests could carry more weight.** `proptest_arithmetic.rs` has no shrinking-friendly coverage of `pow`/`tetrate` monotonicity, `ln`/`exp` beyond layer 0, or `slog`/`tetrate` round-trips. A `Decimal` strategy modelled on `randomDecimalForTesting` would make these cheap.
- **No `examples/`, no `benches/`.** Speed is the crate's pitch and there is not a single number anywhere. A criterion bench for `mul`/`add`/`pow`/`tetrate`/`Display`/parse at layers 0 to 3 would take an hour and would also tell you whether the 24-byte layout matters.
- **`godot3` / gdnative 0.11.3** (last release January 2023, unmaintained). It was promised removal in 0.3.0. Drop it in the next breaking release.
- Missing tooling worth adding: `cargo-semver-checks` in CI (given how much the 0.x API has moved), `cargo-deny` (licenses/bans; `cargo audit` already runs), issue and PR templates, `CODEOWNERS`.
- Open issues: #15 is a ten-minute docs change; #18 is blocked on W₋₁ (section 2).

## 6. Suggested sequencing

**0.4.1 (patch, no API change)**
B1, B4, B5 (`cbrt`/`root` only; leave `pow` semantics for 0.5), B6, B7, B8, B9, B10, B11, B12, B14, B16, B19. Parser fixes for `eX`, spaced `PT`, and `;payload`. All of section 4. Merge the Dependabot PRs. Pin the fixture, tighten the parity gate, extend `generate.mjs` to cover what 0.4.1 fixes. Close #15.

**0.5.0 (breaking)**
B2 canonical infinity with `Ord`/`Display`/`to_number`/`Undefined` errors (changes `Debug` output and some `checked_*` results). B3 layer overflow to `inf()` with a new `ArithmeticErrorKind::Overflow`. B13 (`checked_recip`, checked trig, no NaN out of `checked_*`). B15 `from_components` validation. `pow` returns an error or NaN for negative base with non-integer exponent. Drop `godot3`. Layer-0 `powf` fast path (B17) with a changelog note that it diverges from JS by being exact.

**0.6.0 (features)**
W₋₁ and #18 with `excess_slog`. `linear_sroot`, `penta_log`, `linear_penta_root`, `increasing_inverse`. Floored `mod`. Tolerance comparisons. Game helpers. `to_exponential`. `Sum`/`Product`/`From` impls. Wider wasm surface plus tests. Benches. `no_std` if there is demand.
