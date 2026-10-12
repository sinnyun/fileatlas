//! FFmpeg-backed video playback service and lifecycle-owned GPUI transport surface.
//!
//! Frames and timestamps are produced by one FFmpeg decode process. The source PhoTag
//! implementation used separate video and audio processes plus rodio; this port keeps
//! video timeline/speed/rotation behavior and exposes audio capability as unavailable
//! until the host supplies a supported audio output backend.

mod ffmpeg;
mod view;

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
        Ok(Self {
            state: Mutex::new(PlaybackState {
                snapshot: PlaybackSnapshot::default(),
                child: None,
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
            state.snapshot.position_seconds = (state.start_position
                + state.started.map_or(0.0, |t| {
                    t.elapsed().as_secs_f64() * state.snapshot.speed as f64
                }))
            .min(state.snapshot.duration_seconds);
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
            state.snapshot.playing = false;
            state.snapshot.position_seconds = state.snapshot.duration_seconds;
            state.receiver = None;
            state.snapshot.revision = state.snapshot.revision.wrapping_add(1);
        }
        state.snapshot.clone()
    }

    pub fn open(&self, path: impl AsRef<Path>) -> Result<(), PlaybackError> {
        let path = path.as_ref().to_path_buf();
        if !path.is_file() {
            return Err(PlaybackError::MissingMedia(path));
        }
        let info = ffmpeg::probe(&path)?;
        let mut state = self.lock()?;
        stop(&mut state);
        state.pause_after_frame = false;
        state.snapshot.path = Some(path);
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
        if state.snapshot.path.is_some() && (resume || pos < state.snapshot.duration_seconds) {
            start(&mut state, pos)?;
            state.pause_after_frame = !resume;
        }
        state.snapshot.revision = state.snapshot.revision.wrapping_add(1);
        Ok(())
    }

    pub fn step_frames(&self, frames: i32) -> Result<(), PlaybackError> {
        let s = self.snapshot();
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
            state.snapshot.revision = state.snapshot.revision.wrapping_add(1);
        }
    }

    pub fn close(&self) {
        if let Ok(mut state) = self.state.lock() {
            stop(&mut state);
            state.pause_after_frame = false;
            state.snapshot = PlaybackSnapshot {
                revision: state.snapshot.revision.wrapping_add(1),
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
}

fn start(state: &mut PlaybackState, position: f64) -> Result<(), PlaybackError> {
    let path = state.snapshot.path.clone().ok_or(PlaybackError::NoMedia)?;
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
                    slot: "workspace.main".into(),
                    order: 10,
                },
            )],
        )
        .map_err(|e| PlaybackError::Ui(e.to_string()))?;
        register_views_for_plugin(
            &context,
            "photag.media-playback",
            [media_playback_view(Arc::clone(&service))],
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

// Keep idle playback views from outliving the Cordis service.
impl Drop for MediaPlaybackService {
    fn drop(&mut self) {
        if let Ok(state) = self.state.get_mut() {
            stop(state);
        }
    }
}
