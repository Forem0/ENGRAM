# Native application captures

These unedited PNGs were captured from the actual ENGRAM 0.2 preview.2 Tauri executable on Linux, with the cobalt character and Trace desk identity. The React/TypeScript interface runs inside the native WebKit window. Xvfb provides the display and WebKit uses its software-rendering fallback.

The case is generated from synthetic fixture files and processed by the shared analysis engine. No personal directories or real phone backups were inspected. This sample records 33 findings from 15 files, with 10 recorded relationships.

Source: [9709cd8](https://github.com/Forem0/ENGRAM/commit/9709cd8f7e7b453da952eef9eba5d4d7f225922a).
Capture: [successful native Tauri/IPC CI run](https://github.com/Forem0/ENGRAM/actions/runs/40585483665).

Each launch uses `--demo --view VIEW --smoke-test`. The renderer loads a case through the native Rust command bridge and signals readiness after rendering. CI captures the visible native window before the guarded smoke process exits. The captures are 1440 × 940 pixels.

- `workspace.png`: Trace map and source evidence inspector.
- `findings.png`: searchable evidence index.
- `timeline.png`: source chronology with explicit timestamp limitations.
- `connections.png`: recorded relationships between findings.

SHA-256:

```text
80b80572ea1a14d01cb56bba08aa9a4d30fd071dcbdbbec79fac8ba9d89df817  workspace.png
9ad9503eba7463a57ca3aa15edae8fd483334408e904839cf5bb079417956ea6  findings.png
2726a27a2cae52d9a136dfc65d50e80b358c6fae3a9e34eff555dcccc90c6e28  timeline.png
ca52260337388ea15af44679573e535010df1c06f41e75540dde3d89b4f36da6  connections.png
```

Earlier captures remain available in the [preview.1 release source](https://github.com/Forem0/ENGRAM/tree/v0.2.0-preview.1/assets/screenshots); the [0.1 release source](https://github.com/Forem0/ENGRAM/tree/v0.1.0-preview.1/assets/screenshots) retains the historical egui interface.
