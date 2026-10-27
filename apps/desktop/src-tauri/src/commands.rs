use crate::session::{on_worker, Comparison, DesktopState, RuntimeInfo, Session};
use engram_core::{
    compare_cases, demo_case, read_case, scan_path, write_case, Case, ScanOptions, SourceKind,
};
use std::{io::Write, path::Path};
use tauri::{AppHandle, State, WebviewWindow};

fn require_main(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("Evidence commands are restricted to the main memory desk.".into())
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
) -> Result<Option<String>, String> {
    require_main(&window)?;
    let operation = state.session.begin()?;
    let case = operation.session.active_case()?;
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
    on_worker(operation, move |_| {
        write_case(&case, &destination).map_err(|error| error.to_string())?;
        Ok(Some(destination.to_string_lossy().into_owned()))
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
}
