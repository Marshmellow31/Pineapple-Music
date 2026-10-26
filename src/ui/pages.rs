use super::{
    App, Page,
    components::{self, ACCENT, Icon, icon_button},
};
use cadence::{
    audio::{Audio, Command},
    model::{APPLE_ACCOUNT_STATUS, Playlist, Track},
    services::Task,
};
use eframe::egui::{self, RichText, Vec2};
use std::collections::BTreeMap;

enum Action {
    Play(usize),
    Favorite(Track),
    Queue(Track),
    Playlist(u64, Track),
    Album(Track),
    Artist(String),
    RemoveLocal(String),
}
impl App {
    pub(super) fn page(&mut self, ui: &mut egui::Ui) {
        match self.page.clone() {
            Page::Home => self.home(ui),
            Page::Search | Page::New => self.catalog_page(ui),
            Page::Songs => {
                self.heading(ui, "Songs", "Audio imported from your computer");
                self.track_list(ui, self.library.local.clone(), true);
            }
            Page::Favorites => {
                self.heading(
                    ui,
                    "Favorites",
                    "Your saved local files and catalog previews",
                );
                self.track_list(ui, self.library.favorites.clone(), false);
            }
            Page::History => {
                self.heading(ui, "Recently played", "Listening history on this computer");
                self.track_list(ui, self.library.history.clone(), false);
            }
            Page::Albums => self.groups(ui, false),
            Page::Artists => self.groups(ui, true),
            Page::Group(album, artist) => {
                self.heading(
                    ui,
                    if album.is_empty() { &artist } else { &album },
                    if album.is_empty() {
                        "Local artist"
                    } else {
                        &artist
                    },
                );
                let tracks = self
                    .library
                    .local
                    .iter()
                    .filter(|t| t.artist == artist && (album.is_empty() || t.album == album))
                    .cloned()
                    .collect();
                if ui.button("Back to library").clicked() {
                    self.navigate(Page::Albums);
                }
                ui.add_space(12.);
                self.track_list(ui, tracks, false);
            }
            Page::CatalogAlbum(title, artist) => {
                self.heading(ui, &title, &format!("{artist} · Catalog previews"));
                if self.pending {
                    self.loading_rows(ui);
                } else {
                    self.track_list(ui, self.results.clone(), false);
                }
            }
            Page::Playlists => self.playlists(ui),
            Page::Playlist(id) => self.playlist(ui, id),
            Page::Settings => self.settings(ui),
            Page::Account => self.account(ui),
            Page::Radio => {
                self.heading(ui, "Radio", "Apple Music");
                ui.add_space(28.);
                ui.label("Radio playback needs an Apple Music subscription playback provider.");
                ui.weak("Native Apple authorization has not been established for this build.");
                ui.add_space(16.);
                if ui.button("View account status").clicked() {
                    self.navigate(Page::Account);
                }
            }
        }
    }
    fn heading(&self, ui: &mut egui::Ui, title: &str, subtitle: &str) {
        ui.heading(title);
        ui.add_space(3.);
        ui.weak(subtitle);
        ui.add_space(22.);
    }
    fn home(&mut self, ui: &mut egui::Ui) {
        self.heading(ui, "Home", "Your music, in one place.");
        egui::ScrollArea::vertical().id_salt("home").show(ui,|ui|{
            if self.library.local.is_empty() && self.library.history.is_empty() {
                if ui.available_width()<600. || ui.available_height()<320. {
                    ui.horizontal_wrapped(|ui|{
                        if components::primary(ui,"Import audio…").clicked(){self.open_files();}
                        if ui.button("Search the catalog").clicked(){self.navigate(Page::Search);}
                    });
                    ui.add_space(12.);
                    ui.label("Bring in your music, or discover a song in the catalog.");
                    ui.weak("Local files play in full. Catalog tracks play as previews.");
                    return;
                }
                ui.add_space(34.);
                ui.label(RichText::new("Start with something you love.").size(25.).strong());
                ui.add_space(10.);
                ui.label("Bring in your music, or discover a song in the catalog.");
                ui.weak("Local files play in full. Catalog tracks play as previews.");
                ui.add_space(22.);
                ui.horizontal(|ui|{
                    if components::primary(ui,"Import audio…").clicked(){self.open_files();}
                    if ui.button("Search the catalog").clicked(){self.navigate(Page::Search);}
                });
                ui.add_space(46.);ui.separator();ui.add_space(22.);
                ui.label(RichText::new("A library that stays yours").size(18.).strong());
                ui.add_space(6.);
                ui.label("Organize songs into playlists, save favorites, and pick up where you left off.");
                ui.weak("Your imported files remain in their original folders.");
            } else {
                if !self.library.history.is_empty() {
                    ui.label(RichText::new("Recently played").size(21.).strong());ui.add_space(12.);
                    let tracks:Vec<_>=self.library.history.iter().take(5).cloned().collect();
                    for track in tracks {
                        ui.horizontal(|ui|{
                            self.cover(ui,Some(&track),56.);
                            ui.vertical(|ui|{
                                if ui.add(egui::Button::new(RichText::new(&track.title).strong()).frame(false)).clicked(){self.play(vec![track.clone()],0);}
                                ui.label(format!("{} · {}",track.artist,track.kind()));
                            });
                        });ui.add_space(12.);
                    }
                    ui.add_space(20.);
                }
                ui.horizontal(|ui|{
                    ui.label(RichText::new("Your library").size(21.).strong());
                    if ui.button("View songs").clicked(){self.navigate(Page::Songs);}
                });ui.add_space(6.);
                ui.label(format!("{} songs · {} playlists · {} favorites",self.library.local.len(),self.library.playlists.len(),self.library.favorites.len()));
                ui.add_space(16.);if components::primary(ui,"Import audio…").clicked(){self.open_files();}
            }
        });
    }
    fn catalog_page(&mut self, ui: &mut egui::Ui) {
        if self.page == Page::New {
            self.heading(ui, "New & charts", "Apple Music catalog");
            if std::env::var("CADENCE_APPLE_DEVELOPER_TOKEN").is_err() {
                ui.label("Charts require your Apple developer token.");
                ui.weak("Catalog search and previews are available without configuring a developer token.");
                ui.add_space(16.);
                if ui.button("Search the catalog").clicked() {
                    self.navigate(Page::Search);
                }
                return;
            }
            if ui.button("Load top songs").clicked() {
                self.request = self.services.next_generation();
                self.pending = true;
                self.results.clear();
                self.submit(Task::Charts {
                    generation: self.request,
                    country: self.library.settings.country.clone(),
                });
            }
            ui.add_space(14.);
        } else {
            self.heading(ui, "Search", "Explore the catalog. Listen to previews.");
            let mut go = false;
            ui.horizontal(|ui| {
                let response = ui.add_sized(
                    [ui.available_width() - 132., 36.],
                    egui::TextEdit::singleline(&mut self.query)
                        .hint_text("Songs, artists, or albums"),
                );
                go = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if components::primary(ui, "Search").clicked() {
                    go = true;
                }
            });
            if go {
                self.search();
            }
            ui.add_space(18.);
        }
        if self.pending {
            self.loading_rows(ui);
        } else if !self.results.is_empty() {
            self.track_list(ui, self.results.clone(), false);
        } else {
            ui.add_space(22.);
            ui.label(if self.query.is_empty() {
                "Find your next favorite."
            } else {
                "No catalog results loaded."
            });
            ui.weak("Search by song, artist, or album name. Press Enter to search.");
        }
    }
    fn loading_rows(&self, ui: &mut egui::Ui) {
        ui.weak("Loading from the catalog…");
        ui.add_space(12.);
        for i in 0..5 {
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 48.), egui::Sense::hover());
            let color = ui.visuals().widgets.noninteractive.bg_fill;
            ui.painter().rect_filled(
                egui::Rect::from_min_size(rect.min, Vec2::splat(40.)),
                4,
                color,
            );
            ui.painter().rect_filled(
                egui::Rect::from_min_size(
                    rect.min + Vec2::new(56., 8.),
                    Vec2::new(150. + (i % 3) as f32 * 32., 9.),
                ),
                3,
                color,
            );
            ui.painter().rect_filled(
                egui::Rect::from_min_size(rect.min + Vec2::new(56., 25.), Vec2::new(96., 7.)),
                3,
                color,
            );
        }
    }
    fn track_list(&mut self, ui: &mut egui::Ui, tracks: Vec<Track>, local: bool) {
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!tracks.is_empty(), egui::Button::new("Play all"))
                .clicked()
            {
                self.play(tracks.clone(), 0);
            }
            if ui
                .add_enabled(!tracks.is_empty(), egui::Button::new("Add to queue"))
                .clicked()
            {
                for track in &tracks {
                    self.queue.enqueue(track.clone());
                }
                self.notice = format!("Added {} tracks to the queue.", tracks.len());
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_sized(
                    [180., 28.],
                    egui::TextEdit::singleline(&mut self.filter).hint_text("Filter this list"),
                );
            });
        });
        ui.add_space(12.);
        let needle = self.filter.to_lowercase();
        let filtered: Vec<_> = tracks
            .into_iter()
            .filter(|t| {
                needle.is_empty()
                    || format!("{} {} {}", t.title, t.artist, t.album)
                        .to_lowercase()
                        .contains(&needle)
            })
            .collect();
        ui.weak(format!("{} tracks", filtered.len()));
        ui.add_space(6.);
        ui.separator();
        if filtered.is_empty() {
            ui.add_space(30.);
            ui.label(if needle.is_empty() {
                "No songs here yet."
            } else {
                "No songs match this filter."
            });
            if local && components::primary(ui, "Import audio…").clicked() {
                self.open_files();
            }
            return;
        }
        let mut actions = Vec::new();
        egui::ScrollArea::vertical()
            .id_salt("track-list")
            .auto_shrink([false, false])
            .show_rows(ui, 62., filtered.len(), |ui, range| {
                for i in range {
                    let track = &filtered[i];
                    ui.push_id(&track.id, |ui| {
                        let active = self.queue.active().is_some_and(|t| t.id == track.id);
                        egui::Frame::new()
                            .fill(if active {
                                ACCENT.gamma_multiply(0.08)
                            } else {
                                egui::Color32::TRANSPARENT
                            })
                            .inner_margin(6)
                            .corner_radius(4)
                            .show(ui, |ui| {
                                let width = ui.available_width();
                                let album_width = if width > 680. { width * 0.24 } else { 0. };
                                ui.horizontal(|ui| {
                                    self.cover(ui, Some(track), 42.);
                                    ui.allocate_ui_with_layout(
                                        Vec2::new((width - album_width - 242.).max(100.), 48.),
                                        egui::Layout::top_down(egui::Align::Min),
                                        |ui| {
                                            ui.set_width((width - album_width - 242.).max(100.));
                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                        RichText::new(&track.title).strong().color(
                                                            if active {
                                                                ACCENT
                                                            } else {
                                                                ui.visuals().text_color()
                                                            },
                                                        ),
                                                    )
                                                    .frame(false)
                                                    .truncate(),
                                                )
                                                .clicked()
                                            {
                                                actions.push(Action::Play(i));
                                            }
                                            if ui
                                                .add(
                                                    egui::Button::new(&track.artist)
                                                        .frame(false)
                                                        .truncate(),
                                                )
                                                .on_hover_text("View artist")
                                                .clicked()
                                            {
                                                actions.push(Action::Artist(track.artist.clone()));
                                            }
                                        },
                                    );
                                    if album_width > 0. {
                                        ui.allocate_ui_with_layout(
                                            Vec2::new(album_width, 42.),
                                            egui::Layout::left_to_right(egui::Align::Center),
                                            |ui| {
                                                ui.set_width(album_width);
                                                if ui
                                                    .add(
                                                        egui::Button::new(&track.album)
                                                            .frame(false)
                                                            .truncate(),
                                                    )
                                                    .on_hover_text("View album")
                                                    .clicked()
                                                {
                                                    actions.push(Action::Album(track.clone()));
                                                }
                                            },
                                        );
                                    }
                                    ui.small(track.kind());
                                    if icon_button(
                                        ui,
                                        if active && self.playing {
                                            Icon::Pause
                                        } else {
                                            Icon::Play
                                        },
                                        "Play or pause track",
                                        true,
                                        false,
                                    )
                                    .clicked()
                                    {
                                        if active {
                                            self.toggle_play();
                                        } else {
                                            actions.push(Action::Play(i));
                                        }
                                    }
                                    ui.menu_button("More", |ui| {
                                        if ui.button("Add to queue").clicked() {
                                            actions.push(Action::Queue(track.clone()));
                                            ui.close();
                                        }
                                        let saved =
                                            self.library.favorites.iter().any(|t| t.id == track.id);
                                        if ui
                                            .button(if saved {
                                                "Remove from favorites"
                                            } else {
                                                "Add to favorites"
                                            })
                                            .clicked()
                                        {
                                            actions.push(Action::Favorite(track.clone()));
                                            ui.close();
                                        }
                                        ui.menu_button("Add to playlist", |ui| {
                                            if self.library.playlists.is_empty() {
                                                ui.weak("Create a playlist first.");
                                            }
                                            for playlist in &self.library.playlists {
                                                if ui.button(&playlist.name).clicked() {
                                                    actions.push(Action::Playlist(
                                                        playlist.id,
                                                        track.clone(),
                                                    ));
                                                    ui.close();
                                                }
                                            }
                                        });
                                        if local {
                                            ui.separator();
                                            if ui.button("Remove from library").clicked() {
                                                actions.push(Action::RemoveLocal(track.id.clone()));
                                                ui.close();
                                            }
                                        }
                                    });
                                });
                            });
                    });
                }
            });
        for action in actions {
            match action {
                Action::Play(i) => self.play(filtered.clone(), i),
                Action::Favorite(track) => {
                    self.library.toggle_favorite(track);
                    self.persist();
                }
                Action::Queue(track) => {
                    self.queue.enqueue(track);
                    self.notice = "Added to the queue.".into();
                }
                Action::Playlist(id, track) => {
                    if let Some(playlist) = self.library.playlists.iter_mut().find(|p| p.id == id) {
                        playlist.tracks.push(track);
                        self.notice = "Added to the playlist.".into();
                        self.persist();
                    }
                }
                Action::RemoveLocal(id) => {
                    self.library.local.retain(|t| t.id != id);
                    self.persist();
                    self.notice = "Removed from library. The audio file is still on disk.".into();
                }
                Action::Album(track) => {
                    if let Some(id) = track.album_id {
                        self.page = Page::CatalogAlbum(track.album, track.artist);
                        self.request = self.services.next_generation();
                        self.pending = true;
                        self.results.clear();
                        self.submit(Task::Album {
                            generation: self.request,
                            id,
                            country: self.library.settings.country.clone(),
                        });
                    } else {
                        self.navigate(Page::Group(track.album, track.artist));
                    }
                }
                Action::Artist(artist) => {
                    if filtered.iter().any(|t| t.artist == artist && t.is_local()) {
                        self.navigate(Page::Group(String::new(), artist));
                    } else {
                        self.query = artist;
                        self.search();
                    }
                }
            }
        }
    }
    fn groups(&mut self, ui: &mut egui::Ui, artists: bool) {
        self.heading(
            ui,
            if artists { "Artists" } else { "Albums" },
            "From your imported audio",
        );
        let mut groups: BTreeMap<(String, String), Vec<Track>> = BTreeMap::new();
        for track in &self.library.local {
            let key = if artists {
                (String::new(), track.artist.clone())
            } else {
                (track.album.clone(), track.artist.clone())
            };
            groups.entry(key).or_default().push(track.clone());
        }
        if groups.is_empty() {
            ui.label("Import music to build your library.");
            ui.add_space(16.);
            if components::primary(ui, "Import audio…").clicked() {
                self.open_files();
            }
            return;
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            for ((album, artist), tracks) in groups {
                ui.horizontal(|ui| {
                    self.cover(ui, tracks.first(), 76.);
                    ui.vertical(|ui| {
                        if ui
                            .button(
                                RichText::new(if artists { &artist } else { &album })
                                    .size(18.)
                                    .strong(),
                            )
                            .clicked()
                        {
                            self.navigate(Page::Group(album.clone(), artist.clone()));
                        }
                        if !artists {
                            ui.label(&artist);
                        }
                        ui.weak(format!("{} songs", tracks.len()));
                    });
                    if ui.button("Play").clicked() {
                        self.play(tracks.clone(), 0);
                    }
                });
                ui.add_space(18.);
                ui.separator();
                ui.add_space(10.);
            }
        });
    }
    fn playlists(&mut self, ui: &mut egui::Ui) {
        self.heading(ui, "Playlists", "Collections saved on this computer");
        ui.horizontal(|ui| {
            ui.add_sized(
                [230., 32.],
                egui::TextEdit::singleline(&mut self.playlist_name).hint_text("New playlist name"),
            );
            if ui
                .add_enabled(
                    !self.playlist_name.trim().is_empty(),
                    egui::Button::new("Create playlist"),
                )
                .clicked()
            {
                let id = self
                    .library
                    .playlists
                    .iter()
                    .map(|p| p.id)
                    .max()
                    .unwrap_or(0)
                    + 1;
                self.library.playlists.push(Playlist {
                    id,
                    name: self.playlist_name.trim().chars().take(120).collect(),
                    tracks: vec![],
                });
                self.playlist_name.clear();
                self.persist();
            }
        });
        ui.add_space(22.);
        if self.library.playlists.is_empty() {
            ui.label("Make a playlist for whatever comes next.");
            ui.weak("Create one above, then use More beside a song to add it.");
        }
        let playlists = self.library.playlists.clone();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for playlist in playlists {
                ui.horizontal(|ui| {
                    self.cover(ui, playlist.tracks.first(), 54.);
                    ui.vertical(|ui| {
                        if ui
                            .button(RichText::new(&playlist.name).size(17.).strong())
                            .clicked()
                        {
                            self.navigate(Page::Playlist(playlist.id));
                        }
                        ui.weak(format!("{} tracks", playlist.tracks.len()));
                    });
                });
                ui.add_space(16.);
            }
        });
    }
    fn playlist(&mut self, ui: &mut egui::Ui, id: u64) {
        let Some(index) = self.library.playlists.iter().position(|p| p.id == id) else {
            self.navigate(Page::Playlists);
            return;
        };
        let playlist = self.library.playlists[index].clone();
        self.heading(ui, &playlist.name, "Local playlist");
        ui.horizontal(|ui| {
            if ui.button("All playlists").clicked() {
                self.navigate(Page::Playlists);
            }
            ui.menu_button("Edit playlist", |ui| {
                ui.label("Rename");
                let mut name = self.library.playlists[index].name.clone();
                if ui.text_edit_singleline(&mut name).changed() && !name.trim().is_empty() {
                    self.library.playlists[index].name = name.chars().take(120).collect();
                    self.persist();
                }
                if ui.button("Delete playlist").clicked() {
                    self.library.playlists.remove(index);
                    self.navigate(Page::Playlists);
                    self.persist();
                    ui.close();
                }
            });
        });
        ui.add_space(12.);
        if self.page == Page::Playlist(id) {
            if !playlist.tracks.is_empty() {
                ui.collapsing("Remove tracks from this playlist", |ui| {
                    let mut remove = None;
                    for (i, track) in playlist.tracks.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(&track.title);
                            if ui.small_button("Remove").clicked() {
                                remove = Some(i);
                            }
                        });
                    }
                    if let Some(i) = remove {
                        self.library.playlists[index].tracks.remove(i);
                        self.persist();
                    }
                });
            }
            self.track_list(ui, playlist.tracks, false);
        }
    }
    fn settings(&mut self, ui: &mut egui::Ui) {
        self.heading(ui, "Settings", "Playback, appearance, and app capabilities");
        egui::ScrollArea::vertical().show(ui,|ui|{
            ui.label(RichText::new("Appearance").size(18.).strong());ui.add_space(6.);
            if ui.checkbox(&mut self.library.settings.light,"Light appearance").changed(){components::style(ui.ctx(),self.library.settings.light);self.persist();}
            ui.add_space(22.);ui.separator();ui.add_space(18.);
            ui.label(RichText::new("Playback").size(18.).strong());ui.add_space(8.);ui.label(&self.output);ui.weak(&self.media_status);
            ui.weak("Rust decoder and mixer · Windows audio output");
            if ui.button("Reconnect audio").clicked(){
                self.audio=Audio::start();self.send(Command::Volume(self.library.settings.volume));
                self.playing=false;self.loading=false;self.position=0.;
                if let Some(track)=self.queue.active().cloned(){self.load(track,0.);}
            }
            ui.add_space(10.);
            if ui.add(egui::Slider::new(&mut self.library.settings.crossfade_seconds,0.0..=12.0).text("Crossfade (seconds)")).changed(){self.persist();}
            ui.weak("Overlap consecutive playable tracks with an equal-power fade. Off at zero.");
            ui.add_space(22.);ui.separator();ui.add_space(18.);
            ui.label(RichText::new("Catalog").size(18.).strong());ui.add_space(8.);
            ui.horizontal(|ui| {
                ui.label("Country code");
                ui.add_sized([70.,28.],egui::TextEdit::singleline(&mut self.country_draft));
                if ui.add_enabled(cadence::library::valid_country(&self.country_draft),egui::Button::new("Apply")).clicked(){
                    self.library.settings.country=self.country_draft.to_ascii_uppercase();self.persist();
                    self.request=self.services.next_generation();self.results.clear();self.pending=false;
                }
            });
            ui.weak("Two-letter storefront code, such as IN, US, or GB.");
            ui.add_space(22.);ui.separator();ui.add_space(18.);
            ui.label(RichText::new("Apple Music capabilities").size(18.).strong());ui.add_space(8.);
            ui.label(APPLE_ACCOUNT_STATUS);ui.add_space(10.);
            egui::Grid::new("capabilities").num_columns(2).spacing([22.,12.]).striped(true).show(ui,|ui|{
                for(feature,status) in [
                    ("Local audio / catalog previews","Available"),
                    ("Local playlists / favorites / history","Available"),
                    ("Apple sign-in / cloud library","Provider unavailable"),
                    ("Full subscription streams / radio","Provider unavailable"),
                    ("Local ALAC / FLAC decoding","Supported codec; output depends on device"),
                    ("Apple lossless streams / offline downloads","Provider unavailable"),
                    ("Dolby Atmos / music videos","Unavailable"),
                    ("Sound Check / Sound Enhancer / Apple lyrics","Unavailable"),
                ]{ui.label(feature);ui.label(status);ui.end_row();}
            });
            ui.add_space(24.);ui.label(RichText::new("About").size(18.).strong());
            ui.label(format!("Cadence {} · Native Rust edition",env!("CARGO_PKG_VERSION")));
            ui.weak("Ctrl+O Import · Ctrl+F Search · Space Play/Pause · Ctrl+Left/Right Previous/Next");
            ui.weak("Legacy library.json is preserved. New changes are stored transactionally in library.sqlite3.");
        });
    }
    fn account(&mut self, ui: &mut egui::Ui) {
        self.heading(ui, "Apple Music", "Account connection");
        ui.label(
            RichText::new("Native connection unavailable")
                .size(21.)
                .strong(),
        );
        ui.add_space(12.);
        ui.label(APPLE_ACCOUNT_STATUS);
        ui.add_space(12.);
        ui.label("Catalog previews work without signing in. Your Apple subscription library is not connected.");
        ui.add_space(20.);
        if ui.button("Search catalog previews").clicked() {
            self.navigate(Page::Search);
        }
        ui.add_space(20.);
        ui.hyperlink_to(
            "Apple Music integration documentation",
            "https://developer.apple.com/musickit/",
        );
    }
}
