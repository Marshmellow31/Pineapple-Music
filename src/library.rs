use crate::model::{Library, MediaSource, Track};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Deserialize;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::Duration,
};

pub struct Database {
    connection: Connection,
}

impl Database {
    pub fn open(path: &Path, legacy: &Path) -> Result<Self, String> {
        let mut connection = Connection::open(path).map_err(|e| e.to_string())?;
        connection
            .busy_timeout(Duration::from_secs(3))
            .map_err(|e| e.to_string())?;
        connection.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS app_state (id INTEGER PRIMARY KEY CHECK(id=1), version INTEGER NOT NULL, body TEXT NOT NULL);").map_err(|e|e.to_string())?;
        let existing: Option<i64> = connection
            .query_row("SELECT version FROM app_state WHERE id=1", [], |r| r.get(0))
            .optional()
            .map_err(|e| e.to_string())?;
        if existing.is_some_and(|v| v != 1) {
            return Err(
                "This library uses a newer schema. Open it with a compatible Cadence version."
                    .into(),
            );
        }
        if existing.is_none() {
            let library = if legacy.exists() {
                migrate(&std::fs::read(legacy).map_err(|e| e.to_string())?)?
            } else {
                Library::default()
            };
            let transaction = connection.transaction().map_err(|e| e.to_string())?;
            transaction
                .execute(
                    "INSERT INTO app_state(id,version,body) VALUES (1,1,?1)",
                    params![serde_json::to_string(&library).map_err(|e| e.to_string())?],
                )
                .map_err(|e| e.to_string())?;
            transaction.commit().map_err(|e| e.to_string())?;
        }
        Ok(Self { connection })
    }

    pub fn load(&self) -> Result<Library, String> {
        let body: String = self
            .connection
            .query_row("SELECT body FROM app_state WHERE id=1", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let mut library: Library = serde_json::from_str(&body)
            .map_err(|e| format!("Library could not be read; data is preserved: {e}"))?;
        library.settings.volume = finite_clamp(library.settings.volume, 0.65, 0.0, 1.0);
        library.settings.crossfade_seconds =
            finite_clamp(library.settings.crossfade_seconds, 0.0, 0.0, 12.0);
        if !valid_country(&library.settings.country) {
            library.settings.country = "IN".into();
        }
        Ok(library)
    }

    pub fn save(&mut self, library: &Library) -> Result<(), String> {
        let body = serde_json::to_string(library).map_err(|e| e.to_string())?;
        let tx = self.connection.transaction().map_err(|e| e.to_string())?;
        tx.execute("UPDATE app_state SET body=?1 WHERE id=1", params![body])
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }
}

fn finite_clamp(value: f32, default: f32, min: f32, max: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        default
    }
}
pub fn valid_country(country: &str) -> bool {
    country.len() == 2 && country.bytes().all(|c| c.is_ascii_alphabetic())
}
pub fn data_dir() -> Result<PathBuf, String> {
    let args: Vec<_> = std::env::args_os().collect();
    let override_path = args
        .iter()
        .position(|a| a == "--data-dir")
        .and_then(|i| args.get(i + 1));
    let dir = if let Some(path) = override_path {
        PathBuf::from(path)
    } else if let Some(path) = std::env::var_os("CADENCE_DATA_DIR") {
        PathBuf::from(path)
    } else {
        directories::ProjectDirs::from("in", "Harshil", "Cadence")
            .ok_or("Windows profile directory is unavailable")?
            .data_local_dir()
            .into()
    };
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct LegacyLibrary {
    local: Vec<LegacyTrack>,
    saved: Vec<LegacyTrack>,
    history: Vec<LegacyTrack>,
    light: bool,
}
#[derive(Deserialize)]
struct LegacyTrack {
    id: String,
    title: String,
    artist: String,
    album: String,
    source: String,
    artwork: Option<String>,
    local: bool,
    duration: f64,
}

fn migrate(bytes: &[u8]) -> Result<Library, String> {
    let old: LegacyLibrary = serde_json::from_slice(bytes).map_err(|e| {
        format!("Legacy library could not be imported; original JSON is preserved: {e}")
    })?;
    let convert = |items: Vec<LegacyTrack>| {
        let mut seen = HashSet::new();
        items
            .into_iter()
            .map(|t| {
                let (id, source) = if t.local {
                    (
                        format!("local:{}", t.source.to_lowercase()),
                        MediaSource::LocalFile(t.source.into()),
                    )
                } else if let Some(id) = t.source.strip_prefix("resolve:catalog:") {
                    (
                        format!("apple:{id}"),
                        MediaSource::AppleCatalogItem(id.into()),
                    )
                } else {
                    (format!("apple:{}", t.id), MediaSource::PreviewUrl(t.source))
                };
                Track {
                    id,
                    title: t.title,
                    artist: t.artist,
                    album: t.album,
                    source,
                    artwork: t.artwork,
                    duration: if t.local && t.duration > 0.0 {
                        Some(t.duration)
                    } else {
                        None
                    },
                    album_id: None,
                }
            })
            .filter(|t| seen.insert(t.id.clone()))
            .collect()
    };
    Ok(Library {
        local: convert(old.local),
        favorites: convert(old.saved),
        history: convert(old.history),
        settings: crate::model::Settings {
            light: old.light,
            ..Default::default()
        },
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migration_preserves_local_data_without_importing_credentials_or_claiming_cloud_auth() {
        let bytes=br#"{"user_token":"secret","user_email":"private","light":true,"saved":[{"id":"1","title":"Song","artist":"Artist","album":"Album","source":"https://audio-ssl.itunes.apple.com/a.m4a","artwork":null,"local":false,"duration":240}],"local":[]}"#;
        let result = migrate(bytes).unwrap();
        assert_eq!(result.favorites.len(), 1);
        assert!(result.settings.light);
        let encoded = serde_json::to_string(&result).unwrap();
        assert!(!encoded.contains("secret"));
        assert!(!encoded.contains("private"));
        assert!(result.favorites[0].duration.is_none());
    }
    #[test]
    fn database_migrates_once_and_leaves_original_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let legacy = dir.path().join("library.json");
        let original = br#"{"saved":[],"light":true}"#;
        std::fs::write(&legacy, original).unwrap();
        let path = dir.path().join("library.sqlite3");
        let mut db = Database::open(&path, &legacy).unwrap();
        let mut state = db.load().unwrap();
        assert!(state.settings.light);
        state.settings.light = false;
        db.save(&state).unwrap();
        drop(db);
        assert!(
            !Database::open(&path, &legacy)
                .unwrap()
                .load()
                .unwrap()
                .settings
                .light
        );
        assert_eq!(std::fs::read(legacy).unwrap(), original);
    }
    #[test]
    fn corrupt_legacy_data_is_not_overwritten_or_marked_migrated() {
        let dir = tempfile::tempdir().unwrap();
        let legacy = dir.path().join("library.json");
        std::fs::write(&legacy, b"bad").unwrap();
        let path = dir.path().join("library.sqlite3");
        assert!(Database::open(&path, &legacy).is_err());
        std::fs::write(&legacy, b"{}").unwrap();
        assert!(Database::open(&path, &legacy).is_ok());
    }
}
