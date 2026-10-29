#!/usr/bin/env bash
# External Linux CI exercise of the ordinary ENGRAM Tauri window and OS pickers.
# NOT a production app entry point; no renderer JS injection or arbitrary-path IPC.
#
# Run from the checked-out repository, after the native debug app/CLI are built:
#   timeout 180s xvfb-run -a -s '-screen 0 1600x1100x24' \
#     dbus-run-session -- env ENGRAM_EXPORT_X=1035 ENGRAM_EXPORT_Y=35 \
#     bash scripts/native-report-smoke.sh native-captures/report-smoke
#
# Required CI packages in addition to existing native-capture dependencies:
#   xdg-desktop-portal xdg-desktop-portal-gtk
#
# Export-button coordinates are app-relative and explicitly set by CI. Center
# 1035,35 was verified against the actual 1440x940 v0.6 native workspace capture
# from source a39f14f70d0d07ccf6e744b8608492f3848fb3d7, run40869502577.
# Modal actions use normal keyboard focus traversal, not screen coordinates.
set -euo pipefail

: "${DISPLAY:?Run inside xvfb-run or an isolated test X display.}"
: "${DBUS_SESSION_BUS_ADDRESS:?Run inside dbus-run-session.}"
: "${ENGRAM_EXPORT_X:?Set the verified native Export report button x coordinate.}"
: "${ENGRAM_EXPORT_Y:?Set the verified native Export report button y coordinate.}"
[[ "$ENGRAM_EXPORT_X" =~ ^[0-9]+$ && "$ENGRAM_EXPORT_Y" =~ ^[0-9]+$ ]] || {
  echo 'Export coordinates must be nonnegative integers.' >&2
  exit 2
}
for executable in xdotool import python3 cmp realpath dbus-update-activation-environment; do
  command -v "$executable" >/dev/null || { echo "Missing CI tool: $executable" >&2; exit 2; }
done

app=$(realpath "${ENGRAM_APP_BINARY:-target/debug/engram-desktop}")
cli=$(realpath "${ENGRAM_CLI_BINARY:-target/debug/engram}")
[[ -x "$app" && -x "$cli" ]] || { echo 'Build the actual app and CLI first.' >&2; exit 2; }
out=$(realpath -m "${1:-native-captures/report-smoke}")
if [[ -e "$out" ]]; then
  echo "Choose a NEW report-smoke directory; existing data is never replaced: $out" >&2
  exit 2
fi
mkdir -p "$out"
exec > >(tee "$out/interaction.log") 2>&1

# The real rfd picker uses the session's XDG FileChooser portal. Install its GTK
# backend and update D-Bus activation before invoking it; no mocked chooser.
export XDG_CURRENT_DESKTOP=GNOME
if [[ -z "${XDG_RUNTIME_DIR:-}" ]]; then
  XDG_RUNTIME_DIR=$(mktemp -d)
  export XDG_RUNTIME_DIR
  chmod 700 "$XDG_RUNTIME_DIR"
fi
if [[ -n "${XAUTHORITY:-}" ]]; then
  dbus-update-activation-environment DISPLAY XAUTHORITY XDG_CURRENT_DESKTOP XDG_RUNTIME_DIR
else
  dbus-update-activation-environment DISPLAY XDG_CURRENT_DESKTOP XDG_RUNTIME_DIR
fi

app_pid=''
app_window=''
cleanup() {
  local status=$?
  trap - EXIT
  if [[ "$status" -ne 0 ]]; then
    echo "Native report exercise failed with status $status. No success claim was written."
    import -window root "$out/failure-desktop.png" 2>/dev/null || true
    xdotool search --onlyvisible --name '.' 2>/dev/null | while read -r id; do
      xdotool getwindowname "$id" 2>/dev/null || true
    done || true
  fi
  if [[ -n "$app_pid" ]]; then
    kill "$app_pid" 2>/dev/null || true
    wait "$app_pid" 2>/dev/null || true
  fi
  exit "$status"
}
trap cleanup EXIT

