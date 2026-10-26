# Native backend feasibility result — 25 October 2026

**Result: partial activation success; full backend feasibility is not established.**

This is a historical summary of local research. Raw probe reports, generated bindings, staged Apple files, and the research worker remain local-only; artifact filenames below refer to that local investigation.

The user approved registering and running a separate initialization-only test
package. That test was performed. Its registration was subsequently removed.
No working Apple Music subscriber playback or requested feature parity is claimed.

## Verified

| Gate | Result | Evidence |
|---|---|---|
| Unmodified service DLL loading | Pass with adjacent process-local WinUI activation manifest | `loader-regfree-adjacent.json` |
| Rust factory access | AccountViewModel, Player, ServiceProvider factories available | `final-factories.json` |
| Player object construction | Returned successfully; not proof of playback | `rust-objects.json` |
| Direct PlaybackSettings activation | Not available (`0x80040111`); metadata exposes settings through AppSettings | Factory reports and generated bindings |
| Unpackaged app-context initialization | Fails: `0x80073D54`, APPMODEL_ERROR_NO_PACKAGE | `rust-prepared-package-context.json`; Windows SDK `winerror.h` |
| Separate package registration and activation | Succeeded under `Harshil.Cadence.NativeBackendLab_rfk6rajqweb6w` | Registration observed and packaged report generated |
| Packaged app-context initialization | Crashed before returning; Windows records `0xC0000602` in combase.dll | `rust-packaged-context.json`, `packaged-context-errors.json` |
| Test package cleanup | Removed; package lookup returned zero registrations | Final cleanup check |
| Default build blocks context initialization | Pass: exits with a diagnostic before loading Apple DLLs | `default-context-guard.json` |

## What the investigation found

The original DLL-loading failure came from missing WinUI activation registration.
An exception-type trace identified a class-not-registered error, and static
inspection located a reference to
`Microsoft.UI.Xaml.Data.PropertyChangedEventArgs`. Staging its unchanged native
dependencies and placing a private activation manifest beside them resolved DLL
loading without global registration.

The next gate requires a package identity. Registering a separate package resolved
that prerequisite but did not produce a working application context: the worker
terminated during the initialization call. The packaged JSON's `complete: false`
is a crash checkpoint, not a successful result.

Static inspection also shows startup code referencing `DidMigrateFairPlayPrefKey`
and an iTunes-related migration path. This is evidence that app-context startup
can do more than create read-only interfaces. It does **not** prove that migration
occurred during these runs. No account-property getters, playback commands,
settings setters or library-enumeration APIs were explicitly called by the probe.
Internal side effects of Apple's initializer were not exhaustively verified.
Consequently further live initialization attempts were stopped, and the default
build now rejects the context phase before loading the native DLLs.

The diagnostic debugger attached only to the separate Cadence worker after its
module-loading stage. It retained exception codes and candidate module-relative
stack addresses, not dumps, account strings or tokens. Candidate stack addresses
are not a fully unwound call stack. No error checks or protected routines were
patched or bypassed.

## What remains unverified

Native sign-in, full subscription playback, crossfade, Sound Enhancer, Sound Check,
lossless/Hi-Res streaming and downloads, Dolby Atmos, video quality, session
persistence, portability and total RAM/CPU use remain unverified. Metadata getter
names and a constructed Player object establish none of those capabilities.

The current result does not justify replacing Cadence's playback backend with this
adapter. Further initialization research should use a disposable, isolated Windows
environment with separate application data. That isolation has not been created.
The existing Apple Music app remains the established implementation of the full
feature set.

## Artifacts and validation

- Source: `native-backend-lab/`, independent of the main application interface.
- `cargo build --release --offline` for the worker: passed.
- `cargo check` with package-launch/codegen/isolated-context features: passed.
- Factory-only release probe: passed its targeted factory checks; PlaybackSettings
  factory absence is explicitly reported.
- Default context guard: verified, with `module_loaded: false`.
- Prepared runtime: 173 staged shipped files were hash-verified.
- The installed `AMP.Services.dll` retained SHA-256
  `993c758adf6f4f68abe931a86d2647b69c3c8dd404f9717eb924b8d732c8c597`.
- The initial context-test archive is preserved separately. The current research
  archive contains the guarded worker and defaults to factory checks.

These are local research results, not a published application release. Branch:
`codex/native-music-prototype`; no commit, push, PR or merge was performed.
