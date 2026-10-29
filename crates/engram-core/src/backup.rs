use crate::{
    database,
    scan::{is_reparse, metadata_for},
    EngramError,
};
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs,
    io::{Cursor, Read},
    path::Path,
    time::{Duration, Instant},
};

const MAX_PLIST_BYTES: u64 = 1024 * 1024;
const MAX_INDEX_BYTES: u64 = 8 * 1024 * 1024;
const MAX_INDEX_ROWS: usize = 25_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupInspectionState {
    Ready,
    Unavailable,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupCategory {
    pub id: String,
    pub label: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupInspection {
    pub name: String,
    pub state: BackupInspectionState,
    pub encrypted: Option<bool>,
    pub device_name: Option<String>,
    pub product_version: Option<String>,
    pub indexed_files: u64,
    pub categories: Vec<BackupCategory>,
    pub warnings: Vec<String>,
}

/// Inspect only the explicitly supplied backup directory. Category counts are
/// bounded, valid regular-file entries in its index, not parsed findings or a
/// guarantee that every referenced blob is present. No device data is modified.
pub fn inspect_backup(path: &Path) -> Result<BackupInspection, EngramError> {
    let metadata = path.symlink_metadata()?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || is_reparse(&metadata) {
        return Err(EngramError::Input(
            "Choose a regular backup directory, not a link or file.".into(),
        ));
    }
    let root = path.canonicalize()?;
    let mut inspection = BackupInspection {
        name: root.file_name().map(|part| bounded_text(&part.to_string_lossy())).filter(|name| !name.is_empty()).unwrap_or_else(|| "Selected backup".into()),
        state: BackupInspectionState::Unavailable,
        encrypted: None,
        device_name: None,
        product_version: None,
        indexed_files: 0,
        categories: Vec::new(),
        warnings: vec![
            "Counts describe listed file paths, not verified media, recovered records or complete device coverage.".into(),
            "The scan uses bounded readers. iCloud-only assets, missing blobs, unsupported schemas and size limits can reduce coverage.".into(),
        ],
    };
    if let Err(error) = inspect_selected(&root, &mut inspection) {
        inspection.state = BackupInspectionState::Unavailable;
        inspection.indexed_files = 0;
        inspection.categories.clear();
        inspection.warnings.push(error.to_string());
    }
    Ok(inspection)
}

fn inspect_selected(root: &Path, inspection: &mut BackupInspection) -> Result<(), EngramError> {
    let plist_bytes =
        read_stable(root, &root.join("Manifest.plist"), MAX_PLIST_BYTES).map_err(|_| {
            input("A readable Manifest.plist is required to identify backup encryption.")
        })?;
    let metadata = plist::Value::from_reader(Cursor::new(plist_bytes))
        .map_err(|_| input("Manifest.plist is malformed or unsupported."))?;
    let metadata = metadata
        .as_dictionary()
        .ok_or_else(|| input("Manifest.plist must contain a metadata dictionary."))?;
    let encrypted = metadata.get("IsEncrypted").and_then(plist::Value::as_boolean)
        .ok_or_else(|| input("Backup encryption could not be determined. IsEncrypted must be an explicit boolean."))?;
    inspection.encrypted = Some(encrypted);
    if encrypted {
        return Err(input("This backup is encrypted. Encrypted-backup reading is not implemented; no password is requested."));
    }
    if let Ok(bytes) = read_stable(root, &root.join("Info.plist"), MAX_PLIST_BYTES) {
        match plist::Value::from_reader(Cursor::new(bytes)) {
            Ok(value) => {
                if let Some(info) = value.as_dictionary() {
                    inspection.device_name = info
                        .get("Device Name")
                        .and_then(plist::Value::as_string)
                        .map(bounded_text)
                        .filter(|value| !value.is_empty());
                    inspection.product_version = info
                        .get("Product Version")
                        .and_then(plist::Value::as_string)
                        .map(bounded_text)
                        .filter(|value| !value.is_empty());
                } else {
                    inspection
                        .warnings
                        .push("Optional Info.plist has an unsupported metadata layout.".into());
                }
            }
            Err(_) => inspection.warnings.push(
                "Optional Info.plist could not be parsed; device metadata is unavailable.".into(),
            ),
        }
    }
    let database_bytes = read_stable(root, &root.join("Manifest.db"), MAX_INDEX_BYTES)
        .map_err(|_| input("Manifest.db is missing, unreadable or larger than the 8 MiB default index-read limit."))?;
    let temporary = tempfile::tempdir()?;
    let database_path = temporary.path().join("manifest.sqlite");
    fs::write(&database_path, database_bytes)?;
    let wal = root.join("Manifest.db-wal");
    match wal.symlink_metadata() {
        Ok(_) => {
            let bytes = read_stable(root, &wal, MAX_INDEX_BYTES).map_err(|_| {
                input("The backup index WAL could not be copied safely within the read limit.")
            })?;
            fs::write(temporary.path().join("manifest.sqlite-wal"), bytes)?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(_) => return Err(input("The backup index WAL could not be inspected.")),
    }
    let connection = Connection::open_with_flags(
        &database_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    connection.busy_timeout(Duration::from_millis(200))?;
    connection.set_limit(
        rusqlite::limits::Limit::SQLITE_LIMIT_LENGTH,
        16 * 1024 * 1024,
    )?;
    connection.set_limit(rusqlite::limits::Limit::SQLITE_LIMIT_SQL_LENGTH, 64 * 1024)?;
    connection.set_limit(rusqlite::limits::Limit::SQLITE_LIMIT_EXPR_DEPTH, 64)?;
    let started = Instant::now();
    connection.progress_handler(
        1_000,
        Some(move || started.elapsed() > Duration::from_secs(2)),
    );
    connection.execute_batch("PRAGMA query_only=ON; PRAGMA trusted_schema=OFF;")?;
    if !database::is_regular_table(&connection, "Files") {
        return Err(input("The backup needs a supported ordinary Files index table; views and virtual tables are excluded."));
    }
    let mut statement = connection.prepare("SELECT substr(fileID,1,41), substr(domain,1,1025), substr(relativePath,1,16385), flags FROM Files LIMIT 25001")
        .map_err(|_| input("The Files index schema is not supported. Required columns: fileID, domain, relativePath and flags."))?;
    let mut rows = statement.query([])?;
    let mut visited = 0_usize;
    let mut metadata_bytes = 0_usize;
    let mut excluded = 0_u64;
    let mut counts = BTreeMap::<&'static str, u64>::new();
    while let Some(row) = rows.next()? {
        if visited >= MAX_INDEX_ROWS || metadata_bytes > 8 * 1024 * 1024 {
            inspection.warnings.push("Index inspection reached its bounded row or metadata budget; displayed counts are partial.".into());
            break;
        }
        visited += 1;
        let identifier: String = row.get(0)?;
        let domain: String = row.get(1)?;
        let relative: String = row.get(2)?;
        let flags: i64 = row.get(3)?;
        if flags != 1 {
            continue;
        }
        metadata_bytes =
            metadata_bytes.saturating_add(identifier.len() + domain.len() + relative.len());
        if identifier.len() != 40
            || !identifier.bytes().all(|byte| byte.is_ascii_hexdigit())
            || domain.len() > 1024
            || relative.len() > 16_384
            || Path::new(&relative).is_absolute()
            || relative.split(['/', '\\']).any(|part| part == "..")
        {
            excluded += 1;
            continue;
        }
        inspection.indexed_files += 1;
        *counts.entry(category_for(&relative)).or_default() += 1;
    }
    if excluded > 0 {
        inspection.warnings.push(format!(
            "Excluded {excluded} invalid index paths or file identifiers."
        ));
    }
    inspection.categories = counts
        .into_iter()
        .map(|(id, count)| BackupCategory {
            id: id.into(),
            label: category_label(id).into(),
            count,
        })
        .collect();
    inspection.state = BackupInspectionState::Ready;
    Ok(())
}

fn category_for(relative: &str) -> &'static str {
    let path = relative.replace('\\', "/").to_ascii_lowercase();
    let filename = path.rsplit('/').next().unwrap_or("");
    let extension = filename.rsplit('.').next().unwrap_or("");
    if matches!(
        extension,
        "png"
            | "jpg"
            | "jpeg"
            | "heic"
            | "heif"
            | "gif"
            | "webp"
            | "mov"
            | "mp4"
            | "m4v"
            | "avi"
            | "ithmb"
            | "thmb"
            | "tiff"
            | "bmp"
    ) {
        "media"
    } else if matches!(filename, "sms.db" | "chat.db") {
        "messages"
    } else if matches!(filename, "history" | "history.db") {
        "browser"
    } else if matches!(extension, "db" | "sqlite" | "sqlite3") {
        "databases"
    } else if matches!(
        extension,
        "txt" | "md" | "pdf" | "doc" | "docx" | "csv" | "json" | "xml" | "log" | "env" | "plist"
    ) {
        "documents"
    } else {
        "other"
    }
}

fn category_label(id: &str) -> &'static str {
    match id {
        "media" => "Media paths",
        "messages" => "Message database paths",
        "browser" => "Browser database paths",
        "databases" => "Other database paths",
        "documents" => "Document and text paths",
        _ => "Other indexed paths",
    }
}

fn bounded_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .take(256)
        .collect::<String>()
        .trim()
        .to_owned()
}

fn input(message: &str) -> EngramError {
    EngramError::Input(message.into())
}

fn read_stable(root: &Path, path: &Path, limit: u64) -> Result<Vec<u8>, EngramError> {
    let before = metadata_for(root, path)?;
    if before.len() > limit {
        return Err(input("Backup metadata exceeds its read limit."));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    let after = metadata_for(root, path)?;
    if bytes.len() as u64 != before.len()
        || after.len() != before.len()
        || before.modified().ok() != after.modified().ok()
    {
        return Err(input(
            "Backup metadata changed while it was inspected. Select the backup again.",
        ));
    }
    Ok(bytes)
}
