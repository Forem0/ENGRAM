# ENGRAM desktop

The native evidence desk uses `eframe`/`egui` and the same `engram-core` engine as the CLI. It runs without a browser server and does not send source files to a service.

```sh
cargo run -p engram-desktop -- --demo
```

The initial dataset is synthetic. Choosing a source folder starts bounded analysis on a worker thread. Findings, timeline, graph links, module states and coverage all come from the resulting case. Graph position groups findings by category; an edge means a relationship is recorded in the case, not a claim of causality or app access.

## Views and controls

- **Memory map:** click a finding node to inspect it; click a category label to open that filtered index.
- **Findings:** search title, logical path, origin or rule; combine category and linked-only filters.
- **Timeline:** inspect available source timestamps. These timestamps do not establish deletion time.
- **Connections:** display only recorded linked findings. Missing relationships are left absent.
- **Compare cases:** choose an earlier JSON case. The desktop lists added, missing, changed and unchanged observations; differing source paths are called out.
- **Modules:** inspect ready, experimental, planned and unavailable capabilities.
- **Save case:** write a derived JSON file outside the source folder. Candidate secret values remain masked; paths and metadata are retained and may still be sensitive.

Dialogs and command-line sources are explicit. The app does not automatically search personal directories. Source files remain unchanged. A failed analysis keeps the previous case visible and shows an error.

```sh
engram-desktop --case earlier-case.json --view findings
engram-desktop --source /path/to/selected/offline-folder
```

## Native screenshot capture

The screenshot switch asks the native renderer for a screenshot after the UI has rendered. It writes the returned pixels as PNG, rather than generating a representation of the layout.

```sh
engram-desktop --demo --view map --screenshot workspace.png --exit-after-screenshot
engram-desktop --demo --view findings --screenshot findings.png --exit-after-screenshot
engram-desktop --demo --view modules --screenshot modules.png --exit-after-screenshot
```

Supported view names are `map`, `findings`, `timeline`, `connections`, `compare`, and `modules`. A graphical session is required. On Linux CI with Mesa and Xvfb installed:

```sh
xvfb-run -a -s '-screen 0 1600x1200x24' env LIBGL_ALWAYS_SOFTWARE=1 \
  target/debug/engram-desktop --demo --view map \
  --screenshot workspace.png --exit-after-screenshot
```

Software OpenGL is a standard renderer fallback. It does not alter the engine or turn demo observations into personal-device results. Screenshot destinations must be new filenames in existing writable directories outside the selected evidence folder. Existing files and links are never replaced; automation using `--exit-after-screenshot` fails with a nonzero exit code if saving fails.
