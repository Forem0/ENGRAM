use base64::{engine::general_purpose::STANDARD, Engine as _};
use engram_core::{
    create_demo_backup_fixture, create_demo_fixture, inspect_backup, scan_path,
    scan_path_with_media, BackupInspectionState, Category, ScanOptions, SourceKind,
    MAX_MEDIA_INPUT_BYTES,
};
use image::{DynamicImage, ImageFormat, RgbImage};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::{fs, io::Cursor, path::Path};

fn png(path: &Path, width: u32, height: u32) {
    let image = RgbImage::from_fn(width, height, |x, y| {
        image::Rgb([(x % 255) as u8, (y % 255) as u8, ((x + y) % 255) as u8])
    });
    DynamicImage::ImageRgb8(image)
        .save_with_format(path, ImageFormat::Png)
        .unwrap();
}

fn metadata(root: &Path, encrypted: Option<plist::Value>) {
    let mut values = plist::Dictionary::new();
    if let Some(value) = encrypted {
        values.insert("IsEncrypted".into(), value);
    }
    plist::Value::Dictionary(values)
        .to_file_xml(root.join("Manifest.plist"))
        .unwrap();
}

fn index(root: &Path) -> Connection {
    let database = Connection::open(root.join("Manifest.db")).unwrap();
    database
        .execute_batch(
            "CREATE TABLE Files(fileID TEXT, domain TEXT, relativePath TEXT, flags INTEGER);",
        )
        .unwrap();
    database
}

#[test]
fn preview_is_generated_from_scanned_bytes_without_modifying_the_source() {
    let folder = tempfile::tempdir().unwrap();
    let source = folder.path().join("image.png");
    png(&source, 120, 80);
    let original = fs::read(&source).unwrap();
    let (case, access) = scan_path_with_media(folder.path(), &ScanOptions::default()).unwrap();
    assert_eq!(access.case_id(), case.id);
    let finding = case
        .findings
        .iter()
        .find(|finding| finding.rule_id == "file.media")
        .unwrap();
    let preview = access.preview(&finding.id).unwrap();
    assert_eq!(preview.finding_id, finding.id);
    assert_eq!((preview.source_width, preview.source_height), (120, 80));
    assert_eq!((preview.width, preview.height), (120, 80));
    assert_eq!(
        preview.source_sha256,
        format!("{:x}", Sha256::digest(&original))
    );
    let rendered = STANDARD
        .decode(
            preview
                .data_url
                .strip_prefix("data:image/png;base64,")
                .unwrap(),
        )
        .unwrap();
    let decoded = image::load_from_memory(&rendered).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (120, 80));
    assert_eq!(fs::read(&source).unwrap(), original);
    let serialized = serde_json::to_string(&case).unwrap();
    assert!(!serialized.contains("data:image"));
    assert!(!serialized.contains("source_width"));
}

#[test]
fn oversized_image_is_resized_to_a_bounded_preview() {
    let folder = tempfile::tempdir().unwrap();
    png(&folder.path().join("wide.png"), 2048, 600);
    let (case, access) = scan_path_with_media(folder.path(), &ScanOptions::default()).unwrap();
    let preview = access.preview(&case.findings[0].id).unwrap();
    assert_eq!((preview.source_width, preview.source_height), (2048, 600));
    assert_eq!((preview.width, preview.height), (1024, 300));
}

#[test]
fn jpeg_input_is_reencoded_as_png() {
    let folder = tempfile::tempdir().unwrap();
    let image = DynamicImage::ImageRgb8(RgbImage::new(30, 20));
    image
        .save_with_format(folder.path().join("photo.jpg"), ImageFormat::Jpeg)
        .unwrap();
    let (case, access) = scan_path_with_media(folder.path(), &ScanOptions::default()).unwrap();
    let preview = access.preview(&case.findings[0].id).unwrap();
    assert!(preview.data_url.starts_with("data:image/png;base64,"));
    assert_eq!((preview.source_width, preview.source_height), (30, 20));
}

