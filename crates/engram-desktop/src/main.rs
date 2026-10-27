#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod map;
mod theme;

use std::path::PathBuf;

#[derive(Default)]
pub struct Launch {
    screenshot: Option<PathBuf>,
    case: Option<PathBuf>,
    source: Option<PathBuf>,
    view: Option<String>,
    exit_after_screenshot: bool,
}

fn main() -> eframe::Result {
    let mut launch = Launch::default();
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--screenshot" => launch.screenshot = arguments.next().map(PathBuf::from),
            "--case" => launch.case = arguments.next().map(PathBuf::from),
            "--source" => launch.source = arguments.next().map(PathBuf::from),
            "--view" => launch.view = arguments.next(),
            "--exit-after-screenshot" => launch.exit_after_screenshot = true,
            "--demo" => (),
            "--help" | "-h" => {
                println!("ENGRAM native desktop\n\n  --demo  Open synthetic demonstration (default)\n  --case FILE  Open an existing case\n  --source DIRECTORY  Analyze an explicitly selected source\n  --view map|findings|timeline|connections|compare|modules\n  --screenshot FILE.png  Capture the actual native app\n  --exit-after-screenshot  Close after capture");
                return Ok(());
            }
            _ => (),
        }
    }
    let image = image::load_from_memory(include_bytes!("../../../assets/mascot.png"))
        .ok()
        .map(|image| image.resize(128, 128, image::imageops::FilterType::Lanczos3).into_rgba8());
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("ENGRAM — The memory desk")
        .with_inner_size([1440.0, 940.0])
        .with_min_inner_size([1060.0, 700.0]);
    if let Some(icon) = image {
        viewport = viewport.with_icon(eframe::egui::IconData {
            width: icon.width(),
            height: icon.height(),
            rgba: icon.into_raw(),
        });
    }
    eframe::run_native(
        "ENGRAM",
        eframe::NativeOptions { viewport, renderer: eframe::Renderer::Glow, ..Default::default() },
        Box::new(move |cc| Ok(Box::new(app::EngramApp::new(cc, launch)))),
    )
}
