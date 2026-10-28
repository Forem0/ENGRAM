use crate::{scan_path, Case, CaseStats, EngramError, ScanOptions, SourceKind};
use rusqlite::{params, Connection};
use std::{
    fs,
    path::Path,
    time::{Duration, SystemTime},
};

/// Write only a new or empty directory. All content is synthetic and intentionally
/// contains obvious demo-shaped values, never working service credentials.
pub fn create_demo_fixture(destination: &Path) -> Result<(), EngramError> {
    if let Ok(metadata) = destination.symlink_metadata() {
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(EngramError::Input(
                "Demo destination must be a new or empty regular directory.".into(),
            ));
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(EngramError::Input(
                    "Demo destination cannot be a reparse point.".into(),
                ));
            }
        }
        if fs::read_dir(destination)?.next().is_some() {
            return Err(EngramError::Input(
                "Demo destination is not empty. Existing files will never be overwritten.".into(),
            ));
        }
    }
    fs::create_dir_all(destination)?;
    let directories = [
        "documents",
        "archives",
        "configuration",
        "diagnostics",
        "browser/chromium",
        "browser/safari",
        "messages",
        "PhotoData/Thumbnails/V2/DCIM/100APPLE",
        "media/DCIM/100APPLE",
    ];
    for directory in directories {
        fs::create_dir_all(destination.join(directory))?;
    }
    fs::write(destination.join("README.txt"), "ENGRAM SYNTHETIC FIXTURE\nThis directory was generated for a demo. No content was recovered from a device. All credentials are deliberately fake.\n")?;
    let notes = "SYNTHETIC project notes\nArchive inspection checklist\n1. Inventory files.\n2. Compare two snapshots.\n3. Treat unmatched artifacts as inconclusive.\n";
    fs::write(destination.join("documents/research-notes.txt"), notes)?;
    fs::write(destination.join("archives/research-notes-copy.txt"), notes)?;
    fs::write(
        destination.join("documents/old-export.csv"),
        "record,source,status\n1,synthetic archive,present\n2,synthetic thumbnail,unmatched\n",
    )?;
    fs::write(destination.join("configuration/demo.env"), "# SYNTHETIC ONLY - not real credentials\napi_key=engram_synthetic_only_not_a_real_credential_01\npassword=engram_demo_password_never_used_02\n")?;
    fs::write(destination.join("diagnostics/old-session.log"), "SYNTHETIC diagnostics\napi_key=engram_synthetic_only_not_a_real_credential_01\nclient_secret=engram_synthetic_only_not_a_real_credential_03\n")?;
    fs::write(destination.join("archives/previous-config.txt"), "SYNTHETIC config snapshot\npassword=engram_demo_password_never_used_02\ntoken=ghp_ENGRAM_SYNTHETIC_NOT_A_REAL_KEY_000000\n")?;
    // A valid, tiny PNG is sufficient for testing byte identity; it is not a real photo.
    let png: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 4,
        0, 0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 218, 99, 100, 248, 15, 0, 1, 5,
        1, 1, 39, 24, 227, 102, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    fs::write(destination.join("media/DCIM/100APPLE/IMG_0001.PNG"), png)?;
    fs::write(
        destination.join("PhotoData/Thumbnails/V2/DCIM/100APPLE/IMG_0001.PNG"),
        png,
    )?;
    fs::write(
        destination.join("PhotoData/Thumbnails/V2/DCIM/100APPLE/UNMATCHED_0042.PNG"),
        png,
    )?;
    fs::write(destination.join("archives/thumbnail-copy.PNG"), png)?;

    {
        let database = Connection::open(destination.join("PhotoData/Photos.sqlite"))?;
        database.execute_batch("CREATE TABLE ZASSET (Z_PK INTEGER PRIMARY KEY, ZFILENAME TEXT, ZDIRECTORY TEXT, ZTRASHEDSTATE INTEGER, ZDATECREATED REAL);")?;
        database.execute(
            "INSERT INTO ZASSET VALUES (1, 'IMG_0001.PNG', 'DCIM/100APPLE', 0, 810000000)",
            [],
        )?;
        database.execute("INSERT INTO ZASSET VALUES (2, 'MISSING_ORIGINAL_0002.PNG', 'DCIM/100APPLE', 1, 811000000)", [])?;
    }
    {
        let database = Connection::open(destination.join("browser/chromium/History"))?;
        database.execute_batch("CREATE TABLE urls (id INTEGER PRIMARY KEY, url TEXT, title TEXT, visit_count INTEGER, last_visit_time INTEGER);")?;
        for (index, (url, title)) in [
            (
                "https://archive.example.invalid/project?private=omitted",
                "Synthetic archive",
            ),
            (
                "https://research.example.invalid/notes",
                "Synthetic research",
            ),
            (
                "https://docs.example.invalid/privacy",
                "Synthetic documentation",
            ),
            ("https://archive.example.invalid/export", "Synthetic export"),
        ]
        .into_iter()
        .enumerate()
        {
            database.execute(
                "INSERT INTO urls VALUES (?1, ?2, ?3, 1, ?4)",
                params![
                    index as i64 + 1,
                    url,
                    title,
                    13_430_000_000_000_000_i64 + index as i64 * 86_400_000_000
                ],
            )?;
        }
    }
    {
        let database = Connection::open(destination.join("browser/safari/History.db"))?;
        database.execute_batch("CREATE TABLE history_items (id INTEGER PRIMARY KEY, url TEXT); CREATE TABLE history_visits (id INTEGER PRIMARY KEY, history_item INTEGER, visit_time REAL);")?;
        database.execute(
            "INSERT INTO history_items VALUES (1, 'https://library.example.invalid/old-notes')",
            [],
        )?;
        database.execute(
            "INSERT INTO history_items VALUES (2, 'https://gallery.example.invalid/album')",
            [],
        )?;
        database.execute("INSERT INTO history_visits VALUES (1, 1, 812000000)", [])?;
        database.execute("INSERT INTO history_visits VALUES (2, 2, 813000000)", [])?;
    }
    {
        let database = Connection::open(destination.join("messages/sms.db"))?;
        database.execute_batch("CREATE TABLE message (ROWID INTEGER PRIMARY KEY, date INTEGER, is_from_me INTEGER, text TEXT);")?;
        for index in 0..4_i64 {
            database.execute(
                "INSERT INTO message VALUES (?1, ?2, ?3, ?4)",
                params![
                    index + 1,
                    812_000_000_000_000_000_i64 + index * 86_400_000_000_000,
                    index % 2,
                    "SYNTHETIC message body - deliberately not exported"
                ],
            )?;
        }
    }
    let modified = SystemTime::UNIX_EPOCH + Duration::from_secs(1_790_000_000);
    for (index, entry) in walkdir::WalkDir::new(destination)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .enumerate()
    {
        let file = fs::OpenOptions::new().write(true).open(entry.path())?;
        file.set_times(
            fs::FileTimes::new()
                .set_modified(modified + Duration::from_secs(index as u64 * 86_400)),
        )?;
    }
    Ok(())
}

