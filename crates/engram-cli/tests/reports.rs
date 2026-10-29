use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn engram(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_engram"))
        .args(arguments)
        .output()
        .expect("the ENGRAM CLI must start")
}

fn text(path: &Path) -> &str {
    path.to_str().expect("temporary test paths are UTF-8")
}

fn successful_json(arguments: &[&str]) -> serde_json::Value {
    let output = engram(arguments);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("--json emits one receipt")
}

fn reviewed_case(path: &Path) -> serde_json::Value {
    let mut case = successful_json(&["demo", "--out", text(path), "--json"]);
    let id = case["findings"][0]["id"].as_str().unwrap().to_owned();
    case["findings"][0]["title"] = "Exported synthetic finding".into();
    case["findings"][0]["path"] = "privateparent/selectedimage".into();
    case["findings"][0]["summary"] = "Optional source evidence detail".into();
    case["findings"][1]["title"] = "Excluded synthetic finding".into();
    case["review"] = serde_json::json!({
        (id): {
            "bookmarked": true,
            "status": "needs_review",
            "note": "Optional analyst annotation"
        }
    });
    fs::write(path, serde_json::to_vec_pretty(&case).unwrap()).unwrap();
    case
}

// Read the produced CSV as a consumer would, including doubled quotes and
// embedded newlines. Keeping the parser here avoids testing by line splitting.
fn csv_rows(value: &str) -> Vec<Vec<String>> {
    let mut characters = value.strip_prefix('\u{feff}').unwrap().chars().peekable();
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    while let Some(character) = characters.next() {
        match character {
            '"' if quoted && characters.peek() == Some(&'"') => {
                characters.next();
                cell.push('"');
            }
            '"' => quoted = !quoted,
            ',' if !quoted => row.push(std::mem::take(&mut cell)),
            '\r' | '\n' if !quoted => {
                if character == '\r' && characters.peek() == Some(&'\n') {
                    characters.next();
                }
                row.push(std::mem::take(&mut cell));
                rows.push(std::mem::take(&mut row));
            }
            character => cell.push(character),
        }
    }
    assert!(!quoted, "CSV must end outside a quoted field");
    if !cell.is_empty() || !row.is_empty() {
        row.push(cell);
        rows.push(row);
    }
    rows
}

#[test]
fn selected_markdown_report_defaults_to_minimal_fields_and_preserves_the_case() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("reviewed.engram.json");
    let case = reviewed_case(&input);
    let original = fs::read(&input).unwrap();
    let id = case["findings"][0]["id"].as_str().unwrap();
    let output = temporary.path().join("selected.report.md");
    let receipt = successful_json(&[
        "export",
        text(&input),
        "--out",
        text(&output),
        "--finding",
        id,
        "--json",
    ]);
    let report = fs::read_to_string(&output).unwrap();
    assert_eq!(receipt["format"], "markdown");
    assert_eq!(receipt["finding_count"], 1);
    assert_eq!(receipt["bytes"].as_u64().unwrap(), report.len() as u64);
    let digest = receipt["sha256"].as_str().unwrap();
    assert_eq!(digest.len(), 64);
    assert!(digest.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert!(report.contains("Exported synthetic finding"));
    assert!(!report.contains("Excluded synthetic finding"));
    assert!(report.contains("selectedimage"));
    assert!(!report.contains("privateparent"));
    assert!(!report.contains("Optional analyst annotation"));
    assert!(!report.contains("Optional source evidence detail"));
    assert_eq!(fs::read(&input).unwrap(), original);
}

#[test]
fn bookmarked_csv_contains_only_saved_bookmarks_and_neutralizes_formula_cells() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("reviewed.engram.json");
    let mut case = reviewed_case(&input);
    let formula = " =HYPERLINK(\"https://example.invalid\",\"synthetic\")";
    case["findings"][0]["title"] = formula.into();
    fs::write(&input, serde_json::to_vec_pretty(&case).unwrap()).unwrap();
    let output = temporary.path().join("bookmarks.csv");
    let receipt = successful_json(&[
        "export",
        text(&input),
        "--out",
        text(&output),
        "--format",
        "csv",
        "--bookmarked",
        "--json",
    ]);
    assert_eq!(receipt["format"], "csv");
    assert_eq!(receipt["finding_count"], 1);
    let bytes = fs::read(output).unwrap();
    assert!(bytes.starts_with(&[0xef, 0xbb, 0xbf]));
    let csv = String::from_utf8(bytes).unwrap();
    assert!(csv.ends_with("\r\n"));
    let rows = csv_rows(&csv);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].len(), rows[1].len());
    let formula_cell = rows[1]
        .iter()
        .find(|cell| cell.contains("HYPERLINK"))
        .unwrap();
    assert!(formula_cell.trim_start().starts_with('\''));
    assert!(!csv.contains("Optional analyst annotation"));
    assert!(!csv.contains("privateparent"));
}

