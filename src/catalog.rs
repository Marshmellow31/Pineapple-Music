use crate::model::MediaSource;
pub use crate::model::Track;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::io::Read;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchResponse {
    results: Vec<CatalogTrack>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogTrack {
    track_id: Option<u64>,
    track_name: Option<String>,
    artist_name: Option<String>,
    collection_name: Option<String>,
    preview_url: Option<String>,
    artwork_url100: Option<String>,
    collection_id: Option<u64>,
}

pub fn is_preview_url(url: &url::Url) -> bool {
    url.scheme() == "https"
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
        && url.host_str().is_some_and(|h| {
            h == "audio-ssl.itunes.apple.com"
                || h.ends_with(".itunes.apple.com")
                || h.ends_with(".mzstatic.com")
        })
}
pub fn client() -> Result<Client, String> {
    Client::builder()
        .connect_timeout(std::time::Duration::from_secs(6))
        .timeout(std::time::Duration::from_secs(18))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Network client could not start.".into())
}
pub fn limited_bytes(response: reqwest::blocking::Response, limit: u64) -> Result<Vec<u8>, String> {
    let response = response.error_for_status().map_err(|e| {
        format!(
            "Request failed ({}).",
            e.status()
                .map(|s| s.as_u16().to_string())
                .unwrap_or_else(|| "network".into())
        )
    })?;
    if response.content_length().is_some_and(|n| n > limit) {
        return Err("Response exceeds size limit".into());
    }
    let mut bytes = Vec::new();
    response
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Download interrupted")?;
    if bytes.len() as u64 > limit {
        return Err("Response exceeds size limit".into());
    }
    Ok(bytes)
}
pub fn is_artwork_url(url: &url::Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && url
            .host_str()
            .is_some_and(|h| h.ends_with(".mzstatic.com") || h.ends_with(".apple.com"))
}
pub fn search(client: &Client, query: &str, country: &str) -> Result<Vec<Track>, String> {
    if !crate::library::valid_country(country) {
        return Err("Use a two-letter country code.".into());
    }
    let response = client
        .get("https://itunes.apple.com/search")
        .query(&[
            ("term", query),
            ("country", country),
            ("entity", "song"),
            ("limit", "100"),
        ])
        .send()
        .map_err(|_| "Catalog unavailable. Check your connection and retry.")?;
    let response: SearchResponse = serde_json::from_slice(&limited_bytes(response, 4_000_000)?)
        .map_err(|_| "The catalog response could not be read.")?;
    parse_results(response)
}

fn parse_results(response: SearchResponse) -> Result<Vec<Track>, String> {
    Ok(response
        .results
        .into_iter()
        .filter_map(|t| {
            let source = t.preview_url?;
            if !is_preview_url(&url::Url::parse(&source).ok()?) {
                return None;
            }
            Some(Track {
                id: format!("apple:{}", t.track_id?),
                title: t.track_name?,
                artist: t.artist_name.unwrap_or_default(),
                album: t.collection_name.unwrap_or_default(),
                source: MediaSource::PreviewUrl(source),
                artwork: t
                    .artwork_url100
                    .map(|u| u.replace("100x100bb", "300x300bb")),
                duration: None,
                album_id: t.collection_id.map(|id| id.to_string()),
            })
        })
        .collect())
}

pub fn album(client: &Client, id: &str, country: &str) -> Result<Vec<Track>, String> {
    if id.is_empty()
        || !id.bytes().all(|b| b.is_ascii_digit())
        || !crate::library::valid_country(country)
    {
        return Err("Invalid album or storefront".into());
    }
    let response = client
        .get("https://itunes.apple.com/lookup")
        .query(&[
            ("id", id),
            ("country", country),
            ("entity", "song"),
            ("limit", "200"),
        ])
        .send()
        .map_err(|_| "Album unavailable. Check your connection.")?;
    let response: SearchResponse = serde_json::from_slice(&limited_bytes(response, 4_000_000)?)
        .map_err(|_| "Album response could not be read")?;
    parse_results(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_previews_have_namespaced_ids_and_no_invented_playback_duration() {
        let response: SearchResponse=serde_json::from_str(r#"{"results":[{"trackId":7,"trackName":"One","previewUrl":"https://audio-ssl.itunes.apple.com/a","collectionId":9},{"trackId":8,"trackName":"Two","previewUrl":"https://evil.test/a"}]}"#).unwrap();
        let tracks = parse_results(response).unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].id, "apple:7");
        assert!(tracks[0].duration.is_none());
    }
    #[test]
    fn accepts_only_https_apple_preview_hosts() {
        for u in [
            "https://audio-ssl.itunes.apple.com/song.m4a",
            "https://audio-ssl.itunes.apple.com.evil.test/x",
            "http://audio-ssl.itunes.apple.com/x",
            "https://evil.test/x",
            "file:///C:/test",
            "https://user@audio-ssl.itunes.apple.com/x",
        ] {
            assert_eq!(
                is_preview_url(&url::Url::parse(u).unwrap()),
                u == "https://audio-ssl.itunes.apple.com/song.m4a",
                "{u}"
            );
        }
    }
}
