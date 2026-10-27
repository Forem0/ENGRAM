# Contributing to ENGRAM

Keep the evidence behind every feature reviewable. A new reader should come with a synthetic fixture, the source format/version, the fields it observes, meaningful failure cases and an explicit module state.

## Development

Read [architecture](docs/ARCHITECTURE.md), [methodology](docs/METHODOLOGY.md) and [building](docs/BUILDING.md). Run the demo or generated fixtures; do not use personal backups in automated development.

```bash
cargo fmt --all -- --check
cargo test -p engram-core -p engram-cli
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p engram-desktop
```

## Reader requirements

- Work only within the explicitly selected source and read limits.
- Preserve original evidence; parse database copies with supported journals.
- Report unsupported schemas, errors and partial coverage.
- Keep timestamp meanings and provenance attached to findings.
- Mask secret candidates before they enter persistent cases or events.
- Establish relationships with an explained rule.
- Test malformed input, false positives and missing data.

Do not validate found credentials online, bypass OS protections, infer a past read from a permission, or label an unmatched artifact as proof of permanent deletion.

## Interface and documentation

The desktop and CLI use the core model. Coordinate changes to `model.rs` with all callers and case persistence. Keep `schema_version` and compatibility behavior explicit.

Use the warm ivory, charcoal and ochre brand. Screenshots should show a clearly marked synthetic source. Do not replace working interactions with decorative controls or invent scan results for a demonstration.

Describe the final change and relevant validation in a pull request. Keep real cases, credentials, phone backups and private paths out of issues, fixtures and screenshots.
