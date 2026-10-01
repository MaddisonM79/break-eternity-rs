# Working on break-eternity-rs

Maddison's Rust port of `break_eternity.js`: a `Decimal` of `(sign, layer, mag)` reaching
10^^9e15. Published on crates.io as `break-eternity-rs`; the import path is
`break_eternity`. Forked from `cozyGalvinism/break-eternity`, now maintained here alone.

## Read first

- **`docs/DESIGN.md`**: representation and the normalisation invariant, infinity, the NaN
  sentinel, the three altitudes (`*_raw` / `checked_*` / panicking), parity policy, testing
  layers, `no_std`. Read it before adding or changing an operation.
- **`CONTRIBUTING.md`**: the pre-push commands, the parity fixture workflow, review rules.
- `docs/repo-review-2026-09-19.md` is a historical record (all addressed in 0.5.0), not a
  to-do list.

## Branches and releases

- The default branch is **`develop`**, not `main`. Every change goes through a PR to `develop`,
  including Maddison's own, as CONTRIBUTING.md asks. Don't push straight to `develop`.
- Pushing a `v*` tag publishes to crates.io via `release.yml`, which can't be undone. Tag
  only when Maddison asks. A release is: bump `Cargo.toml`, move `[Unreleased]` into a dated
  `CHANGELOG.md` section, merge to `develop`, tag `vX.Y.Z` matching `Cargo.toml` exactly.
- Public API changes go under `[Unreleased]` (Keep a Changelog). CI runs `cargo
  semver-checks` against the last crates.io release, so a breaking change pre-1.0 needs a
  minor bump.

## Verify loop

```sh
cargo fmt --all
cargo clippy --features serde,wasm --all-targets -- -D warnings
cargo test --features serde                          # includes the parity gate
cargo build --no-default-features --features libm
cargo test --no-default-features --features libm --lib
```

CI also runs `--all-features` clippy (needs `libclang` for `godot4`), rustdoc with
`-D warnings`, per-feature tests (`proptest`, `schemars`, `bevy_reflect`), wasm tests under
`wasm-bindgen-test-runner`, a `thumbv7em-none-eabihf` no_std build, `cargo deny`, and MSRV
checks. MSRV is per feature: 1.87 for the crate, 1.92 with `bevy_reflect`, 1.94 with
`godot4`. Don't raise the base MSRV casually.

## Rules that are easy to break

- **Every fallible op exists three ways**: a `*_raw` kernel (literal JS port, propagates
  NaN, never panics), `checked_*` (never panics, never returns the sentinel), and the
  panicking plain form. Compound kernels call other `_raw` kernels, never the operators.
- **New operations go in three places**: `tests/fixtures/generate.mjs`, `evaluate()` in
  `tests/parity.rs`, and the tables in `tests/contract.rs`.
- **Parity is zero mismatches against pinned `break_eternity.js` 2.1.3** (~12.4k cases,
  tolerance `1e-8`). Never loosen `PARITY_TOLERANCE`. Fix the port, or add a
  `deliberate_divergence` rule with a written reason. Regenerating the fixture needs Node
  and takes minutes (see CONTRIBUTING.md). `tests/fixtures/node_modules` is gitignored.
- **Anything touching `Eq` / `Hash` / `Ord`** needs contract tests. `PartialEq` is bit
  equality, so loops that test convergence with `==` must also check `has_nan_mag()`.
- **The parser must never panic on any string.** Save files are untrusted input
  (SECURITY.md), and proptest fuzzes it.
- **`no_std`: call float methods in method form** (`x.ln()`, never `f64::ln(x)`) so the
  `libm` shim in `src/math.rs` resolves them. New modules need the same cfg'd import.
- **`README.md` is the crate root doc** (`#![doc = include_str!("../README.md")]`), so its
  code blocks are doctests and every public item needs rustdoc (`#![warn(missing_docs)]`).
- `Cargo.lock` is not committed (library crate).

## Gotchas

- Negative `mag` above layer 0 means a value between 0 and 1, not near 1: at layer 2,
  `mag = -M` is about `10^(-10^M)`.
- Layers above `MAX_SAFE_LAYER` (9e15) become infinity. The layer is `i64`, so layer
  arithmetic must saturate rather than wrap.
- When checking JS reference values, `slog` takes three arguments: `slog(base, iterations,
  linear)`. The two-argument call reads `linear` as `iterations` and skips refinement, which
  explains small "divergences" seen in a REPL.

## Downstream

`Nyra-Security-LLC/rainbow_shapes` pins `0.5.2` with `serde` and `schemars`. Its `rainbow-num`
façade uses only the `checked_*` forms (including the geometric-series pair, `slog` and
`layer_add`), plus `is_finite`, `ulp` and `distinguishable`, and pacing leans on `slog`
accuracy. Changes to those deserve a thought about that game before a release.

<!-- global: v2, reviewed 2026-10-01 -->