alive() {
  kill -0 "$app_pid" 2>/dev/null || {
    echo 'The actual Tauri application exited unexpectedly.' >&2
    cat "$out/app.log" >&2
    return 1
  }
}

find_window() {
  xdotool search --onlyvisible --name "$1" 2>/dev/null | head -n 1 || true
}

visible_window() {
  local candidate=''
  while IFS= read -r candidate; do
    if [[ "$candidate" == "$1" ]]; then return 0; fi
  done < <(xdotool search --onlyvisible --name '.' 2>/dev/null || true)
  return 1
}

wait_window() {
  local title=$1 id=''
  for _ in $(seq 1 150); do
    alive
    id=$(find_window "$title")
    if [[ -n "$id" ]]; then printf '%s\n' "$id"; return 0; fi
    sleep 0.2
  done
  echo "Timed out waiting for a real native window: $title" >&2
  return 1
}

wait_gone() {
  local id=$1
  for _ in $(seq 1 100); do
    alive
    if ! visible_window "$id"; then return 0; fi
    sleep 0.2
  done
  echo "Native picker did not close: $id" >&2
  return 1
}

capture_app() {
  import -window "$app_window" "$out/$1.png"
}

focus_app() {
  # Xvfb has no EWMH window manager; windowactivate would be unreliable here.
  xdotool windowfocus --sync "$app_window"
}

diagnose_chosen_files() {
  # Only inspect bounded top-level names in the controlled checkout/output.
  # Do not traverse source evidence, home directories or arbitrary case paths.
  python3 - "$out" <<'PY'
import pathlib, sys
out = pathlib.Path(sys.argv[1])
print('Controlled export directory files (at most 50):')
for path in sorted(out.iterdir())[:50]:
    if path.is_file():
        print(f'  {path.name}: {path.stat().st_size} bytes')
print('Possible misplaced generated outputs in checkout root (at most 30):')
patterns = ('*.engram.json*', 'case.report.*', 'export-case*', 'actual.report.*')
paths = sorted({p for pattern in patterns for p in pathlib.Path.cwd().glob(pattern)})
for path in paths[:30]:
    if path.is_file():
        print(f'  {path.name}: {path.stat().st_size} bytes')
PY
}

fill_save_picker() {
  local title=$1 destination=$2 stem=$3 picker='' folder='' filename=''
  folder=$(dirname -- "$destination")
  filename=$(basename -- "$destination")
  [[ "$folder" == "$out" && -d "$folder" && ! -e "$destination" ]] || {
    echo 'Picker targets must be new files directly in the controlled output folder.' >&2
    return 1
  }
  picker=$(wait_window "$title")
  echo "Observed actual save picker: $(xdotool getwindowname "$picker")"
  xdotool getwindowgeometry --shell "$picker" > "$out/$stem-picker-geometry.txt"
  import -window "$picker" "$out/$stem-picker.png"
  xdotool windowfocus --sync "$picker"
  # GTK's Save-mode Ctrl+L focuses Name rather than changing the directory.
  # Replace the entire Name with the absolute destination. The initial partial
  # selection preserves .json; Ctrl+A prevents that duplicate suffix. Actual
  # GTK 1096x822 picker capture verified Name field x64..1085,y11..42.
  xdotool mousemove --window "$picker" 300 27 click 1
  xdotool key --clearmodifiers ctrl+a
  xdotool type --clearmodifiers --delay 10 "$destination"
  import -window "$picker" "$out/$stem-picker-filled.png"
  echo "Saving exact absolute Name '$destination' through the actual picker."
  xdotool key --clearmodifiers Return
  # Never issue a blind second Return: if the dialog closed, it would act on
  # another native/renderer window. A still-open picker is a diagnostic failure.
  if ! wait_gone "$picker"; then
    import -window "$picker" "$out/$stem-picker-not-closed.png" 2>/dev/null || true
    diagnose_chosen_files
    return 1
  fi
  capture_app "$stem-after-picker"
  for _ in $(seq 1 150); do
    alive
    if [[ -s "$destination" ]]; then sleep 0.8; return 0; fi
    sleep 0.2
  done
  echo "Native command did not produce its chosen new file: $destination" >&2
  diagnose_chosen_files
  return 1
}

