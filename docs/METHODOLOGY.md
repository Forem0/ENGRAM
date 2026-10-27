# What an ENGRAM finding means

ENGRAM observes an explicitly selected offline source. Its output is a record of that inspection, with bounded coverage. The application does not establish what is currently on a live phone, what every application can access, or what has been securely erased.

## Source selection and boundaries

The engine accepts a directory supplied by the user. A supported unencrypted iPhone backup is identified through its manifest; other selected directories use folder inspection. ENGRAM does not search the computer for private sources, elevate privileges, attach to another process, or unlock a protected vault.

Reads have file-count, per-file and total-byte bounds. Dependency/generated directories and unsupported or inaccessible files can be skipped. Every case carries coverage counters and warnings. A scan that reached a limit is partial, even if it found nothing.

The source is never an output destination. SQLite files and the available associated journals are copied into a temporary working area for parsing. The CLI additionally rejects a case destination inside the source and refuses an existing output filename.

## Findings and confidence

Each finding records its category, source path, origin, locator, rule, available timestamp, confidence, evidence and related IDs. The evidence inspector exposes these fields so a reviewer can understand why a record exists.

| Confidence | Interpretation |
| --- | --- |
| Observed | The supported reader found this artifact in the selected source. |
| Linked | A documented relationship connects artifacts, such as matching file content. |
| Candidate | A pattern or incomplete association warrants review. |

These labels are statements about evidence. They are not numerical probabilities, risk scores or measures of access by another party.

## Media and catalogue association

Media inventory records eligible files. A supported backup reader can map a logical path through `Manifest.db` and attempt an association with a supported Photos catalogue. Multiple copies or derivatives can be connected where the engine has evidence for that connection.

An unlinked thumbnail is **not proof of a permanently deleted original**. Other explanations include schema changes, a partial backup, a cloud-only original, an edited derivative or a relationship that the current parser does not recognize. The preview does not recover arbitrary deleted files from raw flash storage.

The existence of residual images is technically plausible and historically documented, but a viral post is not a reproducible test of a particular OS version. Apple also describes a [Recovered album](https://support.apple.com/en-gb/120347) for files present on a device but outside its Photos library. That does not establish that every such file was permanently deleted or kept for Spotlight.

## Secret candidates

Text detectors identify supported secret-like assignments and formats in the selected material. They do not ask a server whether a value is valid, determine who owns it, or infer that it was stolen. A candidate may be a placeholder, expired value or false positive.

Raw candidate values are masked before they enter persistent findings or events. Within-case relationships can use keyed fingerprints without retaining the raw candidate. Such a relationship means equal candidates in the inspected material, not equal working accounts.

Filenames, URLs, paths, timestamps and record relationships can still be sensitive. Redacting candidate values does not anonymize a case.

## SQLite and journals

Supported Chromium, Safari and SMS readers inspect supported schemas from working copies. They do not represent a complete browser profile, conversation or history of a device. Metadata fields are labelled according to the reader that produced them.

Copying a `-wal` journal alongside a database helps preserve the logical state visible to SQLite. It does **not** implement deleted-record carving. SQLite documents that forensic residue depends on settings such as [`secure_delete`](https://sqlite.org/pragma.html#pragma_secure_delete); overwritten or cleared bytes may not be recoverable. See [SQLite WAL](https://sqlite.org/wal.html) for the distinction between committed journal state and the main database.

## Dates and comparisons

An available filesystem modification time is a modification time. A database observation can use a supported source timestamp. Neither should be renamed "deletion date" unless the source explicitly establishes that meaning.

Case comparison reports added, removed, changed and unchanged findings across the two supplied snapshots. Read the source types, backup dates, warnings and scan limits together. An absent finding can result from changed source coverage, changed parsing or a different snapshot; it does not prove secure deletion.

## iPhone backup limitations

This preview supports a bounded subset of unencrypted local backup data. Encrypted backups are detected and reported rather than treated as empty or partially decrypted. Keychain recovery is not implemented.

Apple explains that [computer backups exclude some data already synced through iCloud](https://support.apple.com/en-ie/108771), including iCloud Photos. [Encrypted backups](https://support.apple.com/en-ie/108353) may contain classes of data that unencrypted backups do not. [Keychain protection classes](https://support.apple.com/guide/security/keychain-data-protection-secb0694df1a/web) also restrict access and migration. A local backup is not an unrestricted image of the phone.

## Demonstration and validation

The demo is generated from synthetic files and inspected by the same engine. It is always labelled synthetic. Demo counts, relationships and timestamps describe those generated fixtures, not a discovery on the user's device.

To validate a new source reader, use controlled files and documented before/after snapshots. Keep the expected artifact, parser version, read limits and actual result together. Publish a support claim only after a reproducible fixture or authorized sanitized sample demonstrates it.
