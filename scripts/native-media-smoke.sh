#!/usr/bin/env bash
# External Linux interaction with the ordinary native v0.7 window and real
# XDG/GTK folder/save pickers. No renderer injection, test-only source paths or
# arbitrary-path IPC. Every source file below is generated synthetic test data.
#
# Run after building the actual application/CLI, from the repository root:
#   timeout 240s xvfb-run -a -s '-screen 0 1600x1100x24' \
#     dbus-run-session -- env ENGRAM_BACKUP_X=104 ENGRAM_BACKUP_Y=791 \
#     ENGRAM_MEDIA_FILES_X=353 ENGRAM_MEDIA_FILES_Y=382 \
#     bash scripts/native-media-smoke.sh native-captures/media-smoke
#
# Coordinates are app-relative, initially observed in the 1440x940 browser
# layout. Actual native captures must be reviewed before claiming UI coverage.
# Save-picker Name coordinates follow the existing verified report exercise.
set -euo pipefail

: "${DISPLAY:?Run inside xvfb-run or an isolated test X display.}"
: "${DBUS_SESSION_BUS_ADDRESS:?Run inside dbus-run-session.}"
: "${ENGRAM_BACKUP_X:?Set the Inspect iPhone backup button x coordinate.}"
: "${ENGRAM_BACKUP_Y:?Set the Inspect iPhone backup button y coordinate.}"
: "${ENGRAM_MEDIA_FILES_X:?Set the Media gallery Files tab x coordinate.}"
: "${ENGRAM_MEDIA_FILES_Y:?Set the Media gallery Files tab y coordinate.}"
for coordinate in "$ENGRAM_BACKUP_X" "$ENGRAM_BACKUP_Y" "$ENGRAM_MEDIA_FILES_X" "$ENGRAM_MEDIA_FILES_Y"; do
  [[ "$coordinate" =~ ^[0-9]+$ ]] || { echo 'Coordinates must be nonnegative integers.' >&2; exit 2; }
done
for executable in xdotool import python3 realpath dbus-update-activation-environment; do
  command -v "$executable" >/dev/null || { echo "Missing CI tool: $executable" >&2; exit 2; }
done

app=$(realpath "${ENGRAM_APP_BINARY:-target/debug/engram-desktop}")
cli=$(realpath "${ENGRAM_CLI_BINARY:-target/debug/engram}")
[[ -x "$app" && -x "$cli" ]] || { echo 'Build the actual app and CLI first.' >&2; exit 2; }
export ENGRAM_SMOKE_VERSION
ENGRAM_SMOKE_VERSION=$(python3 -c 'import pathlib, tomllib; print(tomllib.loads(pathlib.Path("Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"])')
out=$(realpath -m "${1:-native-captures/media-smoke}")
[[ ! -e "$out" ]] || { echo "Choose a NEW media-smoke directory: $out" >&2; exit 2; }
mkdir -p "$out"
exec > >(tee "$out/interaction.log") 2>&1

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
    echo "Native media exercise failed with status $status. No verification success was written."
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
find_window() { xdotool search --onlyvisible --name "$1" 2>/dev/null | head -n 1 || true; }
visible_window() {
  local candidate=''
  while IFS= read -r candidate; do
    [[ "$candidate" != "$1" ]] || return 0
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
  echo "Timed out waiting for an actual native window: $title" >&2
  return 1
}
wait_gone() {
  for _ in $(seq 1 100); do
    alive
    if ! visible_window "$1"; then return 0; fi
    sleep 0.2
  done
  echo "Native picker did not close: $1" >&2
  return 1
}
capture_app() { import -window "$app_window" "$out/$1.png"; }
focus_app() { xdotool windowfocus --sync "$app_window"; }

