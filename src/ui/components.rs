use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};

pub const ACCENT: Color32 = Color32::from_rgb(250, 59, 86);
#[derive(Clone, Copy)]
pub enum Icon {
    Play,
    Pause,
    Next,
    Previous,
    Search,
    Home,
    Albums,
    Artist,
    Music,
    Heart,
    Queue,
    Settings,
    Menu,
    Volume,
    Mini,
    Radio,
}

pub fn draw_icon(p: &egui::Painter, rect: Rect, icon: Icon, color: Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.30;
    let stroke = Stroke::new(1.6, color);
    let at = |x: f32, y: f32| Pos2::new(c.x + x * s, c.y + y * s);
    let line = |a: (f32, f32), b: (f32, f32)| {
        p.line_segment([at(a.0, a.1), at(b.0, b.1)], stroke);
    };
    match icon {
        Icon::Play => {
            p.add(egui::Shape::convex_polygon(
                vec![at(-0.6, -1.), at(-0.6, 1.), at(1., 0.)],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Pause => {
            for x in [-0.6, 0.6] {
                p.rect_filled(
                    Rect::from_center_size(at(x, 0.), Vec2::new(s * 0.4, s * 1.8)),
                    1,
                    color,
                );
            }
        }
        Icon::Next | Icon::Previous => {
            let d = if matches!(icon, Icon::Next) {
                1.0
            } else {
                -1.0
            };
            p.add(egui::Shape::convex_polygon(
                vec![at(-0.8 * d, -0.85), at(-0.8 * d, 0.85), at(0.6 * d, 0.)],
                color,
                Stroke::NONE,
            ));
            line((0.9 * d, -0.9), (0.9 * d, 0.9));
        }
        Icon::Search => {
            p.circle_stroke(at(-0.2, -0.2), s * 0.7, stroke);
            line((0.35, 0.35), (1., 1.));
        }
        Icon::Home => {
            line((-1., -0.1), (0., -1.));
            line((0., -1.), (1., -0.1));
            line((-0.7, -0.2), (-0.7, 1.));
            line((0.7, -0.2), (0.7, 1.));
            line((-0.7, 1.), (0.7, 1.));
        }
        Icon::Albums => {
            for x in [-0.55, 0.55] {
                for y in [-0.55, 0.55] {
                    p.rect_stroke(
                        Rect::from_center_size(at(x, y), Vec2::splat(s * 0.8)),
                        2,
                        stroke,
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }
        Icon::Artist => {
            p.circle_stroke(at(0., -0.5), s * 0.5, stroke);
            p.add(egui::Shape::line(
                vec![
                    at(-0.9, 1.),
                    at(-0.8, 0.5),
                    at(0., 0.2),
                    at(0.8, 0.5),
                    at(0.9, 1.),
                ],
                stroke,
            ));
        }
        Icon::Music => {
            line((-0.4, 0.7), (-0.4, -0.9));
            line((-0.4, -0.9), (0.8, -1.1));
            line((0.8, -1.1), (0.8, 0.5));
            p.circle_filled(at(-0.65, 0.7), s * 0.35, color);
            p.circle_filled(at(0.55, 0.5), s * 0.35, color);
        }
        Icon::Heart => {
            p.add(egui::Shape::closed_line(
                vec![
                    at(0., 1.),
                    at(-1., 0.),
                    at(-0.9, -0.7),
                    at(-0.4, -0.9),
                    at(0., -0.4),
                    at(0.4, -0.9),
                    at(0.9, -0.7),
                    at(1., 0.),
                ],
                stroke,
            ));
        }
        Icon::Queue => {
            for y in [-0.7, 0., 0.7] {
                line((-0.4, y), (1., y));
                p.circle_filled(at(-0.9, y), s * 0.12, color);
            }
        }
        Icon::Menu => {
            for y in [-0.65, 0., 0.65] {
                line((-0.9, y), (0.9, y));
            }
        }
        Icon::Settings => {
            p.circle_stroke(c, s * 0.75, stroke);
            p.circle_stroke(c, s * 0.27, stroke);
            for (a, b) in [
                ((-1.1, 0.), (-0.7, 0.)),
                ((0.7, 0.), (1.1, 0.)),
                ((0., -1.1), (0., -0.7)),
                ((0., 0.7), (0., 1.1)),
            ] {
                line(a, b);
            }
        }
        Icon::Volume => {
            p.add(egui::Shape::closed_line(
                vec![
                    at(-1., -0.4),
                    at(-0.5, -0.4),
                    at(0.1, -0.9),
                    at(0.1, 0.9),
                    at(-0.5, 0.4),
                    at(-1., 0.4),
                ],
                stroke,
            ));
            p.add(egui::Shape::line(
                vec![at(0.6, -0.8), at(0.95, 0.), at(0.6, 0.8)],
                stroke,
            ));
        }
        Icon::Mini => {
            p.rect_stroke(
                rect.shrink(rect.width() * 0.17),
                3,
                stroke,
                egui::StrokeKind::Inside,
            );
            p.rect_filled(Rect::from_min_max(at(0., 0.), at(0.7, 0.6)), 1, color);
        }
        Icon::Radio => {
            p.circle_filled(c, s * 0.18, color);
            for d in [-1., 1.] {
                p.add(egui::Shape::line(
                    vec![at(d * 0.5, -0.6), at(d * 0.8, 0.), at(d * 0.5, 0.6)],
                    stroke,
                ));
                p.add(egui::Shape::line(
                    vec![at(d * 0.9, -1.), at(d * 1.25, 0.), at(d * 0.9, 1.)],
                    stroke,
                ));
            }
        }
    }
}
pub fn icon_button(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    enabled: bool,
    selected: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(32.), Sense::click());
    let response = if enabled {
        response
    } else {
        response.interact(Sense::hover())
    };
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, label));
    if selected || response.hovered() || response.has_focus() {
        ui.painter().rect_filled(
            rect,
            5,
            if selected {
                ACCENT.gamma_multiply(0.18)
            } else {
                ui.visuals().widgets.hovered.bg_fill
            },
        );
    }
    let color = if !enabled {
        ui.visuals().weak_text_color()
    } else if selected {
        ACCENT
    } else {
        ui.visuals().text_color()
    };
    draw_icon(ui.painter(), rect.shrink(4.), icon, color);
    response.on_hover_text(label)
}
pub fn nav(ui: &mut egui::Ui, icon: Icon, label: &str, selected: bool, expanded: bool) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.), Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
    if selected || response.hovered() || response.has_focus() {
        ui.painter().rect_filled(
            rect,
            6,
            if selected {
                ACCENT.gamma_multiply(0.15)
            } else {
                ui.visuals().widgets.hovered.bg_fill
            },
        );
    }
    draw_icon(
        ui.painter(),
        Rect::from_min_size(rect.min + Vec2::new(5., 4.), Vec2::splat(28.)),
        icon,
        if selected {
            ACCENT
        } else {
            ui.visuals().weak_text_color()
        },
    );
    if expanded {
        ui.painter().text(
            rect.min + Vec2::new(42., 18.),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(14.),
            if selected {
                ACCENT
            } else {
                ui.visuals().text_color()
            },
        );
    }
    response.on_hover_text(label).clicked()
}
pub fn primary(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).color(Color32::WHITE))
            .fill(ACCENT)
            .min_size(Vec2::new(116., 34.)),
    )
}
pub fn time(seconds: f64) -> String {
    let seconds = if seconds.is_finite() {
        seconds.max(0.) as u64
    } else {
        0
    };
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
pub fn style(ctx: &egui::Context, light: bool) {
    let mut visuals = if light {
        egui::Visuals::light()
    } else {
        egui::Visuals::dark()
    };
    visuals.panel_fill = if light {
        Color32::from_rgb(253, 252, 252)
    } else {
        Color32::from_rgb(29, 28, 30)
    };
    visuals.window_fill = visuals.panel_fill;
    visuals.override_text_color = Some(if light {
        Color32::from_rgb(35, 33, 36)
    } else {
        Color32::from_rgb(242, 240, 242)
    });
    visuals.weak_text_color = Some(if light {
        Color32::from_rgb(99, 95, 102)
    } else {
        Color32::from_rgb(170, 166, 174)
    });
    visuals.selection.bg_fill = ACCENT.gamma_multiply(0.25);
    visuals.selection.stroke = Stroke::new(1., ACCENT);
    ctx.set_theme(if light {
        egui::Theme::Light
    } else {
        egui::Theme::Dark
    });
    ctx.set_visuals(visuals);
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = Vec2::new(8., 7.);
        s.spacing.button_padding = Vec2::new(10., 6.);
        s.text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(14.));
        s.text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(13.));
        s.text_styles
            .insert(egui::TextStyle::Heading, egui::FontId::proportional(30.));
    });
}
