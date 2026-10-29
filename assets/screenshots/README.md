# Native application captures

## 0.7 media workflow captures

`gallery-v07.png` and `backup-wizard-v07.png` are unedited 1440 × 940 captures from the [successful native job](https://github.com/Forem0/ENGRAM/actions/runs/40926563689/job/123633731128). The app is the actual Tauri/WebKitGTK desktop on Linux/X11. The backup and its 640 × 400 DEMO calibration image were generated solely for this exercise; no personal files or connected phones were inspected.

- Actual PR merge checkout: `0290a23bfbb7457e00d8bf67b3171cd26aeb6686`; tree: `d05cfd901a4a4c2b6c68f55e945641175fdc2892`.
- PR head: `616b6b59235b262f854e3352f8510e7513272176`.
- Artifact: `native-tauri-workspace`, ID `12920103527`; ZIP SHA-256 `d2b4df02443a8e0bb63d538e0bc5e7a68d4878cf07dfbe2265e81886dfd93ffe`.
- Gallery image SHA-256: `49c01cb27630cd8b61c6cb85e603eef46f97ea1f44ff711d3f26fc3b77ce4788`.
- Backup compatibility image SHA-256: `31febc4fde5d7671d25fb6b242a81d40e5f7c02fbcf02917398feb299fde6908`.

All 103 ZIP entries were CRC-checked and both images visually inspected. The regular application used the actual native folder picker, displayed a supported unencrypted index with seven entries, and started analysis only after an explicit action. The saved native case contains 15 findings from seven examined files and matches the CLI analysis. Source names and bytes remain unchanged, and saving the case after image preview produces identical case bytes. The gallery capture visibly shows the generated image, the verified source hash state and **Hide image**. The automated script records the image request; visual inspection establishes the displayed pixels rather than claiming machine-parsed UI text.

[Machine-readable capture provenance](v07-captures.json) records this exact source. Later source fixes and formatting are tracked independently by [PR #8 checks](https://github.com/Forem0/ENGRAM/pull/8/checks); these images are not relabelled as captures of a later commit. Windows/macOS GUI interaction, real personal backups and live devices were not exercised by this capture job.

## 0.6 workspace and report captures

The eight original PNGs show ENGRAM 0.6.0 in its actual Linux Tauri/WebKit window. Xvfb supplies the display and WebKit uses software rendering. Six images cover the workspace views; two show the actual report configuration and completed Markdown receipt. The separately named 0.7 images are documented below; the original captures retain their provenance.

The shared Rust engine generated a synthetic case with 33 findings from 15 files, 10 recorded relationships and 3 coverage notes. Its review starts empty; no user annotations or personal backups are presented as recovered evidence.

- Actual checkout: [9c7d9c3e1e65b9ce95d597b3f59dc41a885afeaa](https://github.com/Forem0/ENGRAM/commit/9c7d9c3e1e65b9ce95d597b3f59dc41a885afeaa), the workflow's GitHub PR merge checkout.
- Checkout tree: `318c51ab3824f9e9e6cfe31f4701f5b82371e057`.
- PR head: [f8d702148433b04bf30a271ebb9464927c26545e](https://github.com/Forem0/ENGRAM/commit/f8d702148433b04bf30a271ebb9464927c26545e). The PR head and actual merge checkout are recorded separately.
- Capture and report interaction: [successful native Tauri/IPC job](https://github.com/Forem0/ENGRAM/actions/runs/40874129225/job/123451027724).
- Verification: [all seven passing CI checks](https://github.com/Forem0/ENGRAM/actions/runs/40874129225).
- Artifact: `native-tauri-workspace`, ID `12898250680`.

The downloaded artifact ZIP matches its API-provided SHA-256 `48456c901369ad3c992e56ffeb7084b3f2dec1e2b4c394a09c5563240297aaaf`. All eight images are 1440 × 940 pixels and were visually inspected. They are copied byte for byte from the artifact, without cropping, compositing or relabelling earlier captures.

## Six startup views

Each startup launch uses `--demo --view VIEW --smoke-test`. The renderer loads the generated case through the real native command bridge and signals readiness after rendering. All six logs contain `ENGRAM_TAURI_SMOKE_OK`. CI captures the visible window before the guarded smoke process exits. These six images establish native launch and initial IPC; the report exercise below is separate evidence of writing a report.

- `workspace.png`: Trace map, Export report action, review filters, viewport controls and inspector. The findings dock continues below the visible fold.
- `findings.png`: searchable paginated index and finding review controls; more rows and pagination are available by scrolling.
- `timeline.png`: source chronology with review filters and timestamp context.
- `connections.png`: actual recorded evidence relationships and their endpoints.
- `compare.png`: native baseline-selection state, before opening an earlier case. No comparison result is invented.
- `modules.png`: implemented and experimental readers, with more states and source notes below the visible fold.

## Actual report interaction

[`scripts/native-report-smoke.sh`](../../scripts/native-report-smoke.sh) launches the ordinary native `--demo` application and drives its visible controls with normal mouse and keyboard events. It uses the actual XDG/GTK destination pickers through the regular Tauri commands, saves a synthetic case, cancels one report picker, then exports Markdown and CSV. Both output files match the shared-engine CLI rendering of the exact saved native snapshot byte for byte.

- `report-export.png` is the artifact's unchanged `report-smoke/report-default-options.png`: 33 current results, Markdown selected, all three private-field opt-ins off.
- `report-receipt.png` is the artifact's unchanged `report-smoke/report-markdown-receipt.png`: completed local Markdown export with 33 findings, file size and SHA-256 receipt.

The interaction log contains `ENGRAM_NATIVE_REPORT_PICKER_OK` and `report-smoke/verification.json` records `native_report_e2e: passed`. Independent checks confirm minimal-field defaults, filename-only structured paths, CSV finding IDs/order, BOM and CRLF records, and unchanged bytes in the saved input case. The actual Markdown and CSV receipt screens were visually inspected and their hashes match the output proof; receipt UI text was not machine-parsed.

| Output | Findings | Bytes | SHA-256 |
| --- | --- | --- | --- |
| Markdown | 33 | 14,083 | `d7f7b51e82219c9f237fb756ff22abbe0a0dc4e660b076dfb3a924d18891505c` |
| CSV | 33 | 11,265 | `a8ab76261e3aa2b53352f3ba15cc40b7c149018e8c1b5d1ff6141ba17c2298f0` |

This exercise establishes the tested Linux/X11/WebKitGTK path. It does not exercise unsaved annotation edits, overwrite-picker confirmation, Windows/macOS GUI interactions or real user evidence. Stale snapshots, failed writes, review retention and output guards have separate Rust regression coverage. See [building and verification](../../docs/BUILDING.md#native-report-interaction) for the full scope.

## Image SHA-256

```text
d8233db452aa14448aaf539188c2e5db7de0fb4fc945617a3e2f1cc49d3d1095  workspace.png
78db6502c8b34ba14b1bba40eb22b6abf93f2ea03e5846df859d784f1d019d89  findings.png
761ca3521f71232a06148fa5d8b2ad00f33a93cc5e0b5395db2e528cadedb80e  timeline.png
d926f479f4af8f0ddf293bb07cd53f0c3a9cc42512ea737bc373531f40ff9b16  connections.png
23612ba5ee02d3b133d7a2adc6f1acf31b53e8efd3e78ee1af3ec96b3b7cbabb  compare.png
376c8b73e0617129d6f5c14ef1bcd0c1e23dfbe7d831cf53e968c1916380bfc8  modules.png
18f8d0485635c77b597791408807666b4fe042fe83299b34ceb7b70a75b0c865  report-export.png
7d31b6e60594e482520aa6d732ba04d7b04b040e0cfe52f127a8b6c964259bc9  report-receipt.png
```

Original design illustrations remain separately labelled under [assets/design/atlas-focus](../design/atlas-focus/README.md). Historical native galleries retain their own evidence in the [0.5 release source](https://github.com/Forem0/ENGRAM/tree/v0.5.0/assets/screenshots), [0.4 release source](https://github.com/Forem0/ENGRAM/tree/v0.4.0/assets/screenshots), [0.3 release source](https://github.com/Forem0/ENGRAM/tree/v0.3.0/assets/screenshots) and [0.2 release source](https://github.com/Forem0/ENGRAM/tree/v0.2.0-preview.2/assets/screenshots).
