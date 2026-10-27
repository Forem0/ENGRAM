//! Offline evidence, bounded source reads, and redacted case files for ENGRAM.
//!
//! Findings describe the supplied snapshot. A candidate is not proof of deletion,
//! credential validity, application access, or complete device coverage.

mod database;
mod demo;
pub mod model;
mod scan;
mod secrets;
mod storage;

pub use demo::{create_demo_fixture, demo_case};
pub use model::*;
pub use scan::scan_path;
pub use storage::{compare_cases, read_case, write_case};

pub(crate) const MAX_FINDINGS: usize = 30_000;
pub(crate) const MAX_DB_ROWS: usize = 1_000;

pub(crate) fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn finding_id(path: &str, rule: &str, locator: &str) -> String {
    digest(format!("{path}\0{rule}\0{locator}").as_bytes())[..24].to_owned()
}

pub(crate) fn time_from_system(time: std::time::SystemTime) -> Option<i64> {
    time.duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|value| i64::try_from(value.as_secs()).ok())
}
