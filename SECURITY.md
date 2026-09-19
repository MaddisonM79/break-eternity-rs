# Security Policy

## Supported Versions

`break-eternity-rs` is a pre-1.0 crate. Only the latest published `0.x` minor release receives security fixes. There is no LTS line.

| Version | Supported          |
| ------- | ------------------ |
| 0.5.x   | :white_check_mark: |
| < 0.5   | :x:                |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues, pull requests, or discussions.**

Use GitHub's private vulnerability reporting:

1. Go to the [Security tab](https://github.com/MaddisonM79/break-eternity-rs/security/advisories) of the repository.
2. Click **Report a vulnerability**.
3. Fill in the form with as much detail as you can: affected versions, reproduction steps, expected vs. observed behavior, and any proof-of-concept code or crash logs.

You can expect:

- An initial acknowledgement within **7 days** of submission.
- A status update at least every **14 days** while the issue is being investigated.
- A coordinated disclosure timeline negotiated together once the impact is understood.

## Scope

This policy covers vulnerabilities in the published `break-eternity-rs` crate. In scope:

- Any panic reachable from `Decimal::from_string` / `FromStr` / `TryFrom<&str>` / the `serde` deserializer on attacker-controlled input (save files, network messages). The parser is contractually panic-free and is fuzzed in CI.
- Unbounded CPU or memory consumption triggered by a short input string (for example a notation that expands to an extremely long formatting call).
- Any `checked_*` method returning an internal NaN sentinel instead of an error.

Out of scope:

- Vulnerabilities in transitive dependencies (please report to the upstream crate).
- Vulnerabilities in `break_eternity.js` (the original JavaScript library) — report those to [Patashu/break_eternity.js](https://github.com/Patashu/break_eternity.js).
- Panics from the documented panicking operator forms (`a / Decimal::zero()`, `(-8).pow(0.5)`, …) — use the `checked_*` forms for untrusted operands.
- Numerical precision issues that do not have a security impact. File these as regular bug reports.

## Security Best Practices for Consumers

- Keep dependencies up to date. Configure Dependabot or `cargo audit` in CI.
- Prefer the `checked_*` methods for arithmetic on values derived from untrusted input.
- Tetration and pentation are hyper-exponential: bound heights at the application layer if they come from user input, since `10^^1e6` is cheap to represent but `slog`/`penta_log` searches on adversarial values can take many iterations.
