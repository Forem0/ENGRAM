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
    prepare_demo_destination(destination)?;
    populate_demo_fixture(destination)
}

fn prepare_demo_destination(destination: &Path) -> Result<(), EngramError> {
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
    Ok(())
}

fn populate_demo_fixture(destination: &Path) -> Result<(), EngramError> {
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

/// Generate a small, clearly synthetic, unencrypted iPhone-style backup for
/// compatibility tests. Identifiers are invented; this never connects a device.
pub fn create_demo_backup_fixture(destination: &Path) -> Result<(), EngramError> {
    prepare_demo_destination(destination)?;
    let fixture = tempfile::tempdir()?;
    create_demo_fixture(fixture.path())?;
    let mut manifest = plist::Dictionary::new();
    manifest.insert("IsEncrypted".into(), plist::Value::Boolean(false));
    plist::Value::Dictionary(manifest)
        .to_file_xml(destination.join("Manifest.plist"))
        .map_err(|_| EngramError::Input("Could not write synthetic backup metadata.".into()))?;
    let mut info = plist::Dictionary::new();
    info.insert(
        "Device Name".into(),
        plist::Value::String("ENGRAM synthetic fixture".into()),
    );
    info.insert(
        "Product Version".into(),
        plist::Value::String("Synthetic schema fixture".into()),
    );
    plist::Value::Dictionary(info)
        .to_file_xml(destination.join("Info.plist"))
        .map_err(|_| EngramError::Input("Could not write synthetic device metadata.".into()))?;
    let index = Connection::open(destination.join("Manifest.db"))?;
    index.execute_batch("CREATE TABLE Files (fileID TEXT PRIMARY KEY, domain TEXT, relativePath TEXT, flags INTEGER);")?;
    let calibration = backup_calibration_png()?;
    for (source, domain, logical) in [
        (
            "media/DCIM/100APPLE/IMG_0001.PNG",
            "CameraRollDomain",
            "Media/DCIM/100APPLE/IMG_0001.PNG",
        ),
        (
            "PhotoData/Thumbnails/V2/DCIM/100APPLE/IMG_0001.PNG",
            "CameraRollDomain",
            "Media/PhotoData/Thumbnails/V2/DCIM/100APPLE/IMG_0001.PNG",
        ),
        (
            "PhotoData/Thumbnails/V2/DCIM/100APPLE/UNMATCHED_0042.PNG",
            "CameraRollDomain",
            "Media/PhotoData/Thumbnails/V2/DCIM/100APPLE/UNMATCHED_0042.PNG",
        ),
        (
            "PhotoData/Photos.sqlite",
            "CameraRollDomain",
            "Media/PhotoData/Photos.sqlite",
        ),
        (
            "documents/research-notes.txt",
            "HomeDomain",
            "Documents/research-notes.txt",
        ),
        (
            "browser/safari/History.db",
            "HomeDomain",
            "Library/Safari/History.db",
        ),
        ("messages/sms.db", "HomeDomain", "Library/SMS/sms.db"),
    ] {
        let identifier =
            crate::digest(format!("synthetic:{domain}:{logical}").as_bytes())[..40].to_owned();
        let directory = destination.join(&identifier[..2]);
        fs::create_dir_all(&directory)?;
        if source.ends_with(".PNG") {
            fs::write(directory.join(&identifier), &calibration)?;
        } else {
            fs::copy(fixture.path().join(source), directory.join(&identifier))?;
        }
        index.execute(
            "INSERT INTO Files VALUES (?1, ?2, ?3, 1)",
            params![identifier, domain, logical],
        )?;
    }
    Ok(())
}

/// A deterministic calibration card, never a photograph or recovered image.
/// Only the iPhone-style fixture uses it; the ordinary fixture stays unchanged.
fn backup_calibration_png() -> Result<Vec<u8>, EngramError> {
    let paper = [244, 240, 229];
    let cobalt = [36, 107, 254];
    let teal = [25, 197, 199];
    let mut card = image::RgbImage::from_fn(640, 400, |x, y| {
        let color = if !(24..616).contains(&x) || !(24..376).contains(&y) {
            paper
        } else if y < 112 {
            cobalt
        } else if x < 40 {
            teal
        } else if (160..304).contains(&y) && (72..216).contains(&x) {
            if x + y > 416 {
                paper
            } else {
                cobalt
            }
        } else if (160..304).contains(&y) && (248..392).contains(&x) {
            let dx = i64::from(x) - 320;
            let dy = i64::from(y) - 232;
            if dx * dx + dy * dy < 42 * 42 {
                paper
            } else {
                teal
            }
        } else if (160..304).contains(&y) && (424..568).contains(&x) {
            if (x - 424) / 16 % 2 == (y - 160) / 16 % 2 {
                paper
            } else {
                cobalt
            }
        } else if (336..352).contains(&y) && (72..568).contains(&x) {
            if x & 8 == 0 {
                teal
            } else {
                paper
            }
        } else if x % 16 == 8 || y & 15 == 0 {
            [34, 52, 73]
        } else {
            [17, 29, 47]
        };
        image::Rgb(color)
    });
    // Five-by-seven bitmap glyphs: DEMO. Integer geometry is reproducible and
    // requires no fonts, external image assets, text rasterizer or network.
    let glyphs = [
        [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
    ];
    for (index, glyph) in glyphs.iter().enumerate() {
        for (row, bits) in glyph.iter().enumerate() {
            for column in 0..5 {
                if bits & (1 << (4 - column)) != 0 {
                    for dy in 0..8 {
                        for dx in 0..8 {
                            card.put_pixel(
                                72 + index as u32 * 48 + column * 8 + dx,
                                40 + row as u32 * 8 + dy,
                                image::Rgb(paper),
                            );
                        }
                    }
                }
            }
        }
    }
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgb8(card)
        .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
        .map_err(|_| EngramError::Input("Could not encode the synthetic calibration card.".into()))?;
    Ok(bytes)
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