#[test]
fn changed_content_is_rejected_even_when_length_is_unchanged() {
    let folder = tempfile::tempdir().unwrap();
    let source = folder.path().join("image.png");
    png(&source, 10, 10);
    let (case, access) = scan_path_with_media(folder.path(), &ScanOptions::default()).unwrap();
    let mut changed = fs::read(&source).unwrap();
    let position = changed.len() / 2;
    changed[position] ^= 1;
    fs::write(&source, changed).unwrap();
    assert!(access
        .preview(&case.findings[0].id)
        .unwrap_err()
        .to_string()
        .contains("changed"));
}

#[test]
fn unknown_ids_and_database_catalogue_records_do_not_grant_file_access() {
    let folder = tempfile::tempdir().unwrap();
    create_demo_fixture(folder.path()).unwrap();
    let (case, access) = scan_path_with_media(folder.path(), &ScanOptions::default()).unwrap();
    assert!(access.preview("../../outside.png").is_err());
    for finding in case
        .findings
        .iter()
        .filter(|finding| finding.rule_id != "file.media")
    {
        assert!(access.preview(&finding.id).is_err());
    }
}

#[test]
fn file_bytes_outside_preview_budget_are_not_authorized() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("too-large.png");
    fs::File::create(&path)
        .unwrap()
        .set_len(MAX_MEDIA_INPUT_BYTES + 1)
        .unwrap();
    let options = ScanOptions {
        max_file_bytes: MAX_MEDIA_INPUT_BYTES + 1,
        max_total_bytes: MAX_MEDIA_INPUT_BYTES + 1,
        ..ScanOptions::default()
    };
    let (case, access) = scan_path_with_media(folder.path(), &options).unwrap();
    assert!(case.findings[0].sha256.is_some());
    assert!(access.preview(&case.findings[0].id).is_err());
}

#[test]
fn unsupported_content_cannot_be_returned_as_a_renderable_image() {
    for bytes in [
        b"<svg xmlns='http://www.w3.org/2000/svg'><script>alert(1)</script></svg>".as_slice(),
        b"GIF89a\x01\0\x01\0".as_slice(),
    ] {
        let folder = tempfile::tempdir().unwrap();
        fs::write(folder.path().join("disguised.png"), bytes).unwrap();
        let (case, access) = scan_path_with_media(folder.path(), &ScanOptions::default()).unwrap();
        assert!(access.preview(&case.findings[0].id).is_err());
    }
}

#[test]
fn excessive_source_pixels_are_rejected_before_full_decode() {
    let folder = tempfile::tempdir().unwrap();
    // Encoding a flat image is compact on disk; its dimensions exceed the
    // independent 16-million-pixel decode bound despite fitting the byte limit.
    let image = DynamicImage::ImageRgb8(RgbImage::new(4001, 4000));
    image
        .save_with_format(folder.path().join("pixel-limit.png"), ImageFormat::Png)
        .unwrap();
    let (case, access) = scan_path_with_media(folder.path(), &ScanOptions::default()).unwrap();
    assert!(access
        .preview(&case.findings[0].id)
        .unwrap_err()
        .to_string()
        .contains("16 million"));
}

#[test]
fn backup_preflight_counts_index_paths_and_reads_optional_device_metadata() {
    let folder = tempfile::tempdir().unwrap();
    create_demo_backup_fixture(folder.path()).unwrap();
    let inspection = inspect_backup(folder.path()).unwrap();
    assert_eq!(inspection.state, BackupInspectionState::Ready);
    assert_eq!(inspection.encrypted, Some(false));
    assert_eq!(inspection.indexed_files, 7);
    assert_eq!(
        inspection
            .categories
            .iter()
            .map(|category| category.count)
            .sum::<u64>(),
        7
    );
    assert_eq!(
        inspection
            .categories
            .iter()
            .find(|category| category.id == "media")
            .unwrap()
            .count,
        3
    );
    assert_eq!(
        inspection.device_name.as_deref(),
        Some("ENGRAM synthetic fixture")
    );
    assert!(inspection
        .warnings
        .iter()
        .any(|warning| warning.contains("not verified media")));
    let (case, access) = scan_path_with_media(folder.path(), &ScanOptions::default()).unwrap();
    assert_eq!(case.source_kind, SourceKind::IosBackup);
    let media = case
        .findings
        .iter()
        .find(|finding| finding.category == Category::Media && finding.rule_id == "file.media")
        .unwrap();
    assert!(access.preview(&media.id).is_ok());
}

