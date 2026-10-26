# Pineapple Music

Pineapple Music is the public repository for **Cadence**, a native Rust Windows music player with an Apple Music-style interface. This rebuild plays local audio and public Apple catalog previews. **Native Apple authorization and subscription playback remain unresolved. This is an interim rebuild, not a complete Apple Music replacement.**

## Run

Build from source (the local dist package is not included in Git and no public binary release has been published):

    cargo run --release --locked

1. Select **Import audio…** or press **Ctrl+O**. Files stay in their original folders.
2. Browse imported audio through **Songs**, **Artists**, or **Albums**.
3. Open **Search**, enter a song/artist/album, and press Enter. These results play **previews**.
4. Use **More** beside a track to save a favorite, add it to the queue, or add it to a local playlist.
5. Open **Playlists** to create a collection. Open one to rename it or remove tracks.
6. Use the queue icon for shuffle, repeat, removal, and track selection.
7. Use the MiniPlayer icon to open a separate window sharing the same playback state.
8. In **Settings**, change appearance, apply a two-letter storefront code, adjust crossfade, or reconnect audio after output failure.

Shortcuts: Space = play/pause; Ctrl+Left/Right = previous/next; Ctrl+F = search; Ctrl+, = settings.

## Implementation

- Removed wry, WebView2, injected website JavaScript, the disconnected login window, and browser-session flags.
- Replaced Windows MediaPlayer playback with Rust Symphonia/Rodio decoding and mixing over CPAL/WASAPI.
- Replaced JSON overwrite saves with SQLite transactions on a service worker.
- Extracted models, queue, services, audio, Windows controls, and UI into modules.
- Added request/playback generations to reject stale asynchronous results.
- Moved artwork HTTP and image decoding off the UI thread; the texture cache tracks at most 24 MiB of RGBA texture data.
- Rebuilt desktop navigation, library views, album lookup, local playlists, queue, themes, settings, and MiniPlayer.

SQLite and Windows APIs are external components. “Rust application” describes our application code, not every dependency or operating-system implementation.

## Data and migration

Data uses the Windows application-data directory for organization Harshil, application Cadence, normally %LOCALAPPDATA%\Harshil\Cadence\data.

First launch imports local files, favorites, history, and appearance from an existing library.json into library.sqlite3. The original JSON remains untouched. Legacy credentials and cloud caches are not imported as an authorized account. Closing waits for acknowledgment of the latest snapshot. Subsequent saves use the database; WAL/SHM companions may exist while running.

For an isolated library:

    .\target\debug\cadence.exe --data-dir "C:\Temp\Cadence-Test"

CADENCE_DATA_DIR is also supported. Close the app before backing up or restoring its database and companions. Keep original audio files in place.

## Current limits

Apple account access, validated cloud synchronization, full subscription audio, radio, Apple lyrics, protected downloads, subscription lossless/Hi-Res, Dolby Atmos, video, Sound Check, and Sound Enhancer are **unavailable**. There is no substitute login.

Local WAV, FLAC, MP3, and ALAC checks and public AAC preview checks are separate from subscription playback. A local ALAC decoder and a sample-rate label do not establish bit-perfect output or Apple lossless streaming. Crossfade uses equal-power overlap; preview loading can delay transitions. Gapless playback is not guaranteed across all formats.

Optional charts require your own CADENCE_APPLE_DEVELOPER_TOKEN environment variable. The ignored apple_developer_token.txt is not read by the application. A developer token does not authorize subscription playback.

The native-backend-lab directory and raw native research remain local-only and are excluded from this repository. A historical feasibility summary is published under evidence/native-feasibility/RESULTS.md. No private Apple binaries are bundled.

## Verification

    cargo fmt --check
    cargo test --offline --locked
    cargo clippy --all-targets --offline --locked -- -D warnings
    cargo build --release --offline --locked
    .\target\debug\cadence.exe --self-test-audio
    .\target\debug\cadence.exe --self-test-catalog
    .\target\debug\cadence.exe --self-test-preview

Audio diagnostics use zero volume; clock progress is not proof of audible output. Live catalog/preview checks require a connection. Reports go under evidence/rebuild; --report selects another path. --self-test-audio --file checks a supplied short audio fixture without deleting it.

Reproducible native-render captures:

    .\target\debug\cadence.exe --data-dir "C:\Temp\Cadence-Test" --search "Daft Punk" --capture "C:\Temp\search.png"
    .\target\debug\cadence.exe --data-dir "C:\Temp\Cadence-Test" --page settings --width 860 --height 600 --capture "C:\Temp\settings.png"
    .\target\debug\cadence.exe --data-dir "C:\Temp\Cadence-Test" --capture-mini "C:\Temp\mini.png"

Captures wait for the request to settle, render from the application, then close it. --light, --scale, and initial pages home/search/songs/albums/settings/account support layout checks.

See evidence/rebuild/IMPLEMENTATION.md for the implementation boundary and evidence/rebuild/VERIFICATION.md for performed checks. Historical integration evidence remains under evidence/native-feasibility.

## Repository status

Public repository: [Marshmellow31/Pineapple-Music](https://github.com/Marshmellow31/Pineapple-Music). This initial source snapshot retains the existing cadence package name, executable name, version 0.1.0, and app ID in.harshil.cadence. Further native Apple integration work remains in progress.
