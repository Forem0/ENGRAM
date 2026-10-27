# Changelog

## [0.3.0 — Desktop workflow update](https://github.com/Forem0/ENGRAM/releases/tag/v0.3.0) — 2026-10-27

- Present ENGRAM as the working local desktop application, with accurate reader states and coverage limits.
- Expand the README into a six-view application tour and a download, inspect, filter, save and compare walkthrough.
- Add practical supported-source workflows, bounded CLI examples and a clearer case-format and architecture reference.
- Align contributor checks, build documentation and security wording with the current Tauri application.

The application and CLI use version 0.3.0 and retain the schema-1 case format. Earlier release artifacts remain unchanged.

## [0.2.0-preview.2 — Visual identity update](https://github.com/Forem0/ENGRAM/releases/tag/v0.2.0-preview.2) — 2026-10-27

- Adopt the user-selected jagged cobalt-blue lightning creature, warm ivory and charcoal.
- Use the Trace desk and Trace map names with the slogan "Find what remains."
- Refresh application artwork, active controls and map styling while preserving the Rust analysis engine, CLI, schema-1 cases, evidence rules and coverage statements.

The application version remains 0.2.0. Verified by the [cross-platform checks and native Tauri/IPC captures](https://github.com/Forem0/ENGRAM/actions/runs/40585483665) and [three-platform native package build](https://github.com/Forem0/ENGRAM/actions/runs/40585602758), from [source 9709cd8](https://github.com/Forem0/ENGRAM/commit/9709cd8f7e7b453da952eef9eba5d4d7f225922a). The published preview.1 artifacts remain unchanged.

## [0.2.0 — Tauri Developer Preview](https://github.com/Forem0/ENGRAM/releases/tag/v0.2.0-preview.1) — 2026-10-27

The desktop source moves from egui to a Tauri 2 shell and a React/TypeScript evidence workspace. The shared Rust engine and CLI remain the analysis and case format authority.

- Typed frontend-to-Rust commands with native source/case dialogs and guarded background operations.
- React category map, filters, evidence inspector, timeline, connections, comparison and module states.
- Browser-only synthetic preview with an actual Rust-generated fixture and explicit native-operation boundaries.
- Frontend type checking and Vitest, native command/session tests, and three-platform locked Rust checks.
- Native Linux Tauri/WebKit startup and IPC smoke verification with actual window captures.
- Platform preview packaging with WebView2/WebKit runtime requirements and a macOS application bundle.

Verified by the [locked cross-platform CI and native Tauri/IPC captures](https://github.com/Forem0/ENGRAM/actions/runs/40575727550). The [native package build](https://github.com/Forem0/ENGRAM/actions/runs/40578609258) produces the Windows, macOS and Linux preview artifacts; release checksums and provenance accompany the downloads.

## [0.1.0 — Developer Preview](https://github.com/Forem0/ENGRAM/releases/tag/v0.1.0-preview.1) — 2026-10-27

Initial native Rust workspace for inspecting explicitly selected offline evidence.

This historical release uses the original egui/eframe interface. Its binaries and screenshots are distinct from the 0.2 Tauri migration.

- Interactive category map, searchable findings, evidence inspector, timeline and case comparison.
- Bounded file inventory, exact-copy relationships and masked secret candidates.
- Local iPhone backup manifest reader and supported SQLite activity parsers.
- Shared engine behind desktop and CLI, with a synthetic fixture demonstration.
- Coverage warnings, explicit confidence labels, source-preserving output guards and import limits.
- Cross-platform build checks and native screenshot workflow.

This preview does not claim complete device coverage, secure deletion, credential validity, Keychain recovery, or proof that another application read a file. See the capability matrix in the README.
