# Changelog

## 0.7.0 — Media gallery and guided backups (unreleased)

- Add a seventh workspace view for recorded media files, thumbnail-path evidence, Photos catalogue rows and unmatched associations.
- Preview selected PNG/JPEG files through native-owned source access captured during an explicit scan. Check the source SHA-256 and generate a metadata-stripped PNG with bounded input, dimensions and output.
- Keep previews transient. Opening a saved case or demonstration does not restore source-read access, even when an imported case reuses a previous ID.
- Add a three-step iPhone backup workflow: native folder selection, bounded index/encryption inspection and explicit analysis. Index counts and actual reader coverage stay separate.
- Require explicit unencrypted metadata; keep encrypted, malformed and unsupported backups unavailable. Cancelled or failed operations retain the current observation and review.
- Add `engram fixture DEST --iphone-backup` for reproducible, clearly synthetic backup tests.
- Preserve schema-1 cases, review annotations and Markdown/CSV reports. OCR and encrypted-backup reading remain separate work.

The 0.7 changes are source development; the current published download remains 0.6.0. Verification records will be added after the new native workflows pass.

## [0.6.0 — Evidence reports](https://github.com/Forem0/ENGRAM/releases/tag/v0.6.0) — 2026-10-29

- Add selected-finding Markdown reports and spreadsheet-safe CSV tables through the shared Rust core.
- Export the desktop's current results, all findings or bookmarks with a native destination dialog and a checksum receipt.
- Keep analyst notes, full recorded paths and source-derived details as separate opt-ins; defaults omit those fields and use filename-only path labels.
- Preserve engine confidence, case evidence and review state. Export is independent from Save case and never marks unsaved review as saved.
- Add `engram export` with explicit finding IDs or saved-bookmark selection, format choice and structured JSON receipts.
- Bound exports to 10,000 known, unique finding IDs and 16 MiB; reject unknown IDs, empty scopes, unsafe destinations and existing outputs without silent fallback or truncation.
- Escape report markup, neutralize spreadsheet formula prefixes and retain deterministic content checksums.

The [0.6.0 verification run](https://github.com/Forem0/ENGRAM/actions/runs/40874129225) records frontend checks, cross-platform Rust/backend tests and Clippy, six native Linux Tauri/IPC views, and an actual Linux report-picker exercise. The report exercise cancels a real picker, writes Markdown and CSV through the normal native command, and compares the outputs with the shared-engine CLI render of the same synthetic snapshot. It checks minimal-field defaults and retains the saved input case unchanged. Rust snapshot regressions cover stale requests, failed writes and review retention; the Linux GUI exercise does not edit annotations or exercise Windows/macOS pickers. Platform archives are produced by the [Build artifacts workflow](https://github.com/Forem0/ENGRAM/actions/workflows/build-artifacts.yml); release assets carry checksums and exact build provenance.

Case schema remains 1; reports do not replace restorable case files. No new recovery or reader-support claims are introduced. Earlier captures and releases retain their original provenance.

## [0.5.0 — Review workspace](https://github.com/Forem0/ENGRAM/releases/tag/v0.5.0) — 2026-10-28

- Add finding bookmarks, Unreviewed / Needs review / Reviewed states, and plain-text analyst notes in the persistent inspector.
- Persist validated, bounded annotations in an optional schema-1 `review` map when saving a new case; older schema-1 cases open with an empty review.
- Add bookmark/review filters and case-level review totals without changing the engine's confidence labels or source evidence.
- Add literal structured search with quoted phrases, exclusions, field operators and note/review predicates.
- Implement bounded map pan and zoom, modifier-wheel zoom, fit-selection controls and keyboard navigation.
- Add search/workspace shortcuts and unsaved-review prompts before case replacement or desktop close. Cancelled or failed saves keep the review dirty.
- Keep baseline review annotations read-only and evidence comparison independent of review-only edits.

Application and CLI version metadata is aligned with 0.5.0. The gallery contains six actual 1440 × 940 native captures from [source 023d641](https://github.com/Forem0/ENGRAM/commit/023d64122c28e35fd8d84fcea9477d8f4f21710c), exercised by the [successful Tauri/IPC capture job](https://github.com/Forem0/ENGRAM/actions/runs/40766085166/job/123090713655). The [verification run](https://github.com/Forem0/ENGRAM/actions/runs/40768086711) records the frontend and platform checks. Platform archives are produced by the [Build artifacts workflow](https://github.com/Forem0/ENGRAM/actions/workflows/build-artifacts.yml); published downloads carry their own checksums and exact build provenance.

Case schema remains 1 with an optional review map. Analyst notes are plaintext metadata in the saved case. Earlier releases retain their original screenshots, interface and provenance.

## [0.4.0 — Atlas Focus interface](https://github.com/Forem0/ENGRAM/releases/tag/v0.4.0) — 2026-10-28

- Implement the selected graphite-and-cobalt Atlas Focus visual direction across the six native workspace views.
- Replace the large introductory header with a compact source, observation-count and case-action toolbar.
- Give file relationships clearer category colors, legible labels and a distinct selected finding on the Trace map.
- Refine the findings table, source chronology, paired connections, baseline-selection state, module coverage and persistent evidence inspector.
- Retain the shared Rust analysis rules, guarded native operations, schema-1 cases, reader states and synthetic-demonstration labels.

Application and CLI version metadata is aligned with 0.4.0. The README gallery contains six actual native application captures, including the comparison view before a saved baseline is selected. The original design illustration gallery remains separate from native execution evidence; earlier releases retain their original appearance and provenance.

Validated by the [seven passing CI checks, including six native Tauri/IPC views](https://github.com/Forem0/ENGRAM/actions/runs/40694985190), from [source b7e7e9f](https://github.com/Forem0/ENGRAM/commit/b7e7e9fb4e4fbd3234ed2955b39f58c128566e60). The [0.4.0 package build](https://github.com/Forem0/ENGRAM/actions/runs/40695041263) records the Windows, macOS and Linux archives.

## [0.3.0 — Desktop workflow update](https://github.com/Forem0/ENGRAM/releases/tag/v0.3.0) — 2026-10-28

- Present ENGRAM as the working local desktop application, with accurate reader states and coverage limits.
- Expand the README into a six-view application tour and a download, inspect, filter, save and compare walkthrough.
- Add practical supported-source workflows, bounded CLI examples and a clearer case-format and architecture reference.
- Align contributor checks, build documentation and security wording with the current Tauri application.

The application and CLI use version 0.3.0 and retain the schema-1 case format. Earlier release artifacts remain unchanged.

Validated by the [seven passing CI checks, including six native Tauri/IPC views](https://github.com/Forem0/ENGRAM/actions/runs/40591718267), from [source d9a559b](https://github.com/Forem0/ENGRAM/commit/d9a559b2768b02582051c79584d4f95df9ebc42f).

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
