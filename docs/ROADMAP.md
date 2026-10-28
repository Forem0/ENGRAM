# ENGRAM roadmap

ENGRAM already provides the native folder-to-case workflow: choose a source, inspect findings and relationships, save an observation, reopen it and compare snapshots. The Rust engine, Tauri desktop and CLI share the same case model and evidence rules.

Reader state is independent of application version. **Ready**, **Experimental**, **Planned** and **Unavailable** describe an implemented reader and its supplied source, not a promise of universal coverage.

## Working foundation

- Bounded folder inventory and supported text secret candidates.
- Fully masked values before persistent secret findings.
- Exact-content relationships and source evidence inspection.
- Synthetic demonstration generated through the actual analysis engine.
- Local JSON cases, reopening and snapshot comparison.
- Optional review metadata: finding bookmarks, review states and plain-text notes saved with a case.
- Structured literal search, review filters and bounded map pan/zoom.
- Trace map, findings, timeline, connections, comparison and module views.
- Experimental unencrypted iPhone manifest/Photos association and supported SQLite activity readers.
- Native source/case dialogs and guarded Rust background operations.

## Desktop workflow

Version 0.5.0 adds the review workflow to Atlas Focus: bookmarks and review states, plain-text notes, field/phrase/exclusion search, real map pan/zoom, selection fitting and keyboard shortcuts. The save-case path validates review metadata against the actual active snapshot. Unsaved-review prompts protect ordinary case replacement and desktop closing; source evidence and engine confidence remain separate from analyst annotations. Release downloads and screenshot provenance follow completed verification for their own source.

Next interface work should strengthen reproducible native interaction coverage, accessibility across platforms, source-specific diagnostics and review ergonomics. Keep older schema-1 cases readable and preserve the bounds and provenance of optional review metadata.

## Reader validation

The next parser priority is reproducible fixtures across supported operating-system and application versions, with a published compatibility matrix. Unknown schemas must remain visible as unsupported or partial coverage.

Useful additions include:

- More supported Photos catalogue schemas.
- Explicit timestamp meanings for each supported source.
- Stronger, documented derivative associations.
- Richer browser metadata where it can be retained without exporting private URL components.
- More malformed-input, truncation and unsupported-schema fixtures.

A new support claim needs a test demonstrating its behavior and limitations.

## Experimental research

- Deleted-record reconstruction from suitable SQLite main files and journals.
- Locally unlocked encrypted backups using the owner's supplied password.
- Image metadata and optional local OCR, with explicit sensitive-content controls.
- Better cross-snapshot identity and explanations for changed findings.

These are research items, not current capabilities. Encryption, missing source material and overwritten bytes can make recovery unavailable.

## Distribution and verification

- Continue native Windows, macOS and Linux packaging and checks.
- Exercise packaged executables on their target platforms.
- Add platform-appropriate signing and notarization.
- Tie runtime support statements to tested artifacts and fixtures.
- Capture all workspace views from actual native application windows.

Previous releases retain their original binaries, appearance and provenance. A screenshot from an earlier build is not evidence that a new native build ran.

## Boundaries that remain

ENGRAM will not turn permission presence into proof of access, validate discovered credentials against online accounts, or present an absent finding as a guarantee of secure erasure. The source and evidence remain reviewable as the project grows.
