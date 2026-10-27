use engram_core::{
    compare_cases, create_demo_fixture, demo_case, read_case, scan_path, write_case, Category,
    ScanOptions, SourceKind,
};
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

fn source_hashes(root: &Path) -> BTreeMap<String, String> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| {
            (
                entry
                    .path()
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                format!("{:x}", Sha256::digest(fs::read(entry.path()).unwrap())),
            )
        })
        .collect()
}

fn plain_backup(root: &Path) {
    let mut dictionary = plist::Dictionary::new();
    dictionary.insert("IsEncrypted".into(), plist::Value::Boolean(false));
    plist::Value::Dictionary(dictionary)
        .to_file_xml(root.join("Manifest.plist"))
        .unwrap();
}

#[test]
fn synthetic_demo_runs_the_real_pipeline_and_labels_its_origin() {
    let case = demo_case();
    assert_eq!(case.source_kind, SourceKind::Demo);
    assert_eq!(case.source_root, "synthetic://engram-demo");
    assert!(
        case.findings.len() >= 25,
        "demo failed: {:?}",
        case.warnings
    );
    assert!(case.stats.files_examined >= 10);
    assert!(case
        .findings
        .iter()
        .any(|finding| !finding.related_ids.is_empty()));
    for category in [
        Category::Secret,
        Category::Media,
        Category::Document,
        Category::Browser,
        Category::Message,
        Category::Database,
    ] {
        assert!(
            case.findings
                .iter()
                .any(|finding| finding.category == category),
            "missing {category:?}"
        );
    }
    assert!(case
        .warnings
        .iter()
        .any(|warning| warning.contains("SYNTHETIC DEMO")));
    let encoded = serde_json::to_string(&case).unwrap();
    assert!(!encoded.contains("engram_demo_password_never_used_02"));
    assert!(!encoded.contains("engram_synthetic_only_not_a_real_credential_01"));
    assert!(!encoded.contains("SYNTHETIC message body"));
    assert!(!encoded.contains("private=omitted"));
}

#[test]
fn secrets_are_fully_masked_and_same_line_candidates_have_distinct_ids() {
    let source = tempfile::tempdir().unwrap();
    fs::write(
        source.path().join("settings.env"),
        "password=synthetic-sensitive-value api_key=synthetic-sensitive-value\n",
    )
    .unwrap();
    let case = scan_path(source.path(), &ScanOptions::default()).unwrap();
    let candidates: Vec<_> = case
        .findings
        .iter()
        .filter(|finding| finding.category == Category::Secret)
        .collect();
    assert_eq!(candidates.len(), 2);
    assert_ne!(candidates[0].id, candidates[1].id);
    assert!(candidates[0].related_ids.contains(&candidates[1].id));
    assert!(case.findings.iter().all(|finding| finding.sha256.is_none()));
    let encoded = serde_json::to_string(&case).unwrap();
    assert!(!encoded.contains("synthetic-sensitive-value"));
    assert!(encoded.contains("[redacted]"));
}

#[test]
fn links_exact_file_copies_but_not_similar_names() {
    let source = tempfile::tempdir().unwrap();
    fs::write(source.path().join("notes.txt"), "An ordinary note").unwrap();
    fs::write(source.path().join("notes-copy.txt"), "An ordinary note").unwrap();
    fs::write(
        source.path().join("notes-different.txt"),
        "A different note",
    )
    .unwrap();
    let case = scan_path(source.path(), &ScanOptions::default()).unwrap();
    let original = case
        .findings
        .iter()
        .find(|finding| finding.path == "notes.txt")
        .unwrap();
    let copy = case
        .findings
        .iter()
        .find(|finding| finding.path == "notes-copy.txt")
        .unwrap();
    let different = case
        .findings
        .iter()
        .find(|finding| finding.path == "notes-different.txt")
        .unwrap();
    assert!(original.related_ids.contains(&copy.id));
    assert!(!original.related_ids.contains(&different.id));
    assert_eq!(original.sha256, copy.sha256);
}

