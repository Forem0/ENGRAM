mod commands;
mod session;

use session::{DesktopState, Launch};
use tauri::{webview::NewWindowResponse, WebviewWindowBuilder};

pub fn run() {
    let launch = match Launch::parse(std::env::args().skip(1)) {
        Ok(launch) => launch,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };
    if launch.help {
        println!("ENGRAM — Trace desk\n\n  --demo         Open the synthetic demonstration (default)\n  --view VIEW    Start in map, findings, timeline, connections, compare or modules\n  --smoke-test   Verify the real frontend/engine bridge and exit\n\nChoose real sources and case files through the native app dialogs.");
        return;
    }
    tauri::Builder::default()
        .manage(DesktopState::new(launch))
        .invoke_handler(tauri::generate_handler![
            commands::get_runtime_info,
            commands::load_demo,
            commands::select_source,
            commands::open_case,
            commands::save_case,
            commands::compare_case,
            commands::finish_smoke_test,
        ])
        .setup(|app| {
            let configuration = app
                .config()
                .app
                .windows
                .first()
                .cloned()
                .ok_or("The main window configuration is missing")?;
            WebviewWindowBuilder::from_config(app, &configuration)?
                .on_navigation(|url| {
                    let packaged = (url.scheme() == "tauri"
                        && url.host_str() == Some("localhost")
                        && url.port().is_none())
                        || (matches!(url.scheme(), "http" | "https")
                            && url.host_str() == Some("tauri.localhost")
                            && url.port().is_none());
                    let development = !cfg!(feature = "custom-protocol")
                        && url.scheme() == "http"
                        && matches!(url.host_str(), Some("localhost" | "127.0.0.1"))
                        && url.port() == Some(1420);
                    packaged || development
                })
                .on_new_window(|_, _| NewWindowResponse::Deny)
                .on_download(|_, _| false)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Cannot run the ENGRAM desktop application");
}
