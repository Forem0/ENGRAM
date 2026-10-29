use crate::session::{on_worker, Comparison, DesktopState, RuntimeInfo, SaveSnapshot, Session};
use engram_core::{
    compare_cases, demo_case, read_case, render_report, scan_path, write_case, write_report, Case,
    CaseReview, ReportFormat, ReportOptions, ReportReceipt, ScanOptions, SourceKind,
};
use std::{io::Write, path::Path};
use tauri::{AppHandle, State, WebviewWindow};

fn require_main(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("Evidence commands are restricted to the main trace desk.".into())
    }
}

fn import_case(session: &Session, source: &Path) -> Result<Case, String> {
    let case = read_case(source).map_err(|error| error.to_string())?;
    session.replace_case(case)
}

fn ensure_demo(case: &Case) -> Result<(), String> {
    if case.id == "demo-unavailable" || case.findings.is_empty() {
        return Err("The synthetic demonstration could not be generated. Choose a source or open a saved case.".into());
    }
    Ok(())
}

fn persist_review_snapshot(
    session: &Session,
    snapshot: &SaveSnapshot,
    destination: &Path,
) -> Result<String, String> {
    session.require_current_snapshot(snapshot)?;
    write_case(snapshot.case(), destination).map_err(|error| error.to_string())?;
    session.accept_saved_review(snapshot);
    Ok(destination.to_string_lossy().into_owned())
}

fn prepare_report_snapshot(
    session: &Session,
    case_id: &str,
    review: CaseReview,
    options: &ReportOptions,
) -> Result<SaveSnapshot, String> {
    let snapshot = session.prepare_save(case_id, review)?;
    // Invalid selections and over-limit output fail before opening a save picker.
    // Rendering is deterministic and is repeated from this immutable snapshot
    // after the picker; the renderer never supplies report content or paths.
    render_report(snapshot.case(), options).map_err(|error| error.to_string())?;
    session.require_current_snapshot(&snapshot)?;
    Ok(snapshot)
}

