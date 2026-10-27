use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Media,
    Secret,
    Document,
    Browser,
    Message,
    Location,
    Database,
}

impl Category {
    pub const ALL: [Self; 7] = [
        Self::Media,
        Self::Secret,
        Self::Document,
        Self::Browser,
        Self::Message,
        Self::Location,
        Self::Database,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Media => "Media",
            Self::Secret => "Secrets",
            Self::Document => "Documents",
            Self::Browser => "Browser",
            Self::Message => "Messages",
            Self::Location => "Location",
            Self::Database => "Databases",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Observed,
    Linked,
    Candidate,
}

impl Confidence {
    pub fn label(self) -> &'static str {
        match self {
            Self::Observed => "Observed",
            Self::Linked => "Linked",
            Self::Candidate => "Candidate",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Folder,
    IosBackup,
    Demo,
}

impl SourceKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Folder => "Selected folder",
            Self::IosBackup => "iPhone backup",
            Self::Demo => "Synthetic demo",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleState {
    Ready,
    Experimental,
    Planned,
    Unavailable,
}

impl ModuleState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::Experimental => "Experimental",
            Self::Planned => "Planned",
            Self::Unavailable => "Unavailable",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub category: Category,
    pub title: String,
    pub path: String,
    pub origin: String,
    pub summary: String,
    pub locator: String,
    pub observed_at: Option<i64>,
    pub bytes: u64,
    pub sha256: Option<String>,
    pub confidence: Confidence,
    pub rule_id: String,
    pub sensitive: bool,
    pub evidence: Vec<Evidence>,
    pub related_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleStatus {
    pub name: String,
    pub state: ModuleState,
    pub detail: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CaseStats {
    pub files_examined: u64,
    pub bytes_examined: u64,
    pub skipped_files: u64,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanEvent {
    pub at_ms: u64,
    pub message: String,
    pub category: Option<Category>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Case {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub source_root: String,
    pub source_kind: SourceKind,
    pub created_at: i64,
    pub completed_at: i64,
    pub findings: Vec<Finding>,
    pub warnings: Vec<String>,
    pub stats: CaseStats,
    pub modules: Vec<ModuleStatus>,
    pub events: Vec<ScanEvent>,
}

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub max_files: usize,
    pub max_file_bytes: u64,
    pub max_total_bytes: u64,
    pub include_hidden: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            max_files: 25_000,
            max_file_bytes: 8 * 1024 * 1024,
            max_total_bytes: 512 * 1024 * 1024,
            include_hidden: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CaseDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
    pub unchanged: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum EngramError {
    #[error("{0}")]
    Input(String),
    #[error("Cannot read or write local data: {0}")]
    Io(#[from] std::io::Error),
    #[error("Cannot parse case: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Cannot read database: {0}")]
    Sqlite(#[from] rusqlite::Error),
}
