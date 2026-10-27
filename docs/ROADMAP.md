# Developer Preview roadmap

The first preview establishes the complete path from an explicitly selected source to a native evidence workspace. Module states are part of the product: **Ready**, **Experimental**, **Planned** and **Unavailable** should describe the actual reader and source.

The 0.2 source moves the desktop to Tauri 2 and React/TypeScript while preserving the Rust core and CLI. Verification includes frontend tests, native command/session tests and an actual Tauri/WebKit IPC launch. Version 0.1 downloads and screenshots describe the earlier egui interface.

## Current foundation

- Bounded folder inventory and supported text secret candidates.
- Masking before persistent secret findings.
- Exact-content relationships and evidence inspection.
- Synthetic demo generated through the analysis engine.
- Local cases, reopening and snapshot comparison.
- Native memory map, findings, timeline, connections and module coverage.
- Experimental unencrypted iPhone index/Photos association and supported SQLite activity readers.

## Reader validation

The next priority is reproducible fixtures across supported operating system and application versions, with a published compatibility matrix. Unknown schemas must remain visible as unsupported or partial coverage.

Useful additions include more Photos catalogue schemas, explicit timestamp semantics, stronger derivative association and richer supported-browser records. These require tests demonstrating the claimed behavior, not only a screen that looks convincing.

## Experimental research

- Deleted-record reconstruction from suitable SQLite main files and journals.
- Locally unlocked encrypted backup support using the owner's supplied password.
- Image metadata and optional local OCR, with explicit sensitive-content controls.
- Better cross-snapshot identity and explanation of changed findings.

These are roadmap items, not capabilities of the current preview. Encryption or overwritten bytes can make recovery unavailable.

## Distribution

- Verify packaged builds on Windows, macOS and Linux.
- Add platform-appropriate signing and notarization.
- Improve keyboard and assistive-technology coverage.
- Publish support statements tied to tested artifacts and fixtures.

## Boundaries that remain

ENGRAM will not turn permission presence into proof of access, validate discovered credentials against online accounts, or present an absent finding as a guarantee of secure erasure. The source and evidence must remain reviewable as the project grows.
