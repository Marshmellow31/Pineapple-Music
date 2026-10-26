use cadence::{
    audio::{Audio, Command, Event},
    catalog,
    model::MediaSource,
};
use std::{
    sync::mpsc::RecvTimeoutError,
    time::{Duration, Instant},
};

pub fn run() -> Option<Result<String, String>> {
    let args: Vec<_> = std::env::args().collect();
    let mode = args.iter().find(|a| {
        matches!(
            a.as_str(),
            "--self-test-audio" | "--self-test-catalog" | "--self-test-preview"
        )
    })?;
    let result=match mode.as_str() {
        "--self-test-audio"=>audio_check(),
        "--self-test-catalog"=>catalog::client().and_then(|client|catalog::search(&client,"Daft Punk","US")).and_then(|tracks|{
            if tracks.is_empty(){Err("Catalog returned no preview tracks.".into())}
            else{Ok(serde_json::json!({"passed":true,"scope":"live catalog metadata and preview URLs","tracks":tracks.len()}))}
        }),
        _=>preview_check(),
    };
    let report = match &result {
        Ok(v) => v.clone(),
        Err(e) => serde_json::json!({"passed":false,"error":e}),
    };
    let output = args
        .iter()
        .position(|a| a == "--report")
        .and_then(|i| args.get(i + 1))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from("evidence/rebuild")
                .join(format!("{}.json", mode.trim_start_matches("--")))
        });
    if let Some(parent) = output.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        return Some(Err(e.to_string()));
    }
    if let Err(e) = std::fs::write(
        output,
        serde_json::to_vec_pretty(&report).unwrap_or_default(),
    ) {
        return Some(Err(e.to_string()));
    }
    Some(result.map(|value| value.to_string()))
}

