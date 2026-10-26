mod capture;
mod components;
mod mini;
mod pages;
use cadence::platform::{MediaKey, NativeControls};
use cadence::{
    audio::{self, Audio, Command},
    model::{Library, Track},
    queue::Queue,
    services::{self, Artwork, Event, Services, Task},
};
use components::{Icon, icon_button};
use eframe::egui::{self, Color32, RichText, Vec2};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::sync::{
    Arc, Mutex,
    mpsc::{self, Receiver, Sender},
};
use std::time::{Duration, Instant};

#[derive(Clone, PartialEq, Eq)]
enum Page {
    Home,
    Search,
    New,
    Radio,
    Songs,
    Albums,
    Artists,
    Favorites,
    History,
    Playlists,
    Playlist(u64),
    Group(String, String),
    CatalogAlbum(String, String),
    Account,
    Settings,
}
pub struct App {
    capture: Option<capture::Capture>,
    mini_capture: Arc<Mutex<Option<capture::Capture>>>,
    mini_state: Arc<Mutex<mini::Snapshot>>,
    mini_tx: Sender<mini::Action>,
    mini_rx: Receiver<mini::Action>,
    media: Option<NativeControls>,
    media_status: String,
    country_draft: String,
    ended: bool,
    library: Library,
    services: Services,
    audio: Audio,
    art: Artwork,
    queue: Queue,
    page: Page,
    query: String,
    filter: String,
    results: Vec<Track>,
    request: u64,
    pending: bool,
    error: Option<String>,
    notice: String,
    expanded: bool,
    show_queue: bool,
    mini: bool,
    playing: bool,
    loading: bool,
    position: f64,
    duration: f64,
    output: String,
    dirty: bool,
    last_save: Instant,
    playlist_name: String,
    fade_requested: Option<u64>,
    seek_drag: Option<f64>,
}
impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, String> {
        let (services, library) = services::default_services()?;
        components::style(&cc.egui_ctx, library.settings.light);
        let media = cc
            .window_handle()
            .ok()
            .and_then(|h| match h.as_raw() {
                RawWindowHandle::Win32(h) => Some(h.hwnd.get()),
                _ => None,
            })
            .and_then(|hwnd| {
                let ctx = cc.egui_ctx.clone();
                NativeControls::new(hwnd, move || ctx.request_repaint()).ok()
            });
        let media_status = if media.is_some() {
            "Windows media keys connected"
        } else {
            "Windows media keys unavailable"
        }
        .into();
        let country_draft = library.settings.country.clone();
        let audio = Audio::start();
        let _ = audio.send(Command::Volume(library.settings.volume));
        let mut capture = capture::Capture::from_args();
        let mini = capture.as_ref().is_some_and(|capture| capture.mini);
        let mini_capture = Arc::new(Mutex::new(if mini { capture.take() } else { None }));
        let (mini_tx, mini_rx) = mpsc::channel();
        let mut app = Self {
            mini_capture,
            mini_state: Arc::new(Mutex::new(mini::Snapshot::default())),
            mini_tx,
            mini_rx,
            capture,
            media,
            media_status,
            country_draft,
            ended: false,
            library,
            services,
            audio,
            art: Artwork::start(),
            queue: Queue::default(),
            page: Page::Home,
            query: String::new(),
            filter: String::new(),
            results: vec![],
            request: 0,
            pending: false,
            error: None,
            notice: String::new(),
            expanded: true,
            show_queue: false,
            mini,
            playing: false,
            loading: false,
            position: 0.,
            duration: 0.,
            output: "Connecting audio…".into(),
            dirty: false,
            last_save: Instant::now(),
            playlist_name: String::new(),
            fade_requested: None,
            seek_drag: None,
        };
        let args: Vec<_> = std::env::args().collect();
        if let Some(page) = args
            .iter()
            .position(|a| a == "--page")
            .and_then(|i| args.get(i + 1))
        {
            app.page = match page.as_str() {
                "search" => Page::Search,
                "songs" => Page::Songs,
                "albums" => Page::Albums,
                "settings" => Page::Settings,
                "account" => Page::Account,
                _ => Page::Home,
            };
        }
        if args.iter().any(|a| a == "--light") {
            app.library.settings.light = true;
            components::style(&cc.egui_ctx, true);
        }
        if let Some(query) = args
            .iter()
            .position(|a| a == "--search")
            .and_then(|i| args.get(i + 1))
        {
            app.query = query.clone();
            app.search();
        }
        if let Some(scale) = args
            .iter()
            .position(|a| a == "--scale")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse::<f32>().ok())
            .filter(|s| s.is_finite() && *s >= 0.75 && *s <= 3.0)
        {
            cc.egui_ctx.set_zoom_factor(scale);
        }
        Ok(app)
    }
    fn submit(&mut self, task: Task) {
        if let Err(e) = self.services.submit(task) {
            self.error = Some(e);
            self.pending = false;
        }
    }
    fn persist(&mut self) {
        self.dirty = true;
    }
    fn flush(&mut self) {
        match self
            .services
            .submit(Task::Save(Box::new(self.library.clone())))
        {
            Ok(()) => {
                self.dirty = false;
                self.last_save = Instant::now();
            }
            Err(e) => self.error = Some(e),
        }
    }
    fn navigate(&mut self, page: Page) {
        if self.page != page {
            self.request = self.services.next_generation();
            self.pending = false;
            self.filter.clear();
            if matches!(page, Page::New | Page::Search) {
                self.results.clear();
            }
        }
        self.page = page;
    }
    fn search(&mut self) {
        if self.query.trim().is_empty() {
            return;
        }
        self.page = Page::Search;
        self.filter.clear();
        self.error = None;
        self.request = self.services.next_generation();
        self.pending = true;
        self.results.clear();
        self.submit(Task::Search {
            generation: self.request,
            query: self.query.trim().into(),
            country: self.library.settings.country.clone(),
        });
    }
    fn open_files(&mut self) {
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter(
                "Audio",
                &["mp3", "m4a", "aac", "flac", "wav", "ogg", "aiff", "alac"],
            )
            .pick_files()
        {
            self.notice = "Reading audio metadata…".into();
            self.submit(Task::Import(paths));
        }
    }
    fn play(&mut self, tracks: Vec<Track>, index: usize) {
        if let Some(track) = self.queue.replace(tracks, index) {
            self.load(track, 0.);
        }
    }
    fn load(&mut self, track: Track, fade: f32) {
        self.ended = false;
        self.loading = true;
        self.playing = false;
        self.position = 0.;
        self.duration = track.duration.unwrap_or(0.);
        self.fade_requested = None;
        self.error = None;
        self.seek_drag = None;
        if let Err(e) = self.audio.send(Command::Play {
            generation: self.queue.generation,
            source: track.source,
            fade,
        }) {
            self.error = Some(e);
            self.loading = false;
        }
    }
    fn next(&mut self, manual: bool, fade: f32) {
        if let Some(track) = self.queue.advance(self.queue.generation, manual) {
            self.load(track, fade);
        } else if !manual {
            self.ended = true;
            self.playing = false;
        }
    }
    fn toggle_play(&mut self) {
        if self.ended {
            if let Some(track) = self.queue.active().cloned() {
                self.load(track, 0.);
            }
        } else if self.queue.active().is_some() {
            self.send(Command::Toggle);
        }
    }
    fn previous(&mut self) {
        if self.position > 3. {
            self.send(Command::Seek(0.));
        } else if let Some(track) = self.queue.previous() {
            self.load(track, 0.);
        }
    }
    fn send(&mut self, command: Command) {
        if let Err(e) = self.audio.send(command) {
            self.error = Some(e);
        }
    }
    fn poll(&mut self, ctx: &egui::Context) {
        self.art.poll(ctx);
        let mut keys = Vec::new();
        if let Some(media) = &mut self.media {
            while let Ok(key) = media.events.try_recv() {
                keys.push(key);
            }
        }
        for key in keys {
            match key {
                MediaKey::Play if !self.playing => self.toggle_play(),
                MediaKey::Pause if self.playing => self.send(Command::Toggle),
                MediaKey::Next => self.next(true, 0.),
                MediaKey::Previous => self.previous(),
                MediaKey::Stop => {
                    self.send(Command::Stop);
                    self.playing = false;
                    self.ended = true;
                }
                _ => {}
            }
        }
        while let Ok(event) = self.services.events.try_recv() {
            match event {
                Event::Results(generation, result) if generation == self.request => {
                    self.pending = false;
                    match result {
                        Ok(tracks) => self.results = tracks,
                        Err(e) => self.error = Some(e),
                    }
                }
                Event::Imported(tracks, errors) => {
                    let mut count = 0;
                    for track in tracks {
                        if !self.library.local.iter().any(|t| t.id == track.id) {
                            self.library.local.push(track);
                            count += 1;
                        }
                    }
                    self.notice = format!("Imported {count} audio file(s).");
                    if !errors.is_empty() {
                        self.error = Some(errors.join("\n"));
                    }
                    self.persist();
                }
                Event::SaveFailed(e) => {
                    self.error = Some(format!("Library was not saved: {e}"));
                    self.dirty = true;
                }
                _ => {}
            }
        }
        while let Ok(event) = self.audio.events.try_recv() {
            match event {
                audio::Event::Ready {
                    sample_rate,
                    channels,
                } => self.output = format!("WASAPI · {sample_rate} Hz · {channels} channels"),
                audio::Event::Started {
                    generation,
                    duration,
                } if generation == self.queue.generation => {
                    self.loading = false;
                    self.duration = duration.unwrap_or(self.duration);
                    if let Some(track) = self.queue.active().cloned() {
                        self.library.remember(track);
                        self.persist();
                    }
                }
                audio::Event::State {
                    generation,
                    position,
                    playing,
                } if generation == self.queue.generation => {
                    self.position = position;
                    self.playing = playing;
                }
                audio::Event::Ended(generation) if generation == self.queue.generation => {
                    self.next(false, 0.)
                }
                audio::Event::Failed {
                    generation,
                    message,
                } if generation == 0 || generation == self.queue.generation => {
                    self.error = Some(message);
                    self.ended = true;
                    self.loading = false;
                    self.playing = false;
                    if generation == 0 {
                        self.output = "Audio disconnected".into();
                    }
                }
                _ => {}
            }
        }
        if let Some(media) = &mut self.media
            && media.update(self.queue.active(), self.playing).is_err()
        {
            self.media_status = "Windows media controls could not update".into();
        }
        let fade = self.library.settings.crossfade_seconds;
        if self.playing
            && !self.loading
            && fade > 0.
            && self.duration > f64::from(fade) * 2.
            && self.duration - self.position <= f64::from(fade)
            && self.fade_requested != Some(self.queue.generation)
        {
            self.fade_requested = Some(self.queue.generation);
            if let Some(track) = self.queue.advance(self.queue.generation, false) {
                self.load(track, fade);
            }
        }
        if self.dirty && self.last_save.elapsed() > Duration::from_millis(500) {
            self.flush();
        }
    }
    fn cover(&mut self, ui: &mut egui::Ui, track: Option<&Track>, size: f32) {
        if let Some(texture) = self.art.get(track.and_then(|t| t.artwork.as_deref())) {
            ui.add(
                egui::Image::new(&texture)
                    .fit_to_exact_size(Vec2::splat(size))
                    .corner_radius(5),
            );
        } else {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
            ui.painter().rect_filled(
                rect,
                5,
                if self.library.settings.light {
                    Color32::from_rgb(231, 228, 231)
                } else {
                    Color32::from_rgb(49, 46, 51)
                },
            );
            components::draw_icon(
                ui.painter(),
                rect.shrink(size * 0.20),
                Icon::Music,
                ui.visuals().weak_text_color(),
            );
        }
    }
    fn transport(&mut self, ui: &mut egui::Ui, compact: bool) {
        let active = self.queue.active().cloned();
        let enabled = active.is_some() && !self.loading;
        let width = ui.available_width();
        if compact || width < 600. {
            self.compact_transport(ui);
            return;
        }
        let center_width = (width - if compact { 124. } else { 314. }).max(140.);
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(if compact { 108. } else { 124. }, 64.),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    if icon_button(ui, Icon::Previous, "Previous track", enabled, false).clicked() {
                        self.previous();
                    }
                    if icon_button(
                        ui,
                        if self.playing {
                            Icon::Pause
                        } else {
                            Icon::Play
                        },
                        if self.playing { "Pause" } else { "Play" },
                        enabled,
                        false,
                    )
                    .clicked()
                    {
                        self.toggle_play();
                    }
                    if icon_button(ui, Icon::Next, "Next track", enabled, false).clicked() {
                        self.next(true, 0.);
                    }
                },
            );
            ui.allocate_ui_with_layout(
                Vec2::new(center_width, 64.),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(center_width);
                    ui.horizontal(|ui| {
                        self.cover(ui, active.as_ref(), 36.);
                        ui.vertical(|ui| {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(
                                        active
                                            .as_ref()
                                            .map(|t| t.title.as_str())
                                            .unwrap_or("Nothing playing"),
                                    )
                                    .strong(),
                                )
                                .truncate(),
                            );
                            ui.add(
                                egui::Label::new(if self.loading {
                                    "Loading audio…".into()
                                } else {
                                    active
                                        .as_ref()
                                        .map(|t| format!("{} · {}", t.artist, t.kind()))
                                        .unwrap_or_else(|| "Choose something to listen to".into())
                                })
                                .truncate(),
                            );
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.spacing_mut().slider_width = (center_width - 84.).max(70.);
                        ui.small(components::time(self.position));
                        let mut position = self.seek_drag.unwrap_or(self.position);
                        let response = ui.add_enabled(
                            enabled && self.duration > 0.,
                            egui::Slider::new(&mut position, 0.0..=self.duration.max(1.))
                                .show_value(false)
                                .trailing_fill(true),
                        );
                        if response.dragged() {
                            self.seek_drag = Some(position);
                        }
                        if response.drag_stopped() || (response.changed() && !response.dragged()) {
                            self.send(Command::Seek(position));
                            self.seek_drag = None;
                        }
                        ui.small(if self.duration > 0. {
                            components::time(self.duration)
                        } else {
                            "—".into()
                        });
                    });
                },
            );
            if !compact {
                ui.allocate_ui_with_layout(
                    Vec2::new(170., 64.),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        ui.set_width(170.);
                        ui.horizontal(|ui| {
                            if icon_button(ui, Icon::Queue, "Show queue", true, self.show_queue)
                                .clicked()
                            {
                                self.show_queue = !self.show_queue;
                            }
                            if icon_button(ui, Icon::Mini, "Open MiniPlayer", true, self.mini)
                                .clicked()
                            {
                                self.mini = !self.mini;
                            }
                            if icon_button(
                                ui,
                                Icon::Heart,
                                "Favorite current track",
                                active.is_some(),
                                active.as_ref().is_some_and(|t| {
                                    self.library.favorites.iter().any(|s| s.id == t.id)
                                }),
                            )
                            .clicked()
                                && let Some(track) = active.clone()
                            {
                                self.library.toggle_favorite(track);
                                self.persist();
                            }
                        });
                        ui.horizontal(|ui| {
                            components::draw_icon(
                                ui.painter(),
                                egui::Rect::from_min_size(ui.cursor().min, Vec2::splat(18.)),
                                Icon::Volume,
                                ui.visuals().weak_text_color(),
                            );
                            ui.add_space(20.);
                            if ui
                                .add(
                                    egui::Slider::new(&mut self.library.settings.volume, 0.0..=1.0)
                                        .show_value(false),
                                )
                                .changed()
                            {
                                self.send(Command::Volume(self.library.settings.volume));
                                self.persist();
                            }
                        });
                    },
                );
            }
        });
    }
    fn compact_transport(&mut self, ui: &mut egui::Ui) {
        let active = self.queue.active().cloned();
        let enabled = active.is_some() && !self.loading;
        let width = ui.available_width();
        ui.horizontal(|ui| {
            if icon_button(ui, Icon::Previous, "Previous track", enabled, false).clicked() {
                self.previous();
            }
            if icon_button(
                ui,
                if self.playing {
                    Icon::Pause
                } else {
                    Icon::Play
                },
                if self.playing { "Pause" } else { "Play" },
                enabled,
                false,
            )
            .clicked()
            {
                self.toggle_play();
            }
            if icon_button(ui, Icon::Next, "Next track", enabled, false).clicked() {
                self.next(true, 0.);
            }
            ui.add_space((width - 224.).max(0.));
            if icon_button(ui, Icon::Queue, "Show queue", true, self.show_queue).clicked() {
                self.show_queue = !self.show_queue;
            }
            if icon_button(ui, Icon::Mini, "Toggle MiniPlayer", true, self.mini).clicked() {
                self.mini = !self.mini;
            }
        });
        ui.horizontal(|ui| {
            ui.add_sized(
                [(width - 146.).max(80.), 20.],
                egui::Label::new(
                    active
                        .as_ref()
                        .map(|t| t.title.as_str())
                        .unwrap_or("Nothing playing"),
                )
                .truncate(),
            );
            ui.spacing_mut().slider_width = 100.;
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(18.), egui::Sense::hover());
            components::draw_icon(ui.painter(), rect, Icon::Volume, ui.visuals().text_color());
            if ui
                .add(
                    egui::Slider::new(&mut self.library.settings.volume, 0.0..=1.0)
                        .show_value(false),
                )
                .changed()
            {
                self.send(Command::Volume(self.library.settings.volume));
                self.persist();
            }
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().slider_width = (width - 82.).max(80.);
            ui.small(components::time(self.position));
            let mut position = self.seek_drag.unwrap_or(self.position);
            let response = ui.add_enabled(
                enabled && self.duration > 0.,
                egui::Slider::new(&mut position, 0.0..=self.duration.max(1.)).show_value(false),
            );
            if response.dragged() {
                self.seek_drag = Some(position);
            }
            if response.drag_stopped() || (response.changed() && !response.dragged()) {
                self.send(Command::Seek(position));
                self.seek_drag = None;
            }
            ui.small(if self.duration > 0. {
                components::time(self.duration)
            } else {
                "—".into()
            });
        });
    }
    fn sidebar(&mut self, ui: &mut egui::Ui, expanded: bool) {
        ui.horizontal(|ui| {
            if icon_button(ui, Icon::Menu, "Toggle sidebar", true, false).clicked() {
                self.expanded = !self.expanded;
            }
            if expanded {
                ui.label(RichText::new("Cadence").size(21.).strong());
            }
        });
        ui.add_space(12.);
        if expanded {
            let response = ui.add_sized(
                [ui.available_width(), 30.],
                egui::TextEdit::singleline(&mut self.query).hint_text("Search the catalog"),
            );
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.search();
            }
        }
        ui.add_space(12.);
        for (page, icon, name) in [
            (Page::Home, Icon::Home, "Home"),
            (Page::Search, Icon::Search, "Search"),
            (Page::New, Icon::Albums, "New & charts"),
            (Page::Radio, Icon::Radio, "Radio"),
        ] {
            if components::nav(ui, icon, name, self.page == page, expanded) {
                self.navigate(page);
            }
        }
        ui.add_space(20.);
        if expanded {
            ui.weak("Library");
        }
        for (page, icon, name) in [
            (Page::History, Icon::Queue, "Recently played"),
            (Page::Artists, Icon::Artist, "Artists"),
            (Page::Albums, Icon::Albums, "Albums"),
            (Page::Songs, Icon::Music, "Songs"),
            (Page::Favorites, Icon::Heart, "Favorites"),
            (Page::Playlists, Icon::Queue, "Playlists"),
        ] {
            if components::nav(ui, icon, name, self.page == page, expanded) {
                self.navigate(page);
            }
        }
        ui.add_space(14.);
        if expanded
            && ui
                .add_sized(
                    [ui.available_width(), 32.],
                    egui::Button::new("Import audio…"),
                )
                .clicked()
        {
            self.open_files();
        }
        ui.add_space(16.);
        if components::nav(
            ui,
            Icon::Settings,
            "Settings",
            self.page == Page::Settings,
            expanded,
        ) {
            self.navigate(Page::Settings);
        }
        if expanded && ui.button("Apple account").clicked() {
            self.navigate(Page::Account);
        }
    }
    fn queue_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Up Next");
            if ui.button("Close").clicked() {
                self.show_queue = false;
            }
        });
        ui.add_space(10.);
        ui.horizontal_wrapped(|ui| {
            if ui.selectable_label(self.queue.shuffle, "Shuffle").clicked() {
                self.queue.set_shuffle(!self.queue.shuffle);
            }
            if ui.button(self.queue.repeat.label()).clicked() {
                self.queue.repeat = self.queue.repeat.cycle();
            }
        });
        ui.separator();
        if self.queue.items.is_empty() {
            ui.label("Play a song or add tracks to the queue.");
        }
        let tracks = self.queue.items.clone();
        let mut play = None;
        let mut remove = None;
        egui::ScrollArea::vertical()
            .id_salt("queue-scroll")
            .show(ui, |ui| {
                for (i, track) in tracks.iter().enumerate() {
                    ui.push_id(i, |ui| {
                        ui.add(
                            egui::Label::new(RichText::new(&track.title).color(
                                if Some(i) == self.queue.current {
                                    components::ACCENT
                                } else {
                                    ui.visuals().text_color()
                                },
                            ))
                            .truncate(),
                        );
                        ui.add(egui::Label::new(&track.artist).truncate());
                        ui.horizontal(|ui| {
                            if ui.small_button("Play").clicked() {
                                play = Some(i);
                            }
                            if ui
                                .add_enabled(
                                    Some(i) != self.queue.current,
                                    egui::Button::new("Remove").small(),
                                )
                                .clicked()
                            {
                                remove = Some(i);
                            }
                        });
                        ui.add_space(14.);
                    });
                }
            });
        if let Some(i) = play
            && let Some(track) = self.queue.select(i)
        {
            self.load(track, 0.);
        }
        if let Some(i) = remove {
            self.queue.remove(i);
        }
    }
}
impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll(ctx);
        while let Ok(action) = self.mini_rx.try_recv() {
            match action {
                mini::Action::Toggle => self.toggle_play(),
                mini::Action::Previous => self.previous(),
                mini::Action::Next => self.next(true, 0.),
                mini::Action::Seek(position) => self.send(Command::Seek(position)),
                mini::Action::Volume(volume) => {
                    self.library.settings.volume = volume;
                    self.send(Command::Volume(volume));
                    self.persist();
                }
                mini::Action::Close => self.mini = false,
                mini::Action::Queue => {
                    self.show_queue = true;
                    ctx.send_viewport_cmd_to(
                        egui::ViewportId::ROOT,
                        egui::ViewportCommand::Minimized(false),
                    );
                    ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
                }
            }
        }
        let track = self.queue.active();
        *self.mini_state.lock().unwrap_or_else(|e| e.into_inner()) = mini::Snapshot {
            title: track.map(|t| t.title.clone()).unwrap_or_default(),
            artist: track
                .map(|t| format!("{} · {}", t.artist, t.kind()))
                .unwrap_or_default(),
            position: self.position,
            duration: self.duration,
            playing: self.playing,
            loading: self.loading,
            has_track: track.is_some(),
            volume: self.library.settings.volume,
        };
        ctx.request_repaint_after(Duration::from_millis(
            if self.playing || self.loading || self.pending {
                100
            } else {
                500
            },
        ));
    }

    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();

        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::O)) {
            self.open_files();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::F)) {
            self.navigate(Page::Search);
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Comma)) {
            self.navigate(Page::Settings);
        }
        if !ctx.egui_wants_keyboard_input() {
            if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
                self.toggle_play();
            }
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::ArrowRight)) {
                self.next(true, 0.);
            }
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::ArrowLeft)) {
                self.previous();
            }
        }
        let expanded =
            self.expanded && root.available_width() > if self.show_queue { 1120. } else { 950. };
        egui::Panel::left("navigation")
            .exact_size(if expanded { 220. } else { 60. })
            .frame(
                egui::Frame::new()
                    .fill(if self.library.settings.light {
                        Color32::from_rgb(242, 240, 242)
                    } else {
                        Color32::from_rgb(24, 23, 26)
                    })
                    .inner_margin(12),
            )
            .show(root, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("sidebar-scroll")
                    .show(ui, |ui| self.sidebar(ui, expanded));
            });
        egui::Panel::top("transport")
            .exact_size(if root.available_width() < 624. {
                116.
            } else {
                94.
            })
            .frame(
                egui::Frame::new()
                    .inner_margin(12)
                    .fill(if self.library.settings.light {
                        Color32::from_rgb(247, 245, 247)
                    } else {
                        Color32::from_rgb(35, 33, 36)
                    }),
            )
            .show(root, |ui| self.transport(ui, false));
        let narrow_queue = self.show_queue && root.available_width() < 620.;
        if self.show_queue && !narrow_queue {
            egui::Panel::right("queue")
                .default_size(270.)
                .size_range(220.0..=350.)
                .frame(egui::Frame::new().inner_margin(18))
                .show(root, |ui| self.queue_panel(ui));
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(ctx.global_style().visuals.panel_fill)
                    .inner_margin(if root.available_width() < 600. {
                        16
                    } else {
                        28
                    }),
            )
            .show(root, |ui| {
                if let Some(error) = self.error.clone() {
                    egui::Frame::new()
                        .fill(components::ACCENT.gamma_multiply(0.10))
                        .inner_margin(12)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(error).color(
                                    if self.library.settings.light {
                                        Color32::from_rgb(175, 24, 51)
                                    } else {
                                        Color32::from_rgb(255, 153, 167)
                                    },
                                ));
                                if ui.small_button("Dismiss").clicked() {
                                    self.error = None;
                                }
                            });
                        });
                    ui.add_space(12.);
                }
                if !self.notice.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.weak(&self.notice);
                        if ui.small_button("Dismiss").clicked() {
                            self.notice.clear();
                        }
                    });
                    ui.add_space(8.);
                }
                if narrow_queue {
                    self.queue_panel(ui);
                } else {
                    self.page(ui);
                }
            });
        if let Some(capture) = &mut self.capture
            && !capture.mini
        {
            capture.update(&ctx, self.pending);
        }
        if self.mini {
            mini::show(
                &ctx,
                self.mini_state.clone(),
                self.mini_tx.clone(),
                self.mini_capture.clone(),
            );
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Err(error) = self.services.save_final(&self.library) {
            let _ = rfd::MessageDialog::new()
                .set_title("Library could not be saved")
                .set_description(format!(
                    "The latest library changes were not saved: {error}"
                ))
                .set_level(rfd::MessageLevel::Error)
                .show();
        }
    }
}
