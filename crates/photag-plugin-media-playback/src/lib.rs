//! FFmpeg-backed video playback service and lifecycle-owned GPUI transport surface.
//!
//! Frames and timestamps are produced by one FFmpeg decode process. The source PhoTag
//! implementation used separate video and audio processes plus rodio; the service keeps
//! separate FFmpeg video and PCM audio decoders, with rodio output isolated to its owning
//! worker thread so the Cordis service remains Send + Sync.

mod ffmpeg;
mod view;

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use cordis::{Context, Plugin, Service};
use photag_ui_bridge::{
    ContributionKind, UiContribution, register_for_plugin, register_views_for_plugin,
};

pub use view::media_playback_view;

#[derive(Clone, Debug, PartialEq)]
pub struct PlaybackSnapshot {
    pub path: Option<PathBuf>,
    /// Indexed row identity for callers that opened playback from the library.
    pub media_id: Option<i64>,
    pub has_video: bool,
    pub frame_path: Option<PathBuf>,
    pub playing: bool,
    pub position_seconds: f64,
    pub duration_seconds: f64,
    pub frame_rate: f64,
    pub width: u32,
    pub height: u32,
    /// FFprobe rotation metadata normalized to clockwise quarter-turns.
    pub rotation_quarters: u8,
    pub speed: f32,
    pub volume: f32,
    pub audio_available: bool,
    pub error: Option<String>,
    pub revision: u64,
}

impl Default for PlaybackSnapshot {
    fn default() -> Self {
        Self {
            path: None,
            media_id: None,
            has_video: false,
            frame_path: None,
            playing: false,
            position_seconds: 0.0,
            duration_seconds: 0.0,
            frame_rate: 30.0,
            width: 0,
            height: 0,
            rotation_quarters: 0,
            speed: 1.0,
            volume: 1.0,
            audio_available: false,
            error: None,
            revision: 0,
        }
    }
}

struct PlaybackState {
    snapshot: PlaybackSnapshot,
    child: Option<Child>,
    audio: Option<AudioSession>,
    receiver: Option<mpsc::Receiver<PathBuf>>,
    started: Option<Instant>,
    start_position: f64,
    pause_after_frame: bool,
    cache: PathBuf,
}

/// Cordis-owned playback state and FFmpeg process lifecycle.
pub struct MediaPlaybackService {
    state: Mutex<PlaybackState>,
}
impl Service for MediaPlaybackService {
    const NAME: &'static str = "photag/media-playback";
}

impl MediaPlaybackService {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Result<Self, PlaybackError> {
        let cache = cache_dir.into();
        std::fs::create_dir_all(&cache)?;
        cleanup_aged_frames(&cache);
        Ok(Self {
            state: Mutex::new(PlaybackState {
                snapshot: PlaybackSnapshot::default(),
                child: None,
                audio: None,
                receiver: None,
                started: None,
                start_position: 0.0,
                pause_after_frame: false,
                cache,
            }),
        })
    }

