use super::{
    capture::Capture,
    components::{self, Icon, icon_button},
};
use eframe::egui::{self, RichText, Vec2};
use std::{
    sync::{Arc, Mutex, mpsc::Sender},
    time::Duration,
};

#[derive(Clone, Default)]
pub struct Snapshot {
    pub title: String,
    pub artist: String,
    pub position: f64,
    pub duration: f64,
    pub playing: bool,
    pub loading: bool,
    pub has_track: bool,
    pub volume: f32,
}
pub enum Action {
    Toggle,
    Previous,
    Next,
    Seek(f64),
    Volume(f32),
    Close,
    Queue,
}

pub fn show(
    ctx: &egui::Context,
    state: Arc<Mutex<Snapshot>>,
    sender: Sender<Action>,
    capture: Arc<Mutex<Option<Capture>>>,
) {
    ctx.show_viewport_deferred(
        egui::ViewportId::from_hash_of("cadence-mini"),
        egui::ViewportBuilder::default()
            .with_title("Cadence MiniPlayer")
            .with_inner_size([470., 200.])
            .with_min_inner_size([390., 180.]),
        move |ui, _| {
            let snapshot = state.lock().unwrap_or_else(|e| e.into_inner()).clone();
            let repaint = ui.ctx().clone();
            let send = |action| {
                let _ = sender.send(action);
                repaint.request_repaint_of(egui::ViewportId::ROOT);
            };
            if ui.ctx().input(|i| i.viewport().close_requested()) {
                send(Action::Close);
            }
            ui.ctx()
                .request_repaint_after(Duration::from_millis(if snapshot.playing {
                    100
                } else {
                    500
                }));
            if let Some(capture) = capture.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
                capture.update(ui.ctx(), false);
            }
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(ui.visuals().panel_fill)
                        .inner_margin(18),
                )
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(if snapshot.has_track {
                                &snapshot.title
                            } else {
                                "Nothing playing"
                            })
                            .strong()
                            .size(17.),
                        )
                        .truncate(),
                    );
                    ui.add(
                        egui::Label::new(if snapshot.loading {
                            "Loading audio…"
                        } else if snapshot.has_track {
                            &snapshot.artist
                        } else {
                            "Choose a track in the main window"
                        })
                        .truncate(),
                    );
                    ui.add_space(8.);
                    ui.horizontal(|ui| {
                        let enabled = snapshot.has_track && !snapshot.loading;
                        if icon_button(ui, Icon::Previous, "Previous track", enabled, false)
                            .clicked()
                        {
                            send(Action::Previous);
                        }
                        if icon_button(
                            ui,
                            if snapshot.playing {
                                Icon::Pause
                            } else {
                                Icon::Play
                            },
                            if snapshot.playing { "Pause" } else { "Play" },
                            enabled,
                            false,
                        )
                        .clicked()
                        {
                            send(Action::Toggle);
                        }
                        if icon_button(ui, Icon::Next, "Next track", enabled, false).clicked() {
                            send(Action::Next);
                        }
                        if icon_button(ui, Icon::Queue, "Show queue in main window", true, false)
                            .clicked()
                        {
                            send(Action::Queue);
                        }
                        ui.spacing_mut().slider_width = 100.;
                        let mut volume = snapshot.volume;
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::splat(18.), egui::Sense::hover());
                        components::draw_icon(
                            ui.painter(),
                            rect,
                            Icon::Volume,
                            ui.visuals().text_color(),
                        );
                        if ui
                            .add(egui::Slider::new(&mut volume, 0.0..=1.0).show_value(false))
                            .changed()
                        {
                            send(Action::Volume(volume));
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.small(components::time(snapshot.position));
                        ui.spacing_mut().slider_width = (ui.available_width() - 68.).max(80.);
                        let seek_id = ui.make_persistent_id("mini-seek");
                        let mut position = ui
                            .data(|data| data.get_temp::<f64>(seek_id))
                            .unwrap_or(snapshot.position);
                        let response = ui.add_enabled(
                            snapshot.has_track && snapshot.duration > 0.,
                            egui::Slider::new(&mut position, 0.0..=snapshot.duration.max(1.))
                                .show_value(false),
                        );
                        if response.dragged() {
                            ui.data_mut(|data| data.insert_temp(seek_id, position));
                        }
                        if response.drag_stopped() || (response.changed() && !response.dragged()) {
                            send(Action::Seek(position));
                            ui.data_mut(|data| data.remove::<f64>(seek_id));
                        }
                        ui.small(if snapshot.duration > 0. {
                            components::time(snapshot.duration)
                        } else {
                            "—".into()
                        });
                    });
                });
        },
    );
}
