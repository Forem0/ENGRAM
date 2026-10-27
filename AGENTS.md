# ENGRAM development

ENGRAM uses a Tauri 2 native shell, a React/TypeScript renderer and a shared Rust analysis core for the user's explicitly selected offline data. Its brand is warm ivory, charcoal and ochre, with a hand-drawn brain archivist. The release is a Developer Preview; label demo, experimental and unavailable functionality accurately. The historical 0.1 release used egui; do not present its binaries or captures as evidence of the current Tauri UI.

Never scan the user's personal directories during development. Exercise the engine with generated synthetic fixtures. Never commit real backups, credentials, personal messages or machine paths. Never validate discovered credentials against a service. Never write to source evidence, follow symlinks, bypass OS protections, or claim that an unlinked file proves permanent deletion.

`crates/engram-core/src/model.rs` is the shared interface. Coordinate interface changes before changing it. The core owns parsing and evidence; desktop and CLI use the same core. Keep source reads bounded, report partial coverage and skip reasons, and mask secrets before they enter any persistent case or event.

The renderer lives in `apps/desktop`, and native commands in `apps/desktop/src-tauri`. Keep filesystem selection and case persistence in Rust. Do not add unrestricted filesystem, shell, remote content or arbitrary path IPC to make a UI control work. Source paths stored inside imported cases remain inert metadata.

Run formatting, core/CLI and native backend tests, clippy, frontend type checking/Vitest, and the production frontend/native builds. Build the frontend before a standalone Cargo desktop check or use the Tauri CLI. Standalone embedded desktop builds need `engram-desktop/custom-protocol`. Verify an actual Tauri window and IPC; browser fixture screenshots are additional UI QA and must never be labelled native captures. The browser fixture must come from the real Rust CLI synthetic demo, not invented result counts. Documentation must describe final observed behavior.

Both lockfiles are part of the reproducible build. A bootstrap CI run may upload a resolved Cargo.lock during migration; commit it and require a subsequent successful locked-source check before calling the release verified. Never bypass an operating-system application-control block to run a local compiler.

