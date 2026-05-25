# Security Policy

## Supported Versions

`break-eternity-rs` is a pre-1.0 crate. Only the latest published `0.x` release receives security fixes. There is no LTS line.

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1   | :x:                |

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

This policy covers vulnerabilities in the published `break-eternity-rs` crate. Out of scope:

- Vulnerabilities in transitive dependencies (please report to the upstream crate).
- Vulnerabilities in `break_eternity.js` (the original JavaScript library) — report those to [Patashu/break_eternity.js](https://github.com/Patashu/break_eternity.js).
- Denial-of-service via maliciously crafted input to the `TryFrom<&str>` parser when used on **trusted** data. Save-file parsing of untrusted user-supplied save data **is** in scope; please report.
- Numerical precision issues that do not have a security impact. File these as regular bug reports.

## Security Best Practices for Consumers

- Keep dependencies up to date. Configure Dependabot or `cargo audit` in CI.
- Treat `Decimal::try_from(&str)` input from untrusted sources (e.g., imported save files from other players) as adversarial; expect parse errors and bound iteration depth at the application layer if you accept very deeply tetrated expressions.
