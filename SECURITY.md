# Security policy

ENGRAM is a Developer Preview that handles potentially sensitive local material. Review [privacy boundaries](docs/PRIVACY.md) before examining a source or sharing a case.

## Reporting

For a suspected vulnerability, use GitHub's private vulnerability reporting **if it is enabled for this repository**. Otherwise contact the maintainer through an available private channel shown on the repository profile. Do not publish a real secret, backup or personal case as a public reproduction.

A useful report includes the affected commit, platform, impact, a synthetic reproduction and the relevant source/reader boundary. No particular response time or security certification is promised by this preview.

## Areas worth testing

- Source path traversal, symlink/junction escape and accidental source modification.
- Raw secret values reaching saved findings, events, logs or screenshots.
- Unbounded reads, malformed manifests and hostile SQLite files.
- Case/schema validation and misleading incomplete-coverage states.
- Control characters in filenames affecting terminal output.

The tool does not authenticate with discovered credentials, unlock protected vaults, or prove absence of residual data. A finding that correctly reports an accessible artifact is not by itself evidence of an OS vulnerability.