save_case() {
  local destination=$1 stem=$2 picker='' title='^Save a new case outside the source folder$'
  [[ "$(dirname -- "$destination")" == "$out" && ! -e "$destination" ]] || return 1
  for _ in $(seq 1 25); do
    alive
    picker=$(find_window "$title")
    if [[ -n "$picker" ]]; then break; fi
    focus_app
    xdotool key --clearmodifiers ctrl+s
    sleep 0.8
  done
  [[ -n "$picker" ]] || picker=$(wait_window "$title")
  echo "Observed actual save picker: $(xdotool getwindowname "$picker")"
  xdotool getwindowgeometry --shell "$picker" > "$out/$stem-picker-geometry.txt"
  import -window "$picker" "$out/$stem-picker.png"
  xdotool windowfocus --sync "$picker"
  # Replace all of GTK's Name with the absolute new output filename. Ctrl+L
  # does not navigate directories reliably in Save mode; this matches the
  # actual v0.6 picker interaction already exercised by native-report-smoke.
  xdotool mousemove --window "$picker" 300 27 click 1
  xdotool key --clearmodifiers ctrl+a
  xdotool type --clearmodifiers --delay 8 "$destination"
  import -window "$picker" "$out/$stem-picker-filled.png"
  xdotool key --clearmodifiers Return
  if ! wait_gone "$picker"; then
    import -window "$picker" "$out/$stem-picker-not-closed.png" 2>/dev/null || true
    return 1
  fi
  for _ in $(seq 1 100); do
    alive
    if [[ -s "$destination" ]]; then sleep 0.8; capture_app "$stem-saved"; return 0; fi
    sleep 0.2
  done
  echo "The native save command did not produce its chosen file: $destination" >&2
  return 1
}

open_backup_wizard() {
  focus_app
  xdotool mousemove --window "$app_window" "$ENGRAM_BACKUP_X" "$ENGRAM_BACKUP_Y" click 1
  sleep 0.6
}
choose_backup_folder() {
  # showModal initially focuses Close. Its reverse-Tab boundary reaches the
  # final enabled Choose backup folder action without renderer scripting.
  focus_app
  xdotool key --clearmodifiers --delay 120 shift+Tab Return
}

# The test fixture is created only in this new controlled output directory.
source="$out/synthetic-iphone-backup"
"$cli" fixture "$source" --iphone-backup --json > "$out/fixture-receipt.json"
python3 - "$source" "$out/source-before.json" <<'PY'
import hashlib, json, pathlib, sys
root, destination = map(pathlib.Path, sys.argv[1:])
files = sorted(path for path in root.rglob('*') if path.is_file())
assert 0 < len(files) < 100
assert all(not path.is_symlink() and path.stat().st_size <= 8 * 1024 * 1024 for path in files)
snapshot = {path.relative_to(root).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest() for path in files}
destination.write_text(json.dumps(snapshot, sort_keys=True) + '\n', encoding='utf-8')
PY
"$cli" scan "$source" --out "$out/expected-backup.engram.json" --json > "$out/expected-scan.stdout.json"

echo 'Launching ordinary --demo --view gallery; no test-only application mode.'
"$app" --demo --view gallery > "$out/app.log" 2>&1 &
app_pid=$!
app_window=$(wait_window '^ENGRAM')
xdotool getwindowgeometry --shell "$app_window" > "$out/window-geometry.txt"
focus_app
sleep 2
capture_app initial-gallery
save_case "$out/demo-before-cancel.engram.json" demo-before

# Cancel the real folder picker, then close the wizard and save the still-open
# synthetic case separately. Later byte/JSON checks prove case retention.
open_backup_wizard
capture_app backup-wizard-start
choose_backup_folder
folder_title='^Choose the iPhone backup folder containing Manifest.db$'
picker=$(wait_window "$folder_title")
import -window "$picker" "$out/backup-cancel-picker.png"
xdotool windowfocus --sync "$picker"
xdotool key --clearmodifiers Escape
wait_gone "$picker"
sleep 0.6
capture_app backup-picker-cancelled
focus_app
xdotool key --clearmodifiers Escape
sleep 0.4
save_case "$out/demo-after-cancel.engram.json" demo-after

