# break-eternity-rs

[![crates.io](https://img.shields.io/crates/v/break-eternity-rs.svg)](https://crates.io/crates/break-eternity-rs)
[![docs.rs](https://img.shields.io/docsrs/break-eternity-rs)](https://docs.rs/break-eternity-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.94-blue.svg)](https://www.rust-lang.org)

A Rust numerical library for representing numbers from `10^^9e15` down to `10^-(10^^9e15)`. Built for incremental and idle games, where speed matters more than perfect precision and `f64` runs out somewhere around floor 7 of the upgrade tree.

It is a port of [`break_eternity.js`](https://github.com/Patashu/break_eternity.js) 2.1.3 and is parity-tested against it on more than twelve thousand cases.

## Quick start

```sh
cargo add break-eternity-rs
```

```rust
use break_eternity::Decimal;

// Compound growth past the edge of f64.
let mut points = Decimal::one();
let growth = Decimal::from_finite(1.05);

for _ in 0..20_000 {
    points *= growth;
}

assert!(points > Decimal::maximum());
println!("{points}");
```

`points` here is roughly `10^424` — well past anything `f64` can hold, computed in well under a second.

Or, to pin manually instead of `cargo add`:

```toml
[dependencies]
break-eternity-rs = "0.5"
```

## Features

Only `std` is on by default. Enable the others in `Cargo.toml`:

```toml
[dependencies]
break-eternity-rs = { version = "0.5", features = ["serde"] }
```

| Flag | Adds | Notes |
|------|------|-------|
| `std` (default) | Standard-library float math | Turn it off for `no_std`; see below. |
| `libm` | Pure-Rust float math from [`libm`](https://crates.io/crates/libm) | Required when `std` is off. Ignored when `std` is on. |
| `serde` | `Serialize` / `Deserialize` (string-based) | Round-trips through `Display` / `FromStr`. |
| `godot4` | `GodotConvert` / `FromGodot` / `ToGodot` for [`godot`](https://crates.io/crates/godot) | Godot 4 / gdext. Round-trip via `GString`. |
| `wasm` | `JsDecimal` class via [`wasm-bindgen`](https://crates.io/crates/wasm-bindgen) | Exposes `Decimal` to JavaScript with a `break_eternity.js`-like method surface. |

Godot 3 (`gdnative`) support was removed in 0.5.0.

### `no_std`

The crate needs `alloc` (for parsing and formatting) but not `std`:

```toml
[dependencies]
break-eternity-rs = { version = "0.5", default-features = false, features = ["libm"] }
```

Every operation is available and behaves identically; the only difference is that the
transcendental functions come from `libm` instead of the platform's C library, so results can
differ from a `std` build in the last bit. `serde` works without `std` too. `wasm` and `godot4`
imply `std`.

## Creating a `Decimal`

Integers convert with `From`; floats with `TryFrom` (NaN and infinities are rejected). Going back, `TryFrom<Decimal>` for every integer type is exact (`NotInteger` / `Overflow` on failure) and the `to_*_saturating` methods truncate and clamp:

```rust
use break_eternity::{ArithmeticErrorKind, Decimal};

assert_eq!(u32::try_from(Decimal::from(42)).unwrap(), 42);
assert_eq!(i8::try_from(Decimal::from(300)).unwrap_err().kind, ArithmeticErrorKind::Overflow);
assert_eq!(Decimal::from_finite(7.9).to_u64_saturating(), 7);
assert_eq!(Decimal::try_from("1e100").unwrap().to_i64_saturating(), i64::MAX);
```

```rust
use break_eternity::Decimal;

// Infallible — finite f64 only (debug-asserted).
let a = Decimal::from_finite(1.5);

// Fallible — rejects NaN / ±∞.
let b = Decimal::try_from(f64::INFINITY); // Err(ArithmeticError { .. })

// From a string, three equivalent ways.
let c: Decimal = "1.234e567".parse().unwrap();
let c2 = Decimal::from_string("1.234e567").unwrap();
let c3: Decimal = "1.234e567".try_into().unwrap();

// From components (auto-normalized).
let d = Decimal::from_components(1, 2, 30.0);
let e = Decimal::from_mantissa_exponent(1.234, 567.0);

// Any signed or unsigned integer via `From`.
let f = Decimal::from(42_i32);
let g = Decimal::from(u128::MAX);
# let _ = (a, b, c, c2, c3, d, e, f, g);
```

Fields are private. Use the accessors `sign()`, `layer()`, `mag()`, `mantissa()`, and `exponent()` to read state, and `is_zero()`, `is_positive()`, `is_negative()`, `is_finite()`, `is_infinite()` for quick predicates. `Decimal` is `Copy + Clone`, so values can flow through expressions without explicit `.clone()` calls.

### Common constants

| Constructor | Value |
|-------------|-------|
| `Decimal::zero()` | `0` |
| `Decimal::one()` | `1` |
| `Decimal::neg_one()` | `-1` |
| `Decimal::two()` | `2` |
| `Decimal::ten()` | `10` |
| `Decimal::inf()` | `+∞` |
| `Decimal::neg_inf()` | `-∞` |
| `Decimal::maximum()` | `f64::MAX` lifted into `Decimal` |
| `Decimal::minimum()` | `f64::MIN_POSITIVE` lifted into `Decimal` |
| `Decimal::layer_safe_max()` | largest finite value, about `10^^9e15` |
| `Decimal::layer_safe_min()` | smallest positive value, about `1 / 10^^9e15` |

### Accepted string formats

Letters are case-insensitive, surrounding whitespace is trimmed, and commas are ignored.

```plain
M             === M                        (also "1,000,000", ".5", "+5")
eX            === 10^X
MeX           === M*10^X                   (subnormals like "2.47e-324" keep their digits)
eXeY          === 10^(XeY)
MeXeY         === M*10^(XeY)
eeX           === 10^10^X
eeXeY         === 10^10^(XeY)
eeeX          === 10^10^10^X
eeee... (N es) X         === 10^10^10^ ... (N 10^s) X
(e^N)X        === 10^10^10^ ... (N 10^s) X   (negative or fractional N means 10^^N with payload X)
N PT X        === 10^10^10^ ... (N 10^s) X
N PT (X)      === 10^10^10^ ... (N 10^s) X
NpX           === 10^10^10^ ... (N 10^s) X
XfN           === 10^10^10^ ... (N 10^s) X
X^Y           === X^Y
X^^N          === X^X^X^ ... (N X^s) 1
X^^N;Y        === X^X^X^ ... (N X^s) Y
X^^^N         === X^^X^^X^^ ... (N X^^s) 1
X^^^N;Y       === X^^X^^X^^ ... (N X^^s) Y
Infinity, -Infinity, inf, -inf
```

The parser never panics. Malformed input is `Err(BreakEternityError::ParseError)`; syntactically valid input with no defined value (such as `"(-2)^^2.5"`) is `Err(BreakEternityError::ParseUndefined)`. `"NaN"` is rejected because NaN is not a representable `Decimal`. Fractional tetration heights in a literal use the analytic approximation; use `Decimal::from_string_with_mode` to choose.

## Operations

The standard arithmetic operators (`+`, `-`, `*`, `/`, `%`) work on owned and borrowed `Decimal` values in every combination, plus the compound-assign forms (`+=` etc.). Primitives work on either side: `Decimal::from(2) * 3` and `1.5 + Decimal::one()` both compile, and so do `Decimal::from(2) == 2.0` and `Decimal::from(2) < 3_u8`. Iterators of `Decimal` (or `&Decimal`) support `.sum()` and `.product()`.

The tables below show the method form of each operation along with its checked variant (where one exists). Methods listed without a checked variant are infallible by construction.

### Arithmetic and sign

| Op | Method | Checked |
|----|--------|---------|
| `+` | `add` | `checked_add` |
| `-` | `sub` | `checked_sub` |
| `*` | `mul` | `checked_mul` |
| `/` | `div` | `checked_div` |
| `%` (truncated) | `rem` | `checked_rem` |
| floored modulo | `rem_floored` | `checked_rem_floored` |
| `-x` | unary `-` (`std::ops::Neg`) | — |
| `\|x\|` | `abs` | — |
| `1/x` | `recip` | `checked_recip` |

### Powers and roots

| Method | Description | Checked |
|--------|-------------|---------|
| `pow(exp)` | aᵇ (exact `powf` for ordinary floats; negative bases need integer exponents) | `checked_pow` |
| `pow_base(base)` | baseᵃ | `checked_pow_base` |
| `sqr()` | a² | — |
| `cube()` | a³ | — |
| `sqrt()` | √a | `checked_sqrt` |
| `cbrt()` | ∛a (real for negative inputs) | — |
| `root(n)` | n-th root (odd integer roots of negatives are real) | `checked_root` |
| `exp()` | eᵃ | — |
| `pow10()` | 10ᵃ | — |

### Logarithms

| Method | Description | Checked |
|--------|-------------|---------|
| `ln()` | natural log | `checked_ln` |
| `log10()` | base-10 log | `checked_log10` |
| `log2()` | base-2 log | `checked_log2` |
| `log(base)` | log to arbitrary base | `checked_log` |
| `abs_log10()` | log10 of the absolute value | `checked_abs_log10` |
| `p_log10()` | log10, but `0` for zero or negative input; never fails | — |

### Tetration and beyond

All tetration-family methods take a `mode: TetrationMode` argument. `TetrationMode::Analytic` (the default) matches `break_eternity.js`'s critical-section interpolation for bases ≤ 10, rescaled for bases between `e^(1/e)` and 2. `TetrationMode::Linear` uses the older closed-form approximation. Bases > 10 always fall back to linear regardless of mode, and the super-root and penta-root functions always use the linear approximation (there is no consistent analytic one).

| Method | Description | Checked |
|--------|-------------|---------|
| `tetrate(height, payload, mode)` | a^^n with optional residual payload; supports fractional, negative, and infinite heights and bases in the convergence zone `(0, e^(1/e)]` | `checked_tetrate` |
| `iteratedlog(base, n, mode)` | iterated logarithm | `checked_iteratedlog` |
| `slog(base, mode)` | super-logarithm | `checked_slog` |
| `layer_add(diff, base, mode)` | shift the value's slog by `diff` | `checked_layer_add` |
| `layer_add_10(diff, mode)` | shift internal layer (base 10) | `checked_layer_add_10` |
| `ssqrt()` | super square root (inverse of a^^2) | `checked_ssqrt` |
| `linear_sroot(degree)` | super-root of any degree | `checked_linear_sroot` |
| `pentate(height, payload, mode)` | a^^^n | `checked_pentate` |
| `penta_log(base, mode)` | penta-logarithm | `checked_penta_log` |
| `linear_penta_root(degree)` | penta-root | `checked_linear_penta_root` |

`InverseSearch` wraps any strictly monotone `Decimal -> Decimal` function and inverts it numerically across the whole representable range (the port of `increasingInverse`).

### Special functions

| Method | Description | Checked |
|--------|-------------|---------|
| `gamma()` | Γ(a); exact for integers up to 171 | `checked_gamma` |
| `factorial()` | a! (via gamma) | `checked_factorial` |
| `ln_gamma()` | ln(Γ(a)) | `checked_ln_gamma` |
| `lambertw()` | Lambert W, principal branch `W₀` | `checked_lambertw` |
| `lambertw_branch(LambertBranch)` | `W₀` or the real non-principal branch `W₋₁` | `checked_lambertw_branch` |

### Trigonometry and hyperbolics

| Method | Notes |
|--------|-------|
| `sin` / `cos` / `tan` | Standard trig at layer 0. For inputs at layer ≥ 1 the angle modulo 2π cannot be resolved in `f64`, so these return `0` rather than a pseudo-random phase (same as `break_eternity.js`). |
| `asin` / `acos` / `atan` | Inverse trig; `asin`/`acos` have `checked_` forms for out-of-range inputs, `atan` saturates to ±π/2. |
| `sinh` / `cosh` / `tanh` | Hyperbolic. |
| `asinh` / `acosh` / `atanh` | Inverse hyperbolic; `acosh`/`atanh` have `checked_` forms. |

### Rounding, comparison, clamping

| Method | Description |
|--------|-------------|
| `round` / `floor` / `ceil` / `trunc` / `fract` | Standard rounding modes (`ceil(1e-20)` is `1`, `floor(-2.5)` is `-3`). |
| `round_to_places` / `floor_to_places` / `ceil_to_places` / `trunc_to_places` | Rounding at a decimal place, returning a `Decimal` (`19.995.round_to_places(2)` is `20`; negative places round to tens, hundreds, ...). |
| `round_to_significant` | Rounding to significant figures at any layer (`1.23456e100` at 3 digits is `1.23e100`). |
| `is_integer` | `true` for finite whole numbers (everything at layer 1 and above with positive `mag`). |
| `cmp` / `cmpabs` | `std::cmp::Ordering`, signed and by magnitude. |
| `max` / `min` / `maxabs` / `minabs` | Pairwise selection. |
| `clamp` / `clamp_min` / `clamp_max` | Range clamping. |
| `approx_eq` / `approx_ne` / `approx_lt` / `approx_le` / `approx_gt` / `approx_ge` / `cmp_tolerance` | Tolerance-based comparisons. |

### Game helpers

Associated functions for the two cost curves every incremental game ends up with:

| Function | Answers |
|----------|---------|
| `afford_geometric_series(resources, start, ratio, owned)` | how many items can I buy when each costs `ratio` times the last? |
| `sum_geometric_series(n, start, ratio, owned)` | what do the next `n` of those cost in total? |
| `afford_arithmetic_series(resources, start, add, owned)` | how many when each costs `add` more than the last? |
| `sum_arithmetic_series(n, start, add, owned)` | what do the next `n` of those cost? |
| `efficiency_of_purchase(cost, current_rps, delta_rps)` | Frozen-Cookies efficiency score; lower is better. |

Each has a `checked_` twin for degenerate inputs. See `examples/idle_loop.rs`.

## Examples

### Currency growing each tick

A typical idle-game inner loop: add some base income, then apply a multiplier.

```rust
use break_eternity::Decimal;

let mut money = Decimal::from(10_i32);
let per_tick = Decimal::try_from("1.5e10").unwrap();
let multiplier = Decimal::from_finite(1.0001);

for _ in 0..50_000 {
    money = (money + per_tick) * multiplier;
}

assert!(money > Decimal::try_from("1e15").unwrap());
println!("After 50k ticks: {money}");
```

### Upgrade unlock at a threshold

`Decimal` implements `Ord`, so comparisons against an arbitrarily large cost are just `>=`.

```rust
use break_eternity::Decimal;

let cost = Decimal::try_from("1e100").unwrap();
let mut wallet = Decimal::try_from("3e100").unwrap();

if wallet >= cost {
    wallet -= cost;
    println!("Upgrade unlocked. Remaining: {wallet}");
}
```

### Buying in bulk

```rust
use break_eternity::Decimal;

let wallet: Decimal = "1e30".parse().unwrap();
let base_cost = Decimal::from(100);
let ratio = Decimal::from_finite(1.15);
let owned = Decimal::from(40);

let n = Decimal::afford_geometric_series(wallet, base_cost, ratio, owned);
let total = Decimal::sum_geometric_series(n, base_cost, ratio, owned);
assert!(total <= wallet);
println!("can buy {n} more for {}", total.to_precision(4));
```

### Player-facing number formatting

`Display` prints a compact scientific form. `to_fixed`, `to_precision`, and `to_exponential` give a fixed digit count for HUD text, and the `{:e}` / `{:E}` format specifiers honour their precision.

```rust
use break_eternity::Decimal;

let score = Decimal::try_from("3.141592653589793e42").unwrap();

// Layer-1 values are stored as log10, so Display keeps about 14 significant digits.
assert!(score.to_string().starts_with("3.14159265358") && score.to_string().ends_with("e42"));
assert_eq!(score.to_fixed(2), "3.14e42");
assert_eq!(score.to_precision(4), "3.142e42");
assert_eq!(format!("{score:.1e}"), "3.1e42");
assert_eq!(Decimal::from(-5).to_fixed(2), "-5.00");
assert_eq!(Decimal::from_finite(0.00123).to_precision(2), "0.0012");
```

For the notations players actually read, use `to_notation` (or `display`, which writes into a formatter without allocating). All of them print values under 1000 as plain fixed-point numbers, carry rounding across thresholds (`999_999.5` is `1.00 M`, not `1000.00 K`), and switch to `e` + logarithm once an exponent passes `1e9`.

```rust
use break_eternity::{Decimal, Notation};

let gold = Decimal::from_finite(1_234_567_890.0);
assert_eq!(gold.to_notation(Notation::Scientific, 2), "1.23e9");
assert_eq!(gold.to_notation(Notation::Engineering, 2), "1.23e9");
assert_eq!(gold.to_notation(Notation::Standard, 2), "1.23 B");
assert_eq!(gold.to_notation(Notation::Letters, 2), "1.23 c");
assert_eq!(gold.to_notation(Notation::Logarithm, 2), "e9.09");

let tower: Decimal = "10^^3".parse().unwrap(); // 10^10^10
assert_eq!(tower.to_notation(Notation::Scientific, 2), "e1.00e10");
assert_eq!(tower.to_notation(Notation::Logarithm, 2), "ee10.00");
assert_eq!(format!("{}", Decimal::from(42).display(Notation::Standard, 0)), "42");
```

The illion names follow the Antimatter Dimensions scheme (`K`, `M`, `B`, `T`, `Qa`, `Qt`, `Sx`, `Sp`, `Oc`, `No`, `Dc`, `UDc`, ... `Ce`, ... `MI`). `break_eternity::notation::standard_abbreviation` and `letters_abbreviation` expose the suffix tables if you want to lay the mantissa out yourself.

### Saving and loading with `serde`

Enable `features = ["serde"]`. The string form makes the save file human-readable and round-trips exactly.

```rust,ignore
use break_eternity::Decimal;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Save {
    money: Decimal,
    prestige_count: u32,
}

let save = Save {
    money: Decimal::try_from("1e500").unwrap(),
    prestige_count: 7,
};

let json = serde_json::to_string(&save).unwrap();
let back: Save = serde_json::from_str(&json).unwrap();
assert_eq!(save.money, back.money);
```

### Godot 4

Enable `features = ["godot4"]`. `Decimal` crosses the `GDScript` boundary as a `GString`, so you can store and pass it around freely.

```rust,ignore
use break_eternity::Decimal;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init)]
struct Score {
    #[var]
    value: GString,
}

#[godot_api]
impl Score {
    #[func]
    fn add(&mut self, amount: GString) {
        let current: Decimal = self.value.to_string().parse().unwrap();
        let extra: Decimal = amount.to_string().parse().unwrap();
        self.value = (current + extra).to_string().into();
    }
}
```

### A factorial `f64` cannot hold

Outside games, the crate is also a convenient way to evaluate expressions that overflow `f64`.

```rust
use break_eternity::{Decimal, TetrationMode};

let huge = Decimal::from(1000_i32).factorial();
assert!(huge > Decimal::maximum());
println!("1000! = {huge}");

let tower = Decimal::ten().tetrate(Some(4.0), None, TetrationMode::Analytic); // 10^10^10^10
println!("10^^4 = {tower}");
assert!((tower.slog(None, TetrationMode::Analytic).to_number() - 4.0).abs() < 1e-9);
```

## Fallible vs. panicking arithmetic

Undefined results (division by zero, `ln` of a non-positive number, `lambertw` outside its domain, a negative base with a fractional exponent, `inf - inf`, and so on) are surfaced through the type system. Every operation that can fail comes in two flavors:

```rust
use break_eternity::Decimal;
let a = Decimal::from(2_i32);
let b = Decimal::from(3_i32);

// Panicking: matches integer-overflow convention.
let c = a + b;
let d = a.pow(b);

// Fallible: returns Result<Decimal, ArithmeticError>.
let c = a.checked_add(&b).unwrap();
let d = a.checked_pow(&b).unwrap();
# let _ = (c, d);
```

Use the `checked_*` form when accepting untrusted input (save files, user expressions) or when the operands could plausibly produce an undefined value. Use the operator form when arithmetic is well-defined by construction. A `checked_*` method never returns a NaN in disguise, and a panicking form never returns one either.

## Infinity

`Decimal::inf()` and `Decimal::neg_inf()` are first-class values. They order above and below every finite value, survive `Display` / parse round-trips, and follow the usual extended-real rules: `inf + 1 == inf`, `1 / inf == 0`, `inf.ln() == inf`, `(-inf).exp() == 0`. The genuinely undefined combinations (`inf - inf`, `inf * 0`, `inf / inf`, `inf % x`) are `ArithmeticErrorKind::Undefined` from the checked forms and panic from the operators. Any operation whose layer would exceed `MAX_SAFE_LAYER` (9e15) saturates to infinity rather than overflowing.

## Equality

`PartialEq` is **exact**: values are equal iff `sign`, `layer`, and `mag` are bit-identical. `-0.0` is canonicalized to `0.0` so equal-comparing values also hash equal, which makes `Decimal` a valid `HashMap` key.

For tolerance-based comparison use `approx_eq` and friends:

```rust
use break_eternity::Decimal;

let a = Decimal::from_finite(1.0);
let b = Decimal::from_finite(1.0 + 1e-12);
assert!(a != b);                  // exact equality fails
assert!(a.approx_eq(&b, 1e-10));  // tolerance equality holds
assert!(a.approx_le(&b, 1e-10));
```

## Internal representation

A `Decimal` is `sign * 10^10^10^...(layer times) mag`. So a layer-0 number is just `sign * mag`, a layer-1 number is `sign * 10^mag`, a layer-2 number is `sign * 10^10^mag`, and so on.

If `layer > 0` and `mag < 0.0`, the number's exponent is negative — i.e. `sign * 10^-10^10^10^...mag`.

- `sign` is `-1`, `0`, or `1`.
- `layer` is a non-negative integer, at most `MAX_SAFE_LAYER` for finite values.
- `mag` is an `f64`, normalized as follows: if it is above `9e15`, take `log10(mag)` and increment `layer`. If it is below `log10(9e15)` (about 15.954) and `layer > 0`, take `10.0_f64.powf(mag)` and decrement `layer`. At layer 0 the sign is extracted from negative `mag`. Zeroes (`sign == 0 || (mag == 0.0 && layer == 0)`) become `0, 0, 0` in all fields, and an infinite `mag` becomes the canonical infinity.

## Precision and divergences from `break_eternity.js`

The crate follows the upstream algorithms closely enough that `tests/fixtures/parity.json` (12,414 cases generated from `break_eternity.js` 2.1.3) passes with zero mismatches. The remaining, deliberate differences are all cases where the JS answer is a quirk rather than a definition:

- `x mod 0`, `0 ^ negative`, and the zeroth root are errors here; JS returns `0` or `x`.
- Floored modulo of an exact multiple with opposite signs is `0` here; JS returns `±|b|`.
- Square roots and gamma/factorial of negative values above layer 0 are errors here; JS returns sign-mangled values.
- Layer-0 powers use `f64::powf`, so `2^10` is exactly `1024` where JS carries a few ulps of log-domain error.
- Small-integer factorials are exact rather than Stirling approximations.
- Malformed strings are errors rather than `0`, and `"NaN"` is rejected.

## Minimum supported Rust version

The crate is built and tested against `rustc 1.94` (Rust 2021 edition). The MSRV is dictated by the optional `godot` (gdext) dependency; the core crate alone compiles on considerably older toolchains but that is not tested.

## Contributing

Bug reports, math improvements, and PRs are all welcome. The crate is parity-tested against [`break_eternity.js`](https://github.com/Patashu/break_eternity.js); reports of new failing cases — with a JS reference value to compare against — are especially appreciated. See [CONTRIBUTING.md](https://github.com/MaddisonM79/break-eternity-rs/blob/develop/CONTRIBUTING.md) for the workflow and how to regenerate the parity fixture, and [docs/DESIGN.md](https://github.com/MaddisonM79/break-eternity-rs/blob/develop/docs/DESIGN.md) for how the crate is structured.

Issues and pull requests live at [github.com/MaddisonM79/break-eternity-rs](https://github.com/MaddisonM79/break-eternity-rs).

## Acknowledgements

- [Patashu](https://github.com/Patashu) — author of [break_eternity.js](https://github.com/Patashu/break_eternity.js), the original library this work descends from.
- [cozyGalvinism](https://github.com/cozyGalvinism) — wrote [break-eternity](https://github.com/cozyGalvinism/break-eternity), the Rust port this crate is forked from.
- Naruyoko — for [OmegaNum.js](https://github.com/Naruyoko/OmegaNum.js)'s modulo implementation, reused here.

## License

Licensed under the MIT License — see [LICENSE](https://github.com/MaddisonM79/break-eternity-rs/blob/develop/LICENSE).
