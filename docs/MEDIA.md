# Media gallery and iPhone backup workflow

This document describes **ENGRAM 0.7.0**, available from the [release downloads](https://github.com/Forem0/ENGRAM/releases/tag/v0.7.0). The [completed locked-source verification](https://github.com/Forem0/ENGRAM/actions/runs/40929679972) covers frontend, backend and Linux native interaction; [capture provenance](../assets/screenshots/README.md#07-media-workflow-captures) identifies each image's original checkout. Downloaded packages retain their separate build and checksum records.

## Inspect media in context

1. In the native desktop app, use **Choose source** to inspect an explicitly selected folder, or use **Inspect iPhone backup** for the guided workflow below.
2. Open **Media gallery**. The shared search, category, confidence and review filters still apply; the gallery does not create a second set of findings.
3. Select a media card. Its source context shows the recorded path, available timestamp, rule, size and recorded relationships. A timestamp retains its original meaning; it is not an assumed deletion date.
4. For an eligible PNG or JPEG file, choose **Preview image**. The native session checks the source and returns a newly encoded, bounded PNG. The view shows its source dimensions and the verified source SHA-256.
5. Use **Hide image**, select another finding, change observations or leave the gallery to remove the displayed preview. Preview contents are not added to the saved case or exported report.

The gallery pages through six matching media cards at a time. It opens image contents only after an explicit preview request; opening the gallery is not an instruction to decode all matching files.

### What the groups mean

| Filter | Meaning |
| --- | --- |
| All media | Media-category findings matching the shared filters |
| Files | Recorded media-file findings without recorded thumbnail-path evidence |
| Thumbnail paths | File findings carrying the engine's explicit thumbnail-path evidence |
| Catalogue rows | Supported Photos catalogue records, which are metadata rather than image files |
| Unmatched | Thumbnail-path findings for which inspected catalogue rows produced no match or ambiguous filename matches |

These groups follow the engine's recorded rules and evidence. They do not establish visual identity, secure deletion, an original image's absence from every location, or access by another application. A catalogue record can be inspected without having an available image. Follow its recorded relationships to look for a corresponding file; the interface does not invent an image for the record.

## Preview availability and bounds

Previews require a `file.media` finding with a captured content hash from a real source scan in the current native session. The renderer sends only its expected case ID and finding ID. It cannot send a filesystem path, image bytes or an imported evidence object to authorize a read.

The Rust engine returns an opaque, transient `MediaAccess` alongside a freshly scanned case. This registry binds eligible finding IDs to physical source files, lengths and hashes observed during that scan. The physical mapping remains inside Rust and is not serializable as a case. For iPhone backups, it records the actual manifest-mapped blob, rather than treating a logical path as a path on disk.

Before rendering a preview, Rust checks the active observation, registered source, regular-file boundaries, links/reparse points, stable content and SHA-256. A changed, missing, excluded or unreadable source is an error, not a replacement image. The previous case remains available.

| Bound | Current implementation |
| --- | --- |
| Input formats | PNG and JPEG content; the interface offers previews for `.png`, `.jpg` and `.jpeg` file findings |
| Input size | At most 8 MiB, independently of a larger scan read budget |
| Source dimensions | At most 16 million pixels; decoder width and height limits also apply |
| Preview dimensions | At most 1024 pixels per edge, preserving aspect ratio without upscaling |
| Encoded output | At most 4 MiB of newly encoded PNG bytes |
| Decoder allocation setting | 128 MiB; library allocation limits are additional to the explicit dimension and pixel checks |

Source EXIF, text chunks and other image metadata are not copied into the newly encoded PNG. This does **not** redact the pixels: an image can still display private text, faces or other personal content. Source bytes and decoded pixels are processed locally in memory; there is no cloud image service or preview file written to source evidence.

Supported image orientation metadata is applied to the pixels before resizing and metadata removal. **Source dimensions** in the viewer describe those display-oriented dimensions before resizing. The original file and its recorded SHA-256 remain unchanged.

HEIC/HEIF, videos, TIFF, WebP and other media can retain their existing inventory metadata, but do not gain a PNG/JPEG preview decoder through this feature. OCR and deleted-image reconstruction are not implemented by the gallery.

## Saved cases and demonstrations

Schema-1 cases remain compatible. Saving a case retains its evidence and optional analyst review, without image bytes or the source-access registry. Opening or comparing a saved case treats its source paths as inert metadata and does not probe those paths. Even reopening a case with the same recorded ID does not restore its previous source-read authority.

To inspect image contents again, explicitly choose and scan the actual source. A normal synthetic demonstration also has no retained source authority after its temporary generation. The browser demonstration can show media cards and their evidence, but native source selection and image reads remain unavailable.

Image previews do not change bookmarks, notes, review states or the saved-review identity. Save case remains the operation that persists an edited review; exporting a report remains separate.

## Check an iPhone backup before scanning

Open **Inspect iPhone backup** in the desktop sidebar to start **Inspect an iPhone backup**:

1. **Choose a backup:** select the local directory containing `Manifest.plist` and `Manifest.db` through the native folder picker. Preparation does not connect to a phone or discover private device storage.
2. **Check compatibility:** inspect the reported encryption state, optional device label and version, bounded file-entry totals and coverage notes. **Index supported** means the supplied unencrypted backup has a readable supported ordinary `Files` table. It is not a successful full analysis or a compatibility guarantee for every catalogue schema.
3. **Inspect evidence:** choose **Start analysis**. Rust rechecks the staged native source, scans it with the shared engine, and replaces the current case only on success. The resulting observation opens in the gallery. Review **Modules** for the readers that actually ran and the reasons for partial coverage.

File-entry categories describe valid regular-file paths listed in the index. Counts do not establish blob presence or successful decoding. Inspection is bounded to 25,000 index rows, an 8 MiB main-index read, an 8 MiB available WAL read, a metadata budget and SQLite query limits. Each plist read is bounded to 1 MiB. Truncated coverage is described in the notes.

Missing or malformed metadata, unknown encryption state, encrypted backups and unsupported index schemas stay **Unavailable**. An explicit boolean `IsEncrypted: false` is required before an unencrypted backup scan proceeds. The app does not request or test a backup password. Optional `Info.plist` fields are reported metadata, not an independently verified device identity or operating-system support claim.

**Choose another backup** replaces the staged selection. **Cancel** clears it and retains the current observation. Selecting and checking a backup does not discard an unsaved review; the desktop's review guard applies before entering the replacement workflow. A failed scan retains the previous case. After a failed or completed scan, choose the source again to make another attempt because scan tokens are single-use.

## Native command boundary

These commands are registered in the invoke handler, app command manifest and local main-window capability:

| Command | Input from the renderer | Result |
| --- | --- | --- |
| `select_backup` | None | Native folder selection and bounded inspection, plus an opaque staged-selection token; cancellation returns no selection |
| `scan_backup` | Native selection token | Rechecked source scan and a new validated case; consumes the token |
| `cancel_backup` | Native selection token | Invalidates that matching staged selection |
| `preview_media` | Expected case ID and finding ID | Bounded PNG preview, dimensions and the matching source hash |

Tokens identify a staged native selection, not an arbitrary renderer-provided path. Stale tokens cannot consume a newer selection. Replacing the case invalidates staged backup selections; importing a case or loading the demo also revokes captured media access. Saving review metadata preserves the current scan's media authority.

All source work uses the existing guarded native worker. Overlapping operations are rejected. Browser UI tests alone do not prove that the packaged renderer can invoke these commands; actual Tauri/IPC verification must be recorded for the revision being released.

## Reproduce with generated data

The CLI fixture command accepts a new or empty destination directory. Existing contents are never overwritten:

```sh
engram fixture ./synthetic-iphone --iphone-backup
engram scan ./synthetic-iphone --out ./synthetic-iphone.engram.json
```

The generated backup contains seven invented file entries, hashed blob paths, an unencrypted manifest, synthetic device metadata, PNG files and supported example database schemas. It does not contain recovered personal data or establish compatibility with an arbitrary iOS version. In the desktop, choose this generated directory through the wizard to exercise the real workflow. Keep saved cases and reports outside the inspected directory.

`engram fixture ./synthetic-folder` continues to generate the ordinary folder fixture. The CLI does not have a separate image-preview command; PNG/JPEG preview access belongs to the native desktop session.

Tests cover source/hash changes, preview limits, unsupported content, metadata stripping, image orientation, malformed/encrypted backup metadata, ordinary-table checks, WAL snapshots and native selection lifetime. The [0.7 source run](https://github.com/Forem0/ENGRAM/actions/runs/40929679972) records their execution and the real Linux folder/scan/preview workflow. Linux native interaction, Windows/macOS host checks, Apple Silicon cross-compilation and downloadable package verification are distinct evidence; the source run does not establish Windows/macOS GUI behavior or ARM64 runtime execution.
