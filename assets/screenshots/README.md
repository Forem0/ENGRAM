# Native application captures

These unedited PNGs were captured from the actual ENGRAM 0.3.0 desktop application on Linux. The React/TypeScript interface runs inside the native Tauri/WebKit window. Xvfb provides the display and WebKit uses its software-rendering fallback.

The case is generated from synthetic fixture files and processed by the shared analysis engine. No personal directories or real phone backups were inspected. This sample records 33 findings from 15 files, with 10 recorded relationships.

Source: [d9a559b](https://github.com/Forem0/ENGRAM/commit/d9a559b2768b02582051c79584d4f95df9ebc42f).
Capture: [successful six-view native Tauri/IPC CI run](https://github.com/Forem0/ENGRAM/actions/runs/40591718267).

The downloaded artifact ZIP (ID `12774313170`) was verified against SHA-256 `a89beb24c5f2a3f4b7ecf2fe2a13bc39eaad4a6ba9a396fddcb3cc16e06f2de1` before extraction. All six logs contain `ENGRAM_TAURI_SMOKE_OK`.

Each launch uses `--demo --view VIEW --smoke-test`. The renderer loads a case through the native Rust command bridge and signals readiness after rendering. CI captures the visible native window before the guarded smoke process exits. The captures are 1440 × 940 pixels.

- `workspace.png`: Trace map and source evidence inspector.
- `findings.png`: searchable evidence index.
- `timeline.png`: source chronology with explicit timestamp limitations.
- `connections.png`: recorded relationships between findings.
- `compare.png`: the native baseline-selection state before opening an earlier case; no comparison result is invented.
- `modules.png`: implemented and experimental readers at the top of the source coverage view; more module states are available by scrolling.

SHA-256:

```text
8af5a1ee764122eb50ec49b38cc12d4c98b4d82a4ca3fa8805bacdd6266386cd  workspace.png
802d101cd85c3bd6d3908a3932e33254efda36b8a85bfd4fca4ffdde50ed18c7  findings.png
934de597e71d1eb7f58ccf16cbca5004bd526f973d3f7237a41070bbd0f01870  timeline.png
7d99711870cc6853fec94bbfaa9f2f300f85a7af620a24cba814b2effd2be792  connections.png
01df36dd8649f440005232ead7e663a6acd39727121a4764b8edbddc2242082b  compare.png
424ab9fabfec271dc2c11d30aba562488fad04985fff47bbaa3e5be37af7f59c  modules.png
```

Earlier captures remain available in the [0.2 release source](https://github.com/Forem0/ENGRAM/tree/v0.2.0-preview.2/assets/screenshots).