#[test]
fn backup_fixture_previews_are_visible_calibration_cards_with_identical_source_bytes() {
    let folder = tempfile::tempdir().unwrap();
    create_demo_backup_fixture(folder.path()).unwrap();
    let (case, access) = scan_path_with_media(folder.path(), &ScanOptions::default()).unwrap();
    let media: Vec<_> = case
        .findings
        .iter()
        .filter(|finding| finding.rule_id == "file.media")
        .collect();
    assert_eq!(media.len(), 3);
    let mut source_bytes = Vec::new();
    for finding in &media {
        let logical = format!("synthetic:{}:{}", finding.origin, finding.path);
        let identifier = format!("{:x}", Sha256::digest(logical.as_bytes()))[..40].to_owned();
        let bytes = fs::read(folder.path().join(&identifier[..2]).join(&identifier)).unwrap();
        let expected_hash = format!("{:x}", Sha256::digest(&bytes));
        assert_eq!(finding.sha256.as_deref(), Some(expected_hash.as_str()));
        let preview = access.preview(&finding.id).unwrap();
        assert_eq!((preview.source_width, preview.source_height), (640, 400));
        assert_eq!((preview.width, preview.height), (640, 400));
        assert_eq!(preview.source_sha256, expected_hash);
        let rendered = STANDARD
            .decode(
                preview
                    .data_url
                    .strip_prefix("data:image/png;base64,")
                    .unwrap(),
            )
            .unwrap();
        let image = image::load_from_memory(&rendered).unwrap().to_rgba8();
        assert_eq!(image.dimensions(), (640, 400));
        assert!(image.pixels().all(|pixel| pixel[3] == 255));
        assert_eq!(*image.get_pixel(0, 0), image::Rgba([244, 240, 229, 255]));
        assert_eq!(*image.get_pixel(30, 30), image::Rgba([36, 107, 254, 255]));
        assert_eq!(*image.get_pixel(28, 150), image::Rgba([25, 197, 199, 255]));
        assert_ne!(image.get_pixel(80, 180), image.get_pixel(320, 232));
        source_bytes.push(bytes);
    }
    assert!(source_bytes.windows(2).all(|pair| pair[0] == pair[1]));
    for finding in &media {
        assert!(media
            .iter()
            .filter(|other| other.id != finding.id)
            .all(|other| finding.related_ids.contains(&other.id)));
    }
}

#[test]
fn blank_or_control_only_optional_device_metadata_is_absent() {
    let folder = tempfile::tempdir().unwrap();
    create_demo_backup_fixture(folder.path()).unwrap();
    let mut info = plist::Dictionary::new();
    info.insert("Device Name".into(), plist::Value::String(" \t\n ".into()));
    info.insert(
        "Product Version".into(),
        plist::Value::String(" \r\n ".into()),
    );
    plist::Value::Dictionary(info)
        .to_file_xml(folder.path().join("Info.plist"))
        .unwrap();
    let inspection = inspect_backup(folder.path()).unwrap();
    assert_eq!(inspection.state, BackupInspectionState::Ready);
    assert_eq!(inspection.device_name, None);
    assert_eq!(inspection.product_version, None);
}

#[cfg(unix)]
#[test]
fn blank_backup_directory_label_uses_a_nonempty_fallback() {
    let parent = tempfile::tempdir().unwrap();
    let source = parent.path().join(" \t ");
    create_demo_backup_fixture(&source).unwrap();
    let inspection = inspect_backup(&source).unwrap();
    assert_eq!(inspection.state, BackupInspectionState::Ready);
    assert_eq!(inspection.name, "Selected backup");
}

