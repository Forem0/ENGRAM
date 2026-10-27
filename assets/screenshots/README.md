# Native application captures

These unedited PNGs were captured from the actual ENGRAM 0.2 Tauri executable on Linux. The React/TypeScript interface runs inside the native WebKit window. Xvfb provides the display and WebKit uses its software-rendering fallback.

The case is generated from synthetic fixture files and processed by the shared analysis engine. No personal directories or real phone backups were inspected. This sample records 33 findings from 15 files, with 10 recorded relationships.

Source: [a663762a](https://github.com/Forem0/ENGRAM/commit/a663762a283aa5ed578c0f1f0003c49b35ad158c).
Capture: [successful native Tauri/IPC CI run](https://github.com/Forem0/ENGRAM/actions/runs/40575727550).

Each launch uses `--demo --view VIEW --smoke-test`. The renderer loads a case through the native Rust command bridge and signals readiness after rendering. CI captures the visible native window before the guarded smoke process exits. The captures are 1440 × 940 pixels.

- `workspace.png`: memory map and source evidence inspector.
- `findings.png`: searchable evidence index.
- `timeline.png`: source chronology with explicit timestamp limitations.
- `connections.png`: recorded relationships between findings.

SHA-256:

```text
40fb5b5012ee995e93d71555f23040401235b4499d49ab4635465d8deaaf42a8  workspace.png
53ecf22837a2ca603d2d76cb8d55ddda1a1ed31b08190ca23082920eb3a5c017  findings.png
45b459652d0bc323a3c685987d9c352f2373b279995b5e2544ed68694853e7f1  timeline.png
3e37cfbd9531ddfacb8806629d1c547f6962edadb445fa0aa0b7d01220fcefa4  connections.png
```

The earlier egui captures remain available in the [0.1 release source](https://github.com/Forem0/ENGRAM/tree/v0.1.0-preview.1/assets/screenshots).
