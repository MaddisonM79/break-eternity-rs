# break-eternity-rs

[![crates.io](https://img.shields.io/crates/v/break-eternity-rs.svg)](https://crates.io/crates/break-eternity-rs)
[![docs.rs](https://img.shields.io/docsrs/break-eternity-rs)](https://docs.rs/break-eternity-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.94-blue.svg)](https://www.rust-lang.org)

A Rust numerical library for representing numbers from `10^^1e308` down to `10^-(10^^1e308)`. Built for incremental and idle games, where speed matters more than perfect precision and `f64` runs out somewhere around floor 7 of the upgrade tree.

## Quick start

```sh
cargo add break-eternity-rs
```

```rust
use break_eternity::Decimal;

// Compound growth past the edge of f64.
let mut points = Decimal::one();
let growth = Decimal::try_from("1.05").unwrap();

for _ in 0..20_000 {
    points *= growth;
}

assert!(points > Decimal::from_finite(f64::MAX));
println!("{points}");
```

`points` here is roughly `10^424` — well past anything `f64` can hold, computed in well under a second.

Or, to pin manually instead of `cargo add`:

```toml
[dependencies]
break-eternity-rs = "0.2"
```

## Features

All features are off by default. Enable them in `Cargo.toml`:

```toml
[dependencies]
break-eternity-rs = { version = "0.2", features = ["serde"] }
```

| Flag | Adds | Notes |
|------|------|-------|
| `serde` | `Serialize` / `Deserialize` (string-based) | Round-trips through `Display` / `TryFrom<&str>`. |
| `godot4` | `GodotConvert` / `FromGodot` / `ToGodot` for [`godot`](https://crates.io/crates/godot) | Godot 4 / gdext. Round-trip via `GString`. |
| `godot3` | `FromVariant` / `ToVariant` for [`gdnative`](https://crates.io/crates/gdnative) | Godot 3. **Deprecated**, scheduled for removal in 0.3.0. |
| `wasm` | `JsDecimal` class via [`wasm-bindgen`](https://crates.io/crates/wasm-bindgen) | Exposes `Decimal` to JavaScript. |

## Creating a `Decimal`

```rust
use break_eternity::Decimal;

// Infallible — finite f64 only (debug-asserted).
let a = Decimal::from_finite(1.5);

// Fallible — rejects NaN / ±∞.
let b = Decimal::try_from(f64::INFINITY); // Err(ArithmeticError { .. })

// From a string.
let c: Decimal = "1.234e567".try_into().unwrap();

// From components (auto-normalized).
let d = Decimal::from_components(1, 2, 30.0);
let e = Decimal::from_mantissa_exponent(1.234, 567.0);

// Any signed or unsigned integer up to 64 bits via `From`.
let f = Decimal::from(42_i32);
# let _ = (a, b, c, d, e, f);
```

Fields are private. Use the accessors `sign()`, `layer()`, `mag()`, `mantissa()`, and `exponent()` to read state. `Decimal` is `Copy + Clone`, so values can flow through expressions without explicit `.clone()` calls.

### Common constants

| Constructor | Value |
|-------------|-------|
| `Decimal::zero()` | `0` |
| `Decimal::one()` | `1` |
| `Decimal::neg_one()` | `-1` |
| `Decimal::two()` | `2` |
| `Decimal::ten()` | `10` |
| `Decimal::inf()` | `+∞` sentinel |
| `Decimal::neg_inf()` | `-∞` sentinel |
| `Decimal::maximum()` | `f64::MAX` lifted into `Decimal` |
| `Decimal::minimum()` | smallest safe positive magnitude |

### Accepted string formats

```plain
M             === M
eX            === 10^X
MeX           === M*10^X
eXeY          === 10^(XeY)
MeXeY         === M*10^(XeY)
eeX           === 10^10^X
eeXeY         === 10^10^(XeY)
eeeX          === 10^10^10^X
eeeXeY        === 10^10^10^(XeY)
eeee... (N es) X         === 10^10^10^ ... (N 10^s) X
(e^N)X        === 10^10^10^ ... (N 10^s) X
N PT X        === 10^10^10^ ... (N 10^s) X
N PT (X)      === 10^10^10^ ... (N 10^s) X
NpX           === 10^10^10^ ... (N 10^s) X
X^Y           === X^Y
X^^N          === X^X^X^ ... (N X^s) 1
X^^N;Y        === X^X^X^ ... (N X^s) Y
X^^^N         === X^^X^^X^^ ... (N X^^s) 1
X^^^N;Y       === X^^X^^X^^ ... (N X^^s) Y
```

## Operations

The standard arithmetic operators (`+`, `-`, `*`, `/`, `%`) work on owned and borrowed `Decimal` values in every combination, plus the compound-assign forms (`+=` etc.). Primitives work on either side: `Decimal::from(2) * 3` and `1.5 + Decimal::one()` both compile.

The tables below show the method-form of each operation along with its checked variant (where one exists). All methods listed without a `checked_*` column are infallible by construction or use a NaN sentinel for domain failures.

### Arithmetic and sign

| Op | Method | Checked |
|----|--------|---------|
| `+` | `add` | `checked_add` |
| `-` | `sub` | `checked_sub` |
| `*` | `mul` | `checked_mul` |
| `/` | `div` | `checked_div` |
| `%` | `rem` | `checked_rem` |
| `-x` | `neg` | — |
| `\|x\|` | `abs` | — |
| `1/x` | `recip` | — |

### Powers and roots

| Method | Description | Checked |
|--------|-------------|---------|
| `pow(exp)` | aᵇ | `checked_pow` |
| `sqr()` | a² | — |
| `cube()` | a³ | — |
| `sqrt()` | √a | `checked_sqrt` |
| `cbrt()` | ∛a | — |
| `root(n)` | n-th root | — |
| `exp()` | eᵃ | — |
| `pow10()` | 10ᵃ | — |

### Logarithms

| Method | Description | Checked |
|--------|-------------|---------|
| `ln()` | natural log | `checked_ln` |
| `log10()` | base-10 log | `checked_log10` |
| `log2()` | base-2 log | `checked_log2` |
| `log(base)` | log to arbitrary base | `checked_log` |

### Tetration and beyond

| Method | Description | Checked |
|--------|-------------|---------|
| `tetrate(height, payload)` | a^^n with optional residual payload | `checked_tetrate` |
| `ssqrt()` | super-square-root (inverse of a^^2) | `checked_ssqrt` |
| `iteratedlog(base, n)` | iterated logarithm | `checked_iteratedlog` |
| `slog(base)` | super-logarithm | `checked_slog` |
| `layer_add(diff, base)` | shift internal layer | — |
| `layer_add_10(diff)` | shift internal layer (base 10) | — |
| `pentate(height, payload)` | a^^^n | `checked_pentate` |

### Special functions

| Method | Description | Checked |
|--------|-------------|---------|
| `gamma()` | Γ(a) | `checked_gamma` |
| `factorial()` | a! (via gamma) | `checked_factorial` |
| `ln_gamma()` | ln(Γ(a)) | — |
| `lambertw()` | Lambert W; returns `Result<Decimal, BreakEternityError>` | `checked_lambertw` |

### Trigonometry and hyperbolics

| Method | Notes |
|--------|-------|
| `sin` / `cos` / `tan` | Standard trig. For very large inputs the phase is meaningless, so these collapse to identity-ish values. |
| `asin` / `acos` / `atan` | Inverse trig. |
| `sinh` / `cosh` / `tanh` | Hyperbolic. |
| `asinh` / `acosh` / `atanh` | Inverse hyperbolic. |

### Rounding, comparison, clamping

| Method | Description |
|--------|-------------|
| `round` / `floor` / `ceil` / `trunc` | Standard rounding modes. |
| `cmp` / `cmpabs` | `std::cmp::Ordering`, signed and by magnitude. |
| `max` / `min` / `maxabs` / `minabs` | Pairwise selection. |
| `clamp` / `clamp_min` / `clamp_max` | Range clamping. |
| `approx_eq(other, tol)` | Tolerance-based equality. |

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

Crank the multiplier or the tick count up a bit and you blow past `f64::MAX` — see the Quick start at the top of this file for a version that does.

### Upgrade unlock at a threshold

`Decimal` implements `PartialOrd`, so comparisons against an arbitrarily large cost are just `>=`.

```rust
use break_eternity::Decimal;

let cost = Decimal::try_from("1e100").unwrap();
let mut wallet = Decimal::try_from("3e100").unwrap();

if wallet >= cost {
    wallet -= cost;
    println!("Upgrade unlocked. Remaining: {wallet}");
}
```

### Player-facing number formatting

`Display` prints a compact scientific form. `to_fixed` and `to_precision` give a fixed digit count for HUD text.

```rust
use break_eternity::Decimal;

let score = Decimal::try_from("3.141592653589793e42").unwrap();

let display = format!("{score}");
let fixed = score.to_fixed(2);
let precision = score.to_precision(4);

assert!(display.contains("e42"));
assert!(fixed.contains("e42"));
assert!(precision.contains("e42"));
```

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
        let current: Decimal = self.value.to_string().as_str().try_into().unwrap();
        let extra: Decimal = amount.to_string().as_str().try_into().unwrap();
        self.value = (current + extra).to_string().into();
    }
}
```

### A factorial `f64` cannot hold

Outside games, the crate is also a convenient way to evaluate expressions that overflow `f64`.

```rust
use break_eternity::Decimal;

let huge = Decimal::from(1000_i32).factorial();
assert!(huge > Decimal::from_finite(f64::MAX));
println!("1000! = {huge}");

let tower = Decimal::ten().tetrate(Some(4.0), None); // 10^10^10^10
println!("10^^4 = {tower}");
```

## Fallible vs. panicking arithmetic

Starting in 0.2, undefined results (division by zero, `ln` of a non-positive number, `lambertw` outside its domain, and so on) are surfaced through the type system. Most arithmetic methods come in two flavors:

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

Use the `checked_*` form when accepting untrusted input (save files, user expressions) or when the operands could plausibly produce NaN. Use the operator form when arithmetic is well-defined by construction.

## Equality

`PartialEq` is **exact** in 0.2 — values are equal iff `sign`, `layer`, and `mag` are bit-identical. `-0.0` is canonicalized to `0.0` so equal-comparing values also hash equal, which makes `Decimal` a valid `HashMap` key.

For tolerance-based comparison use `approx_eq`:

```rust
use break_eternity::Decimal;

let a = Decimal::from_finite(1.0);
let b = Decimal::from_finite(1.0 + 1e-12);
assert!(a != b);                  // exact equality fails
assert!(a.approx_eq(&b, 1e-10));  // tolerance equality holds
```

## Internal representation

A `Decimal` is `sign * 10^10^10^...(layer times) mag`. So a layer-0 number is just `sign * mag`, a layer-1 number is `sign * 10^mag`, a layer-2 number is `sign * 10^10^mag`, and so on.

If `layer > 0` and `mag < 0.0`, the number's exponent is negative — i.e. `sign * 10^-10^10^10^...mag`.

- `sign` is `-1`, `0`, or `1`.
- `layer` is a non-negative integer.
- `mag` is an `f64`, normalized as follows: if it is above `1e15`, take `log10(mag)` and increment `layer`. If it is below `log10(9e15)` (about 15.954) and `layer > 0`, take `10.0_f64.powf(mag)` and decrement `layer`. At layer 0 the sign is extracted from negative `mag`. Zeroes (`sign == 0 || (mag == 0.0 && layer == 0)`) become `0, 0, 0` in all fields.

## Minimum supported Rust version

The crate is built and tested against `rustc 1.94` (Rust 2021 edition). Earlier versions may work but are not guaranteed.

## Contributing

Bug reports, math improvements, and PRs are all welcome. The crate is parity-tested against [`break_eternity.js`](https://github.com/Patashu/break_eternity.js); divergences in math-heavy methods (`gamma`, `slog`, `lambertw`, edge cases of `pow`) are tracked, and reports of new failing cases — with a JS reference value to compare against — are especially appreciated.

Issues and pull requests live at [github.com/MaddisonM79/break-eternity-rs](https://github.com/MaddisonM79/break-eternity-rs).

## Acknowledgements

- [Patashu](https://github.com/Patashu) — author of [break_eternity.js](https://github.com/Patashu/break_eternity.js), the original library this work descends from.
- [cozyGalvinism](https://github.com/cozyGalvinism) — wrote [break-eternity](https://github.com/cozyGalvinism/break-eternity), the Rust port this crate is forked from.
- Naruyoko — for [OmegaNum.js](https://github.com/Naruyoko/OmegaNum.js)'s modulo implementation, reused here.

## License

Licensed under the MIT License — see [LICENSE](https://github.com/MaddisonM79/break-eternity-rs/blob/develop/LICENSE).
