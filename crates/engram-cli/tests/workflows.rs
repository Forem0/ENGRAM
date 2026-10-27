use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::collections::BTreeMap;

fn engram(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_engram"))
        .args(arguments)
        .output()
        .expect("the ENGRAM CLI must start")
}

fn text(path: &Path) -> &str { path.to_str().expect("temporary test paths are UTF-8") }

fn successful_json(arguments: &[&str]) -> serde_json::Value {
    let output = engram(arguments);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    serde_json::from_slice(&output.stdout).expect("--json must produce one JSON value")
}

fn source_snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(root: &Path, directory: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() { visit(root, &path, files); }
            else { files.insert(path.strip_prefix(root).unwrap().to_string_lossy().into_owned(), fs::read(path).unwrap()); }
        }
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}

#[test]
fn doctor_is_machine_readable_and_does_not_require_a_source() {
    let info = successful_json(&["doctor", "--json"]);
    assert_eq!(info["application"], "ENGRAM");
    assert_eq!(info["network_required"], false);
    assert_eq!(info["credential_validation"], false);
    assert_eq!(info["default_limits"]["max_files"], 25_000);
}

#[test]
fn fixture_scan_inspect_compare_roundtrip_preserves_source() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    successful_json(&["fixture", text(&source), "--json"]);
    let source_before = source_snapshot(&source);
    let first = temp.path().join("first.engram.json");
    let second = temp.path().join("second.engram.json");
    let first_case = successful_json(&["scan", text(&source), "--out", text(&first), "--json"]);
    assert!(first_case["findings"].as_array().unwrap().len() >= 3);
    assert!(first_case["stats"]["files_examined"].as_u64().unwrap() >= 3);
    let reopened = successful_json(&["inspect", text(&first), "--json"]);
    assert_eq!(reopened["id"], first_case["id"]);
    successful_json(&["scan", text(&source), "--out", text(&second), "--json"]);
    let comparison = successful_json(&["compare", text(&first), text(&second), "--json"]);
    assert!(comparison["diff"]["added"].as_array().unwrap().is_empty());
    assert!(comparison["diff"]["removed"].as_array().unwrap().is_empty());
    assert!(!comparison["diff"]["unchanged"].as_array().unwrap().is_empty());
    assert!(!source.join("first.engram.json").exists());
    assert_eq!(source_snapshot(&source), source_before, "case creation and SQLite reads must leave every source byte and filename unchanged");
}

#[test]
fn potential_password_never_appears_in_stdout_or_saved_case() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    fs::create_dir(&source).unwrap();
    let secret = "Synthetic-Only-DoNotUse-44!";
    let config = format!("password = \"{secret}\"\n");
    fs::write(source.join("settings.txt"), &config).unwrap();
    let saved = temp.path().join("case.engram.json");
    let output = engram(&["scan", text(&source), "--out", text(&saved), "--json"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(!String::from_utf8_lossy(&output.stdout).contains(secret));
    assert!(!String::from_utf8_lossy(&output.stderr).contains(secret));
    let case_text = fs::read_to_string(saved).unwrap();
    assert!(!case_text.contains(secret));
    let case: serde_json::Value = serde_json::from_str(&case_text).unwrap();
    assert!(case["findings"].as_array().unwrap().iter().any(|f| f["category"] == "secret"));
    assert_eq!(fs::read_to_string(source.join("settings.txt")).unwrap(), config);
}

#[test]
fn scan_refuses_to_put_its_case_in_the_source_or_overwrite_an_output() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("note.txt"), "Synthetic example.").unwrap();
    let inside = source.join("nested/case.json");
    let output = engram(&["scan", text(&source), "--out", text(&inside)]);
    assert!(!output.status.success());
    assert!(!source.join("nested").exists());
    let existing = temp.path().join("existing.json");
    fs::write(&existing, "preserve this file").unwrap();
    assert!(!engram(&["scan", text(&source), "--out", text(&existing)]).status.success());
    assert_eq!(fs::read_to_string(existing).unwrap(), "preserve this file");
}

#[test]
fn invalid_limits_and_missing_sources_fail_without_a_case() {
    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("missing");
    let saved = temp.path().join("case.json");
    assert!(!engram(&["scan", text(&missing), "--out", text(&saved)]).status.success());
    assert!(!engram(&["scan", text(&missing), "--out", text(&saved), "--max-files", "0"]).status.success());
    assert!(!saved.exists());
}

#[test]
fn demo_case_carries_an_explicit_synthetic_label() {
    let case = successful_json(&["demo", "--json"]);
    assert_eq!(case["source_kind"], "demo");
    assert!(!case["findings"].as_array().unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn human_output_escapes_hostile_filename_control_characters() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("note\u{1b}[31m.txt"), "Synthetic note").unwrap();
    let saved = temp.path().join("case.json");
    let output = engram(&["scan", text(&source), "--out", text(&saved)]);
    assert!(output.status.success());
    assert!(!output.stdout.contains(&0x1b));
    assert!(!output.stderr.contains(&0x1b));
}