#[test]
fn read_limits_report_partial_coverage_without_loading_oversized_content() {
    let source = tempfile::tempdir().unwrap();
    for index in 0..8 {
        fs::write(
            source.path().join(format!("file-{index}.txt")),
            "password=synthetic-sensitive-value".repeat(20),
        )
        .unwrap();
    }
    let options = ScanOptions {
        max_files: 3,
        max_file_bytes: 64,
        max_total_bytes: 128,
        include_hidden: true,
    };
    let case = scan_path(source.path(), &options).unwrap();
    assert!(case.stats.files_examined <= 3);
    assert!(case.stats.bytes_examined <= 128);
    assert!(!case
        .findings
        .iter()
        .any(|finding| finding.category == Category::Secret));
    assert!(case.findings.iter().all(|finding| finding.sha256.is_none()));
    assert!(case
        .warnings
        .iter()
        .any(|warning| warning.contains("File limit")));
    assert!(case
        .warnings
        .iter()
        .any(|warning| warning.contains("metadata only")));
}

#[test]
fn total_budget_is_shared_across_files() {
    let source = tempfile::tempdir().unwrap();
    for index in 0..8 {
        fs::write(
            source.path().join(format!("file-{index}.txt")),
            "plain text file contents",
        )
        .unwrap();
    }
    let options = ScanOptions {
        max_total_bytes: 48,
        ..ScanOptions::default()
    };
    let case = scan_path(source.path(), &options).unwrap();
    assert!(case.stats.bytes_examined <= options.max_total_bytes);
    assert!(case.stats.skipped_files > 0);
    assert!(case
        .warnings
        .iter()
        .any(|warning| warning.contains("Total source-read budget")));
}

#[test]
fn hidden_and_dependency_policy_is_explicit() {
    let source = tempfile::tempdir().unwrap();
    fs::create_dir(source.path().join("node_modules")).unwrap();
    fs::write(
        source.path().join(".env"),
        "password=synthetic-hidden-value",
    )
    .unwrap();
    fs::write(
        source.path().join("node_modules/ignored.txt"),
        "password=synthetic-dependency-value",
    )
    .unwrap();
    let included = scan_path(source.path(), &ScanOptions::default()).unwrap();
    assert_eq!(
        included
            .findings
            .iter()
            .filter(|finding| finding.category == Category::Secret)
            .count(),
        1
    );
    let excluded = scan_path(
        source.path(),
        &ScanOptions {
            include_hidden: false,
            ..ScanOptions::default()
        },
    )
    .unwrap();
    assert!(excluded.findings.is_empty());
    assert!(excluded
        .warnings
        .iter()
        .any(|warning| warning.contains("Excluded")));
    assert!(scan_path(
        source.path(),
        &ScanOptions {
            max_files: 0,
            ..ScanOptions::default()
        }
    )
    .is_err());
}

#[test]
fn encrypted_backup_fails_clearly_and_leaves_source_unchanged() {
    let source = tempfile::tempdir().unwrap();
    fs::write(
        source.path().join("Manifest.db"),
        b"not-a-plaintext-database",
    )
    .unwrap();
    let mut dictionary = plist::Dictionary::new();
    dictionary.insert("IsEncrypted".into(), plist::Value::Boolean(true));
    plist::Value::Dictionary(dictionary)
        .to_file_xml(source.path().join("Manifest.plist"))
        .unwrap();
    let before = source_hashes(source.path());
    let error = scan_path(source.path(), &ScanOptions::default()).unwrap_err();
    assert!(error.to_string().contains("Encrypted iPhone backups"));
    assert_eq!(before, source_hashes(source.path()));
}

#[test]
fn backup_mapping_rejects_invalid_ids_and_preserves_logical_paths() {
    let source = tempfile::tempdir().unwrap();
    plain_backup(source.path());
    let id = "0123456789abcdef0123456789abcdef01234567";
    fs::create_dir(source.path().join(&id[..2])).unwrap();
    fs::write(
        source.path().join(&id[..2]).join(id),
        "ordinary archived document",
    )
    .unwrap();
    {
        let database = Connection::open(source.path().join("Manifest.db")).unwrap();
        database
            .execute_batch(
                "CREATE TABLE Files (fileID TEXT, domain TEXT, relativePath TEXT, flags INTEGER);",
            )
            .unwrap();
        database
            .execute(
                "INSERT INTO Files VALUES (?1,'HomeDomain','Documents/note.txt',1)",
                [id],
            )
            .unwrap();
        database
            .execute(
                "INSERT INTO Files VALUES ('../../escape','HomeDomain','Documents/bad.txt',1)",
                [],
            )
            .unwrap();
        database
            .execute(
                "INSERT INTO Files VALUES (?1,'HomeDomain','../outside.txt',1)",
                [id],
            )
            .unwrap();
    }
    let before = source_hashes(source.path());
    let case = scan_path(source.path(), &ScanOptions::default()).unwrap();
    assert_eq!(case.source_kind, SourceKind::IosBackup);
    assert!(case
        .findings
        .iter()
        .any(|finding| finding.path == "Documents/note.txt" && finding.origin == "HomeDomain"));
    assert!(case.stats.skipped_files >= 2);
    assert_eq!(before, source_hashes(source.path()));
}

