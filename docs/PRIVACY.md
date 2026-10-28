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

## Secret masking

Raw secret candidates are masked before they become persistent findings and events. Matching a supported format means only that the source contains a candidate worth reviewing. ENGRAM does not determine that it is active, leaked, owned by someone else or accepted by a service.

Case metadata remains sensitive. Paths can reveal usernames and project names; URLs can reveal private services; filenames, dates and relationships can reveal personal activity. A masked case is **not an anonymized case**. JSON cases are plaintext files and should be protected like other private exports.

## Unsupported protections

ENGRAM does not bypass encryption, brute-force passwords, unlock Keychain, recover a device passcode, read an arbitrary application's sandbox, extract browser sessions from a live process, or remove data from a phone. Encrypted backup support is not implemented; the source is reported as unavailable rather than silently decrypted or scanned as empty.

## Sharing a report

Use `engram fixture <new-folder>` to produce a synthetic reproduction. Include the application version, operating system, command, module state and redacted error. Review every exported path, URL and metadata field before sharing.

Do not upload a real phone backup, personal case, password database, private key or message archive in a GitHub issue. The issue template asks for synthetic reproduction steps rather than private evidence.

## What the tool cannot prove

- That no residual data exists outside the selected readable source.
- That a missing artifact has been securely erased everywhere.
- That another application read or transmitted a file.
- That a secret-shaped candidate is a working credential.
- That a local backup is a complete image of current device storage.

The evidence inspector and coverage warnings exist to keep those limits visible while the tool remains useful.
