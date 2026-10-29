fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "get_runtime_info",
            "load_demo",
            "select_source",
            "open_case",
            "save_case",
            "export_report",
            "compare_case",
            "finish_smoke_test",
        ]),
    ))
    .expect("Cannot build the ENGRAM desktop permission manifest");
}
