use engram_core::{validate_review, Case, CaseDiff, CaseReview};
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
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
                    ]
                    .contains(&view.as_str())
                    {
                        return Err("Unknown view. Choose map, findings, timeline, connections, compare or modules.".into());
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
                active: Mutex::new(None),
                busy: AtomicBool::new(false),
                revision: AtomicU64::new(0),
                launch,
            }),
        }
    }
}

pub(crate) struct Session {
    active: Mutex<Option<Case>>,
    busy: AtomicBool,
    revision: AtomicU64,
    pub launch: Launch,
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
        self.active
            .lock()
            .map_err(|_| "The current case is unavailable; restart the application.".to_owned())?
            .clone()
            .ok_or_else(|| "Open a case, choose a source or load the demonstration first.".into())
    }

    pub fn replace_case(&self, case: Case) -> Result<Case, String> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| "The current case is unavailable; restart the application.".to_owned())?;
        *active = Some(case.clone());
        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(case)
    }

    pub fn prepare_save(
        &self,
        expected_case_id: &str,
        review: CaseReview,
    ) -> Result<SaveSnapshot, String> {
        let snapshot = {
            let active = self.active.lock().map_err(|_| {
                "The current case is unavailable; restart the application.".to_owned()
            })?;
            let case = active.as_ref().ok_or_else(|| {
                "Open a case, choose a source or load the demonstration first.".to_owned()
            })?;
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
        let active = self
            .active
            .lock()
            .map_err(|_| "The current case is unavailable; restart the application.".to_owned())?;
        if self.revision.load(Ordering::Relaxed) != snapshot.revision
            || active
                .as_ref()
                .is_none_or(|case| case.id != snapshot.case.id)
        {
            return Err("The active case changed. Save the current case again.".into());
        }
        Ok(())
    }

    /// Accept only the review from the snapshot that was actually persisted.
    /// A cancelled or failed save never calls this method. If another case has
    /// replaced the snapshot, retain that case and still report the saved file.
    pub fn accept_saved_review(&self, snapshot: &SaveSnapshot) -> bool {
        let Ok(mut active) = self.active.lock() else {
            return false;
        };
        if self.revision.load(Ordering::Relaxed) != snapshot.revision {
            return false;
        }
        let Some(case) = active.as_mut().filter(|case| case.id == snapshot.case.id) else {
            return false;
        };
        case.review = snapshot.case.review.clone();
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
    use engram_core::{demo_case, ReviewAnnotation, ReviewStatus};

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
}
