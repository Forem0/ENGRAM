use engram_core::{Case, CaseDiff};
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
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
                launch,
            }),
        }
    }
}

pub(crate) struct Session {
    active: Mutex<Option<Case>>,
    busy: AtomicBool,
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
        Ok(case)
    }

    pub fn busy(&self) -> bool {
        self.busy.load(Ordering::Acquire)
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
    }
}