#[test]
fn current_wal_rows_are_read_from_a_copy_without_source_modification() {
    let source = tempfile::tempdir().unwrap();
    let database_path = source.path().join("History");
    let connection = Connection::open(&database_path).unwrap();
    connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE urls (id INTEGER PRIMARY KEY, url TEXT, last_visit_time INTEGER);").unwrap();
    connection
        .execute(
            "INSERT INTO urls VALUES (1,?1,13370745600000000)",
            ["https://wal.example.invalid/private?secret=omitted"],
        )
        .unwrap();
    let before = source_hashes(source.path());
    let case = scan_path(source.path(), &ScanOptions::default()).unwrap();
    assert!(
        case.findings
            .iter()
            .any(|finding| finding.category == Category::Browser
                && finding.title.contains("wal.example.invalid")),
        "{:?}",
        case.warnings
    );
    assert_eq!(before, source_hashes(source.path()));
    assert!(!serde_json::to_string(&case)
        .unwrap()
        .contains("secret=omitted"));
    drop(connection);
}

#[test]
fn case_roundtrip_is_atomic_noclobber_and_source_safe() {
    let source = tempfile::tempdir().unwrap();
    fs::write(source.path().join("note.txt"), "ordinary note").unwrap();
    let case = scan_path(source.path(), &ScanOptions::default()).unwrap();
    let output = tempfile::tempdir().unwrap();
    let output_root = output.path().canonicalize().unwrap();
    let destination = output_root.join("case.json");
    write_case(&case, &destination).unwrap();
    let reopened = read_case(&destination).unwrap();
    assert_eq!(reopened.id, case.id);
    let original = fs::read(&destination).unwrap();
    assert!(write_case(&case, &destination).is_err());
    assert_eq!(original, fs::read(&destination).unwrap());
    let forbidden = source.path().join("new-directory/case.json");
    assert!(write_case(&case, &forbidden).is_err());
    assert!(!source.path().join("new-directory").exists());
    let traversal = source.path().join("new-directory/../../case.json");
    assert!(write_case(&case, &traversal).is_err());
    assert!(!source.path().join("new-directory").exists());
}

#[test]
fn invalid_case_schemas_and_oversized_inputs_are_rejected() {
    let output = tempfile::tempdir().unwrap();
    let mut case = demo_case();
    case.schema_version = 99;
    let invalid = output.path().join("invalid.json");
    fs::write(&invalid, serde_json::to_vec(&case).unwrap()).unwrap();
    assert!(read_case(&invalid).is_err());
    let oversized = output.path().join("huge.json");
    fs::File::create(&oversized)
        .unwrap()
        .set_len(65 * 1024 * 1024)
        .unwrap();
    assert!(read_case(&oversized).is_err());
}

#[test]
fn comparisons_use_stable_finding_identity_and_observed_changes() {
    let source = tempfile::tempdir().unwrap();
    fs::write(source.path().join("changed.txt"), "before").unwrap();
    fs::write(source.path().join("removed.txt"), "removed").unwrap();
    fs::write(source.path().join("unchanged.txt"), "unchanged").unwrap();
    let before = scan_path(source.path(), &ScanOptions::default()).unwrap();
    fs::write(source.path().join("changed.txt"), "after").unwrap();
    fs::remove_file(source.path().join("removed.txt")).unwrap();
    fs::write(source.path().join("added.txt"), "added").unwrap();
    let after = scan_path(source.path(), &ScanOptions::default()).unwrap();
    let diff = compare_cases(&before, &after);
    assert_eq!(diff.added.len(), 1);
    assert_eq!(diff.removed.len(), 1);
    assert_eq!(diff.changed.len(), 1);
    assert_eq!(diff.unchanged.len(), 1);
}

#[test]
fn fixture_generation_never_overwrites_existing_data() {
    let source = tempfile::tempdir().unwrap();
    fs::write(source.path().join("keep.txt"), "keep me").unwrap();
    assert!(create_demo_fixture(source.path()).is_err());
    assert_eq!(
        fs::read_to_string(source.path().join("keep.txt")).unwrap(),
        "keep me"
    );
}

