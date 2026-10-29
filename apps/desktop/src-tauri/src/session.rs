use engram_core::{validate_review, Case, CaseDiff, CaseReview, MediaAccess, MediaPreview};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
};

#[derive(Clone, Debug)]
pub(crate) struct Launch {
    pub initial_view: String,
    pub smoke_test: bool,
    pub help: bool,
}

impl Default for Launch {
    fn default() -> Self {
        Self {
            initial_view: "map".into(),
            smoke_test: false,
            help: false,
        }
    }
}

impl Launch {
    pub fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut launch = Self::default();
        let mut arguments = arguments.into_iter();
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--demo" => (),
                "--smoke-test" => launch.smoke_test = true,
                "--help" | "-h" => launch.help = true,
                "--view" => {
                    let view = arguments.next().ok_or("--view requires a view name")?;
                    if ![
                        "map",
                        "findings",
                        "timeline",
                        "connections",
                        "compare",
                        "modules",
                        "gallery",
                    ]
                    .contains(&view.as_str())
                    {
                        return Err("Unknown view. Choose map, findings, timeline, connections, compare, modules or gallery.".into());
                    }
                    launch.initial_view = view;
                }
                _ => return Err(format!("Unsupported argument: {argument}. Use --help.")),
            }
        }
        Ok(launch)
    }
}

#[derive(Serialize)]
pub(crate) struct RuntimeInfo {
    pub version: &'static str,
    pub engine: &'static str,
    pub schema_version: u32,
    pub initial_view: String,
    pub smoke_test: bool,
}

#[derive(Serialize)]
pub(crate) struct Comparison {
    pub before: Case,
    pub diff: CaseDiff,
}

pub(crate) struct DesktopState {
    pub session: Arc<Session>,
}

impl DesktopState {
    pub fn new(launch: Launch) -> Self {
        Self {
            session: Arc::new(Session {
                data: Mutex::new(SessionData::default()),
                busy: AtomicBool::new(false),
                revision: AtomicU64::new(0),
                backup_sequence: AtomicU64::new(1),
                launch,
            }),
        }
    }
}

pub(crate) struct Session {
    data: Mutex<SessionData>,
    busy: AtomicBool,
    revision: AtomicU64,
    backup_sequence: AtomicU64,
    pub launch: Launch,
}

#[derive(Default)]
struct SessionData {
    active: Option<ActiveCase>,
    pending_backup: Option<PendingBackup>,
}

struct ActiveCase {
    case: Case,
    media: Option<MediaAccess>,
}

struct PendingBackup {
    token: String,
    source: PathBuf,
}

