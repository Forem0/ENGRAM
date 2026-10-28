# Native application captures

These unedited PNGs show the actual ENGRAM 0.4.0 Atlas Focus application on Linux. React and TypeScript run inside the native Tauri/WebKit window; Xvfb supplies the display and WebKit uses software rendering.

The shared Rust engine generated the synthetic case: 33 findings from 15 files, with 10 recorded relationships and 3 notes. No personal directories or phone backups were inspected.

Source: [b7e7e9f](https://github.com/Forem0/ENGRAM/commit/b7e7e9fb4e4fbd3234ed2955b39f58c128566e60).
Capture and verification: [successful six-view native Tauri/IPC run](https://github.com/Forem0/ENGRAM/actions/runs/40694985190).

The artifact ZIP (ID `12818728000`) was downloaded and verified against its API-provided SHA-256 `b845aee8d986a3b5c2f1f67557841bdb5232ae3ade37f16d3def82d8ac35d5a2`. All six logs contain `ENGRAM_TAURI_SMOKE_OK`.

Each launch uses `--demo --view VIEW --smoke-test`. The renderer loads its case through the real native Rust command bridge and signals readiness after rendering. CI captures the visible 1440 × 940 window before the guarded smoke process exits. All six images were visually inspected.

- `workspace.png`: Trace map, compact findings dock and source evidence inspector. Only recorded relationships are drawn; the initially selected database has no sampled links.
- `findings.png`: searchable, paginated evidence index.
- `timeline.png`: source chronology with explicit timestamp limitations.
- `connections.png`: recorded relationships and selectable evidence endpoints.
- `compare.png`: native baseline selection before opening an earlier case; no comparison results are invented.
- `modules.png`: implemented and experimental readers; more states and coverage notes are available by scrolling.

SHA-256:

```text
235cc20dc89599c8a392720a4aac06b95bd5952526c5fa3cc5bab4a026f57246  workspace.png
4be508aa31ff1065101f7ebe6707c03f235722f03130e15c634e7bedf200b895  findings.png
05b160a2708c3b3946084b16ec92f4fdcb1077780b431cd532907ad85356a23b  timeline.png
e993c3f3b75acee965b7e294300660582f9ff964824d9c7e5282a8a96bf6668f  connections.png
b9b1b8e09d3aa4c81102efda6974448ecb951c8ea60238e9af98b4fd7c464ae8  compare.png
9d60316b432622c82e2ffc2ff2e37bcdad473225fca03f44976656147d2dbe1f  modules.png
```

Original design illustrations remain separately labelled under [assets/design/atlas-focus](../design/atlas-focus/README.md). Historical native captures remain in the [0.3 release source](https://github.com/Forem0/ENGRAM/tree/v0.3.0/assets/screenshots) and [0.2 release source](https://github.com/Forem0/ENGRAM/tree/v0.2.0-preview.2/assets/screenshots).
