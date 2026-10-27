use crate::{
    database::{self, PhotoAsset},
    digest, finding_id, secrets, time_from_system, Case, CaseStats, Category, Confidence,
    EngramError, Evidence, Finding, ModuleState, ModuleStatus, ScanEvent, ScanOptions, SourceKind,
    MAX_FINDINGS,
};
use chrono::Utc;
use rand::RngCore;
use rusqlite::{Connection, OpenFlags};
use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    fs,
    io::{Cursor, Read},
    path::{Component, Path, PathBuf},
    time::{Duration, Instant},
};
use walkdir::{DirEntry, WalkDir};

#[derive(Clone)]
struct InputFile {
    physical: PathBuf,
    logical: String,
    origin: String,
    bytes: u64,
    modified: Option<i64>,
}

struct Snapshot {
    _temporary: tempfile::TempDir,
    connection: Connection,
}

struct Engine<'a> {
    root: PathBuf,
    options: &'a ScanOptions,
    started: Instant,
    case: Case,
    case_key: [u8; 32],
    file_groups: HashMap<String, Vec<usize>>,
    secret_groups: HashMap<String, Vec<usize>>,
    finding_ids: HashSet<String>,
    photos: Vec<PhotoAsset>,
    inputs: Vec<InputFile>,
}

fn evidence(label: &str, value: impl ToString) -> Evidence {
    Evidence {
        label: label.into(),
        value: value.to_string(),
    }
}

fn modules() -> Vec<ModuleStatus> {
    let entries = [
        ("File inventory", ModuleState::Ready, "Bounded metadata and content reads from an explicitly selected folder; links and reparse points excluded."),
        ("Secret candidates", ModuleState::Ready, "Accessible-text patterns. Values are fully masked; case-local HMAC identifiers link repeated candidates. No validity checks."),
        ("Duplicate connections", ModuleState::Ready, "Exact byte-content matches within read limits. Related records are bounded to 16 neighbours."),
        ("Cases and comparison", ModuleState::Ready, "Local JSON cases and evidence/metadata comparison. Disappearance is not proof of secure erasure."),
        ("iPhone backup", ModuleState::Experimental, "Unencrypted Manifest.db blob mapping only. Encrypted backups and unsupported layouts are rejected."),
        ("Photos catalogue", ModuleState::Experimental, "ZASSET metadata and filename/path associations, capped at 1,000 rows per database. Unmatched never means proven deleted."),
        ("Browser activity", ModuleState::Experimental, "Chromium urls and Safari history tables. Website origins only; private URL parts and page titles omitted."),
        ("Message metadata", ModuleState::Experimental, "Apple message table timestamps and direction only. Bodies and contacts are not exported."),
        ("Deleted-record recovery", ModuleState::Planned, "WAL/free-page carving is not implemented. WAL is used only to read the current copied database snapshot."),
        ("Keychain recovery", ModuleState::Planned, "No extraction, decryption, or recovery of protected Keychain records."),
        ("Location and OCR", ModuleState::Planned, "No coordinate extraction or image-text recognition in this preview."),
        ("Application access", ModuleState::Planned, "No claims about which applications actually read a file or currently have permission."),
    ];
    entries
        .into_iter()
        .map(|(name, state, detail)| ModuleStatus {
            name: name.into(),
            state,
            detail: detail.into(),
        })
        .collect()
}

#[cfg(windows)]
fn is_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x400 != 0
}
#[cfg(not(windows))]
fn is_reparse(_metadata: &fs::Metadata) -> bool {
    false
}

fn metadata_for(root: &Path, path: &Path) -> Result<fs::Metadata, EngramError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| EngramError::Input("Source path escaped the selected directory.".into()))?;
    let mut cursor = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(part) = component else {
            return Err(EngramError::Input(
                "Non-relative source path rejected.".into(),
            ));
        };
        cursor.push(part);
        let metadata = cursor.symlink_metadata()?;
        if metadata.file_type().is_symlink() || is_reparse(&metadata) {
            return Err(EngramError::Input(
                "Symbolic links and filesystem reparse points are excluded.".into(),
            ));
        }
    }
    let metadata = path.symlink_metadata()?;
    if !metadata.is_file() {
        return Err(EngramError::Input("Source is not a regular file.".into()));
    }
    let canonical = path.canonicalize()?;
    if !canonical.starts_with(root) {
        return Err(EngramError::Input(
            "Source resolves outside the selected directory.".into(),
        ));
    }
    Ok(metadata)
}