impl Session {
    pub fn begin(self: &Arc<Self>) -> Result<Operation, String> {
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "Another operation is running. Wait for it to finish.".to_owned())?;
        Ok(Operation {
            session: Arc::clone(self),
        })
    }

    pub fn active_case(&self) -> Result<Case, String> {
        self.data
            .lock()
            .map_err(|_| "The current case is unavailable; restart the application.".to_owned())?
            .active
            .as_ref()
            .map(|active| active.case.clone())
            .ok_or_else(|| "Open a case, choose a source or load the demonstration first.".into())
    }

    pub fn replace_case(&self, case: Case) -> Result<Case, String> {
        self.install_case(case, None)
    }

    pub fn replace_scanned_case(&self, case: Case, access: MediaAccess) -> Result<Case, String> {
        if access.case_id() != case.id {
            return Err("Media access belongs to another source observation.".into());
        }
        self.install_case(case, Some(access))
    }

    fn install_case(&self, case: Case, media: Option<MediaAccess>) -> Result<Case, String> {
        let mut data = self
            .data
            .lock()
            .map_err(|_| "The current case is unavailable; restart the application.".to_owned())?;
        data.active = Some(ActiveCase {
            case: case.clone(),
            media,
        });
        data.pending_backup = None;
        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(case)
    }

    pub fn preview_media(
        &self,
        expected_case_id: &str,
        finding_id: &str,
    ) -> Result<MediaPreview, String> {
        // Hold the native observation while the bounded decode runs. A loaded
        // JSON case can supply metadata, but can never authorize a source read.
        let data = self
            .data
            .lock()
            .map_err(|_| "The current case is unavailable; restart the application.".to_owned())?;
        let active = data
            .active
            .as_ref()
            .ok_or("Open a source observation first.")?;
        if active.case.id != expected_case_id {
            return Err("The active case changed. Select an image in the current case.".into());
        }
        let finding = active
            .case
            .findings
            .iter()
            .find(|finding| finding.id == finding_id)
            .filter(|finding| {
                finding.category == engram_core::Category::Media && finding.rule_id == "file.media"
            })
            .ok_or("Select an observed media file from the current case.")?;
        let expected_hash = finding.sha256.as_deref()
            .ok_or("This image was not read within the scan limits. Inspect the source again before previewing it.")?;
        let access = active.media.as_ref().ok_or(
            "Image bytes are not attached to this case. Choose the original source explicitly to inspect its images."
        )?;
        let preview = access
            .preview(finding_id)
            .map_err(|error| error.to_string())?;
        if preview.finding_id != finding.id || preview.source_sha256 != expected_hash {
            return Err("Image bytes do not match the recorded source observation. Inspect the source again.".into());
        }
        Ok(preview)
    }

    pub fn clear_pending_backup(&self) -> Result<(), String> {
        self.data
            .lock()
            .map_err(|_| {
                "The backup selection is unavailable; restart the application.".to_owned()
            })?
            .pending_backup = None;
        Ok(())
    }

    pub fn stage_backup(&self, source: PathBuf) -> Result<String, String> {
        // This is a one-use local selection identifier, not an authorization
        // secret. The renderer never interprets it or sends a filesystem path.
        let mut sequence = self.backup_sequence.load(Ordering::Relaxed);
        loop {
            let next = sequence.checked_add(1).ok_or_else(|| {
                "Backup selection limit reached; restart the application.".to_owned()
            })?;
            match self.backup_sequence.compare_exchange_weak(
                sequence,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(current) => sequence = current,
            }
        }
        let token = format!("backup-{sequence:016x}");
        self.data
            .lock()
            .map_err(|_| {
                "The backup selection is unavailable; restart the application.".to_owned()
            })?
            .pending_backup = Some(PendingBackup {
            token: token.clone(),
            source,
        });
        Ok(token)
    }

    pub fn take_backup(&self, token: &str) -> Result<PathBuf, String> {
        let mut data = self.data.lock().map_err(|_| {
            "The backup selection is unavailable; restart the application.".to_owned()
        })?;
        if data
            .pending_backup
            .as_ref()
            .is_none_or(|pending| pending.token != token)
        {
            return Err("This backup selection expired. Choose the backup folder again.".into());
        }
        Ok(data
            .pending_backup
            .take()
            .expect("Matching pending selection exists")
            .source)
    }

    pub fn cancel_backup(&self, token: &str) -> Result<(), String> {
        self.take_backup(token).map(|_| ())
    }

    pub fn prepare_save(
        &self,
        expected_case_id: &str,
        review: CaseReview,
    ) -> Result<SaveSnapshot, String> {
        let snapshot = {
            let data = self.data.lock().map_err(|_| {
                "The current case is unavailable; restart the application.".to_owned()
            })?;
            let case = &data
                .active
                .as_ref()
                .ok_or_else(|| {
                    "Open a case, choose a source or load the demonstration first.".to_owned()
                })?
                .case;
            if expected_case_id != case.id {
                return Err("The active case changed. Save the current case again.".into());
            }
            let review =
                validate_review(&case.findings, &review).map_err(|error| error.to_string())?;
            let mut snapshot_case = case.clone();
            snapshot_case.review = review;
            SaveSnapshot {
                case: snapshot_case,
                revision: self.revision.load(Ordering::Relaxed),
            }
        };
        self.require_current_snapshot(&snapshot)?;
        Ok(snapshot)
    }

    pub fn require_current_snapshot(&self, snapshot: &SaveSnapshot) -> Result<(), String> {
        let data = self
            .data
            .lock()
            .map_err(|_| "The current case is unavailable; restart the application.".to_owned())?;
        if self.revision.load(Ordering::Relaxed) != snapshot.revision
            || data
                .active
                .as_ref()
                .is_none_or(|active| active.case.id != snapshot.case.id)
        {
            return Err("The active case changed. Save the current case again.".into());
        }
        Ok(())
    }

    /// Accept only the review from the snapshot that was actually persisted.
    /// A cancelled or failed save never calls this method. If another case has
    /// replaced the snapshot, retain that case and still report the saved file.
    pub fn accept_saved_review(&self, snapshot: &SaveSnapshot) -> bool {
        let Ok(mut data) = self.data.lock() else {
            return false;
        };
        if self.revision.load(Ordering::Relaxed) != snapshot.revision {
            return false;
        }
        let Some(active) = data
            .active
            .as_mut()
            .filter(|active| active.case.id == snapshot.case.id)
        else {
            return false;
        };
        active.case.review = snapshot.case.review.clone();
        self.revision.fetch_add(1, Ordering::Relaxed);
        true
    }

    pub fn busy(&self) -> bool {
        self.busy.load(Ordering::Acquire)
    }
}

