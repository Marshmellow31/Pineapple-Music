use crate::catalog::Track;
use crate::model::MediaSource;
use serde::{Deserialize, Serialize};

fn developer_token() -> Result<String, String> {
    std::env::var("CADENCE_APPLE_DEVELOPER_TOKEN").ok().filter(|t| !t.trim().is_empty())
        .ok_or_else(|| "Native API access needs your own CADENCE_APPLE_DEVELOPER_TOKEN. Native account authorization is not implemented.".into())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub track_count: Option<usize>,
    pub artwork_url: Option<String>,
}

#[derive(Deserialize)]
struct ApiData<T> {
    data: Option<Vec<T>>,
    next: Option<String>,
}

#[derive(Deserialize)]
struct ChartResults {
    results: Option<ChartCategories>,
}

#[derive(Deserialize)]
struct ChartCategories {
    songs: Option<Vec<ChartSongData>>,
}

#[derive(Deserialize)]
struct ChartSongData {
    data: Option<Vec<ApiSongItem>>,
}

#[derive(Deserialize)]
struct ApiPlaylistItem {
    id: String,
    attributes: Option<ApiPlaylistAttributes>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiPlaylistAttributes {
    name: Option<String>,
    description: Option<ApiDescription>,
    artwork: Option<ApiArtwork>,
}

#[derive(Deserialize)]
struct ApiDescription {
    standard: Option<String>,
}

#[derive(Deserialize)]
struct ApiArtwork {
    url: Option<String>,
}

#[derive(Deserialize)]
struct ApiSongItem {
    id: String,
    attributes: Option<ApiSongAttributes>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiSongAttributes {
    name: Option<String>,
    artist_name: Option<String>,
    album_name: Option<String>,
    duration_in_millis: Option<u64>,
    artwork: Option<ApiArtwork>,
    previews: Option<Vec<ApiPreview>>,
    play_params: Option<ApiPlayParams>,
}

#[derive(Deserialize)]
struct ApiPreview {
    url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiPlayParams {
    catalog_id: Option<String>,
}

fn format_artwork(raw_url: Option<&str>, size: u32) -> Option<String> {
    let url = raw_url?;
    if url.is_empty() {
        return None;
    }
    let formatted = url
        .replace("{w}", &size.to_string())
        .replace("{h}", &size.to_string())
        .replace("{f}", "jpg");
    Some(formatted)
}

fn song_item_to_track(item: ApiSongItem) -> Option<Track> {
    let attrs = item.attributes?;
    let title = attrs.name.unwrap_or_else(|| "Unknown Track".into());
    let artist = attrs.artist_name.unwrap_or_else(|| "Unknown Artist".into());
    let album = attrs.album_name.unwrap_or_default();
    let _catalog_duration = attrs
        .duration_in_millis
        .map(|m| m as f64 / 1000.0)
        .unwrap_or(30.0);
    let artwork = format_artwork(attrs.artwork.as_ref().and_then(|a| a.url.as_deref()), 300);

    // Determine source: preview URL or fallback
    let source = if let Some(previews) = attrs.previews {
        if let Some(p) = previews.into_iter().next() {
            p.url.unwrap_or_default()
        } else {
            String::new()
        }
    } else if let Some(params) = attrs.play_params {
        if let Some(cat_id) = params.catalog_id {
            format!("resolve:catalog:{}", cat_id)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    Some(Track {
        id: format!("apple:{}", item.id),
        title,
        artist,
        album,
        source: if let Some(id) = source.strip_prefix("resolve:catalog:") {
            MediaSource::AppleCatalogItem(id.into())
        } else {
            MediaSource::PreviewUrl(source)
        },
        artwork,
        duration: None,
        album_id: None,
    })
}

fn build_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Network client creation failed: {e}"))
}

fn collection<T: serde::de::DeserializeOwned>(path: &str, token: &str) -> Result<Vec<T>, String> {
    if token.trim().is_empty() || token == "demo_active" {
        return Err("Sign in to Apple Music first.".into());
    }
    let client = build_client()?;
    let mut next = Some(format!("https://api.music.apple.com{path}"));
    let mut visited = std::collections::HashSet::new();
    let mut result = Vec::new();
    while let Some(raw) = next.take() {
        let url = url::Url::parse(&raw).map_err(|_| "Invalid Apple pagination URL")?;
        if url.scheme() != "https"
            || url.host_str() != Some("api.music.apple.com")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.port_or_known_default() != Some(443)
        {
            return Err("Apple returned an unexpected pagination host.".into());
        }
        if !visited.insert(raw.clone()) {
            return Err("Apple pagination repeated a page.".into());
        }
        let response = client
            .get(url.clone())
            .header("Authorization", format!("Bearer {}", developer_token()?))
            .header("Music-User-Token", token)
            .send()
            .map_err(|_| "Could not reach Apple Music. Check your connection.")?;
        match response.status().as_u16() {
            401 => return Err("Apple authorization expired. Sign in again.".into()),
            403 => {
                return Err(
                    "Apple refused library access. Check your subscription and authorization."
                        .into(),
                );
            }
            429 => return Err("Apple rate limited this request. Please retry later.".into()),
            code if !(200..300).contains(&code) => {
                return Err(format!("Apple Music returned HTTP {code}."));
            }
            _ => {}
        }
        let page: ApiData<T> = response
            .json()
            .map_err(|_| "Apple returned an unreadable library response.")?;
        result.extend(page.data.unwrap_or_default());
        next = page
            .next
            .map(|path| {
                url.join(&path)
                    .map(|u| u.to_string())
                    .map_err(|_| "Invalid next page")
            })
            .transpose()?;
    }
    Ok(result)
}

pub fn get_user_playlists(user_token: &str) -> Result<Vec<Playlist>, String> {
    Ok(
        collection::<ApiPlaylistItem>("/v1/me/library/playlists?limit=100", user_token)?
            .into_iter()
            .filter_map(|p| {
                let attrs = p.attributes?;
                Some(Playlist {
                    id: p.id,
                    name: attrs.name?,
                    description: attrs.description.and_then(|d| d.standard),
                    track_count: None,
                    artwork_url: format_artwork(
                        attrs.artwork.as_ref().and_then(|a| a.url.as_deref()),
                        300,
                    ),
                })
            })
            .collect(),
    )
}

pub fn get_playlist_tracks(user_token: &str, playlist_id: &str) -> Result<Vec<Track>, String> {
    if playlist_id.is_empty()
        || !playlist_id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b".-_".contains(&c))
    {
        return Err("Invalid playlist identifier.".into());
    }
    Ok(collection::<ApiSongItem>(
        &format!("/v1/me/library/playlists/{playlist_id}/tracks?limit=100"),
        user_token,
    )?
    .into_iter()
    .filter_map(song_item_to_track)
    .collect())
}

pub fn get_user_library_songs(user_token: &str) -> Result<Vec<Track>, String> {
    Ok(
        collection::<ApiSongItem>("/v1/me/library/songs?limit=100", user_token)?
            .into_iter()
            .filter_map(song_item_to_track)
            .collect(),
    )
}

pub fn get_recently_played(user_token: &str) -> Result<Vec<Track>, String> {
    Ok(
        collection::<ApiSongItem>("/v1/me/recent/played/tracks?limit=30", user_token)?
            .into_iter()
            .filter_map(song_item_to_track)
            .collect(),
    )
}

/// Fetch global Top Charts from Apple Music catalog
pub fn get_top_charts(country: &str) -> Result<Vec<Track>, String> {
    let client = build_client()?;
    let country_lower = country.to_lowercase();
    let url = format!(
        "https://api.music.apple.com/v1/catalog/{country_lower}/charts?types=songs&limit=30"
    );
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", developer_token()?))
        .send()
        .map_err(|e| format!("Failed to fetch charts: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Charts API returned HTTP {}", resp.status()));
    }

    let body: ChartResults = resp
        .json()
        .map_err(|e| format!("Failed to parse charts JSON: {e}"))?;

    let song_data = body
        .results
        .and_then(|r| r.songs)
        .and_then(|s| s.into_iter().next())
        .and_then(|d| d.data)
        .unwrap_or_default();

    let tracks = song_data
        .into_iter()
        .filter_map(song_item_to_track)
        .collect();

    Ok(tracks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_artwork_replaces_placeholders() {
        let raw = "https://is1-ssl.mzstatic.com/image/thumb/Music/test.jpg/{w}x{h}bb.{f}";
        let res = format_artwork(Some(raw), 300).unwrap();
        assert_eq!(
            res,
            "https://is1-ssl.mzstatic.com/image/thumb/Music/test.jpg/300x300bb.jpg"
        );
    }

    #[test]
    #[ignore = "Live Apple service check; run explicitly with --ignored"]
    fn top_charts_live_fetch() {
        let tracks = get_top_charts("us").expect("Failed to fetch top charts from Apple API");
        assert!(!tracks.is_empty(), "Top charts should return songs");
        assert!(!tracks[0].title.is_empty(), "First track has title");
        assert!(!tracks[0].artist.is_empty(), "First track has artist");
    }
}
