#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod diagnostics;
mod ui;
fn main() {
    if let Some(result) = diagnostics::run() {
        match result {
            Ok(report) => {
                println!("{report}");
                return;
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
    }
    let args: Vec<String> = std::env::args().collect();
    let size = |key: &str, default: f32| {
        args.iter()
            .position(|a| a == key)
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse::<f32>().ok())
            .filter(|v| v.is_finite())
            .unwrap_or(default)
    };
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([
                size("--width", 1240.).max(860.),
                size("--height", 820.).max(600.),
            ])
            .with_min_inner_size([860., 600.])
            .with_app_id("in.harshil.cadence"),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    if let Err(error) = eframe::run_native(
        "Cadence",
        options,
        Box::new(|cc| Ok(Box::new(ui::App::new(cc)?))),
    ) {
        let _ = rfd::MessageDialog::new()
            .set_title("Cadence could not start")
            .set_description(error.to_string())
            .set_level(rfd::MessageLevel::Error)
            .show();
    }
}