#[derive(Debug)]
pub(crate) struct SaveSnapshot {
    case: Case,
    revision: u64,
}

impl SaveSnapshot {
    pub fn case(&self) -> &Case {
        &self.case
    }
}

pub(crate) struct Operation {
    pub session: Arc<Session>,
}

impl Drop for Operation {
    fn drop(&mut self) {
        self.session.busy.store(false, Ordering::Release);
    }
}

pub(crate) async fn on_worker<T, F>(operation: Operation, work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&Session) -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || {
        let result = work(&operation.session);
        drop(operation);
        result
    })
    .await
    .map_err(|_| {
        "The operation stopped unexpectedly. The previous case has been retained.".to_owned()
    })?
}

#[cfg(test)]
mod tests {
    use super::*;
    use engram_core::{
        create_demo_fixture, demo_case, scan_path_with_media, Category, ReviewAnnotation,
        ReviewStatus, ScanOptions,
    };

    fn scanned_fixture() -> (tempfile::TempDir, Case, MediaAccess) {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("synthetic-source");
        create_demo_fixture(&source).unwrap();
        let (case, access) = scan_path_with_media(&source, &ScanOptions::default()).unwrap();
        (directory, case, access)
    }

    fn image_finding(case: &Case) -> &engram_core::Finding {
        case.findings
            .iter()
            .find(|finding| {
                finding.category == Category::Media
                    && finding.rule_id == "file.media"
                    && finding.path.ends_with("IMG_0001.PNG")
            })
            .unwrap()
    }

    fn marked_review(case: &Case) -> CaseReview {
        CaseReview::from([(
            case.findings[0].id.clone(),
            ReviewAnnotation {
                bookmarked: true,
                status: ReviewStatus::NeedsReview,
                note: "Review this observation.".into(),
            },
        )])
    }

    #[test]
    fn native_operation_guard_rejects_overlap_and_recovers_after_drop() {
        let state = DesktopState::new(Launch::default());
        let operation = state.session.begin().unwrap();
        assert!(state.session.begin().is_err());
        drop(operation);
        assert!(state.session.begin().is_ok());
    }

    #[test]
    fn renderer_runtime_contract_and_launch_view_are_explicit() {
        let launch =
            Launch::parse(["--view".into(), "timeline".into(), "--smoke-test".into()]).unwrap();
        assert_eq!(launch.initial_view, "timeline");
        assert!(launch.smoke_test);
        assert!(Launch::parse(["--view".into(), "arbitrary".into()]).is_err());
        assert!(Launch::parse(["--view".into()]).is_err());
        let info = RuntimeInfo {
            version: env!("CARGO_PKG_VERSION"),
            engine: "engram-core",
            schema_version: 1,
            initial_view: launch.initial_view,
            smoke_test: launch.smoke_test,
        };
        let info = serde_json::to_value(info).unwrap();
        assert_eq!(info["schema_version"], 1);
        assert_eq!(info["initial_view"], "timeline");
        assert_eq!(info["engine"], "engram-core");
    }

    #[test]
    fn absence_of_active_case_is_an_error_and_not_an_empty_success() {
        let state = DesktopState::new(Launch::default());
        assert!(state.session.active_case().is_err());
        assert!(state
            .session
            .prepare_save("missing-case", CaseReview::new())
            .is_err());
    }