fn input_file(
    root: &Path,
    physical: PathBuf,
    logical: String,
    origin: String,
) -> Result<InputFile, EngramError> {
    if logical.len() > 16_384 || origin.len() > 1024 {
        return Err(EngramError::Input(
            "Source path exceeds preview shape limits.".into(),
        ));
    }
    let metadata = metadata_for(root, &physical)?;
    Ok(InputFile {
        physical,
        logical,
        origin,
        bytes: metadata.len(),
        modified: metadata.modified().ok().and_then(time_from_system),
    })
}

fn allowed_entry(entry: &DirEntry, include_hidden: bool, excluded: &Cell<u64>) -> bool {
    if entry.depth() == 0 {
        return true;
    }
    let name = entry.file_name().to_string_lossy();
    let dependency = entry.file_type().is_dir()
        && matches!(
            name.as_ref(),
            ".git"
                | ".svn"
                | "node_modules"
                | "target"
                | ".engram"
                | ".chroma"
                | ".venv"
                | "venv"
                | "__pycache__"
        );
    let hidden = !include_hidden && name.starts_with('.');
    let link = entry
        .path()
        .symlink_metadata()
        .map(|metadata| metadata.file_type().is_symlink() || is_reparse(&metadata))
        .unwrap_or(true);
    if dependency || hidden || link {
        excluded.set(excluded.get() + 1);
        return false;
    }
    true
}

pub fn scan_path(path: &Path, options: &ScanOptions) -> Result<Case, EngramError> {
    if options.max_files == 0
        || options.max_files > 1_000_000
        || options.max_file_bytes == 0
        || options.max_file_bytes > 256 * 1024 * 1024
        || options.max_total_bytes == 0
        || options.max_total_bytes > 8 * 1024 * 1024 * 1024
    {
        return Err(EngramError::Input("Scan limits must be positive and within preview bounds (1M files, 256 MiB/file, 8 GiB total).".into()));
    }
    let metadata = path.symlink_metadata()?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || is_reparse(&metadata) {
        return Err(EngramError::Input(
            "Choose a regular folder, not a link, reparse point or individual file.".into(),
        ));
    }
    let root = path.canonicalize()?;
    if root.to_string_lossy().len() > 16_384 {
        return Err(EngramError::Input(
            "Selected root path exceeds preview shape limits.".into(),
        ));
    }
    let now = Utc::now().timestamp();
    let mut case_key = [0; 32];
    rand::rng().fill_bytes(&mut case_key);
    let source_kind = if root.join("Manifest.db").is_file() {
        SourceKind::IosBackup
    } else {
        SourceKind::Folder
    };
    let title = root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Local investigation".into());
    let id = digest(format!("{now}-{}-{}", root.display(), digest(&case_key)).as_bytes())[..20]
        .to_owned();
    let mut engine = Engine {
        root: root.clone(), options, started: Instant::now(), case_key,
        case: Case { schema_version: 1, id, title, source_root: root.to_string_lossy().into_owned(), source_kind, created_at: now, completed_at: now, findings: Vec::new(), warnings: vec![
            "Results describe only the selected offline source at scan time; missing data does not establish secure deletion or complete device coverage.".into(),
            "Candidate credentials are not validated. Secret values, message bodies and private URL components are not exported.".into(),
        ], stats: CaseStats::default(), modules: modules(), events: Vec::new() },
        file_groups: HashMap::new(), secret_groups: HashMap::new(), finding_ids: HashSet::new(), photos: Vec::new(), inputs: Vec::new(),
    };
    engine.event("Opened explicitly selected offline source", None);
    if source_kind == SourceKind::IosBackup {
        engine.collect_backup()?;
    } else {
        engine.collect_folder();
    }
    engine
        .inputs
        .sort_by(|one, two| one.logical.cmp(&two.logical));
    engine.event(
        &format!("Indexed {} source files", engine.inputs.len()),
        None,
    );
    for input in engine.inputs.clone() {
        if engine.started.elapsed() > Duration::from_secs(120) {
            engine.warn("Scan work-time budget reached; remaining files were not analysed.".into());
            break;
        }
        if engine.case.findings.len() >= MAX_FINDINGS {
            engine.warn("Finding limit reached; remaining files were not analysed.".into());
            break;
        }
        engine.process(&input);
    }
    engine.connect_duplicates();
    engine.connect_photos();
    engine.event("Completed bounded analysis and evidence connections", None);
    engine.case.completed_at = Utc::now().timestamp();
    engine.case.stats.elapsed_ms = engine
        .started
        .elapsed()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX);
    Ok(engine.case)
}