open_report() {
  focus_app
  xdotool mousemove --window "$app_window" "$ENGRAM_EXPORT_X" "$ENGRAM_EXPORT_Y" click 1
  sleep 0.6
  # WebKit may initially focus Close or the tabindex=-1 dialog itself. Both
  # reverse-Tab to the enabled final export action, then Tab wraps to Close.
  # Establish the same keyboard starting point without inspecting/injecting JS.
  xdotool key --clearmodifiers --delay 120 shift+Tab Tab
}

submit_fresh_report() {
  # showModal initially focuses Close. Shift+Tab uses WorkspaceDialog's real
  # focus boundary to reach its last enabled control: Choose destination/export.
  xdotool key --clearmodifiers --delay 120 shift+Tab Return
}

echo 'Launching ordinary --demo; startup --smoke-test is deliberately absent.'
"$app" --demo --view map > "$out/app.log" 2>&1 &
app_pid=$!
app_window=$(wait_window '^ENGRAM')
xdotool getwindowgeometry --shell "$app_window" > "$out/window-geometry.txt"
focus_app
sleep 2
capture_app initial-workspace

# Ctrl+S is the app's ordinary Save case shortcut. Retry only until its actual
# picker exists, allowing native startup/real synthetic engine generation time.
save_title='^Save a new case outside the source folder$'
save_window=''
for attempt in $(seq 1 20); do
  alive
  save_window=$(find_window "$save_title")
  if [[ -n "$save_window" ]]; then break; fi
  focus_app
  xdotool key --clearmodifiers ctrl+s
  sleep 1
done
[[ -n "$save_window" ]] || save_window=$(wait_window "$save_title")
fill_save_picker "$save_title" "$out/export-case.engram.json" snapshot
capture_app saved-synthetic-case

# Confirm the snapshot really is synthetic before any export comparisons.
python3 - "$out/export-case.engram.json" <<'PY'
import hashlib, json, pathlib, sys
path = pathlib.Path(sys.argv[1])
raw = path.read_bytes()
case = json.loads(raw)
assert case['schema_version'] == 1
assert case['source_kind'] == 'demo'
assert case['findings']
(path.parent / 'snapshot-before.json').write_text(json.dumps({
    'sha256': hashlib.sha256(raw).hexdigest(), 'bytes': len(raw)
}) + '\n', encoding='utf-8')
print(f"Saved actual native synthetic snapshot: {len(case['findings'])} findings")
PY

for format in markdown csv; do
  extension=md
  if [[ "$format" == csv ]]; then extension=csv; fi
  "$cli" export "$out/export-case.engram.json" \
    --out "$out/expected.report.$extension" --format "$format" --json \
    > "$out/expected-$format-receipt.json"
done

# First exercise cancellation through the actual report command/picker. The
# cancel test chooses no destination, and must not create either actual report.
open_report
capture_app report-default-options
submit_fresh_report
report_title='^Export a new report outside the source folder$'
cancel_picker=$(wait_window "$report_title")
import -window "$cancel_picker" "$out/report-cancel-picker.png"
xdotool windowfocus --sync "$cancel_picker"
xdotool key --clearmodifiers Escape
wait_gone "$cancel_picker"
sleep 0.6
[[ ! -e "$out/actual.report.md" && ! -e "$out/actual.report.csv" ]]
capture_app report-cancelled
focus_app
xdotool key --clearmodifiers Escape
sleep 0.3

# Default compact Markdown: all current results, three private options omitted.
open_report
capture_app report-markdown-options
submit_fresh_report
fill_save_picker "$report_title" "$out/actual.report.md" report-markdown
capture_app report-markdown-receipt
cmp "$out/expected.report.md" "$out/actual.report.md"
focus_app
xdotool key --clearmodifiers Escape
sleep 0.3

