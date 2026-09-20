---
name: Bug report
about: Wrong result, panic, or unexpected behaviour
labels: bug
---

## What happened

<!-- One or two sentences. -->

## Minimal reproduction

```rust
use break_eternity::Decimal;

let x: Decimal = "1e100".parse().unwrap();
// ...
```

## Expected

<!-- For numeric bugs, the matching break_eternity.js output is the best reference:
     `node -e "const D=require('break_eternity.js'); console.log(D.fromString('1e100').sqrt().toString())"` -->

## Environment

- break-eternity-rs version:
- `rustc --version`:
- Features enabled (`serde`, `godot4`, `wasm`):
