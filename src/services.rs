use crate::{
    catalog,
    library::{self, Database},
    model::{Library, MediaSource, Track},
};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread,
};

pub enum Task {
    Search {
        generation: u64,
        query: String,
        country: String,
    },
    Import(Vec<PathBuf>),
    Album {
        generation: u64,
        id: String,
        country: String,
    },
    Shutdown,
    Save(Box<Library>),
    FinalSave(Box<Library>, SyncSender<Result<(), String>>),
    Charts {
        generation: u64,
        country: String,
    },
}
pub enum Event {
    Results(u64, Result<Vec<Track>, String>),
    Imported(Vec<Track>, Vec<String>),
    SaveFailed(String),
}
pub struct Services {
    sender: SyncSender<Task>,
    pub events: Receiver<Event>,
    pub generation: Arc<AtomicU64>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Services {
    pub fn start(dir: PathBuf) -> Result<(Self, Library), String> {
        let db = Database::open(&dir.join("library.sqlite3"), &dir.join("library.json"))?;
        let library = db.load()?;
        let (sender, tasks) = mpsc::sync_channel(16);
        let (events_tx, events) = mpsc::channel();
        let generation = Arc::new(AtomicU64::new(0));
        let active = generation.clone();
        let worker = thread::Builder::new()
            .name("cadence-services".into())
            .spawn(move || {
                let mut db = db;
                let client = catalog::client();
                while let Ok(task) = tasks.recv() {
                    let event = match task {
                        Task::Shutdown => break,
                        Task::Album {
                            generation,
                            id,
                            country,
                        } => {
                            if generation != active.load(Ordering::Relaxed) {
                                continue;
                            }
                            Event::Results(
                                generation,
                                client
                                    .as_ref()
                                    .map_err(Clone::clone)
                                    .and_then(|c| catalog::album(c, &id, &country)),
                            )
                        }
                        Task::Save(library) => {
                            if let Err(e) = db.save(&library) {
                                let _ = events_tx.send(Event::SaveFailed(e));
                            }
                            continue;
                        }
                        Task::FinalSave(library, reply) => {
                            let _ = reply.send(db.save(&library));
                            continue;
                        }
                        Task::Import(paths) => {
                            let mut tracks = Vec::new();
                            let mut errors = Vec::new();
                            for path in paths {
                                match import_file(&path) {
                                    Ok(t) => tracks.push(t),
                                    Err(e) => errors.push(e),
                                }
                            }
                            Event::Imported(tracks, errors)
                        }
                        Task::Search {
                            generation,
                            query,
                            country,
                        } => {
                            if generation != active.load(Ordering::Relaxed) {
                                continue;
                            }
                            Event::Results(
                                generation,
                                client
                                    .as_ref()
                                    .map_err(Clone::clone)
                                    .and_then(|c| catalog::search(c, &query, &country)),
                            )
                        }
                        Task::Charts {
                            generation,
                            country,
                        } => Event::Results(generation, crate::apple_api::get_top_charts(&country)),
                    };
                    if events_tx.send(event).is_err() {
                        break;
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok((
            Self {
                sender,
                events,
                generation,
                worker: Some(worker),
            },
            library,
        ))
    }
    pub fn submit(&self, task: Task) -> Result<(), String> {
        self.sender
            .try_send(task)
            .map_err(|_| "Background tasks are busy. Please retry in a moment.".into())
    }
    /// Closing waits for the last snapshot to commit, including when the task queue was full.
    pub fn save_final(&self, library: &Library) -> Result<(), String> {
        let (reply, result) = mpsc::sync_channel(1);
        self.sender
            .send(Task::FinalSave(Box::new(library.clone()), reply))
            .map_err(|_| "The library service stopped before saving.".to_owned())?;
        result
            .recv()
            .map_err(|_| "The library service stopped before confirming the save.".to_owned())?
    }
    pub fn next_generation(&self) -> u64 {
        self.generation.fetch_add(1, Ordering::Relaxed) + 1
    }
}

pub fn import_file(path: &Path) -> Result<Track, String> {
    use symphonia::core::{
        formats::FormatOptions,
        io::{MediaSourceStream, MediaSourceStreamOptions},
        meta::MetadataOptions,
        probe::Hint,
    };
    let path = path
        .canonicalize()
        .map_err(|_| "An audio file could not be found.".to_owned())?;
    let file =
        std::fs::File::open(&path).map_err(|_| "An audio file could not be opened.".to_owned())?;
    let stream = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        hint.with_extension(ext);
    }
    let mut probed = symphonia::default::get_probe()
        .format(
            &hint,
            stream,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|_| {
            format!(
                "Unsupported or damaged audio: {}",
                path.file_name().unwrap_or_default().to_string_lossy()
            )
        })?;
    let mut tags = HashMap::new();
    let mut read_tags = |revision: &symphonia::core::meta::MetadataRevision| {
        for tag in revision.tags() {
            if let Some(key) = tag.std_key {
                tags.insert(format!("{key:?}"), tag.value.to_string());
            }
        }
    };
    if let Some(rev) = probed.metadata.get().as_ref().and_then(|m| m.current()) {
        read_tags(rev);
    }
    if let Some(rev) = probed.format.metadata().current() {
        read_tags(rev);
    }
    let params = &probed
        .format
        .default_track()
        .ok_or("No audio track found")?
        .codec_params;
    let duration = params.time_base.zip(params.n_frames).map(|(base, frames)| {
        let t = base.calc_time(frames);
        t.seconds as f64 + t.frac
    });
    Ok(Track {
        id: format!("local:{}", path.to_string_lossy().to_lowercase()),
        title: tags.remove("TrackTitle").unwrap_or_else(|| {
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into()
        }),
        artist: tags
            .remove("Artist")
            .unwrap_or_else(|| "Unknown Artist".into()),
        album: tags
            .remove("Album")
            .unwrap_or_else(|| "Imported audio".into()),
        source: MediaSource::LocalFile(path),
        artwork: None,
        duration,
        album_id: None,
    })
}

struct Decoded {
    size: [usize; 2],
    rgba: Vec<u8>,
}
pub struct Artwork {
    sender: SyncSender<String>,
    events: Receiver<(String, Result<Decoded, String>)>,
    requested: HashSet<String>,
    cache: HashMap<String, (eframe::egui::TextureHandle, u64)>,
    tick: u64,
    bytes: usize,
}
impl Default for Artwork {
    fn default() -> Self {
        Self::start()
    }
}
impl Artwork {
    pub fn start() -> Self {
        let (sender, requests) = mpsc::sync_channel::<String>(128);
        let (tx, events) = mpsc::sync_channel(8);
        thread::spawn(move || {
            let client = catalog::client();
            while let Ok(url) = requests.recv() {
                let result = (|| {
                    if !url::Url::parse(&url).is_ok_and(|u| catalog::is_artwork_url(&u)) {
                        return Err("Invalid artwork host".into());
                    }
                    let response = client
                        .as_ref()
                        .map_err(Clone::clone)?
                        .get(&url)
                        .send()
                        .map_err(|_| "Artwork unavailable".to_owned())?;
                    let bytes = catalog::limited_bytes(response, 2_000_000)?;
                    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
                        .with_guessed_format()
                        .map_err(|e| e.to_string())?;
                    let mut limits = image::Limits::default();
                    limits.max_image_width = Some(4096);
                    limits.max_image_height = Some(4096);
                    limits.max_alloc = Some(64 * 1024 * 1024);
                    reader.limits(limits);
                    let decoded = reader
                        .decode()
                        .map_err(|e| e.to_string())?
                        .thumbnail(384, 384)
                        .to_rgba8();
                    Ok(Decoded {
                        size: [decoded.width() as usize, decoded.height() as usize],
                        rgba: decoded.into_raw(),
                    })
                })();
                if tx.send((url, result)).is_err() {
                    break;
                }
            }
        });
        Self {
            sender,
            events,
            requested: HashSet::new(),
            cache: HashMap::new(),
            tick: 0,
            bytes: 0,
        }
    }
    pub fn poll(&mut self, ctx: &eframe::egui::Context) {
        for _ in 0..4 {
            let Ok((url, result)) = self.events.try_recv() else {
                break;
            };
            if let Ok(image) = result {
                while self.bytes + image.rgba.len() > 24 * 1024 * 1024 {
                    let Some(key) = self
                        .cache
                        .iter()
                        .min_by_key(|(_, (_, tick))| *tick)
                        .map(|(key, _)| key.clone())
                    else {
                        break;
                    };
                    if let Some((old, _)) = self.cache.remove(&key) {
                        self.bytes = self.bytes.saturating_sub(old.size()[0] * old.size()[1] * 4);
                    }
                    self.requested.remove(&key);
                }
                self.bytes += image.rgba.len();
                self.cache.insert(
                    url.clone(),
                    (
                        ctx.load_texture(
                            url,
                            eframe::egui::ColorImage::from_rgba_unmultiplied(
                                image.size,
                                &image.rgba,
                            ),
                            Default::default(),
                        ),
                        self.tick,
                    ),
                );
            }
        }
    }
    pub fn get(&mut self, url: Option<&str>) -> Option<eframe::egui::TextureHandle> {
        let url = url?;
        self.tick = self.tick.wrapping_add(1);
        if let Some((texture, tick)) = self.cache.get_mut(url) {
            *tick = self.tick;
            return Some(texture.clone());
        }
        if !self.requested.contains(url) && self.sender.try_send(url.to_owned()).is_ok() {
            if self.requested.len() > 4096 {
                self.requested.retain(|u| self.cache.contains_key(u));
            }
            self.requested.insert(url.to_owned());
        }
        None
    }
}

pub fn default_services() -> Result<(Services, Library), String> {
    Services::start(library::data_dir()?)
}

impl Drop for Services {
    fn drop(&mut self) {
        let _ = self.sender.send(Task::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shutdown_drains_accepted_library_writes() {
        let dir = tempfile::tempdir().unwrap();
        let (services, mut library) = Services::start(dir.path().into()).unwrap();
        library.settings.light = true;
        services.submit(Task::Save(Box::new(library))).unwrap();
        drop(services);
        let db = Database::open(
            &dir.path().join("library.sqlite3"),
            &dir.path().join("library.json"),
        )
        .unwrap();
        assert!(db.load().unwrap().settings.light);
    }
    #[test]
    fn final_save_commits_latest_snapshot_after_queued_writes() {
        let dir = tempfile::tempdir().unwrap();
        let (services, mut library) = Services::start(dir.path().into()).unwrap();
        services
            .submit(Task::Save(Box::new(library.clone())))
            .unwrap();
        library.settings.light = true;
        services.save_final(&library).unwrap();
        drop(services);
        let db = Database::open(
            &dir.path().join("library.sqlite3"),
            &dir.path().join("library.json"),
        )
        .unwrap();
        assert!(db.load().unwrap().settings.light);
    }
}
