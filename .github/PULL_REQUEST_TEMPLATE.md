## Summary

<!-- What changed and why. Link the issue if there is one. -->

## Checklist

- [ ] `cargo fmt --all && cargo clippy --features serde,wasm --all-targets -- -D warnings`
- [ ] `cargo test --features serde` (including the parity fixture)
- [ ] Public API changes are documented and listed under `[Unreleased]` in `CHANGELOG.md`
- [ ] Numeric behaviour changes are covered by a parity case or a regression test with a JS reference value
