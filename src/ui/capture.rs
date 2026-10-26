use eframe::egui;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

pub struct Capture {
    pub mini: bool,
    path: PathBuf,
    start: Instant,
    requested: bool,
    ready_since: Option<Instant>,
}
impl Capture {
    pub fn from_args() -> Option<Self> {
        let args: Vec<_> = std::env::args_os().collect();
        let path = args
            .iter()
            .position(|a| a == "--capture" || a == "--capture-mini")
            .and_then(|i| args.get(i + 1))?;
        Some(Self {
            mini: args.iter().any(|a| a == "--capture-mini"),
            path: PathBuf::from(path),
            start: Instant::now(),
            requested: false,
            ready_since: None,
        })
    }
    pub fn update(&mut self, ctx: &egui::Context, pending: bool) {
        if pending {
            self.ready_since = None;
        } else if self.ready_since.is_none() {
            self.ready_since = Some(Instant::now());
        }
        if !self.requested
            && self
                .ready_since
                .is_some_and(|start| start.elapsed() > Duration::from_secs(4))
        {
            self.requested = true;
            let path = self.path.clone();
            let close = ctx.clone();
            ctx.request_screenshot(move |image| {
                let bytes: Vec<_> = image
                    .pixels
                    .iter()
                    .flat_map(|pixel| pixel.to_array())
                    .collect();
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Err(error) = image::save_buffer(
                    &path,
                    &bytes,
                    image.width() as u32,
                    image.height() as u32,
                    image::ColorType::Rgba8,
                ) {
                    eprintln!("Capture failed: {error}");
                }
                close.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
            });
        }
        if self.start.elapsed() > Duration::from_secs(45) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
