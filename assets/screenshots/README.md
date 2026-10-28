# Native application captures

These unedited PNGs show ENGRAM 0.5.0 in its actual Linux Tauri/WebKit window. Xvfb supplies the display and WebKit uses software rendering. The interface includes case review controls and the map viewport toolbar.

The shared Rust engine generated a synthetic case with 33 findings from 15 files, 10 recorded relationships and 3 coverage notes. Its review starts empty; no user annotations or personal backups are presented as recovered evidence.

Source: [023d641](https://github.com/Forem0/ENGRAM/commit/023d64122c28e35fd8d84fcea9477d8f4f21710c).
Capture: [successful native Tauri/IPC job](https://github.com/Forem0/ENGRAM/actions/runs/40766085166/job/123090713655).
Verification record: [CI run](https://github.com/Forem0/ENGRAM/actions/runs/40766085166).

The downloaded artifact ZIP (ID `12850117723`) matches its API-provided SHA-256 `06aec2ffaff5fd8d804c2c1f00912067ef09afa6ac442e912ce31ec83e84b8fe`. All six logs contain `ENGRAM_TAURI_SMOKE_OK`. All six images were visually inspected.

Each launch uses `--demo --view VIEW --smoke-test`. The renderer loads the generated case through the real native command bridge and signals readiness after rendering. CI captures the visible 1440 × 940 window before the guarded smoke process exits. These captures establish native launch and initial IPC, not every interactive workflow; persistence and stale/cancelled saves have separate Rust regression coverage.

- `workspace.png`: Trace map, review filters, viewport controls and inspector. The findings dock continues below the visible fold.
- `findings.png`: searchable paginated index and finding review controls; more rows and pagination are available by scrolling.
- `timeline.png`: source chronology with review filters and timestamp context.
- `connections.png`: actual recorded evidence relationships and their endpoints.
- `compare.png`: native baseline-selection state, before opening an earlier case. No comparison result is invented.
- `modules.png`: implemented and experimental readers, with more states and source notes below the visible fold.

SHA-256:

```text
47810a6cc99b8ba37c9b981cf807d79c8ff90c9eab6715810c6b9241bf357bea  workspace.png
e4174d4efa777cf32860bf8dcf6cf4bbbc726d24586f1b18ed57fc9547d468ed  findings.png
465bc3d9e964ca7a78adbeffec0f567dbf4430e6df5fa3cbf53c0a98a024131d  timeline.png
8f2fc641618954c2a1a0064b33dec8de5e8ea62b4c0c7467449a715e9f66958e  connections.png
524d911f1178b5d8bc30dcdc143d94f65a5cbdb7661f7efe51aa24447046e1c1  compare.png
2694fbfdbb7e79abae4ce3e11637969f4876f9b30a95393024c9a8463715f322  modules.png
```

Original design illustrations remain separately labelled under [assets/design/atlas-focus](../design/atlas-focus/README.md). Historical native galleries retain their own evidence in the [0.4 release source](https://github.com/Forem0/ENGRAM/tree/v0.4.0/assets/screenshots), [0.3 release source](https://github.com/Forem0/ENGRAM/tree/v0.3.0/assets/screenshots) and [0.2 release source](https://github.com/Forem0/ENGRAM/tree/v0.2.0-preview.2/assets/screenshots).
