# Cadence

## Product goal

A lightweight Windows music application retaining the user's required Apple Music functionality. The application UI and application backend must be Rust with no embedded browser. Full Apple Music functionality remains the target; native subscription integration is unresolved.

## Required capabilities — not yet achieved

Apple account access and cloud library; full subscriber playback; verified advanced playback parity; Sound Enhancer and level; Sound Check; lossless and Hi-Res streaming/download settings; Dolby Atmos; video playback/download quality; standard queue and transport controls.

## Current implementation

Native Rust/egui interface with Symphonia/Rodio decoding, CPAL/WASAPI output, transactional SQLite persistence, local playlists, catalog previews, and Windows media-control integration. The subscription backend is unresolved. The separate native-backend feasibility prototype has reached factory/object activation but fails packaged service startup.

## Engineering constraints

Do not fabricate account state, substitute chart results for a user's library, equate preview playback with subscriber playback, or present unsupported settings as functioning controls. Compare total memory/CPU of all required processes only after equivalent playback functionality works. Further native-initialization research must isolate application data from the normal Apple installation.