#[test]
fn notes_paths_and_evidence_require_explicit_opt_ins() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("reviewed.engram.json");
    reviewed_case(&input);
    let output = temporary.path().join("detailed.report.md");
    successful_json(&[
        "export",
        text(&input),
        "--out",
        text(&output),
        "--bookmarked",
        "--include-notes",
        "--include-paths",
        "--include-details",
        "--json",
    ]);
    let report = fs::read_to_string(output).unwrap();
    assert!(report.contains("Optional analyst annotation"));
    assert!(report.contains("privateparent"));
    assert!(report.contains("Optional source evidence detail"));
}

#[test]
fn opted_in_csv_notes_roundtrip_unicode_quotes_commas_and_newlines() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("reviewed.engram.json");
    let mut case = reviewed_case(&input);
    let id = case["findings"][0]["id"].as_str().unwrap().to_owned();
    let note = "Проверено, \"synthetic\" only\nSecond line";
    case["review"][id.as_str()]["note"] = note.into();
    fs::write(&input, serde_json::to_vec_pretty(&case).unwrap()).unwrap();
    let output = temporary.path().join("notes.csv");
    successful_json(&[
        "export",
        text(&input),
        "--out",
        text(&output),
        "--format",
        "csv",
        "--bookmarked",
        "--include-notes",
        "--json",
    ]);
    let rows = csv_rows(&fs::read_to_string(output).unwrap());
    assert_eq!(rows.len(), 2);
    let note_column = rows[0]
        .iter()
        .position(|cell| cell == "analyst_note")
        .unwrap();
    assert_eq!(rows[1][note_column], note);
}

#[test]
fn empty_bookmarks_unknown_ids_and_duplicate_ids_never_fall_back_to_all_findings() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("case.engram.json");
    let case = successful_json(&["demo", "--out", text(&input), "--json"]);
    let id = case["findings"][0]["id"].as_str().unwrap();
    let empty = temporary.path().join("empty.md");
    assert!(!engram(&[
        "export",
        text(&input),
        "--out",
        text(&empty),
        "--bookmarked"
    ])
    .status
    .success());
    assert!(!empty.exists());
    let unknown = temporary.path().join("unknown.md");
    assert!(!engram(&[
        "export",
        text(&input),
        "--out",
        text(&unknown),
        "--finding",
        "unknown-synthetic-id",
    ])
    .status
    .success());
    assert!(!unknown.exists());
    let duplicate = temporary.path().join("duplicate.md");
    assert!(!engram(&[
        "export",
        text(&input),
        "--out",
        text(&duplicate),
        "--finding",
        id,
        "--finding",
        id,
    ])
    .status
    .success());
    assert!(!duplicate.exists());
}

#[test]
fn export_rejects_conflicting_selectors_invalid_formats_and_control_ids() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("case.engram.json");
    let case = successful_json(&["demo", "--out", text(&input), "--json"]);
    let id = case["findings"][0]["id"].as_str().unwrap();
    let output = temporary.path().join("not-written.md");
    for flags in [
        vec!["--finding", id, "--bookmarked"],
        vec!["--format", "html"],
        vec!["--finding", "hostile\nidentity"],
    ] {
        let mut arguments = vec!["export", text(&input), "--out", text(&output)];
        arguments.extend(flags);
        assert!(!engram(&arguments).status.success());
        assert!(!output.exists());
    }
}

#[test]
fn export_does_not_overwrite_the_case_existing_outputs_or_recorded_source() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("case.engram.json");
    let mut case = reviewed_case(&input);
    let source = temporary.path().join("source");
    fs::create_dir(&source).unwrap();
    case["source_root"] = source
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .into_owned()
        .into();
    fs::write(&input, serde_json::to_vec_pretty(&case).unwrap()).unwrap();
    let original = fs::read(&input).unwrap();
    assert!(!engram(&["export", text(&input), "--out", text(&input)])
        .status
        .success());
    assert_eq!(fs::read(&input).unwrap(), original);
    let existing = temporary.path().join("existing.md");
    fs::write(&existing, "preserve this output").unwrap();
    assert!(!engram(&["export", text(&input), "--out", text(&existing)])
        .status
        .success());
    assert_eq!(
        fs::read_to_string(existing).unwrap(),
        "preserve this output"
    );
    let inside = source.join("new/report.md");
    assert!(!engram(&["export", text(&input), "--out", text(&inside)])
        .status
        .success());
    assert!(!source.join("new").exists());
    assert_eq!(fs::read_dir(source).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn report_destination_controls_are_rejected_before_writing() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("case.engram.json");
    successful_json(&["demo", "--out", text(&input), "--json"]);
    let output = temporary.path().join("report\u{1b}[31m.md");
    let receipt = engram(&["export", text(&input), "--out", text(&output)]);
    assert!(!receipt.status.success());
    assert!(!output.exists());
    assert!(receipt.stdout.is_empty());
    assert!(!receipt.stdout.contains(&0x1b));
    assert!(!receipt.stderr.contains(&0x1b));
    assert!(String::from_utf8_lossy(&receipt.stderr).contains("control characters"));
}