    #[test]
    fn stale_renderer_case_ids_and_dangling_review_are_rejected_before_a_picker() {
        let state = DesktopState::new(Launch::default());
        let original = demo_case();
        state.session.replace_case(original.clone()).unwrap();
        assert!(state
            .session
            .prepare_save("wrong-case-id", marked_review(&original))
            .is_err());
        let review = CaseReview::from([("missing-finding".into(), ReviewAnnotation::default())]);
        assert!(state.session.prepare_save(&original.id, review).is_err());
        assert_eq!(state.session.active_case().unwrap().review, original.review);
    }

    #[test]
    fn preparing_and_cancelling_a_save_leave_the_session_review_unchanged() {
        let state = DesktopState::new(Launch::default());
        let original = demo_case();
        state.session.replace_case(original.clone()).unwrap();
        let operation = state.session.begin().unwrap();
        let snapshot = operation
            .session
            .prepare_save(&original.id, marked_review(&original))
            .unwrap();
        assert!(!snapshot.case().review.is_empty());
        assert_eq!(state.session.active_case().unwrap().review, original.review);
        drop(snapshot);
        drop(operation);
        assert!(!state.session.busy());
        assert_eq!(state.session.active_case().unwrap().review, original.review);
    }

    #[test]
    fn replacing_a_case_with_the_same_id_still_invalidates_its_save_snapshot() {
        let state = DesktopState::new(Launch::default());
        let original = demo_case();
        state.session.replace_case(original.clone()).unwrap();
        let snapshot = state
            .session
            .prepare_save(&original.id, marked_review(&original))
            .unwrap();
        let mut replacement = original.clone();
        replacement.title = "A later snapshot with the same imported ID".into();
        state.session.replace_case(replacement.clone()).unwrap();
        assert!(state.session.require_current_snapshot(&snapshot).is_err());
        assert!(!state.session.accept_saved_review(&snapshot));
        assert_eq!(
            state.session.active_case().unwrap().title,
            replacement.title
        );
        assert_eq!(
            state.session.active_case().unwrap().review,
            replacement.review
        );
    }

    #[test]
    fn accepting_a_saved_snapshot_updates_only_its_review_and_invalidates_reuse() {
        let state = DesktopState::new(Launch::default());
        let original = demo_case();
        state.session.replace_case(original.clone()).unwrap();
        let review = marked_review(&original);
        let snapshot = state
            .session
            .prepare_save(&original.id, review.clone())
            .unwrap();
        assert!(state.session.accept_saved_review(&snapshot));
        let active = state.session.active_case().unwrap();
        assert_eq!(active.review, review);
        assert_eq!(active.title, original.title);
        assert_eq!(active.source_root, original.source_root);
        assert_eq!(active.findings.len(), original.findings.len());
        assert!(state.session.require_current_snapshot(&snapshot).is_err());
        assert!(!state.session.accept_saved_review(&snapshot));
    }

    #[test]
    fn gallery_is_an_explicit_native_launch_view() {
        assert_eq!(
            Launch::parse(["--view".into(), "gallery".into()])
                .unwrap()
                .initial_view,
            "gallery"
        );
    }

    #[test]
    fn scanned_media_preview_matches_the_native_observation() {
        let (_directory, case, access) = scanned_fixture();
        let state = DesktopState::new(Launch::default());
        let finding = image_finding(&case);
        let id = finding.id.clone();
        let hash = finding.sha256.clone().unwrap();
        state
            .session
            .replace_scanned_case(case.clone(), access)
            .unwrap();
        let preview = state.session.preview_media(&case.id, &id).unwrap();
        assert_eq!(preview.finding_id, id);
        assert_eq!(preview.source_sha256, hash);
        assert!(preview.data_url.starts_with("data:image/png;base64,"));
        assert!((1..=1024).contains(&preview.width));
        assert!((1..=1024).contains(&preview.height));
        assert!(state.session.preview_media("stale-case", &id).is_err());
        assert!(state
            .session
            .preview_media(&case.id, "unknown-finding")
            .is_err());
    }

    #[test]
    fn source_access_cannot_be_attached_to_a_different_case() {
        let (_directory, mut case, access) = scanned_fixture();
        let state = DesktopState::new(Launch::default());
        let original = demo_case();
        state.session.replace_case(original.clone()).unwrap();
        case.id = "different-observation".into();
        assert!(state.session.replace_scanned_case(case, access).is_err());
        assert_eq!(state.session.active_case().unwrap().id, original.id);
    }