# Close -> current-scope radio -> format radio. Down chooses CSV, and three
# reverse Tabs return through scope/Close to the final enabled export button.
open_report
xdotool key --clearmodifiers --delay 120 Tab Tab Down
capture_app report-csv-options
xdotool key --clearmodifiers --delay 120 shift+Tab shift+Tab shift+Tab Return
fill_save_picker "$report_title" "$out/actual.report.csv" report-csv
capture_app report-csv-receipt
cmp "$out/expected.report.csv" "$out/actual.report.csv"

# Independently hash exact native output bytes, compare shared-engine CLI
# receipts, parse CSV as a consumer, and record honest scope of this exercise.
python3 - "$out" <<'PY'
import csv, hashlib, io, json, pathlib, sys
out = pathlib.Path(sys.argv[1])
case_path = out / 'export-case.engram.json'
case_bytes = case_path.read_bytes()
case = json.loads(case_bytes)
before = json.loads((out / 'snapshot-before.json').read_text(encoding='utf-8'))
assert before['sha256'] == hashlib.sha256(case_bytes).hexdigest()
assert before['bytes'] == len(case_bytes)
reports = []
for fmt, extension in [('markdown', 'md'), ('csv', 'csv')]:
    path = out / f'actual.report.{extension}'
    content = path.read_bytes()
    receipt = json.loads((out / f'expected-{fmt}-receipt.json').read_text(encoding='utf-8'))
    digest = hashlib.sha256(content).hexdigest()
    assert receipt['format'] == fmt
    assert receipt['finding_count'] == len(case['findings'])
    assert receipt['bytes'] == len(content)
    assert receipt['sha256'] == digest
    assert content == (out / f'expected.report.{extension}').read_bytes()
    reports.append({'format': fmt, 'file': path.name, 'bytes': len(content),
                    'findings': receipt['finding_count'], 'sha256': digest,
                    'matches_cli_render_of_exact_native_saved_snapshot': True})
csv_bytes = (out / 'actual.report.csv').read_bytes()
assert csv_bytes.startswith(b'\xef\xbb\xbf') and csv_bytes.endswith(b'\r\n')
rows = list(csv.DictReader(io.StringIO(csv_bytes.decode('utf-8-sig'), newline='')))
assert len(rows) == len(case['findings'])
assert [row['finding_id'] for row in rows] == [f['id'] for f in case['findings']]
assert not {'source_root', 'analyst_note', 'summary', 'origin', 'locator', 'evidence'} & set(rows[0])
for row, finding in zip(rows, case['findings']):
    assert row['exporter_version'] == '0.6.0'
    assert row['details_included'] == 'false' and row['notes_included'] == 'false'
    assert row['path_fields'] == 'filename_only'
    expected = next((p for p in reversed(finding['path'].replace('\\', '/').split('/')) if p), '(unnamed file)')
    assert row['path'] == expected
proof = {'native_report_e2e': 'passed', 'version': '0.6.0', 'platform': 'Linux/X11/WebKitGTK',
         'source_kind': 'demo', 'snapshot_file': case_path.name,
         'snapshot_sha256': hashlib.sha256(case_bytes).hexdigest(),
         'saved_input_case_unchanged': True,
         'mechanism': 'ordinary native window, renderer actions, Tauri IPC and actual XDG/GTK save pickers',
         'actual_picker_cancellation_completed': True,
         'defaults_omit_private_optional_fields': True,
         'reports': reports,
         'receipt_ui_screenshots': ['report-markdown-receipt.png', 'report-csv-receipt.png'],
         'receipt_ui_text_machine_parsed': False,
         'not_exercised_here': ['unsaved annotation edits', 'overwrite picker confirmation',
                                'Windows/macOS GUI interactions', 'real user evidence']}
(out / 'verification.json').write_text(json.dumps(proof, indent=2) + '\n', encoding='utf-8')
print(json.dumps(proof, indent=2))
PY
echo 'ENGRAM_NATIVE_REPORT_PICKER_OK'