    pub fn snapshot(&self) -> PlaybackSnapshot {
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(_) => return PlaybackSnapshot::default(),
        };
        if state.snapshot.playing {
            state.snapshot.position_seconds = current_position(&state);
        }
        let mut decoder_finished = false;
        let mut pause_frame_ready = false;
        while let Some(receiver) = &state.receiver {
            match receiver.try_recv() {
                Ok(frame) => {
                    state.snapshot.frame_path = Some(frame);
                    state.snapshot.revision = state.snapshot.revision.wrapping_add(1);
                    if state.pause_after_frame {
                        pause_frame_ready = true;
                        break;
                    }
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    decoder_finished = true;
                    break;
                }
            }
        }
        if pause_frame_ready {
            state.snapshot.position_seconds = state.start_position;
            state.pause_after_frame = false;
            stop(&mut state);
        }
        if decoder_finished {
            stop(&mut state);
            state.snapshot.position_seconds = state.snapshot.duration_seconds;
            state.snapshot.revision = state.snapshot.revision.wrapping_add(1);
        }
        if state.snapshot.playing
            && state.snapshot.duration_seconds > 0.0
            && current_position(&state) >= state.snapshot.duration_seconds
        {
            stop(&mut state);
            state.snapshot.position_seconds = state.snapshot.duration_seconds;
            state.snapshot.revision = state.snapshot.revision.wrapping_add(1);
        }
        state.snapshot.clone()
    }

    pub fn open(&self, path: impl AsRef<Path>) -> Result<(), PlaybackError> {
        self.open_media(path, None)
    }

    /// Open media while preserving its indexed identity for Tags/More actions.
    pub fn open_media(
        &self,
        path: impl AsRef<Path>,
        media_id: Option<i64>,
    ) -> Result<(), PlaybackError> {
        let path = path.as_ref().to_path_buf();
        if !path.is_file() {
            return Err(PlaybackError::MissingMedia(path));
        }
        let mut state = self.lock()?;
        stop(&mut state);
        state.pause_after_frame = false;
        let volume = state.snapshot.volume;
        let speed = state.snapshot.speed;
        let revision = state.snapshot.revision;
        state.snapshot = PlaybackSnapshot {
            volume,
            speed,
            revision,
            ..PlaybackSnapshot::default()
        };
        let info = match ffmpeg::probe(&path) {
            Ok(info) => info,
            Err(error) => {
                state.snapshot.revision = revision.wrapping_add(1);
                return Err(error);
            }
        };
        state.snapshot.path = Some(path);
        state.snapshot.media_id = media_id;
        state.snapshot.has_video = info.has_video;
        state.snapshot.frame_path = None;
        state.snapshot.duration_seconds = info.duration;
        state.snapshot.frame_rate = info.fps;
        state.snapshot.width = info.width;
        state.snapshot.height = info.height;
        state.snapshot.rotation_quarters = info.rotation_quarters;
        state.snapshot.position_seconds = 0.0;
        state.snapshot.error = None;
        start(&mut state, 0.0)?;
        state.snapshot.revision = state.snapshot.revision.wrapping_add(1);
        Ok(())
    }

    pub fn toggle_play_pause(&self) -> Result<(), PlaybackError> {
        let mut state = self.lock()?;
        if state.snapshot.path.is_none() {
            return Ok(());
        }
        if state.snapshot.playing {
            state.snapshot.position_seconds = current_position(&state);
            stop(&mut state);
            state.snapshot.playing = false;
        } else {
            let pos = if state.snapshot.position_seconds >= state.snapshot.duration_seconds {
                0.0
            } else {
                state.snapshot.position_seconds
            };
            state.pause_after_frame = false;
            start(&mut state, pos)?;
        }
        state.snapshot.revision = state.snapshot.revision.wrapping_add(1);
        Ok(())
    }

    pub fn seek(&self, seconds: f64) -> Result<(), PlaybackError> {
        let mut state = self.lock()?;
        let pos = seconds.clamp(0.0, state.snapshot.duration_seconds.max(0.0));
        let resume = state.snapshot.playing;
        stop(&mut state);
        state.pause_after_frame = false;
        state.snapshot.position_seconds = pos;
        if state.snapshot.path.is_some() {
            if state.snapshot.has_video && (resume || pos < state.snapshot.duration_seconds) {
                start(&mut state, pos)?;
                state.pause_after_frame = !resume;
            } else if !state.snapshot.has_video && resume {
                start(&mut state, pos)?;
            }
        }
        state.snapshot.revision = state.snapshot.revision.wrapping_add(1);
        Ok(())
    }

    pub fn step_frames(&self, frames: i32) -> Result<(), PlaybackError> {
        let s = self.snapshot();
        if !s.has_video {
            return Ok(());
        }
        self.seek(s.position_seconds + frames as f64 / s.frame_rate.max(1.0))?;
        if let Ok(mut state) = self.state.lock() {
            if state.snapshot.playing {
                state.pause_after_frame = true;
            }
        }
        Ok(())
    }

    pub fn set_speed(&self, speed: f32) -> Result<(), PlaybackError> {
        let mut state = self.lock()?;
        let value = speed.clamp(0.1, 8.0);
        if (value - state.snapshot.speed).abs() < 0.01 {
            return Ok(());
        }
        let position = current_position(&state);
        let resume = state.snapshot.playing;
        stop(&mut state);
        state.pause_after_frame = false;
        state.snapshot.speed = value;
        state.snapshot.position_seconds = position;
        if resume {
            start(&mut state, position)?;
        }
        state.snapshot.revision = state.snapshot.revision.wrapping_add(1);
        Ok(())
    }

    pub fn set_volume(&self, volume: f32) {
        if let Ok(mut state) = self.state.lock() {
            state.snapshot.volume = volume.clamp(0.0, 1.5);
            if let Some(audio) = &state.audio {
                let _ = audio
                    .commands
                    .send(AudioCommand::SetVolume(state.snapshot.volume));
            }
            state.snapshot.revision = state.snapshot.revision.wrapping_add(1);
        }
    }

    pub fn close(&self) {
        if let Ok(mut state) = self.state.lock() {
            stop(&mut state);
            state.pause_after_frame = false;
            let volume = state.snapshot.volume;
            let speed = state.snapshot.speed;
            state.snapshot = PlaybackSnapshot {
                revision: state.snapshot.revision.wrapping_add(1),
                volume,
                speed,
                ..PlaybackSnapshot::default()
            };
        }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, PlaybackState>, PlaybackError> {
        self.state.lock().map_err(|_| PlaybackError::StatePoisoned)
    }
}

