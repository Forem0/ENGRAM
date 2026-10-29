# Privacy and threat boundaries

ENGRAM is intended for inspecting data you own or have authorization to examine. The source is selected explicitly and analyzed locally.

## Data that stays on the machine

The analysis engine does not send files, candidate secrets, hashes, paths or events to a remote service. It does not call an AI model, test login endpoints, query breach databases or emit telemetry. Building the application downloads dependencies; repository badges and links are separate from the local analyzer.

The Tauri renderer uses bundled React assets and a narrow Rust IPC command surface. Production source/case selection happens in native dialogs, and the Rust session holds the active case. The UI is not granted an unrestricted filesystem, shell or remote-page capability. WebView2/WebKitGTK are platform runtime dependencies, distinct from a remote analysis service.

The Vite browser preview contains a Rust-generated synthetic case and cannot inspect real sources through native commands. Its screenshots must be labelled browser fixture captures, not evidence that the native application ran.

## Source evidence

Readers do not intentionally alter source files. Database parsing uses temporary working copies and the available supported sidecars. Symlinks are not a route for broadening the selected directory. Read errors, bounds and unsupported data are limitations of coverage, not a clean bill of health.

Store saved cases outside the selected source. The CLI enforces this rule and refuses to overwrite an existing output. Use a new export filename in the desktop as well.

Imported case source paths are treated as metadata. Opening or comparing a case does not probe its recorded source, including a path that names a network share. A real source is read only after explicit selection.

## Media and staged backup sources in 0.7 development

The 0.7 development source adds a media gallery and guided backup selection; the current public download remains 0.6.0 until a separately verified release is published. [MEDIA.md](MEDIA.md) describes the source implementation and its bounds without claiming completed native verification.

Selecting a source and completing a fresh native scan creates an opaque Rust-only registry for eligible media files. Saved cases do not contain this authority. Importing a case, including one with the same recorded ID, or loading the demonstration revokes it; recorded paths do not become permission to read a file. Preview requests contain only a case ID and finding ID. Rust resolves the captured physical file, rechecks its path boundaries and compares its current bytes with the recorded SHA-256 before decoding.

Image contents open only after **Preview image** is requested. PNG/JPEG inputs are bounded to 8 MiB and 16 million pixels. The generated PNG is limited to 1024 pixels per edge and 4 MiB, and source metadata is not copied. Image bytes and decoded pixels stay local and are not added to a saved JSON case or Markdown/CSV report. Metadata stripping does not hide private information visible in the image itself. No OCR, face recognition or image upload is introduced by this workflow.

The iPhone wizard first checks an explicitly selected local directory. It reads bounded backup metadata and a working copy of its supported index and available WAL. Device labels and version fields are supplied by the backup, not independently verified identities. File-entry totals describe listed paths and do not prove that all corresponding blobs exist or can be parsed.

The native session retains the staged source behind a one-use token. React never supplies its path to start a scan. Cancelling clears the matching selection; a later selection supersedes an earlier token; replacing the active case invalidates the staged selection. Before scanning, Rust rechecks that the selected source still has a supported unencrypted index. Encrypted or unknown encryption states remain unavailable, without requesting or testing a password.

## Secret masking

Raw secret candidates are masked before they become persistent findings and events. Matching a supported format means only that the source contains a candidate worth reviewing. ENGRAM does not determine that it is active, leaked, owned by someone else or accepted by a service.

Case metadata remains sensitive. Paths can reveal usernames and project names; URLs can reveal private services; filenames, dates and relationships can reveal personal activity. A masked case is **not an anonymized case**. JSON cases are plaintext files and should be protected like other private exports.

## Unsupported protections

ENGRAM does not bypass encryption, brute-force passwords, unlock Keychain, recover a device passcode, read an arbitrary application's sandbox, extract browser sessions from a live process, or remove data from a phone. Encrypted backup support is not implemented; the source is reported as unavailable rather than silently decrypted or scanned as empty.

## Sharing a report

Report export defaults to omitting analyst notes, source root and source-derived detail fields, with filename-only values for the structured path field. These options minimize fields; they do not anonymize arbitrary imported text. Titles, filenames, dates, IDs and hashes can still identify activity. An explicit details opt-in can contain private source text or paths; entered notes are plain text and are not automatically redacted.

The desktop chooses the report destination through a native dialog. React sends a case ID, review metadata and selected IDs/options; Rust uses its own validated evidence snapshot. Reports are written to new files outside source evidence and existing outputs are not overwritten. Exporting current review changes does not save them back to the restorable case or clear the unsaved-review indicator. The CLI exports only metadata already present in its supplied saved case.

Use `engram fixture <new-folder>` to produce a synthetic reproduction. Include the application version, operating system, command, module state and redacted error. Review every exported path, URL and metadata field before sharing.

Do not upload a real phone backup, personal case, password database, private key or message archive in a GitHub issue. The issue template asks for synthetic reproduction steps rather than private evidence.

## What the tool cannot prove

- That no residual data exists outside the selected readable source.
- That a missing artifact has been securely erased everywhere.
- That another application read or transmitted a file.
- That a secret-shaped candidate is a working credential.
- That a local backup is a complete image of current device storage.

The evidence inspector and coverage warnings exist to keep those limits visible while the tool remains useful.