#[test]
fn encrypted_missing_and_unknown_encryption_states_remain_unavailable() {
    for encrypted in [
        Some(plist::Value::Boolean(true)),
        None,
        Some(plist::Value::String("false".into())),
    ] {
        let folder = tempfile::tempdir().unwrap();
        metadata(folder.path(), encrypted);
        drop(index(folder.path()));
        let inspection = inspect_backup(folder.path()).unwrap();
        assert_eq!(inspection.state, BackupInspectionState::Unavailable);
        assert_eq!(inspection.indexed_files, 0);
        assert!(inspection.categories.is_empty());
        assert!(scan_path(folder.path(), &ScanOptions::default()).is_err());
    }
}

#[test]
fn ordinary_folder_and_malformed_plist_are_not_empty_successful_backups() {
    let folder = tempfile::tempdir().unwrap();
    assert_eq!(
        inspect_backup(folder.path()).unwrap().state,
        BackupInspectionState::Unavailable
    );
    fs::write(folder.path().join("Manifest.plist"), b"not a property list").unwrap();
    assert_eq!(
        inspect_backup(folder.path()).unwrap().state,
        BackupInspectionState::Unavailable
    );
}

#[test]
fn unsupported_files_schema_and_familiar_looking_views_are_rejected() {
    for schema in [
        "CREATE TABLE Files(wrong TEXT)",
        "CREATE VIEW Files AS SELECT 'a' AS fileID, 'b' AS domain, 'c' AS relativePath, 1 AS flags",
    ] {
        let folder = tempfile::tempdir().unwrap();
        metadata(folder.path(), Some(plist::Value::Boolean(false)));
        let database = Connection::open(folder.path().join("Manifest.db")).unwrap();
        database.execute_batch(schema).unwrap();
        drop(database);
        let inspection = inspect_backup(folder.path()).unwrap();
        assert_eq!(inspection.state, BackupInspectionState::Unavailable);
        assert_eq!(inspection.indexed_files, 0);
    }
}

#[test]
fn supported_empty_index_is_distinct_from_an_unsupported_backup() {
    let folder = tempfile::tempdir().unwrap();
    metadata(folder.path(), Some(plist::Value::Boolean(false)));
    drop(index(folder.path()));
    let inspection = inspect_backup(folder.path()).unwrap();
    assert_eq!(inspection.state, BackupInspectionState::Ready);
    assert_eq!(inspection.indexed_files, 0);
}

#[test]
fn invalid_index_identifiers_and_parent_paths_are_excluded_and_reported() {
    let folder = tempfile::tempdir().unwrap();
    metadata(folder.path(), Some(plist::Value::Boolean(false)));
    let database = index(folder.path());
    database
        .execute(
            "INSERT INTO Files VALUES (?1, 'HomeDomain', '../outside.png', 1)",
            ["a".repeat(40)],
        )
        .unwrap();
    database
        .execute(
            "INSERT INTO Files VALUES ('invalid', 'HomeDomain', 'good.png', 1)",
            [],
        )
        .unwrap();
    database
        .execute(
            "INSERT INTO Files VALUES (?1, 'HomeDomain', 'good.png', 1)",
            ["b".repeat(40)],
        )
        .unwrap();
    drop(database);
    let inspection = inspect_backup(folder.path()).unwrap();
    assert_eq!(inspection.state, BackupInspectionState::Ready);
    assert_eq!(inspection.indexed_files, 1);
    assert!(inspection
        .warnings
        .iter()
        .any(|warning| warning.contains("Excluded 2")));
}

