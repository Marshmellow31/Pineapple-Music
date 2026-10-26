# Native Rust rebuild verification — 26 October 2026

**Result: verified interim native build. Full Apple Music replacement remains unresolved.**

## Build and automated tests

| Check | Result |
|---|---|
| cargo fmt --check | Pass |
| cargo test --offline --locked | 16 passed, 0 failed, 1 explicitly ignored live charts test |
| cargo clippy --all-targets --offline --locked -- -D warnings | Pass |
| cargo build --release --offline --locked | Pass |
| Windows dependency graph search for wry/WebView2/Tauri/WebKit | No matches |

Tests cover stale playback completion, shuffle/repeat and removal edges, protected-source rejection, crossfade gains, migration/credential exclusion, corrupt legacy recovery, accepted-save drainage and acknowledgment of the latest snapshot at shutdown, and PCM duration/seeking.

The skipped test needs a developer token. The separately executed public catalog check is not a substitute for authenticated charts or user-library verification.

## Live local checks

| Scope | Result / evidence |
|---|---|
| WAV playback clock, pause, seek, resume, completion | Passed in debug and final release; self-test-audio.json, release-audio.json |
| FLAC metadata and transport | Passed with synthetic silent fixture; flac.json |
| MP3 metadata and transport | Passed with synthetic silent fixture; mp3.json |
| ALAC/M4A metadata and transport | Passed after correcting file-length-aware decoding; alac.json |
| Public catalog search | 100 preview tracks returned; self-test-catalog.json |
| Live Apple AAC preview | Decoded Get Lucky preview, playback clock advanced; debug and final release reports self-test-preview.json / release-preview.json |
| Native context guard | Rejected before loading Apple DLLs; native-context-guard.json |

All audio checks used **volume zero**. They establish decoding/transport and output-stream clock behavior, not audible speaker output, subscription entitlement, protected playback, or lossless/Atmos delivery. Observed WASAPI device configuration was 48,000 Hz / two channels.

The initial ALAC diagnostic failed. Inspection identified Decoder construction without file length; using the file-aware builder resolved the tested fixture. Only the current corrected result is marked passed.

Crossfade has an equal-power gain unit test and a dual-sink implementation. Audible transition quality, delay under network load, and format-specific gapless behavior were not exhaustively verified.

## Native rendering

Actual application-render PNGs are retained under .impeccable/review:

- home-desktop.png, search-desktop.png, home-light.png: 1240 × 820.
- settings-small.png: 860 × 600.
- scale-150.png and scale-200.png: 860 × 600 physical windows with 1.5 / 2.0 application zoom. These are application-zoom checks, not certification of every Windows DPI/device configuration.
- mini.png: 470 × 200 deferred native MiniPlayer, idle state.
- release-home.png: final optimized executable, isolated empty library.

First captures exposed background/contrast and column-alignment defects; corrected captures were inspected. Independent review then identified compact transport/content-fit and stale design documentation. A verdict pass scored all three requested fixes **resolved**. Its ship disposition covers those fixes only. The additional MiniPlayer capture shows unclipped idle controls; it does not prove hidden-window playback continuity or hardware interactions.

The MiniPlayer uses a deferred viewport and command channel into the same controller. Root event polling lives in App::logic, including hidden-window logic; the former immediate-window capture path was replaced after testing exposed its limitation.

No HTML/CSS detector was run because this is a native Rust surface.

## Resource observation

release-idle-sample.json records three one-second-spaced startup/idle samples from one isolated final-release process:

- Working set: 83,144,704 to 86,769,664 bytes, approximately 79.3–82.8 MiB.
- Private bytes: 79,155,200 to 82,767,872 bytes, approximately 75.5–78.9 MiB.
- No direct child process and no loaded WebView/private Apple backend module matched the inspected names.

This short startup sample is not a long-running benchmark or equivalent-functionality comparison with the original app. Shared OS-service overhead, playback workloads, GPU accounting, suspend/resume, and large libraries were not measured. No RAM-saving percentage or complete-parity claim is made.

## Remaining acceptance work

Native Apple authorization and session restoration; full subscriber tracks and cloud operations; subscription lossless/Hi-Res/Atmos, protected downloads/offline licenses, radio, lyrics, video, Sound Check, and Sound Enhancer remain unavailable. See IMPLEMENTATION.md for the native-provider gate.

Audible output, physical media-key presses, real device changes, suspend/resume, hidden-window queue interaction, clean-machine installation, and large-library workloads still need live validation.

## Recovery and distribution

The private pre-edit snapshot and WebView-removal checkpoint are outside the repository. Original user JSON was not overwritten; migrations were exercised using isolated fixtures. Historical research evidence is preserved. The obsolete dist build is archived before the new local package is written.

The executable retains version 0.1.0 and app identity in.harshil.cadence. Package SHA256SUMS.txt identifies the actual rebuilt executable. No signing, installer deployment, or public release is claimed.

This verification snapshot was recorded before initial repository publication on branch codex/native-music-prototype, when there were no commits or configured remote. For current source-publication state, see the repository history. No public binary release or PR/merge is claimed.