fn current_position(state: &PlaybackState) -> f64 {
    if state.snapshot.playing {
        if let Some(audio) = &state.audio {
            let (sender, receiver) = mpsc::sync_channel(1);
            if audio.commands.send(AudioCommand::Position(sender)).is_ok()
                && let Ok(Some(position)) = receiver.recv_timeout(Duration::from_millis(100))
            {
                return (state.start_position + position).min(state.snapshot.duration_seconds);
            }
        }
        (state.start_position
            + state.started.map_or(0.0, |t| {
                t.elapsed().as_secs_f64() * state.snapshot.speed as f64
            }))
        .min(state.snapshot.duration_seconds)
    } else {
        state.snapshot.position_seconds
    }
}

fn stop(state: &mut PlaybackState) {
    state.snapshot.playing = false;
    state.started = None;
    state.receiver = None;
    if let Some(mut child) = state.child.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    if let Some(audio) = state.audio.take() {
        let _ = audio.commands.send(AudioCommand::Stop);
        let _ = audio.thread.join();
    }
    state.snapshot.audio_available = false;
}

enum AudioCommand {
    Position(mpsc::SyncSender<Option<f64>>),
    SetVolume(f32),
    Stop,
}

struct AudioSession {
    commands: mpsc::Sender<AudioCommand>,
    thread: std::thread::JoinHandle<()>,
}

struct ChannelAudioSource {
    rx: mpsc::Receiver<i16>,
}

impl Iterator for ChannelAudioSource {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        self.rx.recv().ok()
    }
}

