# Media playback plugin ports

Status: FFmpeg-backed video decode and transport controls are implemented. PhoTag audio output and full timeline interactions are not yet migrated.

## Lifecycle and dependencies

- Cordis plugin: `photag.media-playback`; service identity: `photag/media-playback` (`MediaPlaybackService`).
- The plugin publishes its service and registers `media-playback/transport` plus the `media-playback/transport-view` GPUI view. Cordis stop removes its UI entries; dropping the service kills an active FFmpeg child.
- The crate is registered in the workspace and the shell starts `MediaPlaybackPlugin`. No other Cordis service is required. `MediaPlaybackConfig.cache_dir` sets frame cache; default is `%LOCALAPPDATA%/PhoTag/playback` (system temp when unavailable).
- Runtime executables: `ffmpeg` and `ffprobe`, searched beside the app, in `ffmpeg/bin`, then on `PATH`. Windows child processes are spawned without console windows.
- Rust dependencies: `cordis-rs` 0.11.1, `gpui-kit` 0.7.1, `photag-ui-bridge`, `image` 0.25 with PNG only, `serde_json` 1, and `thiserror` 2. No audio device dependency is included.

## Inputs, outputs, and behavior

- Call `open(path)` to probe video dimensions, frame rate, duration, and rotation metadata, then begin decode. `snapshot()` returns the latest decoded frame path and current playhead state.
- `toggle_play_pause`, `seek`, `step_frames`, and `set_speed` are the transport operations. Speed is clamped to 0.1–8×; changing speed snapshots the current playhead and restarts decode there. `set_volume` preserves the source 0–1.5 volume range in state but cannot currently affect output because audio is unavailable. `close` and service drop stop the decoder process.
- FFmpeg's automatic display-matrix rotation is retained. Probe rotation is also exposed as normalized clockwise quarter-turn metadata and displayed by the view. Odd quarter-turns swap the expected output dimensions.
- The GPUI surface provides play/pause, ±5 second seek, frame step, 0.5×/1×/2× speed, close, current time, and the most recently decoded frame.

## Events, consumers, and limits

- No Cordis events are currently produced. Gallery obtains `MediaPlaybackService`, calls `open` for selected video media, and closes it when returning to the gallery. Shell startup owns plugin registration; Windows decoder/runtime behavior remains to be verified.
- Source PhoTag keeps video and audio synchronized using independent FFmpeg video/PCM streams and a rodio sink; this port currently decodes video only. Audio codec availability, device selection, PCM synchronization, and volume output are therefore not migrated. The `audio_available` field remains false.
- This view currently does not implement a draggable seek bar, source speed popup/presets, boost rates, or cleanup of aged cached PNG frames. FFmpeg availability and codec support vary by installation; failures are returned by `open` as `PlaybackError`.
- To inspect failures, check FFmpeg/FFprobe process availability and path first, then inspect `PlaybackError` at the service caller. Decoder output currently suppresses FFmpeg stderr, so codec-specific diagnostics are limited.
