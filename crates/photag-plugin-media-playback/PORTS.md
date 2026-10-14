# Media playback plugin ports

Status: FFmpeg-backed video playback and PCM audio output are implemented with a modal player, transport, volume, timeline seeking, speed controls, and aged frame-cache cleanup. S6 adds audio-only media using the existing FFmpeg-to-rodio PCM path and a neutral stage glyph; the reference PhoTag source only probes video streams and only starts audio alongside video, so audio-only playback is an explicit extension for this redesigned interface. Windows output-device, codec, timing, and interaction behavior still requires PhoTag MCP validation.

## Lifecycle and dependencies

- Cordis plugin: `photag.media-playback`; service identity: `photag/media-playback` (`MediaPlaybackService`).
- The plugin publishes its service and registers `media-playback/transport` plus the `media-playback/transport-view` GPUI view in `workspace.modal`. Cordis stop removes its UI entries; dropping the service kills an active FFmpeg child.
- The crate is registered in the workspace and the shell starts `MediaPlaybackPlugin`. No other Cordis service is required. `MediaPlaybackConfig.cache_dir` sets frame cache; default is `%LOCALAPPDATA%/PhoTag/playback` (system temp when unavailable).
- Runtime executables: `ffmpeg` and `ffprobe`, searched beside the app, in `ffmpeg/bin`, then on `PATH`. Windows child processes are spawned without console windows.
- Rust dependencies: `cordis-rs` 0.11.1, `gpui-kit` 0.7.1, `photag-ui-bridge`, `image` 0.25 with PNG only, `serde_json` 1, `thiserror` 2, and `rodio` 0.19 with its native output backend.

## Inputs, outputs, and behavior

- Call `open(path)` to probe stream type and duration, then begin playback; it remains a compatibility wrapper for `open_media(path, None)`. Library callers use `open_media(path, Some(media_id))`; `snapshot()` carries that optional indexed ID for Tags/More actions. Video files also probe dimensions, frame rate, and rotation and return the latest decoded frame path. Audio-only files set `PlaybackSnapshot::has_video` to false, retain their real container duration, and play PCM through the same rodio worker; width/height remain zero and the view does not draw a fabricated waveform.
- `toggle_play_pause`, `seek`, `step_frames`, `set_speed`, and `set_volume` are the transport operations. Speed is clamped to 0.1–8×; changing speed snapshots the current playhead and restarts both decoders there. Volume preserves the source 0–1.5 range and updates the active rodio sink.
- Audio is decoded by an independent FFmpeg process to interleaved stereo PCM at 44.1 kHz, then streamed through a bounded channel to rodio on a dedicated worker thread. The sink playback clock drives the reported playhead while the audio source is active; elapsed wall time is the fallback when output is unavailable or the audio stream ends before video. Audio-only EOF is bounded by the probed media duration. Pause, seek, speed changes, EOF, close, and Cordis cleanup stop the active FFmpeg children, sink, and worker.
- FFmpeg's automatic display-matrix rotation is retained. Probe rotation is also exposed as normalized clockwise quarter-turn metadata and displayed by the view. Odd quarter-turns swap the expected output dimensions.
- The GPUI surface is a `workspace.modal` overlay with a title/close bar, video frame or audio glyph, play/pause, timeline, ±5 second seek, video-only frame step, volume, and speed controls. While a video or audio item is previewed, Preview opens it through `MediaPlaybackService`; Preview routes Space to play/pause, Left/Right to seek −/+5 seconds, and Escape to close the media preview, matching the source video overlay. The draggable playhead slider seeks on release, the volume slider controls the source 0–1.5 range, and speed controls expose the source's continuous 0.1×–2× slider at 0.05× steps and presets (0.25×, 0.5×, 0.75×, 1×, 1.25×, 1.5×, 2×), plus the source's 3×/4×/8× boosts.
- Service initialization removes only this player's `frame-<pid>-<index>.png` cache files whose modification time is at least seven days old. Unrelated files and newer frames are preserved; filesystem metadata and removal errors are skipped.
- Playback surfaces use `photag_ui_bridge::theme` shared text, backdrop-aware base, audio-stage, elevated, and active accent tokens.

## Events, consumers, and limits

- No Cordis events are currently produced. Gallery and Preview obtain `MediaPlaybackService` and pass indexed IDs through `open_media`; direct callers may still use `open`. Playback optionally captures Tagging and FileOperations services at plugin start for indexed tag assignment and file reveal actions, without a dependency on Gallery internals. Shell stops Playback before those providers. Callers without indexed IDs retain playback controls while Tags/More reports the unavailable state. Windows decoder/runtime behavior remains to be verified.
- Source video behavior uses independent FFmpeg video and PCM streams with rodio audio resampling and sink timing. Audio-only stream classification and duration JSON parsing have unit coverage; actual device startup failure falls back to a silent elapsed-time transport. Audio codec availability, output-device synchronization, seek/drag behavior, audible volume/rate changes, and cache cleanup behavior require Windows runtime validation. FFmpeg availability and codec support vary by installation; probe failures return `PlaybackError::ProbeFailed` and Preview launches the source path in the system default app. Decoder process failures remain visible as `PlaybackError` in Preview.
- To inspect failures, check FFmpeg/FFprobe process availability and path first, then inspect `PlaybackError` at the service caller. Decoder output currently suppresses FFmpeg stderr, so codec-specific diagnostics are limited.

Playback UI now receives optional Cordis `TaggingService` and `FileOperationsService` references at registration. With an indexed `media_id`, Tags uses `TaggingService::for_media/tag_tree/assign/remove` and More uses `FileOperationsService::reveal_in_file_manager`; missing optional services or an unindexed path are shown as unavailable. `open_media(path, Option<i64>)` preserves identity in `PlaybackSnapshot`; `open(path)` remains compatible.

## Localization integration (2026-10-13)

Visible control labels consume the public UI Bridge translator supplied by
photag.i18n. Stable commands, source settings/schema values, media paths and
user-created tag data are unchanged. Missing translations fall back to English;
OS/backend errors and some generated diagnostics are not fully localized.
The real MCP-only validation coverage and remaining source layout differences
are recorded in docs/validation-report.md; screenshots do not prove pointer
interaction or animation parity.

## v5 design 01 presentation (2026-10-14)

Playback fills the same PhoTag window with an opaque stage, a slim filename header and fixed
bottom transport. Tags/More share UI Bridge surface state between buttons and MCP requests.
The view supports focused Escape, Space and Left/Right seeking through existing service
methods. Volume is bounded by the public service to [0,1.5]; MCP `set_volume` uses the same
method. The view only shows output-unavailable feedback during active playback; stopping at
the end no longer mislabels a stopped audio session as an unavailable device. Actual sound
quality/codec breadth/device coverage requires separate listening and Windows tests.

## 2026-10-14 current implementation and MCP evidence

Reference05 rendered with decoded320x180 H264 frames contained across the stage. PCM/H264 playback state, pause/seek2/speed1.25/volume0.4/Tags/More were exercised through same-process PhoTag MCP. Footer is fixed; speed/volume controls bounded. Sound listening, allcodecs, native file dialogs remain unverified.