fn wait_for(
    audio: &Audio,
    deadline: Instant,
    mut predicate: impl FnMut(Event) -> Result<bool, String>,
) -> Result<(), String> {
    while Instant::now() < deadline {
        match audio.events.recv_timeout(Duration::from_millis(250)) {
            Ok(event) => {
                if predicate(event)? {
                    return Ok(());
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(_) => return Err("Audio worker disconnected.".into()),
        }
    }
    Err("Audio diagnostic timed out.".into())
}
fn audio_check() -> Result<serde_json::Value, String> {
    let generated_path = std::env::temp_dir().join(format!(
        "cadence-diagnostic-{}-{}.wav",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos()
    ));
    let args: Vec<_> = std::env::args_os().collect();
    let supplied = args
        .iter()
        .position(|a| a == "--file")
        .and_then(|i| args.get(i + 1));
    let path = supplied
        .map(std::path::PathBuf::from)
        .unwrap_or(generated_path);
    if supplied.is_none() {
        std::fs::write(&path, wav_fixture(3.)).map_err(|e| e.to_string())?;
    }
    let imported = cadence::services::import_file(&path)?;
    let result = (|| {
        let audio = Audio::start();
        let mut output = String::new();
        wait_for(
            &audio,
            Instant::now() + Duration::from_secs(8),
            |event| match event {
                Event::Ready {
                    sample_rate,
                    channels,
                } => {
                    output = format!("{sample_rate} Hz / {channels} channels");
                    Ok(true)
                }
                Event::Failed { message, .. } => Err(message),
                _ => Ok(false),
            },
        )?;
        audio.send(Command::Volume(0.))?;
        audio.send(Command::Play {
            generation: 10,
            source: MediaSource::LocalFile(path.clone()),
            fade: 0.,
        })?;
        wait_for(
            &audio,
            Instant::now() + Duration::from_secs(8),
            |event| match event {
                Event::State {
                    generation: 10,
                    position,
                    playing: true,
                } => Ok(position > 0.2),
                Event::Failed { message, .. } => Err(message),
                _ => Ok(false),
            },
        )?;
        audio.send(Command::Toggle)?;
        wait_for(&audio, Instant::now() + Duration::from_secs(3), |event| {
            Ok(matches!(
                event,
                Event::State {
                    generation: 10,
                    playing: false,
                    ..
                }
            ))
        })?;
        audio.send(Command::Seek(1.5))?;
        wait_for(
            &audio,
            Instant::now() + Duration::from_secs(3),
            |event| match event {
                Event::State {
                    generation: 10,
                    position,
                    playing: false,
                } => Ok((position - 1.5).abs() < 0.2),
                Event::Failed { message, .. } => Err(message),
                _ => Ok(false),
            },
        )?;
        audio.send(Command::Toggle)?;
        wait_for(&audio, Instant::now() + Duration::from_secs(5), |event| {
            Ok(matches!(event, Event::Ended(10)))
        })?;
        audio.send(Command::Stop)?;
        Ok(
            serde_json::json!({"passed":true,"decoder":"Symphonia via Rodio","output":output,
            "imported_title":imported.title,"imported_artist":imported.artist,"format":path.extension().map(|e|e.to_string_lossy()),
            "checks":["local audio loaded","playback clock advanced","pause event","seek while paused","resume and end-of-track event"],
            "volume":0,"audible_output_verified":false,"apple_subscription_playback_verified":false}),
        )
    })();
    if supplied.is_none() {
        let _ = std::fs::remove_file(path);
    }
    result
}
fn preview_check() -> Result<serde_json::Value, String> {
    let tracks = catalog::search(&catalog::client()?, "Daft Punk", "US")?;
    let track = tracks.first().ok_or("No catalog previews returned.")?;
    let audio = Audio::start();
    audio.send(Command::Volume(0.))?;
    audio.send(Command::Play {
        generation: 5,
        source: track.source.clone(),
        fade: 0.,
    })?;
    let mut duration = None;
    wait_for(
        &audio,
        Instant::now() + Duration::from_secs(35),
        |event| match event {
            Event::Started { duration: d, .. } => {
                duration = d;
                Ok(false)
            }
            Event::State {
                generation: 5,
                playing: true,
                position,
            } => Ok(position > 0.3),
            Event::Failed { message, .. } => Err(message),
            _ => Ok(false),
        },
    )?;
    audio.send(Command::Stop)?;
    Ok(
        serde_json::json!({"passed":true,"scope":"live Apple preview decoded and playback clock advanced","track":track.title,
        "duration_seconds":duration,"volume":0,"audible_output_verified":false,"apple_subscription_playback_verified":false}),
    )
}
fn wav_fixture(seconds: f64) -> Vec<u8> {
    let rate = 44100u32;
    let length = (seconds * f64::from(rate)) as u32 * 2;
    let mut data = Vec::new();
    data.extend_from_slice(b"RIFF");
    data.extend_from_slice(&(36 + length).to_le_bytes());
    data.extend_from_slice(b"WAVEfmt ");
    data.extend_from_slice(&16u32.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&rate.to_le_bytes());
    data.extend_from_slice(&(rate * 2).to_le_bytes());
    data.extend_from_slice(&2u16.to_le_bytes());
    data.extend_from_slice(&16u16.to_le_bytes());
    data.extend_from_slice(b"data");
    data.extend_from_slice(&length.to_le_bytes());
    data.resize(data.len() + length as usize, 0);
    data
}
#[cfg(test)]
mod tests {
    use super::*;
    use rodio::Source;
    #[test]
    fn rust_decoder_recognizes_pcm_duration_and_can_seek() {
        let mut decoder = rodio::Decoder::try_from(std::io::Cursor::new(wav_fixture(2.))).unwrap();
        assert_eq!(decoder.channels(), 1);
        assert_eq!(decoder.sample_rate(), 44100);
        assert!((decoder.total_duration().unwrap().as_secs_f64() - 2.).abs() < 0.01);
        decoder.try_seek(Duration::from_millis(1000)).unwrap();
        assert!(decoder.next().is_some());
    }
}