open_backup_wizard
choose_backup_folder
picker=$(wait_window "$folder_title")
echo "Observed actual folder picker: $(xdotool getwindowname "$picker")"
xdotool getwindowgeometry --shell "$picker" > "$out/backup-folder-picker-geometry.txt"
import -window "$picker" "$out/backup-folder-picker.png"
xdotool windowfocus --sync "$picker"
xdotool key --clearmodifiers ctrl+l
xdotool type --clearmodifiers --delay 8 "$source"
import -window "$picker" "$out/backup-folder-picker-filled.png"
xdotool key --clearmodifiers Return
# One Return only. If GTK navigates rather than submitting, retain its actual
# screenshot and fail for review instead of acting blindly on a different UI.
if ! wait_gone "$picker"; then
  import -window "$picker" "$out/backup-folder-picker-not-closed.png" 2>/dev/null || true
  exit 1
fi
sleep 2
capture_app backup-compatibility-before-analysis
focus_app
xdotool key --clearmodifiers shift+Tab
capture_app backup-start-analysis-focus
xdotool key --clearmodifiers Return
sleep 0.6
capture_app backup-analysis-requested
save_case "$out/actual-backup.engram.json" actual-backup

# Inspect the saved case before continuing. The selected first finding can
# already be a file, so derive keyboard control counts from its actual records.
python3 - "$out" "$source" <<'PY'
import hashlib, json, os, pathlib, sys
out, source = map(pathlib.Path, sys.argv[1:])
case = json.loads((out / 'actual-backup.engram.json').read_bytes())
expected = json.loads((out / 'expected-backup.engram.json').read_bytes())
assert case['schema_version'] == 1 and case['source_kind'] == 'ios_backup'
assert os.path.normpath(case['source_root']) == str(source.resolve())
assert case['findings'] == expected['findings']
def file_row(finding):
    return finding['category'] == 'media' and finding['rule_id'] == 'file.media' and not any(field['label'] == 'Thumbnail path' for field in finding['evidence'])
files = [finding for finding in case['findings'] if file_row(finding)]
assert files and files[0]['sha256']
first, selected = files[0], case['findings'][0]
def related_count(finding):
    return min(4, sum(item['id'] in finding['related_ids'] for item in case['findings']))
# Files -> Thumbnail paths -> Catalogue rows -> Unmatched -> optional selected
# Preview and relationship controls -> first visible file card.
tabs = 4 + (1 + related_count(selected) if file_row(selected) else 0)
reverse_tabs = related_count(first) + 1
plan = {'first_file_id': first['id'], 'first_file_path': first['path'],
        'first_file_sha256': first['sha256'], 'tabs_to_first_card': tabs,
        'reverse_tabs_to_preview': reverse_tabs}
(out / 'media-keyboard-plan.json').write_text(json.dumps(plan, indent=2) + '\n', encoding='utf-8')
print(json.dumps(plan, indent=2))
PY

# Success notices temporarily push content down; wait for their existing 6s
# auto-dismiss before using the Files tab location in the unscrolled layout.
sleep 7
focus_app
capture_app backup-gallery-before-selection
xdotool mousemove --window "$app_window" "$ENGRAM_MEDIA_FILES_X" "$ENGRAM_MEDIA_FILES_Y" click 1
sleep 0.3
tabs=$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["tabs_to_first_card"])' "$out/media-keyboard-plan.json")
reverse_tabs=$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["reverse_tabs_to_preview"])' "$out/media-keyboard-plan.json")
for _ in $(seq 1 "$tabs"); do xdotool key --clearmodifiers Tab; sleep 0.1; done
capture_app media-file-card-focus
xdotool key --clearmodifiers Return
sleep 0.4
capture_app media-file-selected
for _ in $(seq 1 "$reverse_tabs"); do xdotool key --clearmodifiers shift+Tab; sleep 0.1; done
capture_app media-preview-action-focus
xdotool key --clearmodifiers Return
sleep 2
capture_app media-preview-requested
save_case "$out/after-preview.engram.json" after-preview

