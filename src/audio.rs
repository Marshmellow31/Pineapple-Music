use crate::{catalog, model::MediaSource};
use rodio::{Decoder, OutputStreamBuilder, Sink, Source};
use std::{
    fs::File,
    io::Cursor,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread,
    time::{Duration, Instant},
};

pub enum Command {
    Play {
        generation: u64,
        source: MediaSource,
        fade: f32,
    },
    Toggle,
    Seek(f64),
    Volume(f32),
    Stop,
    Shutdown,
}
#[derive(Debug)]
pub enum Event {
    Ready {
        sample_rate: u32,
        channels: u16,
    },
    Started {
        generation: u64,
        duration: Option<f64>,
    },
    State {
        generation: u64,
        position: f64,
        playing: bool,
    },
    Ended(u64),
    Failed {
        generation: u64,
        message: String,
    },
}
type AudioSource = Box<dyn Source<Item = f32> + Send>;
struct Loaded {
    generation: u64,
    source: Result<AudioSource, String>,
    fade: f32,
}

pub struct Audio {
    sender: SyncSender<Command>,
    pub events: Receiver<Event>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Default for Audio {
    fn default() -> Self {
        Self::start()
    }
}
impl Audio {
    pub fn start() -> Self {
        let (sender, commands) = mpsc::sync_channel(32);
        let (tx, events) = mpsc::channel();
        let worker=thread::Builder::new().name("cadence-audio".into()).spawn(move|| {
            let mut stream=match OutputStreamBuilder::open_default_stream() {
                Ok(s)=>s,
                Err(_)=>{let _=tx.send(Event::Failed{generation:0,message:"Audio output is unavailable. Connect an output device, then use Reconnect audio in Settings.".into()});return;}
            };
            stream.log_on_drop(false);
            let _=tx.send(Event::Ready{sample_rate:stream.config().sample_rate(),channels:stream.config().channel_count()});
            let (load_tx,load_rx)=mpsc::sync_channel::<(u64,MediaSource,f32)>(1);
            let (loaded_tx,loaded_rx)=mpsc::sync_channel(1);
            let latest=Arc::new(AtomicU64::new(0));let latest_load=latest.clone();
            thread::spawn(move||{
                let client=catalog::client();
                while let Ok((generation,source,fade))=load_rx.recv() {
                    if generation!=latest_load.load(Ordering::Relaxed){continue;}
                    let source=load_source(&source,client.as_ref().map_err(Clone::clone));
                    if generation==latest_load.load(Ordering::Relaxed) && loaded_tx.send(Loaded{generation,source,fade}).is_err(){break;}
                }
            });
            let mut sink:Option<Sink>=None;let mut outgoing:Option<Sink>=None;
            let mut generation=0;let mut active_generation=0;let mut volume=0.65;let mut fade_elapsed=0.0;let mut fade_duration=0.0;
            let mut last_tick=Instant::now();let mut last_event=Instant::now();let mut ended=false;
            loop {
                match commands.recv_timeout(Duration::from_millis(25)) {
                    Ok(Command::Shutdown)|Err(mpsc::RecvTimeoutError::Disconnected)=>break,
                    Ok(Command::Play{generation:g,source,fade})=>{
                        latest.store(g,Ordering::Relaxed);generation=g;ended=false;
                        if load_tx.try_send((g,source,fade)).is_err(){
                            if let Some(s)=sink.take(){s.stop();}
                        if let Some(s)=outgoing.take(){s.stop();}
                            let _=tx.send(Event::Failed{generation:g,message:"Audio is still loading. Try playing the track again.".into()});
                        }
                    }
                    Ok(Command::Toggle)=>{if let Some(s)=&sink {
                        if s.is_paused(){s.play();if let Some(old)=&outgoing{old.play();}}
                        else{s.pause();if let Some(old)=&outgoing{old.pause();}}
                    }}
                    Ok(Command::Seek(seconds))=>{
                        if let Some(old)=outgoing.take(){old.stop();}
                        fade_duration=0.0;
                        if let Some(s)=&sink {s.set_volume(volume);
                            if let Err(e)=s.try_seek(Duration::from_secs_f64(seconds.max(0.0))) {
                                let _=tx.send(Event::Failed{generation,message:format!("This source could not seek: {e}")});
                            }
                        }
                    }
                    Ok(Command::Volume(v))=>{volume=if v.is_finite(){v.clamp(0.0,1.0)}else{0.65};if outgoing.is_none() && let Some(s)=&sink{s.set_volume(volume);}}
                    Ok(Command::Stop)=>{latest.fetch_add(1,Ordering::Relaxed);if let Some(s)=sink.take(){s.stop();}
                        if let Some(s)=outgoing.take(){s.stop();}ended=true;}
                    Err(mpsc::RecvTimeoutError::Timeout)=>{}
                }
                if let Ok(loaded)=loaded_rx.try_recv()
                    && loaded.generation==latest.load(Ordering::Relaxed) {
                    match loaded.source {
                        Ok(source)=>{
                            active_generation=loaded.generation;
                            let duration=source.total_duration().map(|d|d.as_secs_f64());
                            if let Some(old)=outgoing.take(){old.stop();}
                            fade_duration=loaded.fade.clamp(0.0,12.0);fade_elapsed=0.0;
                            if fade_duration>0.0 {outgoing=sink.take();}
                            else if let Some(old)=sink.take(){old.stop();}
                            let next=Sink::connect_new(stream.mixer());
                            next.set_volume(if outgoing.is_some(){0.0}else{volume});
                            next.append(source);sink=Some(next);ended=false;
                            let _=tx.send(Event::Started{generation:loaded.generation,duration});
                        }
                        Err(message)=>{
                            if let Some(old)=sink.take(){old.stop();}
                        if let Some(old)=outgoing.take(){old.stop();}
                            let _=tx.send(Event::Failed{generation:loaded.generation,message});ended=true;
                        }
                    }
                }
                let now=Instant::now();let dt=now.duration_since(last_tick).as_secs_f32();last_tick=now;
                if let Some(s)=&sink {
                    if let Some(old)=&outgoing {
                        if !s.is_paused(){fade_elapsed+=dt;}
                        let (a,b)=crossfade_gains(fade_elapsed,fade_duration);
                        old.set_volume(volume*a);s.set_volume(volume*b);
                        if fade_elapsed>=fade_duration {old.stop();outgoing=None;s.set_volume(volume);}
                    }
                    if s.empty() && !ended && active_generation==latest.load(Ordering::Relaxed) {ended=true;let _=tx.send(Event::Ended(active_generation));}
                    if last_event.elapsed()>Duration::from_millis(100) {
                        let _=tx.send(Event::State{generation:active_generation,position:s.get_pos().as_secs_f64(),playing:!s.is_paused() && !s.empty()});
                        last_event=Instant::now();
                    }
                }
            }
        }).expect("Could not create the audio worker");
        Self {
            sender,
            events,
            worker: Some(worker),
        }
    }
    pub fn send(&self, command: Command) -> Result<(), String> {
        self.sender.try_send(command).map_err(|_| {
            "Audio engine is unavailable or busy. Use Reconnect audio in Settings.".into()
        })
    }
}
impl Drop for Audio {
    fn drop(&mut self) {
        let _ = self.sender.send(Command::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn load_source(
    source: &MediaSource,
    client: Result<&reqwest::blocking::Client, String>,
) -> Result<AudioSource, String> {
    match source {
        MediaSource::LocalFile(path)=>{
            let file=File::open(path).map_err(|_|"Audio file is missing or cannot be read. Re-import the file from its current location.".to_owned())?;
            Decoder::try_from(file).map(|s|Box::new(s) as AudioSource).map_err(|_|"This audio file is damaged or its codec is unsupported.".into())
        }
        MediaSource::PreviewUrl(raw)=>{
            let url=url::Url::parse(raw).map_err(|_|"Invalid preview URL")?;
            if !catalog::is_preview_url(&url){return Err("Only trusted Apple preview sources are accepted.".into());}
            let response=client?.get(url).send().map_err(|_|"Preview could not load. Check your connection and retry.".to_owned())?;
            let bytes=catalog::limited_bytes(response,32_000_000)?;
            Decoder::try_from(Cursor::new(bytes)).map(|s|Box::new(s) as AudioSource).map_err(|_|"The preview format could not be decoded.".into())
        }
        MediaSource::AppleCatalogItem(_)=>Err("Full Apple Music playback requires a native subscription provider, which is unavailable in this build.".into()),
    }
}

pub fn crossfade_gains(elapsed: f32, duration: f32) -> (f32, f32) {
    let t = if duration > 0.0 {
        (elapsed / duration).clamp(0.0, 1.0)
    } else {
        1.0
    };
    ((1.0 - t).sqrt(), t.sqrt())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equal_power_crossfade_has_correct_endpoints_and_energy() {
        assert_eq!(crossfade_gains(0.0, 4.0), (1.0, 0.0));
        assert_eq!(crossfade_gains(4.0, 4.0), (0.0, 1.0));
        for i in 0..=40 {
            let (a, b) = crossfade_gains(i as f32 / 10.0, 4.0);
            assert!((a * a + b * b - 1.0).abs() < 0.00001);
        }
    }
    #[test]
    fn protected_catalog_items_never_fall_back_to_previews() {
        assert!(
            load_source(
                &MediaSource::AppleCatalogItem("123".into()),
                Err("unused".into())
            )
            .is_err()
        );
    }
}