#[test]
fn case_import_rejects_ambiguous_and_dangling_graph_identities() {
    let output = tempfile::tempdir().unwrap();
    let mut duplicate = demo_case();
    duplicate.findings.push(duplicate.findings[0].clone());
    let path = output.path().join("duplicate.json");
    fs::write(&path, serde_json::to_vec(&duplicate).unwrap()).unwrap();
    assert!(read_case(&path)
        .unwrap_err()
        .to_string()
        .contains("duplicate finding"));
    let mut dangling = demo_case();
    dangling.findings[0].related_ids.push("unknown-node".into());
    let path = output.path().join("dangling.json");
    fs::write(&path, serde_json::to_vec(&dangling).unwrap()).unwrap();
    assert!(read_case(&path)
        .unwrap_err()
        .to_string()
        .contains("dangling"));
}

#[test]
fn comparison_notices_catalogue_state_changes_without_file_changes() {
    let before = demo_case();
    let mut after = before.clone();
    let finding = after
        .findings
        .iter_mut()
        .find(|finding| finding.rule_id == "media.photos_catalogue")
        .unwrap();
    let identity = finding.id.clone();
    let state = finding
        .evidence
        .iter_mut()
        .find(|fact| fact.label == "Catalogue state")
        .unwrap();
    state.value = "Changed synthetic catalogue flag".into();
    let difference = compare_cases(&before, &after);
    assert_eq!(difference.changed, vec![identity]);
}

#[test]
fn empty_directory_trees_have_a_traversal_budget() {
    let source = tempfile::tempdir().unwrap();
    for index in 0..1100 {
        fs::create_dir(source.path().join(format!("directory-{index:04}"))).unwrap();
    }
    let case = scan_path(
        source.path(),
        &ScanOptions {
            max_files: 1,
            ..ScanOptions::default()
        },
    )
    .unwrap();
    assert_eq!(case.stats.files_examined, 0);
    assert!(case
        .warnings
        .iter()
        .any(|warning| warning.contains("Directory-entry")));
}

#[cfg(unix)]
#[test]
fn symlink_sources_are_excluded_and_output_aliases_cannot_write_into_source() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let source = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(
        outside.path().join("secret.txt"),
        "password=synthetic-outside-sensitive",
    )
    .unwrap();
    symlink(
        outside.path().join("secret.txt"),
        source.path().join("secret-link.txt"),
    )
    .unwrap();
    symlink(outside.path(), source.path().join("outside-directory")).unwrap();
    let case = scan_path(source.path(), &ScanOptions::default()).unwrap();
    assert!(case.findings.is_empty());
    assert!(scan_path(
        &source.path().join("outside-directory"),
        &ScanOptions::default()
    )
    .is_err());
    let output_alias = outside.path().join("source-alias");
    symlink(source.path(), &output_alias).unwrap();
    assert!(write_case(&case, &output_alias.join("new/case.json")).is_err());
    assert!(!source.path().join("new").exists());
    let safe_case = outside.path().canonicalize().unwrap().join("case.json");
    write_case(&case, &safe_case).unwrap();
    assert_eq!(
        fs::metadata(safe_case).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn backup_activity_ids_include_domains() {
    let source = tempfile::tempdir().unwrap();
    plain_backup(source.path());
    let ids = [
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    ];
    let manifest = Connection::open(source.path().join("Manifest.db")).unwrap();
    manifest
        .execute_batch(
            "CREATE TABLE Files (fileID TEXT, domain TEXT, relativePath TEXT, flags INTEGER);",
        )
        .unwrap();
    for (index, id) in ids.iter().enumerate() {
        fs::create_dir_all(source.path().join(&id[..2])).unwrap();
        let database = Connection::open(source.path().join(&id[..2]).join(id)).unwrap();
        database.execute_batch("CREATE TABLE urls(id INTEGER, url TEXT, last_visit_time INTEGER); INSERT INTO urls VALUES(1,'https://same.example.invalid',13370745600000000);").unwrap();
        manifest
            .execute(
                "INSERT INTO Files VALUES(?1,?2,'Library/History',1)",
                params![id, format!("AppDomain-demo-{index}")],
            )
            .unwrap();
    }
    drop(manifest);
    let case = scan_path(source.path(), &ScanOptions::default()).unwrap();
    let browser: Vec<_> = case
        .findings
        .iter()
        .filter(|finding| finding.category == Category::Browser)
        .collect();
    assert_eq!(browser.len(), 2);
    assert_ne!(browser[0].id, browser[1].id);
}