    #[test]
    fn preview_must_also_match_the_recorded_finding_hash() {
        let (_directory, mut case, access) = scanned_fixture();
        let state = DesktopState::new(Launch::default());
        let id = image_finding(&case).id.clone();
        case.findings
            .iter_mut()
            .find(|finding| finding.id == id)
            .unwrap()
            .sha256 = Some("0".repeat(64));
        state
            .session
            .replace_scanned_case(case.clone(), access)
            .unwrap();
        assert!(state.session.preview_media(&case.id, &id).is_err());
    }

    #[test]
    fn replacing_with_an_imported_case_of_the_same_id_revokes_media_reads() {
        let (_directory, case, access) = scanned_fixture();
        let state = DesktopState::new(Launch::default());
        let id = image_finding(&case).id.clone();
        state
            .session
            .replace_scanned_case(case.clone(), access)
            .unwrap();
        assert!(state.session.preview_media(&case.id, &id).is_ok());
        // Importing even identical evidence must not grant its stored paths any
        // filesystem authority. No reads are re-created from case.source_root.
        state.session.replace_case(case.clone()).unwrap();
        assert!(state.session.preview_media(&case.id, &id).is_err());
        assert_eq!(
            state.session.active_case().unwrap().findings.len(),
            case.findings.len()
        );
    }

    #[test]
    fn accepting_only_saved_review_keeps_the_scanned_media_authority() {
        let (_directory, case, access) = scanned_fixture();
        let state = DesktopState::new(Launch::default());
        let id = image_finding(&case).id.clone();
        state
            .session
            .replace_scanned_case(case.clone(), access)
            .unwrap();
        let snapshot = state
            .session
            .prepare_save(&case.id, marked_review(&case))
            .unwrap();
        assert!(state.session.accept_saved_review(&snapshot));
        assert!(state.session.preview_media(&case.id, &id).is_ok());
    }

    #[test]
    fn preview_rejects_source_changes_without_mutating_the_case() {
        let (_directory, case, access) = scanned_fixture();
        let state = DesktopState::new(Launch::default());
        let finding = image_finding(&case);
        let id = finding.id.clone();
        let path = PathBuf::from(&case.source_root).join(&finding.path);
        state
            .session
            .replace_scanned_case(case.clone(), access)
            .unwrap();
        std::fs::write(path, b"changed synthetic image").unwrap();
        assert!(state.session.preview_media(&case.id, &id).is_err());
        assert_eq!(state.session.active_case().unwrap().id, case.id);
        assert_eq!(state.session.active_case().unwrap().review, case.review);
    }

    #[test]
    fn backup_tokens_are_single_use_and_stale_tokens_cannot_consume_a_new_selection() {
        let state = DesktopState::new(Launch::default());
        let old = state
            .session
            .stage_backup(PathBuf::from("first-synthetic-selection"))
            .unwrap();
        let current = state
            .session
            .stage_backup(PathBuf::from("second-synthetic-selection"))
            .unwrap();
        assert_ne!(old, current);
        assert!(state.session.take_backup(&old).is_err());
        assert!(state.session.cancel_backup(&old).is_err());
        assert_eq!(
            state.session.take_backup(&current).unwrap(),
            PathBuf::from("second-synthetic-selection")
        );
        assert!(state.session.take_backup(&current).is_err());
    }

    #[test]
    fn cancelling_a_backup_and_replacing_a_case_invalidate_the_selection() {
        let state = DesktopState::new(Launch::default());
        let case = demo_case();
        state.session.replace_case(case.clone()).unwrap();
        let token = state
            .session
            .stage_backup(PathBuf::from("cancelled-synthetic-selection"))
            .unwrap();
        state.session.cancel_backup(&token).unwrap();
        assert!(state.session.take_backup(&token).is_err());
        assert_eq!(state.session.active_case().unwrap().id, case.id);
        let token = state
            .session
            .stage_backup(PathBuf::from("replaced-synthetic-selection"))
            .unwrap();
        state.session.replace_case(case.clone()).unwrap();
        assert!(state.session.take_backup(&token).is_err());
        let token = state
            .session
            .stage_backup(PathBuf::from("picker-cancelled-selection"))
            .unwrap();
        state.session.clear_pending_backup().unwrap();
        assert!(state.session.take_backup(&token).is_err());
    }
}
