use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MediaSource {
    LocalFile(PathBuf),
    PreviewUrl(String),
    AppleCatalogItem(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub source: MediaSource,
    pub artwork: Option<String>,
    pub duration: Option<f64>,
    #[serde(default)]
    pub album_id: Option<String>,
}

impl Track {
    pub fn is_local(&self) -> bool {
        matches!(self.source, MediaSource::LocalFile(_))
    }

    pub fn kind(&self) -> &'static str {
        match self.source {
            MediaSource::LocalFile(_) => "Local file",
            MediaSource::PreviewUrl(_) => "Preview",
            MediaSource::AppleCatalogItem(_) => "Apple Music",
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Playlist {
    pub id: u64,
    pub name: String,
    pub tracks: Vec<Track>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub light: bool,
    pub volume: f32,
    pub country: String,
    pub crossfade_seconds: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            light: false,
            volume: 0.65,
            country: "IN".into(),
            crossfade_seconds: 0.0,
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    pub local: Vec<Track>,
    pub favorites: Vec<Track>,
    pub history: Vec<Track>,
    pub playlists: Vec<Playlist>,
    pub settings: Settings,
}

impl Library {
    pub fn remember(&mut self, track: Track) {
        self.history.retain(|t| t.id != track.id);
        self.history.insert(0, track);
        self.history.truncate(100);
    }

    pub fn toggle_favorite(&mut self, track: Track) {
        if let Some(i) = self.favorites.iter().position(|t| t.id == track.id) {
            self.favorites.remove(i);
        } else {
            self.favorites.insert(0, track);
        }
    }
}

pub const APPLE_ACCOUNT_STATUS: &str = "Apple account connection is unavailable in this build. A native authorization and subscription playback provider has not been established.";
