# Native Rust rebuild: implementation boundary

Date: 26 October 2026.

## Architecture

One Rust package exposes a backend library and a small desktop executable. Modules were extracted within the package before introducing additional crates.

| Module | Responsibility |
|---|---|
| src/model.rs | Typed local/preview/protected sources; namespaced IDs; local library/settings/playlists |
| src/queue.rs | Selection, history, shuffle, repeat, removal, stale-generation checks |
| src/library.rs | SQLite transactions, version check, one-time legacy import, isolated data paths |
| src/services.rs | Bounded background tasks, metadata import, API requests, serialized writes, artwork worker/cache |
| src/catalog.rs | Public search and album lookup, response limits, source checks |
| src/apple_api.rs | Existing developer-token helpers adapted to typed sources; no native account authorization |
| src/audio.rs | Rodio/Symphonia decoding and mixing, CPAL/WASAPI, source preparation, transport, crossfade |
| src/platform.rs | Windows media-key callbacks and now-playing metadata through native-window SMTC |
| src/ui/ | Desktop UI, drawn icons, screens, transport, MiniPlayer, native capture |
| src/diagnostics.rs | Explicit muted playback and live catalog/preview diagnostics |

Removed auth.rs, player.rs, and store.rs. Original contents are in the private baseline snapshot. No wry/WebView2 dependency remains. raw-window-handle remains for native Windows media controls.

The audio actor/source loader keep HTTP and transport work off the UI thread. Rodio owns its decoding/audio pipeline; this does **not** establish an allocation-free or I/O-free real-time callback.

SQLite stores a versioned serialized local-library document in a transactional row. This gives atomic updates and reversible migration; it does not claim a normalized schema or unlimited-library scalability. Large-library cloning and serialization require measurements.

## Apple integration gate: unresolved

The full replacement target is not achieved.

- **Verified this session:** the guarded research binary rejects context initialization with module_loaded=false; see native-context-guard.json.
- **Historical:** separate package initialization failed with 0xC0000602; unpackaged initialization failed with 0x80073D54. See ../native-feasibility/RESULTS.md.
- **Verified from reviewed documentation:** MusicKit describes Apple-platform, Android, and web integrations. No documented Windows/Rust protected-playback contract was established.
- **Needs-confirmation:** usable native provider, real authorization, subscription verification, full tracks, session restoration, cloud operations, and dependent quality/download/video features.

Sources: [MusicKit](https://developer.apple.com/musickit/), [Apple Music API](https://developer.apple.com/documentation/applemusicapi), [Music User Tokens](https://developer.apple.com/documentation/applemusicapi/user-authentication-for-musickit).

No Apple credentials/cookies were collected. No private context was initialized, binary patched, package registered, or system WebView2 runtime removed during this rebuild.

## Remaining scope

1. Establish the native provider gate. Further private-backend initialization research requires a disposable isolated Windows environment, which has not been prepared.
2. Add secure session storage only when a provider supplies a genuine session.
3. Verify full subscription playback, cloud operations, recommendations, radio, lyrics, downloads/licenses, lossless/Hi-Res/Atmos, video, and Apple's processing features.
4. Validate hardware media-key interactions, audible output, suspend/resume, device changes, clean-machine installation, and large libraries.
5. Compare resources with the original app only after equivalent playback works.

Independent native functionality can be tested now. It does not establish these remaining capabilities.

## Recovery and publication

The starting repository had no commits and all source was untracked. A private source copy and hash inventory were made outside the repository before edits. A second checkpoint records the verified WebView-removal stage. Older dist output is preserved outside the shipping path before fresh packaging.

Build-time state before initial repository publication: branch codex/native-music-prototype; no commit, push, PR, merge, or binary release had been performed. Application version/identity remains cadence / 0.1.0 / in.harshil.cadence; artifact hashes distinguish this rebuild from older experiments.
