# break-eternity-rs

[![crates.io](https://img.shields.io/crates/v/break-eternity-rs.svg)](https://crates.io/crates/break-eternity-rs)
[![docs.rs](https://img.shields.io/docsrs/break-eternity-rs)](https://docs.rs/break-eternity-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A fork of [cozyGalvinism's break-eternity](https://github.com/cozyGalvinism/break-eternity), itself a port of [Patashu's break_eternity.js](https://github.com/Patashu/break_eternity.js).

A numerical library to represent numbers as large as 10^^1e308 and as 'small' as 10^-(10^^1e308).

This library focuses less on precision and more on speed. It is intended to be used by games.

## Installation

```sh
cargo add break-eternity-rs
```

Or add it manually to your `Cargo.toml`:

```toml
[dependencies]
break-eternity-rs = "0.2"
```

## Additional Features

This crate has the following optional features:

* `serde` — `Serialize`/`Deserialize` implementations (string-based).
* `godot4` — `GodotConvert`/`FromGodot`/`ToGodot` for the [`godot`](https://crates.io/crates/godot) crate (Godot 4 / gdext).
* `godot3` — **deprecated** — `FromVariant`/`ToVariant` for the [`gdnative`](https://crates.io/crates/gdnative) crate (Godot 3). Will be removed in 0.3.0.
* `wasm` — exposes `Decimal` to JavaScript via [`wasm-bindgen`](https://crates.io/crates/wasm-bindgen).

All features are off by default. Enable as needed via `features = [...]` in your `Cargo.toml`.

## Internal Representation

The internal representation of a Decimal is as follows:
`Decimal::from_components(sign, layer, mag)` == `sign * 10^10^10^...(layer times) mag`. So a layer 0 number is just `sign * mag`, a layer 1 number is `sign * 10^mag`, a layer 2 number is `sign * 10^10^mag` and so on.

If `layer > 0` and `mag < 0.0`, then the number's exponent is negative, e.g. `sign * 10^-10^10^10^ ... mag`.

* `sign` is -1, 0 or 1
* `layer` is a non-negative integer
* `mag` is an `f64`, normalized as follows: If it is above 1e15, `log10(mag)` it and increment layer. If it is below `log10(9e15)` (about 15.954) and `layer > 0`, `10.0_f64.powf(mag)` it and decrement layer. At layer 0, sign is extracted from negative mags. Zeroes (`sign == 0 || (mag == 0.0 && layer == 0)`) become `0, 0, 0` in all fields.

Decimal implements `Copy` and `Clone`, so it can be safely dereferenced without a fuss.

## Creating a Decimal

```rust
use break_eternity::Decimal;

// Infallible — finite f64 only (debug-asserted)
let a = Decimal::from_finite(1.5);

// Fallible — rejects NaN / ±Infinity
let b = Decimal::try_from(f64::INFINITY); // Err(ArithmeticError { kind: Undefined, .. })

// From string
let c: Decimal = "1.234e567".try_into().unwrap();

// From components (auto-normalized)
let d = Decimal::from_components(1, 2, 30.0);
let e = Decimal::from_mantissa_exponent(1.234, 567.0);

// Integer types use From
let f = Decimal::from(42_i32);
```

Fields are private; use accessors (`sign()`, `layer()`, `mag()`, `mantissa()`, `exponent()`) to read state.

### Accepted String representations

```plain
M === M
eX === 10^X
MeX === M*10^X
eXeY === 10^(XeY)
MeXeY === M*10^(XeY)
eeX === 10^10^X
eeXeY === 10^10^(XeY)
eeeX === 10^10^10^X
eeeXeY === 10^10^10^(XeY)
eeee... (N es) X === 10^10^10^ ... (N 10^s) X
(e^N)X === 10^10^10^ ... (N 10^s) X
N PT X === 10^10^10^ ... (N 10^s) X
N PT (X) === 10^10^10^ ... (N 10^s) X
NpX === 10^10^10^ ... (N 10^s) X
X^Y === X^Y
X^^N === X^X^X^ ... (N X^s) 1
X^^N;Y === X^X^X^ ... (N X^s) Y
X^^^N === X^^X^^X^^ ... (N X^^s) 1
X^^^N;Y === X^^X^^X^^ ... (N X^^s) Y
```

## Operations

You can use the regular operators (`+`, `-`, `*`, `/`, `%`, `+=`, `-=`, `*=`, `/=`, `%=`) as well as named functions: `abs, neg, round, floor, ceil, trunc, recip, cmp, cmpabs, max, min, maxabs, minabs, log, log10, ln, pow, root, factorial, gamma, exp, sqrt, tetrate, iteratedlog, layer_add_10, layer_add, slog, ssqrt, lambertw, pentate`.

Primitive number types work on either side: `Decimal::from_finite(1.0) + 2.0`, `3 * Decimal::from(4)`, etc.

### Fallible vs panicking arithmetic

Starting in 0.2, undefined results (division by zero, ln of non-positive, lambertw out of domain, etc.) are surfaced through the type system. Each arithmetic method has two flavors:

```rust
// Panicking: matches integer-overflow convention.
let c = a + b;
let d = a.pow(b);

// Fallible: returns Result<Decimal, ArithmeticError>.
let c = a.checked_add(&b)?;
let d = a.checked_pow(&b)?;
```

Use `checked_*` when you accept untrusted input (save files, user expressions) or when the operands could plausibly produce NaN. Use the operator forms when arithmetic is well-defined by construction.

### Equality

`PartialEq` is **exact** in 0.2 (bit-equal `sign`, `layer`, `mag`). `-0.0` is canonicalized to `0.0` so equal-comparing values also hash equal — `Decimal` is a valid `HashMap` key.

For tolerance-based comparison, use `approx_eq`:

```rust
let a = Decimal::from_finite(1.0);
let b = Decimal::from_finite(1.0 + 1e-12);
assert!(a != b);                  // exact
assert!(a.approx_eq(&b, 1e-10));  // tolerance
```

## Note to bugs

Even though I ported this library, I am not very knowledgeable about math. In fact, I have no idea how to properly apply stuff like tetration, super log etc. So if there are any issues with these functions, you will probably have to explain me what I have to change  in the code.

I know this isn't very professional, but I lack the time to get a degree in math or further educate myself about the topic. This crate was pretty much born out of necessity, since I want to implement this in games I develop.

Other than that, all other kinds of bugs are appreciated.

## Afterword

This crate took a long time to port. Mostly because it IS a huge library. And being a single person, future ports of changes to `break_eternity.js` may also take a while.

Please consider contributing and actively opening pull requests, if you have improvements. It would help out a lot.

There definetly will be bugs, though I am more than willing to fix them.

Also, if you like this library, why not leave a star on this and [Patashu's break_eternity.js](https://github.com/Patashu/break_eternity.js)? It helps boost the popularity of our packages.

## Special Thanks

* Patashu (for taking the time to write this HUGE library)
* Naruyoko (for OmegaNum.js's modulo implementation, which I shamelessly copied)
