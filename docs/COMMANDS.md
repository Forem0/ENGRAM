# CLI commands

The package is `engram-cli`; its binary is named `engram`.

```bash
cargo run -p engram-cli --locked -- --help
cargo install --path crates/engram-cli --locked
engram --help
```

`--json` is a global option and may appear before or after a subcommand. JSON commands emit one structured JSON value to stdout. Errors go to stderr and return a nonzero exit code.

## Doctor

```bash
engram doctor
engram doctor --json
```

Reports the version, platform, default limits and operating boundaries. It does not inspect personal folders, probe a phone, unlock a backup or test credentials.

## Demo

```bash
engram demo
engram demo --out ./demo.engram.json
engram demo --json
```

Generates a synthetic dataset and uses the analysis engine to build the demonstration case. Saved cases retain `source_kind: "demo"`. The demonstration is not evidence that ENGRAM found anything on the user's computer.

## Fixture

```bash
engram fixture ./sample-source
engram fixture ./sample-iphone-backup --iphone-backup --json
```

Writes synthetic files for repeatable engine testing. Use a new destination. This is the one command whose purpose is to create source material; it does not inspect personal data.

`--iphone-backup` creates an explicitly synthetic, unencrypted iPhone-style backup with seven indexed entries and generated DEMO calibration images. Its JSON receipt includes `synthetic: true` and `format: "iphone_backup"`. The ordinary fixture remains unchanged. Use the backup fixture to exercise the native compatibility workflow and media preview without personal data.

## Scan

```bash
engram scan ./sample-source --out ./sample.engram.json
engram scan ./selected-backup --out ./backup.engram.json --json
```

The source must be supplied explicitly. No computer-wide discovery is performed. The output must be a new path outside the selected source. Existing output files are not overwritten.

| Option | Default | Meaning |
| --- | ---: | --- |
| `--max-files` | `25000` | maximum source files examined |
| `--max-file-bytes` | `8388608` | maximum bytes read from one file |
| `--max-total-bytes` | `536870912` | maximum total file bytes read |
| `--skip-hidden` | off | skip dotfiles; otherwise eligible `.env`-style files can be inspected |

All numeric limits must be positive integers. A completed partial inspection still emits its case; read the counters and warnings before interpreting the findings.

Additional hard bounds apply to traversal, database work and imported case shapes. See the [methodology limits](METHODOLOGY.md#source-selection-and-boundaries). The scan's 120-second work budget is checked between operations; it cannot interrupt every operating-system read immediately.

## Inspect

```bash
engram inspect ./sample.engram.json
engram inspect ./sample.engram.json --json
```

Opens the saved case without rescanning its source. The human view shows coverage, category totals, candidates, module status, warnings and the first 30 findings. JSON includes the complete case.

The CLI escapes terminal control characters when displaying untrusted paths or strings. JSON uses normal JSON string escaping.

## Export

The CLI includes `engram export` for reports from saved cases, introduced in **0.6.0** and retained in **0.7.0**.

```bash
engram export ./sample.engram.json --out ./findings.report.md
engram export ./sample.engram.json --out ./bookmarks.csv --format csv --bookmarked
engram export ./sample.engram.json --out ./selection.report.md \
  --finding <id> --finding <another-id> --json
```

Reads one explicitly supplied saved case and renders selected findings without reading its original source. Markdown is the default; `--format csv` creates a UTF-8 BOM-prefixed CSV table with CRLF records. The output must be new and outside source evidence. Existing case and report files are never overwritten.

| Option | Default | Meaning |
| --- | --- | --- |
| `--out <file>` | required | new report destination |
| `--format markdown\|csv` | `markdown` | report format, independent of the filename extension |
| `--finding <id>` | all case findings | select a known ID; repeat for multiple IDs |
| `--bookmarked` | off | select only bookmarks already saved in this case |
| `--include-notes` | off | include entered analyst notes without automatic redaction |
| `--include-paths` | off | include recorded paths and source root instead of filename-only path labels |
| `--include-details` | off | include source-derived summaries and evidence detail fields |

`--finding` and `--bookmarked` are mutually exclusive. Empty bookmarks, unknown IDs, duplicate IDs, more than 10,000 selected findings or more than 16 MiB of rendered output produce an error and no report. Selection retains case order. An invalid request never silently exports all findings instead.

CSV refuses case IDs, selected finding IDs or selected rule IDs that could be interpreted as spreadsheet formulas, preserving their exact identity. Choose Markdown for those identifiers; unselected finding IDs do not block a subset export.

The human response shows the destination, format, finding count, byte count and SHA-256 checksum. `--json` returns a receipt with `path`, `format`, `finding_count`, `bytes` and `sha256`; it does not write report content to stdout. Terminal control characters in displayed paths are escaped.

Reports preserve the supplied observation and confidence labels. They are not restorable cases. CSV formula prefixes are neutralized and Markdown content is escaped, but included metadata can remain sensitive. Path settings affect structured path/source-root fields, not arbitrary source text. Notes are plain text, and the details opt-in can include private source-derived text or paths even when `--include-paths` is off. The command cannot see an unsaved review in another desktop process; use Save case there before exporting that review from the CLI.

## Compare

```bash
engram compare ./before.engram.json ./after.engram.json
engram compare ./before.engram.json ./after.engram.json --json
engram compare ./before.engram.json ./after.engram.json --out ./comparison.json
```

Compares the two saved cases and reports added, removed, changed and unchanged finding IDs. Optional output stores the comparison JSON in a new file. Check that source selection, coverage, dates and parser support are comparable.

"Removed" means that a finding is absent from the later supplied case. It is not proof that the data was securely erased from every copy.

The command does not open or resolve the source paths stored inside imported cases. Secret-value fingerprints are case-local; comparison of unchanged exported metadata does not establish that a secret's underlying value is unchanged.

## Exit status

| Status | Meaning |
| --- | --- |
| `0` | the command completed; a scan may still contain coverage warnings |
| `1` | source, parsing, case or output error |
| `2` | command-line argument error reported by clap |

The CLI does not validate a candidate secret or elevate a result to "compromised" on the basis of its shape.

## Tauri desktop launch

The desktop is a separate binary, `engram-desktop`, with native source/case dialogs and a React/TypeScript renderer inside Tauri 2.

```bash
cd apps/desktop
npm ci
npm run tauri -- dev -- -- --demo
```

The default launch opens a synthetic case. `--view` accepts `map`, `findings`, `timeline`, `connections`, `compare`, `modules` or `gallery`. Choosing a view does not grant access to local image bytes; real sources require explicit native selection and scanning. For an already built production executable:

```bash
./target/release/engram-desktop --demo --view findings
```

The `--smoke-test` flag is reserved for automated native startup/IPC verification. It only succeeds after the renderer loads an actual Rust-generated synthetic case and acknowledges readiness through the guarded command. It is not a scan of a personal source. Native capture details are in [BUILDING.md](BUILDING.md#native-smoke-verification-and-screenshots).

The historical egui flags `--screenshot` and `--exit-after-screenshot` are not the current capture mechanism; CI captures the actual Tauri window externally after its readiness marker.
