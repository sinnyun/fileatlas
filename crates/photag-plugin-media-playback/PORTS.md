# Media playback plugin ports

Status: FFmpeg-backed video and PCM audio playback, transport, volume, draggable timeline seeking, source speed controls, and aged frame-cache cleanup are implemented. Windows device and codec behavior still requires real Windows validation.

## Lifecycle and dependencies

- Cordis plugin: `photag.media-playback`; service identity: `photag/media-playback` (`MediaPlaybackService`).
- The plugin publishes its service and registers `media-playback/transport` plus the `media-playback/transport-view` GPUI view. Cordis stop removes its UI entries; dropping the service kills an active FFmpeg child.
- The crate is registered in the workspace and the shell starts `MediaPlaybackPlugin`. No other Cordis service is required. `MediaPlaybackConfig.cache_dir` sets frame cache; default is `%LOCALAPPDATA%/PhoTag/playback` (system temp when unavailable).
- Runtime executables: `ffmpeg` and `ffprobe`, searched beside the app, in `ffmpeg/bin`, then on `PATH`. Windows child processes are spawned without console windows.
- Rust dependencies: `cordis-rs` 0.11.1, `gpui-kit` 0.7.1, `photag-ui-bridge`, `image` 0.25 with PNG only, `serde_json` 1, `thiserror` 2, and `rodio` 0.19 with its native output backend.

## Inputs, outputs, and behavior

- Call `open(path)` to probe video dimensions, frame rate, duration, and rotation metadata, then begin decode. `snapshot()` returns the latest decoded frame path and current playhead state.
- `toggle_play_pause`, `seek`, `step_frames`, `set_speed`, and `set_volume` are the transport operations. Speed is clamped to 0.1–8×; changing speed snapshots the current playhead and restarts both decoders there. Volume preserves the source 0–1.5 range and updates the active rodio sink.
- Audio is decoded by an independent FFmpeg process to interleaved stereo PCM at 44.1 kHz, then streamed through a bounded channel to rodio on a dedicated worker thread. The sink playback clock drives the reported playhead while the audio source is active; elapsed wall time is the fallback when output is unavailable or the audio stream ends before video. Pause, seek, speed changes, EOF, close, and Cordis cleanup stop both FFmpeg children, the sink, and the worker.
- FFmpeg's automatic display-matrix rotation is retained. Probe rotation is also exposed as normalized clockwise quarter-turn metadata and displayed by the view. Odd quarter-turns swap the expected output dimensions.
- The GPUI surface provides play/pause, ±5 second seek, frame step, close, current time, and the most recently decoded frame. Its draggable playhead slider seeks on release, the volume slider controls the source 0–1.5 range, and speed controls expose the source's continuous 0.1×–2× slider at 0.05× steps and presets (0.25×, 0.5×, 0.75×, 1×, 1.25×, 1.5×, 2×), plus the source's 3×/4×/8× boosts.
- Service initialization removes only this player's `frame-<pid>-<index>.png` cache files whose modification time is at least seven days old. Unrelated files and newer frames are preserved; filesystem metadata and removal errors are skipped.
- Playback surfaces use `photag_ui_bridge::theme` shared text, base, elevated, and active accent tokens.

## Events, consumers, and limits

- No Cordis events are currently produced. Gallery obtains `MediaPlaybackService`, calls `open` for selected video media, and closes it when returning to the gallery. Audio-only media is not supported by this FFmpeg video-frame service and should be handed to the operating system's default player by the caller. Shell startup owns plugin registration; Windows decoder/runtime behavior remains to be verified.
- Source behavior uses independent FFmpeg video and PCM streams with rodio audio resampling and sink timing. Device startup failure falls back to silent playback. Audio codec availability, actual device synchronization, seek/drag behavior, volume output, and cache cleanup behavior require Windows runtime validation. FFmpeg availability and codec support vary by installation; video probe/decode failures are returned by `open` as `PlaybackError`.
- To inspect failures, check FFmpeg/FFprobe process availability and path first, then inspect `PlaybackError` at the service caller. Decoder output currently suppresses FFmpeg stderr, so codec-specific diagnostics are limited.