python3 - "$out" "$source" <<'PY'
import hashlib, json, os, pathlib, sys
out, source = map(pathlib.Path, sys.argv[1:])
before = json.loads((out / 'demo-before-cancel.engram.json').read_bytes())
after = json.loads((out / 'demo-after-cancel.engram.json').read_bytes())
assert before == after and before['source_kind'] == 'demo'
fixture = json.loads((out / 'fixture-receipt.json').read_bytes())
assert fixture['synthetic'] is True and fixture['format'] == 'iphone_backup'
expected = json.loads((out / 'expected-backup.engram.json').read_bytes())
case_path = out / 'actual-backup.engram.json'
case_bytes = case_path.read_bytes()
case = json.loads(case_bytes)
after_preview_path = out / 'after-preview.engram.json'
after_preview_bytes = after_preview_path.read_bytes()
assert json.loads(after_preview_bytes) == case
assert case['schema_version'] == 1 and case['source_kind'] == 'ios_backup'
assert os.path.normpath(case['source_root']) == str(source.resolve())
assert case['findings'] == expected['findings'] and case['modules'] == expected['modules']
assert case['stats']['files_examined'] == 7 == expected['stats']['files_examined']
assert case['stats']['bytes_examined'] == expected['stats']['bytes_examined']
files = sorted(path for path in source.rglob('*') if path.is_file())
assert len(files) < 100
snapshot = {path.relative_to(source).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest() for path in files}
assert snapshot == json.loads((out / 'source-before.json').read_text(encoding='utf-8'))
assert all(not str(value).startswith('data:image') for finding in case['findings'] for value in finding.values())
plan = json.loads((out / 'media-keyboard-plan.json').read_text(encoding='utf-8'))
proof = {
    'native_backup_saved_case': 'verified', 'version': os.environ['ENGRAM_SMOKE_VERSION'],
    'platform': 'Linux/X11/WebKitGTK', 'synthetic_data_only': True,
    'mechanism': 'ordinary native window, renderer controls, Tauri IPC and actual XDG/GTK folder/save pickers',
    'actual_folder_picker_cancellation_completed': True,
    'cancelled_selection_preserved_saved_case': True,
    'actual_saved_case_file': case_path.name,
    'actual_saved_case_sha256': hashlib.sha256(case_bytes).hexdigest(),
    'after_preview_case_file': after_preview_path.name,
    'after_preview_case_sha256': hashlib.sha256(after_preview_bytes).hexdigest(),
    'saving_after_preview_retained_the_same_case': True,
    'source_kind': case['source_kind'], 'indexed_source_files_examined': case['stats']['files_examined'],
    'findings': len(case['findings']), 'native_findings_and_modules_match_cli_scan': True,
    'generated_source_names_and_bytes_unchanged': True,
    'selected_file_keyboard_plan': plan,
    'preview_request_keyboard_sequence_completed': True,
    'preview_ui_screenshot': 'media-preview-requested.png',
    'preview_rendering_requires_visual_review': True,
    'compatibility_ui_text_machine_parsed': False, 'preview_ui_text_machine_parsed': False,
    'not_exercised_here': ['real user backups', 'encrypted backup analysis', 'Windows/macOS native pickers',
                           'OCR', 'unsaved annotation edits', 'preview image extraction or pixel comparison']
}
(out / 'verification.json').write_text(json.dumps(proof, indent=2) + '\n', encoding='utf-8')
print(json.dumps(proof, indent=2))
PY
echo 'ENGRAM_NATIVE_BACKUP_SAVED_CASE_OK — review the actual preview screenshot separately.'