pub fn demo_case() -> Case {
    let operation = (|| -> Result<Case, EngramError> {
        let fixture = tempfile::tempdir()?;
        create_demo_fixture(fixture.path())?;
        let mut case = scan_path(fixture.path(), &ScanOptions::default())?;
        case.title = "The forgotten archive".into();
        case.source_kind = SourceKind::Demo;
        case.source_root = "synthetic://engram-demo".into();
        case.warnings.insert(0, "SYNTHETIC DEMO: generated files and database rows, analysed by the real engine. No device or personal data was scanned. These are not recovered real-world secrets.".into());
        for finding in &mut case.findings {
            finding.origin = "Synthetic fixture".into();
        }
        Ok(case)
    })();
    match operation {
        Ok(case) => case,
        Err(error) => Case {
            schema_version: 1,
            id: "demo-unavailable".into(),
            title: "Demo unavailable".into(),
            source_root: "synthetic://engram-demo".into(),
            source_kind: SourceKind::Demo,
            created_at: chrono::Utc::now().timestamp(),
            completed_at: chrono::Utc::now().timestamp(),
            findings: Vec::new(),
            warnings: vec![format!("Synthetic demo could not be generated: {error}")],
            stats: CaseStats::default(),
            modules: Vec::new(),
            events: Vec::new(),
            review: Default::default(),
        },
    }
}