#[test]
fn preflight_reads_available_wal_without_modifying_original_database_files() {
    let folder = tempfile::tempdir().unwrap();
    metadata(folder.path(), Some(plist::Value::Boolean(false)));
    let database = index(folder.path());
    database
        .execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;")
        .unwrap();
    database
        .execute(
            "INSERT INTO Files VALUES (?1, 'HomeDomain', 'wal-note.txt', 1)",
            ["c".repeat(40)],
        )
        .unwrap();
    let before = fs::read(folder.path().join("Manifest.db")).unwrap();
    let wal_before = fs::read(folder.path().join("Manifest.db-wal")).unwrap();
    let inspection = inspect_backup(folder.path()).unwrap();
    assert_eq!(inspection.state, BackupInspectionState::Ready);
    assert_eq!(inspection.indexed_files, 1);
    assert_eq!(fs::read(folder.path().join("Manifest.db")).unwrap(), before);
    assert_eq!(
        fs::read(folder.path().join("Manifest.db-wal")).unwrap(),
        wal_before
    );
    drop(database);
}

#[test]
fn synthetic_backup_fixture_never_overwrites_an_existing_directory() {
    let folder = tempfile::tempdir().unwrap();
    fs::write(folder.path().join("keep.txt"), b"keep").unwrap();
    assert!(create_demo_backup_fixture(folder.path()).is_err());
    assert_eq!(fs::read(folder.path().join("keep.txt")).unwrap(), b"keep");
}

#[test]
fn oversized_index_and_ill_typed_records_are_unavailable() {
    let folder = tempfile::tempdir().unwrap();
    metadata(folder.path(), Some(plist::Value::Boolean(false)));
    fs::File::create(folder.path().join("Manifest.db"))
        .unwrap()
        .set_len(8 * 1024 * 1024 + 1)
        .unwrap();
    assert_eq!(
        inspect_backup(folder.path()).unwrap().state,
        BackupInspectionState::Unavailable
    );
    fs::remove_file(folder.path().join("Manifest.db")).unwrap();
    let database = index(folder.path());
    database
        .execute(
            "INSERT INTO Files VALUES (NULL, 'HomeDomain', 'missing.png', 1)",
            [],
        )
        .unwrap();
    drop(database);
    assert_eq!(
        inspect_backup(folder.path()).unwrap().state,
        BackupInspectionState::Unavailable
    );
}

#[cfg(unix)]
#[test]
fn replacement_file_and_root_symlinks_cannot_expand_a_captured_source() {
    use std::os::unix::fs::symlink;
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("source");
    fs::create_dir(&root).unwrap();
    let outside = parent.path().join("outside.png");
    png(&outside, 3, 2);
    fs::copy(&outside, root.join("image.png")).unwrap();
    let (case, access) = scan_path_with_media(&root, &ScanOptions::default()).unwrap();
    fs::remove_file(root.join("image.png")).unwrap();
    symlink(&outside, root.join("image.png")).unwrap();
    assert!(access.preview(&case.findings[0].id).is_err());
    fs::remove_file(root.join("image.png")).unwrap();
    fs::rename(&root, parent.path().join("moved")).unwrap();
    symlink(parent.path().join("moved"), &root).unwrap();
    assert!(access.preview(&case.findings[0].id).is_err());
    assert!(inspect_backup(&root).is_err());
}

#[test]
fn metadata_is_removed_when_preview_pixels_are_reencoded() {
    let folder = tempfile::tempdir().unwrap();
    let source = folder.path().join("tagged.jpg");
    let image = DynamicImage::ImageRgb8(RgbImage::new(12, 8));
    let mut bytes = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Jpeg)
        .unwrap();
    let marker = b"SYNTHETIC_METADATA_SHOULD_NOT_BE_COPIED";
    let mut tagged = bytes[..2].to_vec();
    tagged.extend_from_slice(&[0xff, 0xfe]);
    tagged.extend_from_slice(&u16::try_from(marker.len() + 2).unwrap().to_be_bytes());
    tagged.extend_from_slice(marker);
    tagged.extend_from_slice(&bytes[2..]);
    fs::write(&source, tagged).unwrap();
    let (case, access) = scan_path_with_media(folder.path(), &ScanOptions::default()).unwrap();
    let preview = access.preview(&case.findings[0].id).unwrap();
    let output = STANDARD
        .decode(
            preview
                .data_url
                .strip_prefix("data:image/png;base64,")
                .unwrap(),
        )
        .unwrap();
    assert!(!output.windows(marker.len()).any(|window| window == marker));
}