impl Engine<'_> {
    fn warn(&mut self, message: String) {
        if self.case.warnings.len() < 200 && !self.case.warnings.contains(&message) {
            self.case.warnings.push(message);
        } else if self.case.warnings.len() == 200 {
            self.case
                .warnings
                .push("Further warnings omitted; analysis coverage is partial.".into());
        }
    }

    fn event(&mut self, message: &str, category: Option<Category>) {
        if self.case.events.len() < 800 {
            self.case.events.push(ScanEvent {
                at_ms: self
                    .started
                    .elapsed()
                    .as_millis()
                    .try_into()
                    .unwrap_or(u64::MAX),
                message: message.into(),
                category,
            });
        }
    }

    fn push(&mut self, finding: Finding) -> Option<usize> {
        if self.case.findings.len() >= MAX_FINDINGS {
            self.warn("Finding limit reached; analysis coverage is partial.".into());
            return None;
        }
        if !self.finding_ids.insert(finding.id.clone()) {
            self.warn("Repeated source record identity excluded; source may contain duplicate index rows.".into());
            return None;
        }
        self.event(
            &format!("{}: {}", finding.category.label(), finding.title),
            Some(finding.category),
        );
        let index = self.case.findings.len();
        self.case.findings.push(finding);
        Some(index)
    }

    fn collect_folder(&mut self) {
        let excluded = Cell::new(0);
        let include_hidden = self.options.include_hidden;
        let mut iterator = WalkDir::new(&self.root)
            .follow_links(false)
            .max_depth(64)
            .into_iter();
        let entry_budget = self
            .options
            .max_files
            .saturating_mul(4)
            .clamp(1024, 1_000_000);
        let mut visited = 0;
        while let Some(entry) = iterator.next() {
            visited += 1;
            if visited > entry_budget || self.started.elapsed() > Duration::from_secs(120) {
                self.warn(
                    "Directory-entry or scan work-time budget reached; traversal is partial."
                        .into(),
                );
                break;
            }
            if let Ok(entry) = &entry {
                if !allowed_entry(entry, include_hidden, &excluded) {
                    if entry.file_type().is_dir() {
                        iterator.skip_current_dir();
                    }
                    continue;
                }
            }
            match entry {
                Ok(entry) if entry.file_type().is_file() => {
                    if self.inputs.len() >= self.options.max_files {
                        self.warn(format!(
                            "File limit {} reached; remaining source entries were not indexed.",
                            self.options.max_files
                        ));
                        break;
                    }
                    let logical = entry
                        .path()
                        .strip_prefix(&self.root)
                        .unwrap_or(entry.path())
                        .to_string_lossy()
                        .replace('\\', "/");
                    match input_file(
                        &self.root,
                        entry.path().to_owned(),
                        logical,
                        "Selected folder".into(),
                    ) {
                        Ok(file) => self.inputs.push(file),
                        Err(error) => {
                            self.case.stats.skipped_files += 1;
                            self.warn(format!("Source file excluded: {error}"));
                        }
                    }
                }
                Ok(entry) if entry.depth() >= 64 && entry.file_type().is_dir() => self.warn(
                    "Directory depth limit 64 reached; deeper paths were not visited.".into(),
                ),
                Ok(_) => {}
                Err(_) => {
                    self.case.stats.skipped_files += 1;
                    self.warn("One or more source entries could not be enumerated; permissions or filesystem errors limited coverage.".into());
                }
            }
        }
        if excluded.get() > 0 {
            self.warn(format!("Excluded {} link, hidden or dependency-directory entries by policy; excluded directories were not traversed.", excluded.get()));
        }
    }

    fn collect_backup(&mut self) -> Result<(), EngramError> {
        let manifest_plist = input_file(&self.root, self.root.join("Manifest.plist"), "Manifest.plist".into(), "Backup metadata".into())
            .map_err(|_| EngramError::Input("Manifest.db was found but a readable Manifest.plist is required to determine backup encryption.".into()))?;
        let bytes = self.read(&manifest_plist).ok_or_else(|| {
            EngramError::Input("Backup metadata exceeds the source-read budget.".into())
        })?;
        let metadata = plist::Value::from_reader(Cursor::new(bytes)).map_err(|_| {
            EngramError::Input("Manifest.plist is not a supported property list.".into())
        })?;
        let dictionary = metadata.as_dictionary().ok_or_else(|| {
            EngramError::Input("Manifest.plist does not contain a metadata dictionary.".into())
        })?;
        if dictionary
            .get("IsEncrypted")
            .and_then(plist::Value::as_boolean)
            == Some(true)
        {
            return Err(EngramError::Input("Encrypted iPhone backups are not supported by this Developer Preview. ENGRAM will not bypass encryption or request your device passcode.".into()));
        }
        let manifest = input_file(
            &self.root,
            self.root.join("Manifest.db"),
            "Manifest.db".into(),
            "Backup index".into(),
        )?;
        let bytes = self.read(&manifest).ok_or_else(|| {
            EngramError::Input("Manifest.db exceeds the source-read budget.".into())
        })?;
        let snapshot = self.snapshot(&manifest, &bytes)?;
        if !database::is_regular_table(&snapshot.connection, "Files") {
            return Err(EngramError::Input(
                "Manifest Files index must be an ordinary table, not a view or virtual table."
                    .into(),
            ));
        }
        let mut statement = snapshot.connection.prepare("SELECT substr(fileID,1,41), substr(domain,1,1025), substr(relativePath,1,16385), flags FROM Files LIMIT ?1")
            .map_err(|_| EngramError::Input("Manifest.db has no supported Files index; this backup layout is unavailable.".into()))?;
        let limit = i64::try_from(self.options.max_files + 1).unwrap_or(i64::MAX);
        let mut rows = statement.query([limit])?;
        let mut index_rows = 0;
        let mut manifest_rows = Vec::new();
        let mut metadata_bytes = 0_usize;
        while let Some(row) = rows.next()? {
            if index_rows >= self.options.max_files {
                self.warn(format!(
                    "Backup manifest was limited to {} index rows; coverage is partial.",
                    self.options.max_files
                ));
                break;
            }
            index_rows += 1;
            let identifier: String = row.get(0)?;
            let domain: String = row.get(1)?;
            let relative: String = row.get(2)?;
            let flags: i64 = row.get(3)?;
            if flags != 1 {
                continue;
            }
            metadata_bytes =
                metadata_bytes.saturating_add(identifier.len() + domain.len() + relative.len());
            if metadata_bytes > 8 * 1024 * 1024 {
                self.warn("Backup index metadata budget reached; coverage is partial.".into());
                break;
            }
            manifest_rows.push((identifier, domain, relative));
        }
        drop(rows);
        drop(statement);
        for (identifier, domain, relative) in manifest_rows {
            if self.started.elapsed() > Duration::from_secs(120) {
                self.warn(
                    "Backup source lookup work-time budget reached; coverage is partial.".into(),
                );
                break;
            }
            if identifier.len() != 40
                || !identifier.bytes().all(|byte| byte.is_ascii_hexdigit())
                || relative.len() > 16_384
                || domain.len() > 1024
                || Path::new(&relative).is_absolute()
                || relative.split(['/', '\\']).any(|part| part == "..")
            {
                self.case.stats.skipped_files += 1;
                self.warn(
                    "Invalid backup file identifier or logical path excluded from analysis.".into(),
                );
                continue;
            }
            let nested = self.root.join(&identifier[..2]).join(&identifier);
            let physical = if nested.exists() {
                nested
            } else {
                self.root.join(&identifier)
            };
            match input_file(&self.root, physical, relative.replace('\\', "/"), domain) {
                Ok(file) => self.inputs.push(file),
                Err(_) => {
                    self.case.stats.skipped_files += 1;
                    self.warn("Some manifest-listed blobs are unavailable or excluded. A local backup is not a full filesystem image.".into());
                }
            }
        }
        self.event(
            "Mapped unencrypted backup domains and hashed blobs",
            Some(Category::Database),
        );
        self.warn("Backup contents may exclude cloud-only assets, caches and protected data. Current device state cannot be inferred from an older backup.".into());
        Ok(())
    }

    fn read(&mut self, input: &InputFile) -> Option<Vec<u8>> {
        if input.bytes > self.options.max_file_bytes {
            self.case.stats.skipped_files += 1;
            self.warn(format!(
                "Content size limit reached for {}; metadata only ({} bytes).",
                input.logical, input.bytes
            ));
            return None;
        }
        let remaining = self
            .options
            .max_total_bytes
            .saturating_sub(self.case.stats.bytes_examined);
        if input.bytes > remaining {
            self.case.stats.skipped_files += 1;
            self.warn(
                "Total source-read budget reached; subsequent files may have metadata only.".into(),
            );
            return None;
        }
        let operation = (|| -> Result<Vec<u8>, EngramError> {
            let before = metadata_for(&self.root, &input.physical)?;
            if before.len() != input.bytes {
                return Err(EngramError::Input(
                    "Source size changed during analysis.".into(),
                ));
            }
            let mut bytes = Vec::new();
            fs::File::open(&input.physical)?
                .take(input.bytes)
                .read_to_end(&mut bytes)?;
            self.case.stats.bytes_examined += bytes.len() as u64;
            let after = metadata_for(&self.root, &input.physical)?;
            if bytes.len() as u64 != input.bytes
                || after.len() != input.bytes
                || before.modified().ok() != after.modified().ok()
            {
                return Err(EngramError::Input(
                    "Source changed during analysis; unstable content was not parsed.".into(),
                ));
            }
            Ok(bytes)
        })();
        match operation {
            Ok(bytes) => Some(bytes),
            Err(_) => {
                self.case.stats.skipped_files += 1;
                self.warn(format!(
                    "Could not read stable source content for {}; coverage is partial.",
                    input.logical
                ));
                None
            }
        }
    }

    fn snapshot(&mut self, input: &InputFile, bytes: &[u8]) -> Result<Snapshot, EngramError> {
        let temporary = tempfile::tempdir()?;
        let database = temporary.path().join("evidence.sqlite");
        fs::write(&database, bytes)?;
        let sidecar_logical = format!("{}-wal", input.logical);
        let sidecar = self
            .inputs
            .iter()
            .find(|file| file.logical == sidecar_logical && file.origin == input.origin)
            .cloned()
            .or_else(|| {
                let filename = input.physical.file_name()?.to_str()?;
                let physical = input.physical.with_file_name(format!("{filename}-wal"));
                input_file(
                    &self.root,
                    physical,
                    sidecar_logical.clone(),
                    input.origin.clone(),
                )
                .ok()
            });
        if let Some(sidecar) = sidecar {
            let content = self.read(&sidecar).ok_or_else(|| EngramError::Input("Database WAL could not be copied within the source-read limits; database parsing skipped to avoid stale results.".into()))?;
            fs::write(temporary.path().join("evidence.sqlite-wal"), content)?;
        }
        let connection = Connection::open_with_flags(
            &database,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        connection.busy_timeout(Duration::from_millis(200))?;
        connection.set_limit(
            rusqlite::limits::Limit::SQLITE_LIMIT_LENGTH,
            16 * 1024 * 1024,
        )?;
        connection.set_limit(rusqlite::limits::Limit::SQLITE_LIMIT_SQL_LENGTH, 64 * 1024)?;
        connection.set_limit(rusqlite::limits::Limit::SQLITE_LIMIT_EXPR_DEPTH, 64)?;
        let query_started = Instant::now();
        connection.progress_handler(
            1_000,
            Some(move || query_started.elapsed() > Duration::from_secs(2)),
        );
        connection.execute_batch("PRAGMA query_only=ON; PRAGMA trusted_schema=OFF;")?;
        Ok(Snapshot {
            _temporary: temporary,
            connection,
        })
    }

    fn process(&mut self, input: &InputFile) {
        self.case.stats.files_examined += 1;
        let content = self.read(input);
        let hash = content.as_deref().map(digest);
        let extension = Path::new(&input.logical)
            .extension()
            .and_then(|part| part.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let media = matches!(
            extension.as_str(),
            "jpg"
                | "jpeg"
                | "png"
                | "gif"
                | "webp"
                | "heic"
                | "heif"
                | "mov"
                | "mp4"
                | "m4v"
                | "avi"
                | "ithmb"
                | "thmb"
                | "tiff"
                | "bmp"
        );
        let database = content
            .as_deref()
            .is_some_and(|bytes| bytes.starts_with(b"SQLite format 3\0"));
        let document = matches!(
            extension.as_str(),
            "txt"
                | "md"
                | "pdf"
                | "doc"
                | "docx"
                | "xls"
                | "xlsx"
                | "csv"
                | "rtf"
                | "odt"
                | "json"
                | "yaml"
                | "yml"
                | "toml"
                | "ini"
                | "conf"
                | "cfg"
                | "log"
                | "env"
                | "bak"
                | "zip"
                | "tar"
                | "gz"
        ) || Path::new(&input.logical)
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with(".env"));
        let text = if !media && !database {
            content
                .as_deref()
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
        } else {
            None
        };
        let candidates = text
            .map(|value| secrets::scan(value, &self.case_key))
            .unwrap_or_default();
        let contains_secret = !candidates.is_empty();

        if media || document || database {
            let (category, rule, title) = if media {
                (Category::Media, "file.media", "Media file")
            } else if database {
                (Category::Database, "database.file", "SQLite database")
            } else {
                (Category::Document, "file.document", "Document or archive")
            };
            let name: String = Path::new(&input.logical)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .chars()
                .take(256)
                .collect();
            let summary = if content.is_none() {
                "File metadata is present; content was not read within the configured limits."
            } else if database {
                "A SQLite file is present. Supported current rows are read from a temporary copy; deleted records are not carved."
            } else {
                "File bytes are present in this supplied source. Presence alone says nothing about deletion history or application access."
            };
            let item = Finding {
                id: finding_id(&input.logical, rule, &input.origin),
                category,
                title: format!("{title}: {name}"),
                path: input.logical.clone(),
                origin: input.origin.clone(),
                summary: summary.into(),
                locator: "File".into(),
                observed_at: input.modified,
                bytes: input.bytes,
                sha256: if contains_secret { None } else { hash.clone() },
                confidence: Confidence::Observed,
                rule_id: rule.into(),
                sensitive: contains_secret,
                evidence: vec![
                    evidence("Source", &input.origin),
                    evidence("Bytes present", input.bytes),
                    evidence(
                        "Content read",
                        if content.is_some() {
                            "Within configured limits"
                        } else {
                            "Metadata only"
                        },
                    ),
                    evidence("Deletion state", "Not established"),
                ],
                related_ids: Vec::new(),
            };
            if let Some(index) = self.push(item) {
                if let Some(hash) = &hash {
                    self.file_groups
                        .entry(hash.clone())
                        .or_default()
                        .push(index);
                }
            }
        }
        for candidate in candidates {
            let locator = format!("Line {}, column {}", candidate.line, candidate.column);
            let item = Finding {
                id: finding_id(&input.logical, candidate.rule, &format!("{}:{locator}", input.origin)), category: Category::Secret,
                title: format!("Possible {}", candidate.kind.to_ascii_lowercase()), path: input.logical.clone(), origin: input.origin.clone(),
                summary: "Pattern match in accessible text. The value is fully masked; validity, age and deletion state were not tested.".into(),
                locator, observed_at: input.modified, bytes: input.bytes, sha256: None, confidence: Confidence::Candidate,
                rule_id: candidate.rule.into(), sensitive: true,
                evidence: vec![evidence("Detector", candidate.kind), evidence("Value", "[redacted]"), evidence("Case-local match", &candidate.match_id[..24]), evidence("Validation", "Not performed")],
                related_ids: Vec::new(),
            };
            if let Some(index) = self.push(item) {
                self.secret_groups
                    .entry(candidate.match_id)
                    .or_default()
                    .push(index);
            }
        }
        if database {
            if let Some(bytes) = content {
                match self.snapshot(input, &bytes) {
                    Ok(snapshot) => {
                        let parsed =
                            database::inspect(&snapshot.connection, &input.logical, &input.origin);
                        for warning in parsed.warnings {
                            self.warn(warning);
                        }
                        self.photos.extend(parsed.photos);
                        for finding in parsed.findings {
                            self.push(finding);
                        }
                    }
                    Err(error) => self.warn(format!(
                        "Database analysis skipped for {}: {error}",
                        input.logical
                    )),
                }
            }
        }
    }

    fn connect_duplicates(&mut self) {
        let groups: Vec<Vec<usize>> = self
            .file_groups
            .values()
            .chain(self.secret_groups.values())
            .filter(|group| group.len() > 1)
            .cloned()
            .collect();
        for group in groups {
            for (position, &index) in group.iter().enumerate() {
                // A bounded ring keeps links reciprocal without an unbounded hub.
                let mut related = Vec::new();
                for offset in 1..=8.min(group.len() - 1) {
                    for other in [
                        group[(position + offset) % group.len()],
                        group[(position + group.len() - offset) % group.len()],
                    ] {
                        let identity = &self.case.findings[other].id;
                        if !related.contains(identity) {
                            related.push(identity.clone());
                        }
                    }
                }
                let finding = &mut self.case.findings[index];
                for id in related {
                    if !finding.related_ids.contains(&id) {
                        finding.related_ids.push(id);
                    }
                }
                finding.evidence.push(evidence(
                    "Repeated content",
                    format!("{} occurrences in this case", group.len()),
                ));
                if finding.category != Category::Secret {
                    finding.confidence = Confidence::Linked;
                }
            }
        }
    }

    fn connect_photos(&mut self) {
        self.photos
            .retain(|asset| self.finding_ids.contains(&asset.finding_id));
        let mut catalogue: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for asset in &self.photos {
            let stem = Path::new(&asset.filename)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let folder = asset
                .directory
                .rsplit('/')
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            if !stem.is_empty() {
                catalogue
                    .entry(stem)
                    .or_default()
                    .push((folder, asset.finding_id.clone()));
            }
        }
        let mut connections = Vec::new();
        for finding in &mut self.case.findings {
            if finding.rule_id != "file.media"
                || !finding.path.to_ascii_lowercase().contains("thumbnail")
            {
                continue;
            }
            finding.evidence.push(evidence(
                "Thumbnail path",
                "Path suggests a thumbnail/cache; visual identity is not verified",
            ));
            if self.photos.is_empty() {
                finding.evidence.push(evidence(
                    "Catalogue association",
                    "Unavailable; no supported Photos rows were imported",
                ));
                continue;
            }
            let basename = Path::new(&finding.path)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let lower_path = finding.path.to_ascii_lowercase();
            let matches: Vec<_> = catalogue
                .get(&basename)
                .into_iter()
                .flatten()
                .filter(|(folder, _)| folder.is_empty() || lower_path.contains(folder))
                .take(2)
                .collect();
            if matches.len() == 1 {
                finding.confidence = Confidence::Linked;
                connections.push((finding.id.clone(), matches[0].1.clone()));
                finding.evidence.push(evidence("Catalogue association", "Filename stem and available directory match one imported asset row; not a visual/content match"));
            } else {
                finding.confidence = Confidence::Candidate;
                finding.summary = "Thumbnail-path file could not be uniquely associated with the inspected catalogue rows. Cloud-only data, partial coverage and schema differences can explain this; permanent deletion is not established.".into();
                finding.evidence.push(evidence(
                    "Catalogue association",
                    if matches.is_empty() {
                        "No match in inspected rows"
                    } else {
                        "Ambiguous filename matches"
                    },
                ));
            }
        }
        let indices: HashMap<_, _> = self
            .case
            .findings
            .iter()
            .enumerate()
            .map(|(index, finding)| (finding.id.clone(), index))
            .collect();
        for (media, asset) in connections {
            if let (Some(&left), Some(&right)) = (indices.get(&media), indices.get(&asset)) {
                if self.case.findings[left].related_ids.len() >= 128
                    || self.case.findings[right].related_ids.len() >= 128
                {
                    self.warn("Some catalogue graph edges were omitted at the 128-neighbour limit; association facts remain available.".into());
                    continue;
                }
                if !self.case.findings[left].related_ids.contains(&asset) {
                    self.case.findings[left].related_ids.push(asset);
                }
                if !self.case.findings[right].related_ids.contains(&media) {
                    self.case.findings[right].related_ids.push(media);
                }
            }
        }
    }
}
