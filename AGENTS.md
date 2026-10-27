# ENGRAM development

ENGRAM is a native Rust desktop application for examining the user's explicitly selected offline data. Its brand is warm ivory, charcoal and ochre, with a hand-drawn brain archivist. The release is a Developer Preview; label demo, experimental and unavailable functionality accurately.

Never scan the user's personal directories during development. Exercise the engine with generated synthetic fixtures. Never commit real backups, credentials, personal messages or machine paths. Never validate discovered credentials against a service. Never write to source evidence, follow symlinks, bypass OS protections, or claim that an unlinked file proves permanent deletion.

`crates/engram-core/src/model.rs` is the shared interface. Coordinate interface changes before changing it. The core owns parsing and evidence; desktop and CLI use the same core. Keep source reads bounded, report partial coverage and skip reasons, and mask secrets before they enter any persistent case or event.

Run formatting, core/CLI tests, clippy and the native desktop build. Verify the actual desktop visually. Documentation must describe the final observed implementation, not aspirational features.

