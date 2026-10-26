use crate::model::Track;
use std::sync::mpsc::{self, Receiver};
use windows::{
    Foundation::TypedEventHandler,
    Media::{
        MediaPlaybackStatus, MediaPlaybackType, SystemMediaTransportControls,
        SystemMediaTransportControlsButton, SystemMediaTransportControlsButtonPressedEventArgs,
    },
    Win32::{Foundation::HWND, System::WinRT::ISystemMediaTransportControlsInterop},
    core::{HSTRING, factory},
};

pub enum MediaKey {
    Play,
    Pause,
    Next,
    Previous,
    Stop,
}
pub struct NativeControls {
    controls: SystemMediaTransportControls,
    token: i64,
    pub events: Receiver<MediaKey>,
    last: Option<(String, bool)>,
}
impl NativeControls {
    pub fn new(
        hwnd: isize,
        repaint: impl Fn() + Send + Sync + 'static,
    ) -> windows::core::Result<Self> {
        let interop: ISystemMediaTransportControlsInterop =
            factory::<SystemMediaTransportControls, ISystemMediaTransportControlsInterop>()?;
        // The window belongs to the calling UI thread and outlives this adapter.
        let controls: SystemMediaTransportControls =
            unsafe { interop.GetForWindow(HWND(hwnd as *mut _))? };
        controls.SetIsPlayEnabled(true)?;
        controls.SetIsPauseEnabled(true)?;
        controls.SetIsNextEnabled(true)?;
        controls.SetIsPreviousEnabled(true)?;
        controls.SetIsStopEnabled(true)?;
        let (tx, events) = mpsc::channel();
        let token = controls.ButtonPressed(&TypedEventHandler::<
            SystemMediaTransportControls,
            SystemMediaTransportControlsButtonPressedEventArgs,
        >::new(move |_, args| {
            if let Some(args) = args.as_ref() {
                let event = match args.Button()? {
                    SystemMediaTransportControlsButton::Play => Some(MediaKey::Play),
                    SystemMediaTransportControlsButton::Pause => Some(MediaKey::Pause),
                    SystemMediaTransportControlsButton::Next => Some(MediaKey::Next),
                    SystemMediaTransportControlsButton::Previous => Some(MediaKey::Previous),
                    SystemMediaTransportControlsButton::Stop => Some(MediaKey::Stop),
                    _ => None,
                };
                if let Some(event) = event {
                    let _ = tx.send(event);
                    repaint();
                }
            }
            Ok(())
        }))?;
        Ok(Self {
            controls,
            token,
            events,
            last: None,
        })
    }
    pub fn update(&mut self, track: Option<&Track>, playing: bool) -> windows::core::Result<()> {
        let next = track.map(|t| (t.id.clone(), playing));
        if self.last == next {
            return Ok(());
        }
        self.controls.SetIsEnabled(track.is_some())?;
        self.controls.SetPlaybackStatus(if playing {
            MediaPlaybackStatus::Playing
        } else if track.is_some() {
            MediaPlaybackStatus::Paused
        } else {
            MediaPlaybackStatus::Stopped
        })?;
        if let Some(track) = track {
            let display = self.controls.DisplayUpdater()?;
            display.SetType(MediaPlaybackType::Music)?;
            let music = display.MusicProperties()?;
            music.SetTitle(&HSTRING::from(&track.title))?;
            music.SetArtist(&HSTRING::from(&track.artist))?;
            music.SetAlbumTitle(&HSTRING::from(&track.album))?;
            display.Update()?;
        }
        self.last = next;
        Ok(())
    }
}
impl Drop for NativeControls {
    fn drop(&mut self) {
        let _ = self.controls.RemoveButtonPressed(self.token);
        let _ = self.controls.SetIsEnabled(false);
    }
}