fn persist_report_snapshot(
    session: &Session,
    snapshot: &SaveSnapshot,
    options: &ReportOptions,
    destination: &Path,
) -> Result<ReportReceipt, String> {
    session.require_current_snapshot(snapshot)?;
    // Deliberately do not accept_saved_review: a report is not a saved case.
    write_report(snapshot.case(), options, destination).map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn get_runtime_info(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
) -> Result<RuntimeInfo, String> {
    require_main(&window)?;
    Ok(RuntimeInfo {
        version: env!("CARGO_PKG_VERSION"),
        engine: "engram-core",
        schema_version: 1,
        initial_view: state.session.launch.initial_view.clone(),
        smoke_test: state.session.launch.smoke_test,
    })
}

#[tauri::command]
pub(crate) async fn load_demo(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
) -> Result<Case, String> {
    require_main(&window)?;
    let operation = state.session.begin()?;
    on_worker(operation, |session| {
        let case = demo_case();
        ensure_demo(&case)?;
        session.replace_case(case)
    })
    .await
}

#[tauri::command]
pub(crate) async fn select_source(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
) -> Result<Option<Case>, String> {
    require_main(&window)?;
    let operation = state.session.begin()?;
    let Some(selection) = rfd::AsyncFileDialog::new()
        .set_parent(&window)
        .set_title("Select a folder or unencrypted iPhone backup")
        .pick_folder()
        .await
    else {
        return Ok(None);
    };
    let source = selection.path().to_owned();
    on_worker(operation, move |session| {
        let case =
            scan_path(&source, &ScanOptions::default()).map_err(|error| error.to_string())?;
        session.replace_case(case).map(Some)
    })
    .await
}

#[tauri::command]
pub(crate) async fn open_case(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
) -> Result<Option<Case>, String> {
    require_main(&window)?;
    let operation = state.session.begin()?;
    let Some(selection) = rfd::AsyncFileDialog::new()
        .set_parent(&window)
        .set_title("Open an ENGRAM case")
        .add_filter("ENGRAM case", &["json"])
        .pick_file()
        .await
    else {
        return Ok(None);
    };
    let source = selection.path().to_owned();
    on_worker(operation, move |session| {
        import_case(session, &source).map(Some)
    })
    .await
}

#[tauri::command]
pub(crate) async fn save_case(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    case_id: String,
    review: CaseReview,
) -> Result<Option<String>, String> {
    require_main(&window)?;
    let operation = state.session.begin()?;
    // Only review metadata crosses IPC. Evidence and source paths come from the
    // guarded native session, and stale renderer case IDs fail before the picker.
    let snapshot = operation.session.prepare_save(&case_id, review)?;
    let Some(selection) = rfd::AsyncFileDialog::new()
        .set_parent(&window)
        .set_title("Save a new case outside the source folder")
        .set_file_name("case.engram.json")
        .add_filter("ENGRAM case", &["json"])
        .save_file()
        .await
    else {
        return Ok(None);
    };
    let destination = selection.path().to_owned();
    on_worker(operation, move |session| {
        persist_review_snapshot(session, &snapshot, &destination).map(Some)
    })
    .await
}

#[tauri::command]
pub(crate) async fn export_report(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    case_id: String,
    review: CaseReview,
    options: ReportOptions,
) -> Result<Option<ReportReceipt>, String> {
    require_main(&window)?;
    let operation = state.session.begin()?;
    let (operation, snapshot, options) = tauri::async_runtime::spawn_blocking(move || {
        let snapshot = prepare_report_snapshot(&operation.session, &case_id, review, &options)?;
        Ok::<_, String>((operation, snapshot, options))
    })
    .await
    .map_err(|_| {
        "Report preparation stopped. The current case and review were retained.".to_owned()
    })??;
    let picker = rfd::AsyncFileDialog::new()
        .set_parent(&window)
        .set_title("Export a new report outside the source folder");
    let picker = match options.format {
        ReportFormat::Markdown => picker
            .set_file_name("case.report.md")
            .add_filter("Markdown report", &["md"]),
        ReportFormat::Csv => picker
            .set_file_name("case.report.csv")
            .add_filter("CSV table", &["csv"]),
    };
    let Some(selection) = picker.save_file().await else {
        return Ok(None);
    };
    let destination = selection.path().to_owned();
    on_worker(operation, move |session| {
        persist_report_snapshot(session, &snapshot, &options, &destination).map(Some)
    })
    .await
}

#[tauri::command]
pub(crate) async fn compare_case(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
) -> Result<Option<Comparison>, String> {
    require_main(&window)?;
    let operation = state.session.begin()?;
    let current = operation.session.active_case()?;
    let Some(selection) = rfd::AsyncFileDialog::new()
        .set_parent(&window)
        .set_title("Choose an earlier case to compare with the current case")
        .add_filter("ENGRAM case", &["json"])
        .pick_file()
        .await
    else {
        return Ok(None);
    };
    let source = selection.path().to_owned();
    on_worker(operation, move |_| {
        let before = read_case(&source).map_err(|error| error.to_string())?;
        let diff = compare_cases(&before, &current);
        Ok(Some(Comparison { before, diff }))
    })
    .await
}

#[tauri::command]
pub(crate) fn finish_smoke_test(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, DesktopState>,
) -> Result<(), String> {
    require_main(&window)?;
    if !state.session.launch.smoke_test || state.session.busy() {
        return Err("Smoke completion is only available for an idle --smoke-test launch.".into());
    }
    let case = state.session.active_case()?;
    if case.source_kind != SourceKind::Demo {
        return Err("The smoke test must use the synthetic demonstration.".into());
    }
    ensure_demo(&case)?;
    println!("ENGRAM_TAURI_SMOKE_OK");
    let _ = std::io::stdout().flush();
    // Keep the real webview visible long enough for the CI display capture.
    // This delay is outside the event loop, so the final frame remains live.
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(5));
        app.exit(0);
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Launch;
    use engram_core::{ReviewAnnotation, ReviewStatus};

    fn reviewed_metadata(case: &Case) -> CaseReview {
        CaseReview::from([(
            case.findings[0].id.clone(),
            ReviewAnnotation {
                bookmarked: true,
                status: ReviewStatus::Reviewed,
                note: "Checked this source observation.".into(),
            },
        )])
    }

    #[test]
    fn invalid_import_does_not_discard_the_active_case() {
        let state = DesktopState::new(Launch::default());
        let original = demo_case();
        ensure_demo(&original).unwrap();
        let original_id = original.id.clone();
        let original_count = original.findings.len();
        state.session.replace_case(original).unwrap();
        let temporary = tempfile::tempdir().unwrap();
        let malformed = temporary.path().join("broken.engram.json");
        std::fs::write(&malformed, b"{ invalid json").unwrap();
        assert!(import_case(&state.session, &malformed).is_err());
        let retained = state.session.active_case().unwrap();
        assert_eq!(retained.id, original_id);
        assert_eq!(retained.findings.len(), original_count);
    }

    #[test]
    fn opening_a_valid_case_retains_its_validated_evidence() {
        let state = DesktopState::new(Launch::default());
        let original = demo_case();
        ensure_demo(&original).unwrap();
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().join("example.engram.json");
        write_case(&original, &path).unwrap();
        let opened = import_case(&state.session, &path).unwrap();
        assert_eq!(opened.id, original.id);
        assert_eq!(state.session.active_case().unwrap().id, original.id);
    }

    #[test]
    fn saving_reviewed_metadata_persists_and_updates_the_active_case() {
        let state = DesktopState::new(Launch::default());
        let original = demo_case();
        ensure_demo(&original).unwrap();
        state.session.replace_case(original.clone()).unwrap();
        let review = reviewed_metadata(&original);
        let snapshot = state
            .session
            .prepare_save(&original.id, review.clone())
            .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("reviewed.engram.json");
        persist_review_snapshot(&state.session, &snapshot, &output).unwrap();
        let reopened = read_case(&output).unwrap();
        assert_eq!(reopened.review, review);
        assert_eq!(state.session.active_case().unwrap().review, review);
        assert_eq!(reopened.source_root, original.source_root);
        let difference = compare_cases(&original, &reopened);
        assert!(difference.added.is_empty());
        assert!(difference.removed.is_empty());
        assert!(difference.changed.is_empty());
        assert_eq!(difference.unchanged.len(), original.findings.len());
    }

    #[test]
    fn failed_save_preserves_the_active_review_and_existing_output() {
        let state = DesktopState::new(Launch::default());
        let original = demo_case();
        ensure_demo(&original).unwrap();
        state.session.replace_case(original.clone()).unwrap();
        let snapshot = state
            .session
            .prepare_save(&original.id, reviewed_metadata(&original))
            .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("existing.engram.json");
        std::fs::write(&output, "Existing data must remain.").unwrap();
        assert!(persist_review_snapshot(&state.session, &snapshot, &output).is_err());
        assert_eq!(state.session.active_case().unwrap().review, original.review);
        assert_eq!(
            std::fs::read_to_string(&output).unwrap(),
            "Existing data must remain."
        );
    }

    #[test]
    fn successful_report_export_keeps_review_unsaved_and_snapshot_current() {
        let state = DesktopState::new(Launch::default());
        let original = demo_case();
        state.session.replace_case(original.clone()).unwrap();
        let review = reviewed_metadata(&original);
        let options = ReportOptions {
            format: ReportFormat::Csv,
            finding_ids: vec![original.findings[0].id.clone()],
            include_notes: true,
            include_paths: false,
            include_details: false,
        };
        let snapshot = prepare_report_snapshot(&state.session, &original.id, review, &options).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("report.csv");
        let receipt = persist_report_snapshot(&state.session, &snapshot, &options, &output).unwrap();
        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("Checked this source observation."));
        assert_eq!(receipt.finding_count, 1);
        assert_eq!(receipt.format, ReportFormat::Csv);
        assert_eq!(receipt.bytes, content.len());
        assert_eq!(receipt.sha256.len(), 64);
        assert_eq!(state.session.active_case().unwrap().review, original.review);
        assert!(state.session.require_current_snapshot(&snapshot).is_ok());
    }

    #[test]
    fn cancelled_report_selection_drops_the_guard_without_saving_review() {
        let state = DesktopState::new(Launch::default());
        let original = demo_case();
        state.session.replace_case(original.clone()).unwrap();
        let options = ReportOptions {
            format: ReportFormat::Markdown,
            finding_ids: vec![original.findings[0].id.clone()],
            include_notes: false,
            include_paths: false,
            include_details: false,
        };
        let operation = state.session.begin().unwrap();
        let snapshot = prepare_report_snapshot(&operation.session, &original.id, reviewed_metadata(&original), &options).unwrap();
        assert!(snapshot.case().review[&original.findings[0].id].bookmarked);
        drop(snapshot);
        drop(operation);
        assert!(!state.session.busy());
        assert_eq!(state.session.active_case().unwrap().review, original.review);
    }

    #[test]
    fn invalid_report_requests_fail_before_a_picker_or_output() {
        let state = DesktopState::new(Launch::default());
        let original = demo_case();
        state.session.replace_case(original.clone()).unwrap();
        let mut options = ReportOptions {
            format: ReportFormat::Markdown,
            finding_ids: vec!["unknown-finding".into()],
            include_notes: false,
            include_paths: false,
            include_details: false,
        };
        assert!(prepare_report_snapshot(&state.session, &original.id, reviewed_metadata(&original), &options).is_err());
        options.finding_ids = vec![original.findings[0].id.clone()];
        assert!(prepare_report_snapshot(&state.session, "stale-case", reviewed_metadata(&original), &options).is_err());
        let mut review = reviewed_metadata(&original);
        review.get_mut(&original.findings[0].id).unwrap().note = "invalid\0note".into();
        assert!(prepare_report_snapshot(&state.session, &original.id, review, &options).is_err());
        assert_eq!(state.session.active_case().unwrap().review, original.review);
    }

    #[test]
    fn report_snapshot_is_rechecked_after_a_picker_before_writing() {
        let state = DesktopState::new(Launch::default());
        let original = demo_case();
        state.session.replace_case(original.clone()).unwrap();
        let options = ReportOptions {
            format: ReportFormat::Markdown,
            finding_ids: vec![original.findings[0].id.clone()],
            include_notes: false,
            include_paths: false,
            include_details: false,
        };
        let snapshot = prepare_report_snapshot(&state.session, &original.id, reviewed_metadata(&original), &options).unwrap();
        let mut replacement = original.clone();
        replacement.title = "A replacement with the same imported case ID".into();
        state.session.replace_case(replacement.clone()).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("must-not-be-written.md");
        assert!(persist_report_snapshot(&state.session, &snapshot, &options, &output).is_err());
        assert!(!output.exists());
        assert_eq!(state.session.active_case().unwrap().title, replacement.title);
        assert_eq!(state.session.active_case().unwrap().review, replacement.review);
    }

    #[test]
    fn failed_report_write_does_not_replace_existing_data_or_save_review() {
        let state = DesktopState::new(Launch::default());
        let original = demo_case();
        state.session.replace_case(original.clone()).unwrap();
        let options = ReportOptions {
            format: ReportFormat::Markdown,
            finding_ids: vec![original.findings[0].id.clone()],
            include_notes: false,
            include_paths: false,
            include_details: false,
        };
        let snapshot = prepare_report_snapshot(&state.session, &original.id, reviewed_metadata(&original), &options).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("existing.md");
        std::fs::write(&output, "Existing output must remain unchanged.").unwrap();
        assert!(persist_report_snapshot(&state.session, &snapshot, &options, &output).is_err());
        assert_eq!(std::fs::read_to_string(&output).unwrap(), "Existing output must remain unchanged.");
        assert_eq!(state.session.active_case().unwrap().review, original.review);
        assert!(state.session.require_current_snapshot(&snapshot).is_ok());
    }

    #[test]
    fn stale_save_snapshot_is_rejected_before_writing_or_updating_a_replacement() {
        let state = DesktopState::new(Launch::default());
        let original = demo_case();
        ensure_demo(&original).unwrap();
        state.session.replace_case(original.clone()).unwrap();
        let snapshot = state
            .session
            .prepare_save(&original.id, reviewed_metadata(&original))
            .unwrap();
        let mut replacement = original.clone();
        replacement.title = "Replacement with the same imported ID".into();
        state.session.replace_case(replacement.clone()).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("must-not-be-written.engram.json");
        assert!(persist_review_snapshot(&state.session, &snapshot, &output).is_err());
        assert!(!output.exists());
        let retained = state.session.active_case().unwrap();
        assert_eq!(retained.title, replacement.title);
        assert_eq!(retained.review, replacement.review);
    }
}