impl rodio::Source for ChannelAudioSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        2
    }
    fn sample_rate(&self) -> u32 {
        44_100
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

fn start_audio(state: &mut PlaybackState, position: f64) {
    let Some(path) = state.snapshot.path.clone() else {
        return;
    };
    let volume = state.snapshot.volume;
    let speed = state.snapshot.speed;
    let (commands, command_rx) = mpsc::channel();
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let worker = std::thread::Builder::new()
        .name("photag-audio-playback".into())
        .spawn(move || {
            let (stream, handle) = match rodio::OutputStream::try_default() {
                Ok(pair) => pair,
                Err(_) => {
                    let _ = ready_tx.send(false);
                    return;
                }
            };
            let sink = match rodio::Sink::try_new(&handle) {
                Ok(sink) => sink,
                Err(_) => {
                    let _ = ready_tx.send(false);
                    return;
                }
            };
            let mut child = match ffmpeg::command("ffmpeg")
                .args([
                    "-ss",
                    &format!("{position:.3}"),
                    "-i",
                    &path.to_string_lossy(),
                    "-f",
                    "s16le",
                    "-ar",
                    "44100",
                    "-ac",
                    "2",
                    "-vn",
                    "-v",
                    "error",
                    "pipe:1",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .stdin(Stdio::null())
                .spawn()
            {
                Ok(child) => child,
                Err(_) => {
                    let _ = ready_tx.send(false);
                    return;
                }
            };
            let Some(stdout) = child.stdout.take() else {
                let _ = child.kill();
                let _ = child.wait();
                let _ = ready_tx.send(false);
                return;
            };
            let (tx, rx) = mpsc::sync_channel::<i16>(88_200);
            std::thread::spawn(move || {
                let mut reader = std::io::BufReader::with_capacity(16_384, stdout);
                let mut bytes = [0u8; 16_384];
                loop {
                    let count = match reader.read(&mut bytes) {
                        Ok(0) | Err(_) => break,
                        Ok(count) => count,
                    };
                    for sample in bytes[..count - count % 2].chunks_exact(2) {
                        if tx.send(i16::from_le_bytes([sample[0], sample[1]])).is_err() {
                            return;
                        }
                    }
                }
            });
            sink.append(ChannelAudioSource { rx });
            sink.set_volume(volume);
            sink.set_speed(speed);
            let _ = ready_tx.send(true);
            while let Ok(command) = command_rx.recv() {
                match command {
                    AudioCommand::Position(reply) => {
                        let position = (!sink.empty()).then(|| sink.get_pos().as_secs_f64());
                        let _ = reply.send(position);
                    }
                    AudioCommand::SetVolume(value) => sink.set_volume(value),
                    AudioCommand::Stop => break,
                }
            }
            sink.stop();
            let _ = child.kill();
            let _ = child.wait();
            drop(stream);
        });
    let Ok(worker) = worker else {
        return;
    };
    if ready_rx.recv_timeout(Duration::from_secs(3)) == Ok(true) {
        state.audio = Some(AudioSession {
            commands,
            thread: worker,
        });
        state.snapshot.audio_available = true;
    } else {
        let _ = commands.send(AudioCommand::Stop);
        let _ = worker.join();
    }
}

fn start(state: &mut PlaybackState, position: f64) -> Result<(), PlaybackError> {
    let path = state.snapshot.path.clone().ok_or(PlaybackError::NoMedia)?;
    if !state.snapshot.has_video {
        start_audio(state, position);
        state.receiver = None;
        state.child = None;
        state.started = Some(Instant::now());
        state.start_position = position;
        state.snapshot.position_seconds = position;
        state.snapshot.playing = true;
        return Ok(());
    }
    let width = state.snapshot.width.max(2);
    let height = state.snapshot.height.max(2);
    let out_width = (if width > 1280 { 1280 } else { width & !1 }).max(2) as usize;
    let out_height = (if width > 1280 {
        ((height as f64 * 1280.0 / width as f64) as u32) & !1
    } else {
        height & !1
    })
    .max(2) as usize;
    let (tx, rx) = mpsc::sync_channel(2);
    let scale = format!("scale={out_width}:{out_height}");
    let frame_interval = Duration::from_secs_f64(
        1.0 / state.snapshot.frame_rate.max(1.0) / state.snapshot.speed as f64,
    );
    let mut command = ffmpeg::command("ffmpeg");
    let child = command
        .args([
            "-ss",
            &format!("{position:.3}"),
            "-i",
            &path.to_string_lossy(),
            "-vf",
            &scale,
            "-an",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()?;
    let mut child = child;
    let stdout = child.stdout.take().ok_or(PlaybackError::DecoderPipe)?;
    let frame_size = out_width * out_height * 3;
    let cache = state.cache.clone();
    std::thread::spawn(move || {
        let mut reader = std::io::BufReader::with_capacity(frame_size.saturating_mul(2), stdout);
        let mut index = 0u64;
        let mut next_frame_at = Instant::now();
        loop {
            let mut bytes = vec![0; frame_size];
            if std::io::Read::read_exact(&mut reader, &mut bytes).is_err() {
                break;
            }
            let Some(image) = image::RgbImage::from_raw(out_width as u32, out_height as u32, bytes)
            else {
                break;
            };
            let frame = cache.join(format!("frame-{}-{index:08}.png", std::process::id()));
            if image.save(&frame).is_err() {
                break;
            }
            if tx.send(frame).is_err() {
                break;
            }
            index += 1;
            next_frame_at += frame_interval;
            std::thread::sleep(next_frame_at.saturating_duration_since(Instant::now()));
        }
    });
    state.child = Some(child);
    start_audio(state, position);
    state.receiver = Some(rx);
    state.started = Some(Instant::now());
    state.start_position = position;
    state.snapshot.position_seconds = position;
    state.snapshot.playing = true;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum PlaybackError {
    #[error("media file does not exist: {0}")]
    MissingMedia(PathBuf),
    #[error("no media is open")]
    NoMedia,
    #[error("FFmpeg could not inspect media: {0}")]
    ProbeFailed(PathBuf),
    #[error("FFmpeg decoder did not provide a frame pipe")]
    DecoderPipe,
    #[error("playback state lock is poisoned")]
    StatePoisoned,
    #[error("playback cache or process error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid FFprobe JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("could not publish playback service: {0}")]
    Publish(String),
    #[error("could not register playback UI: {0}")]
    Ui(String),
}

#[derive(Clone, Debug, Default)]
pub struct MediaPlaybackConfig {
    pub cache_dir: Option<PathBuf>,
}
pub struct MediaPlaybackPlugin;
impl Plugin for MediaPlaybackPlugin {
    type Config = MediaPlaybackConfig;
    type Input = MediaPlaybackConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = PlaybackError;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        "photag.media-playback".into()
    }
    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }
    async fn apply(&self, context: Context, config: &Self::Input) -> Result<(), Self::ApplyError> {
        let cache = config.cache_dir.clone().unwrap_or_else(default_cache_dir);
        let service = Arc::new(MediaPlaybackService::new(cache)?);
        let _publication = context
            .provide(Arc::clone(&service))
            .map_err(|e| PlaybackError::Publish(e.to_string()))?;
        register_for_plugin(
            &context,
            "photag.media-playback",
            [UiContribution::new(
                "media-playback/transport",
                ContributionKind::Panel {
                    label: "Media playback".into(),
                    slot: "workspace.modal".into(),
                    order: 10,
                },
            )],
        )
        .map_err(|e| PlaybackError::Ui(e.to_string()))?;
        let tagging = context
            .try_service::<photag_plugin_tagging::TaggingService>()
            .ok();
        let file_operations = context
            .try_service::<photag_plugin_file_operations::FileOperationsService>()
            .ok();
        let surfaces = context
            .try_service::<photag_ui_bridge::UiBridgeService>()
            .map_err(|error| PlaybackError::Ui(error.to_string()))?
            .surfaces
            .clone();
        register_views_for_plugin(
            &context,
            "photag.media-playback",
            [media_playback_view(
                Arc::clone(&service),
                tagging,
                file_operations,
                surfaces,
            )],
        )
        .map_err(|e| PlaybackError::Ui(e.to_string()))
    }
}

fn default_cache_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("PhoTag")
        .join("playback")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeking_paused_audio_keeps_output_paused() {
        let mut snapshot = PlaybackSnapshot::default();
        snapshot.path = Some(PathBuf::from("ambient_nature.wav"));
        snapshot.has_video = false;
        snapshot.duration_seconds = 207.25;
        let service = MediaPlaybackService {
            state: Mutex::new(PlaybackState {
                snapshot,
                child: None,
                audio: None,
                receiver: None,
                started: None,
                start_position: 0.0,
                pause_after_frame: false,
                cache: std::env::temp_dir(),
            }),
        };

        service.seek(61.0).unwrap();

        let snapshot = service.snapshot();
        assert_eq!(snapshot.position_seconds, 61.0);
        assert!(!snapshot.playing);
    }
}

/// Remove only this player's frame files that have been inactive for a week.
/// The cache is shared by app instances, so recent frames are left untouched.
fn cleanup_aged_frames(cache: &Path) {
    let Ok(entries) = std::fs::read_dir(cache) else {
        return;
    };
    let cutoff = Duration::from_secs(7 * 24 * 60 * 60);
    for entry in entries.flatten() {
        let path = entry.path();
        if !is_frame_cache_file(&path) {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        if modified.elapsed().is_ok_and(|age| age >= cutoff) {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn is_frame_cache_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let Some(stem) = name.strip_suffix(".png") else {
        return false;
    };
    let Some(rest) = stem.strip_prefix("frame-") else {
        return false;
    };
    let Some((process_id, index)) = rest.split_once('-') else {
        return false;
    };
    !process_id.is_empty()
        && process_id.bytes().all(|byte| byte.is_ascii_digit())
        && !index.is_empty()
        && index.bytes().all(|byte| byte.is_ascii_digit())
}

// Keep idle playback views from outliving the Cordis service.
impl Drop for MediaPlaybackService {
    fn drop(&mut self) {
        if let Ok(state) = self.state.get_mut() {
            stop(state);
        }
    }
}
