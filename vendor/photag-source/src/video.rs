//! Built-in video player and thumbnail extraction using FFmpeg.
//!
//! Video playback uses two ffmpeg subprocesses:
//!   1. Video decoder: outputs raw RGB24 frames -> displayed as egui textures
//!   2. Audio decoder: outputs raw PCM s16le -> played via rodio
//!
//! Video thumbnails are extracted with `ffmpeg -vframes 1` during scanning.

use egui::{self, Color32, ColorImage, TextureHandle, Vec2};
use rodio::Source;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Instant;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// Windows flag to prevent console window flash when spawning processes.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

// ── Utility Functions ───────────────────────────────────────────

/// Find the ffmpeg/ffprobe binary. Checks:
///   1. Next to the PhoTag executable (bundled)
///   2. In a `ffmpeg/bin` subfolder next to the executable
///   3. System PATH (fallback)
fn find_binary(name: &str) -> String {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // Check right next to the exe
            let candidate = exe_dir.join(format!("{}.exe", name));
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
            // Check in ffmpeg/bin subfolder
            let candidate = exe_dir.join("ffmpeg").join("bin").join(format!("{}.exe", name));
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }
    }
    // Fallback: rely on system PATH
    name.to_string()
}

fn ffmpeg_cmd(program: &str) -> Command {
    let binary = find_binary(program);
    let mut cmd = Command::new(binary);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// Check if ffmpeg is available (bundled or system PATH).
pub fn is_ffmpeg_available() -> bool {
    ffmpeg_cmd("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Extract a thumbnail frame from a video as JPEG.
///
/// Performance notes:
///   - `-ss 1 -i` (seek BEFORE input): ffmpeg uses the container index to jump
///     to ~1s in, decoding only one keyframe instead of demuxing from frame 0.
///     This is 5-20x faster for long videos. Reading 1s in also avoids the
///     all-black opening frame common in many videos.
///   - `-ss 1` is best-effort; for very short videos ffmpeg falls back to
///     extracting whatever frame it can.
pub fn extract_video_thumbnail(video_path: &str, output_path: &str) -> bool {
    let ok = ffmpeg_cmd("ffmpeg")
        .args([
            "-ss", "1",
            "-i", video_path,
            "-vframes", "1",
            "-vf", "scale=300:-2",
            "-q:v", "3",
            "-y",
            output_path,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    // For videos shorter than 1 second, the seek can fail. Retry from frame 0.
    if !ok || !std::path::Path::new(output_path).exists() {
        return ffmpeg_cmd("ffmpeg")
            .args([
                "-i", video_path,
                "-vframes", "1",
                "-vf", "scale=300:-2",
                "-q:v", "3",
                "-y",
                output_path,
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    }
    ok
}

/// Format seconds as HH:MM:SS or MM:SS.
pub fn format_time(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{:02}:{:02}:{:02}", h, m, s)
    } else {
        format!("{:02}:{:02}", m, s)
    }
}

// ── Video Info (via ffprobe) ────────────────────────────────────

struct VideoInfo {
    width: u32,
    height: u32,
    fps: f64,
    duration: f64,
}

fn get_video_info(path: &str) -> Option<VideoInfo> {
    // Request rotation metadata too — modern phone videos store dimensions
    // as the raw sensor (e.g. 1920×1080) plus a rotation tag. ffmpeg
    // auto-rotates by default, so the *displayed* frame is height×width.
    // We need that swapped size for the decode buffer or vertical clips
    // come out stretched horizontally.
    let output = ffmpeg_cmd("ffprobe")
        .args([
            "-v", "error",
            "-select_streams", "v:0",
            "-show_entries",
            "stream=width,height,r_frame_rate:stream_tags=rotate:stream_side_data=rotation",
            "-show_entries", "format=duration",
            "-of", "json",
            path,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;

    let stream = json["streams"].as_array()?.first()?;
    let raw_w = stream["width"].as_u64()? as u32;
    let raw_h = stream["height"].as_u64()? as u32;

    // Rotation can live in two places depending on the container:
    //   • mp4/mov: stream.tags.rotate ("90", "180", "270")
    //   • newer ffmpeg: stream.side_data_list[*].rotation (signed degrees,
    //     e.g. -90 for clockwise 90)
    let rotation = stream
        .get("tags")
        .and_then(|t| t.get("rotate"))
        .and_then(|r| r.as_str())
        .and_then(|s| s.parse::<i64>().ok())
        .or_else(|| {
            stream
                .get("side_data_list")
                .and_then(|v| v.as_array())
                .and_then(|arr| {
                    arr.iter().find_map(|o| o.get("rotation").and_then(|r| r.as_i64()))
                })
        })
        .unwrap_or(0);

    let (width, height) = if rotation.rem_euclid(180) == 90 {
        (raw_h, raw_w)
    } else {
        (raw_w, raw_h)
    };

    let fps_str = stream["r_frame_rate"].as_str()?;
    let fps = if let Some((num, den)) = fps_str.split_once('/') {
        num.parse::<f64>().ok()? / den.parse::<f64>().ok()?.max(1.0)
    } else {
        fps_str.parse::<f64>().ok()?
    };
    let fps = fps.clamp(1.0, 120.0);

    let duration = json["format"]["duration"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    Some(VideoInfo { width, height, fps, duration })
}

// ── Channel-based Audio Source for rodio ─────────────────────────

struct ChannelAudioSource {
    rx: mpsc::Receiver<i16>,
}

impl Iterator for ChannelAudioSource {
    type Item = i16;
    fn next(&mut self) -> Option<i16> {
        self.rx.recv().ok()
    }
}

impl Source for ChannelAudioSource {
    fn current_frame_len(&self) -> Option<usize> { None }
    fn channels(&self) -> u16 { 2 }
    fn sample_rate(&self) -> u32 { 44100 }
    fn total_duration(&self) -> Option<std::time::Duration> { None }
}

// ── Video Player ────────────────────────────────────────────────

pub struct VideoPlayer {
    // Video decode
    info: Option<VideoInfo>,
    frame_rx: Option<mpsc::Receiver<ColorImage>>,
    texture: Option<TextureHandle>,
    video_child: Option<Child>,

    // Audio
    audio_child: Option<Child>,
    audio_sink: Option<rodio::Sink>,
    _audio_stream: Option<rodio::OutputStream>,

    // Thread control
    running: Arc<AtomicBool>,

    // Playback state
    playing: bool,
    position: f64,
    start_position: f64,
    play_start: Option<Instant>,
    frames_consumed: usize,
    /// Whether the speed picker popup is currently open. We manage this
    /// ourselves instead of using egui's popup system so that nested popups
    /// (the preset ComboBox) and slider drags don't dismiss the parent.
    speed_popup_open: bool,
    /// When true, the next decoded frame is shown and then playback pauses.
    /// Used by frame-step / paused-seek so the user lands on the new frame
    /// without playback continuing past it.
    pause_after_first_frame: bool,
    /// Output volume, 0.0..1.5. Applied to the rodio sink whenever changed.
    pub volume: f32,
    /// Playback rate, 0.25..3.0. Affects both visual frame consumption and
    /// the audio sink's `set_speed`. We don't restart ffmpeg; we just consume
    /// frames at `elapsed * speed * fps` and let rodio handle audio resampling.
    pub speed: f32,

    path: String,
}

impl VideoPlayer {
    pub fn new() -> Self {
        Self {
            info: None,
            frame_rx: None,
            texture: None,
            video_child: None,
            audio_child: None,
            audio_sink: None,
            _audio_stream: None,
            running: Arc::new(AtomicBool::new(false)),
            playing: false,
            position: 0.0,
            start_position: 0.0,
            play_start: None,
            frames_consumed: 0,
            speed_popup_open: false,
            pause_after_first_frame: false,
            volume: 1.0,
            speed: 1.0,
            path: String::new(),
        }
    }

    /// Set the output volume on the active rodio sink (no-op if no audio).
    pub fn set_volume(&mut self, v: f32) {
        self.volume = v.clamp(0.0, 1.5);
        if let Some(sink) = &self.audio_sink {
            sink.set_volume(self.volume);
        }
    }

    /// Set the playback rate. Updates both audio (via rodio resampling) and
    /// our video pacing logic. Snapshot current position so it doesn't jump.
    pub fn set_speed(&mut self, s: f32) {
        // Wider range to support both fine slow-mo (0.1x) and "boost" presets
        // (up to 8x). rodio handles audio resampling for the whole range.
        let new_speed = s.clamp(0.1, 8.0);
        if (new_speed - self.speed).abs() < 0.01 { return; }
        // Capture position under the OLD speed before swapping, so the
        // playhead stays put when the multiplier changes.
        let pos_now = self.position();
        self.speed = new_speed;
        self.start_position = pos_now;
        self.play_start = Some(Instant::now());
        if let Some(sink) = &self.audio_sink {
            sink.set_speed(self.speed);
        }
    }

    /// Step exactly one frame forward or back (rounded to nearest frame at the
    /// source's native fps).
    pub fn step_frames(&mut self, frames: i32) {
        let fps = self.info.as_ref().map(|i| i.fps).unwrap_or(30.0);
        let delta = frames as f64 / fps.max(1.0);
        let pos = (self.position() + delta).clamp(0.0, self.duration());
        self.seek(pos);
    }

    pub fn is_open(&self) -> bool { !self.path.is_empty() }
    pub fn is_playing(&self) -> bool { self.playing }
    pub fn duration(&self) -> f64 { self.info.as_ref().map(|i| i.duration).unwrap_or(0.0) }
    pub fn texture(&self) -> Option<&TextureHandle> { self.texture.as_ref() }

    pub fn position(&self) -> f64 {
        if self.playing {
            let elapsed = self.play_start.map(|i| i.elapsed().as_secs_f64()).unwrap_or(0.0);
            (self.start_position + elapsed * self.speed as f64).min(self.duration())
        } else {
            self.position
        }
    }

    pub fn open(&mut self, path: &str) {
        self.close();
        let info = match get_video_info(path) {
            Some(i) => i,
            None => {
                // ffmpeg/ffprobe not available — fall back to system player
                open::that_detached(path).ok();
                return;
            }
        };
        self.path = path.to_string();
        self.position = 0.0;
        self.start_position = 0.0;
        self.frames_consumed = 0;
        self.start_playback(&info, 0.0);
        self.info = Some(info);
    }

    fn start_playback(&mut self, info: &VideoInfo, start_pos: f64) {
        self.running = Arc::new(AtomicBool::new(true));

        // Decode dimensions (cap at 1280px wide for performance)
        let (dw, dh) = if info.width > 1280 {
            let scale = 1280.0 / info.width as f64;
            (1280u32, ((info.height as f64 * scale) as u32) & !1)
        } else {
            (info.width & !1, info.height & !1)
        };
        let dw = dw.max(2);
        let dh = dh.max(2);

        self.start_video_decoder(start_pos, dw, dh);
        self.start_audio(start_pos);

        self.playing = true;
        self.play_start = Some(Instant::now());
        self.start_position = start_pos;
        self.frames_consumed = 0;
    }

    fn start_video_decoder(&mut self, start_pos: f64, width: u32, height: u32) {
        let (tx, rx) = mpsc::sync_channel::<ColorImage>(3);
        self.frame_rx = Some(rx);

        let running = self.running.clone();
        let path = self.path.clone();

        let mut child = match ffmpeg_cmd("ffmpeg")
            .args([
                "-ss", &format!("{:.3}", start_pos),
                "-i", &path,
                "-f", "rawvideo",
                "-pix_fmt", "rgb24",
                "-vf", &format!("scale={}:{}", width, height),
                "-an", "-v", "error",
                "pipe:1",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => return,
        };

        let stdout = match child.stdout.take() {
            Some(s) => s,
            None => { self.video_child = Some(child); return; }
        };
        self.video_child = Some(child);

        let frame_size = (width as usize) * (height as usize) * 3;
        let w = width as usize;
        let h = height as usize;

        std::thread::spawn(move || {
            let mut reader = std::io::BufReader::with_capacity(frame_size * 2, stdout);
            while running.load(Ordering::Relaxed) {
                let mut buf = vec![0u8; frame_size];
                if reader.read_exact(&mut buf).is_err() {
                    break;
                }
                let pixels: Vec<Color32> = buf
                    .chunks_exact(3)
                    .map(|c| Color32::from_rgb(c[0], c[1], c[2]))
                    .collect();
                if tx.send(ColorImage { size: [w, h], pixels }).is_err() {
                    break;
                }
            }
        });
    }

    fn start_audio(&mut self, start_pos: f64) {
        // Create audio output (may fail if no audio device — video plays silently)
        let (stream, handle) = match rodio::OutputStream::try_default() {
            Ok(pair) => pair,
            Err(_) => return,
        };
        let sink = match rodio::Sink::try_new(&handle) {
            Ok(s) => s,
            Err(_) => {
                self._audio_stream = Some(stream);
                return;
            }
        };

        let mut child = match ffmpeg_cmd("ffmpeg")
            .args([
                "-ss", &format!("{:.3}", start_pos),
                "-i", &self.path,
                "-f", "s16le",
                "-ar", "44100",
                "-ac", "2",
                "-vn", "-v", "error",
                "pipe:1",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => {
                self._audio_stream = Some(stream);
                return;
            }
        };

        let stdout = match child.stdout.take() {
            Some(s) => s,
            None => {
                self.audio_child = Some(child);
                self._audio_stream = Some(stream);
                return;
            }
        };

        let (audio_tx, audio_rx) = mpsc::sync_channel::<i16>(44100 * 2);
        let running = self.running.clone();

        std::thread::spawn(move || {
            let mut reader = std::io::BufReader::with_capacity(16384, stdout);
            let mut buf = [0u8; 16384];
            while running.load(Ordering::Relaxed) {
                let n = match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                let usable = n - (n % 2);
                for chunk in buf[..usable].chunks_exact(2) {
                    let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                    if audio_tx.send(sample).is_err() {
                        return;
                    }
                }
            }
        });

        sink.append(ChannelAudioSource { rx: audio_rx });
        sink.set_volume(self.volume);
        sink.set_speed(self.speed);

        self.audio_sink = Some(sink);
        self.audio_child = Some(child);
        self._audio_stream = Some(stream);
    }

    /// Call each frame to receive decoded video frames and update texture.
    pub fn poll(&mut self, ctx: &egui::Context) {
        if !self.playing {
            return;
        }
        let fps = match &self.info {
            Some(i) => i.fps,
            None => return,
        };
        let rx = match &self.frame_rx {
            Some(rx) => rx,
            None => return,
        };

        let elapsed = self.play_start.map(|i| i.elapsed().as_secs_f64()).unwrap_or(0.0);
        let target_frame = (elapsed * fps * self.speed as f64) as usize;

        let mut new_frame = None;
        while self.frames_consumed < target_frame {
            match rx.try_recv() {
                Ok(image) => {
                    new_frame = Some(image);
                    self.frames_consumed += 1;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.playing = false;
                    self.position = self.duration();
                    return;
                }
            }
        }

        if let Some(image) = new_frame {
            self.texture = Some(ctx.load_texture(
                "video_frame",
                image,
                egui::TextureOptions::LINEAR,
            ));
            // Frame-step / paused-seek path: we kept playing=true to grab one
            // frame; now that we have it, pause and snapshot the position.
            if self.pause_after_first_frame {
                self.pause_after_first_frame = false;
                self.position = self.position();
                self.playing = false;
                if let Some(sink) = &self.audio_sink {
                    sink.pause();
                }
            }
        }

        if self.playing {
            ctx.request_repaint();
        }
    }

    pub fn toggle_play_pause(&mut self) {
        if self.playing {
            self.position = self.position();
            self.playing = false;
            if let Some(sink) = &self.audio_sink {
                sink.pause();
            }
        } else {
            // Drain queued video frames to avoid burst on resume
            if let Some(rx) = &self.frame_rx {
                while rx.try_recv().is_ok() {}
            }
            self.start_position = self.position;
            self.play_start = Some(Instant::now());
            self.frames_consumed = 0;
            self.playing = true;
            if let Some(sink) = &self.audio_sink {
                sink.play();
            }
        }
    }

    pub fn seek(&mut self, pos: f64) {
        let dur = self.duration();
        let pos = pos.clamp(0.0, dur);
        let was_info = self.info.take();
        // Preserve the paused/playing state across the seek. start_playback
        // unconditionally sets playing=true, which made frame-step buttons
        // act like "play from the next frame" instead of stepping in place.
        let was_playing = self.playing;

        if let Some(info) = was_info {
            self.stop_processes();
            self.position = pos;
            self.texture = None;
            self.start_playback(&info, pos);
            self.info = Some(info);
            if !was_playing {
                // Stay "playing" only long enough for poll() to grab the next
                // decoded frame at the new position, then pause. Without this,
                // poll() returns early on !playing and the new frame is never
                // displayed — making frame-step look broken.
                self.pause_after_first_frame = true;
                if let Some(sink) = &self.audio_sink {
                    sink.pause();
                }
            }
        }
    }

    fn stop_processes(&mut self) {
        self.running.store(false, Ordering::Relaxed);

        // Kill ffmpeg processes first (unblocks threads waiting on pipe reads)
        if let Some(mut c) = self.video_child.take() {
            c.kill().ok();
        }
        if let Some(mut c) = self.audio_child.take() {
            c.kill().ok();
        }

        // Drop audio
        self.audio_sink.take();
        self._audio_stream.take();

        // Drop frame receiver
        self.frame_rx.take();
    }

    pub fn close(&mut self) {
        self.stop_processes();
        self.info = None;
        self.texture = None;
        self.path.clear();
        self.playing = false;
        self.position = 0.0;
        self.frames_consumed = 0;
    }

    /// Draw the video player overlay UI. Returns false if the player should be closed.
    pub fn show_ui(&mut self, ctx: &egui::Context) -> bool {
        if !self.is_open() {
            return false;
        }

        // Keep repainting while playing or waiting for first frame.
        if self.playing || self.texture.is_none() {
            ctx.request_repaint();
        }

        // Keyboard shortcuts
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return false;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            self.toggle_play_pause();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            let pos = (self.position() + 5.0).min(self.duration());
            self.seek(pos);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            let pos = (self.position() - 5.0).max(0.0);
            self.seek(pos);
        }

        let screen = ctx.screen_rect();
        let mut keep_open = true;

        egui::Area::new(egui::Id::new("video_overlay"))
            .fixed_pos(screen.min)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let (rect, resp) =
                    ui.allocate_exact_size(screen.size(), egui::Sense::click());

                // Dark background
                ui.painter().rect_filled(
                    rect,
                    egui::Rounding::ZERO,
                    Color32::from_black_alpha(240),
                );

                // ── Video frame ──
                let controls_height = 70.0;
                let video_area = egui::Rect::from_min_max(
                    rect.min,
                    egui::pos2(rect.max.x, rect.max.y - controls_height),
                );

                if let Some(tex) = &self.texture {
                    let tex_size = tex.size_vec2();
                    let available = video_area.size() * 0.92;
                    let scale =
                        (available.x / tex_size.x).min(available.y / tex_size.y).min(1.0);
                    let draw_size = tex_size * scale;
                    let img_rect =
                        egui::Rect::from_center_size(video_area.center(), draw_size);

                    ui.painter().image(
                        tex.id(),
                        img_rect,
                        egui::Rect::from_min_max(
                            egui::pos2(0.0, 0.0),
                            egui::pos2(1.0, 1.0),
                        ),
                        Color32::WHITE,
                    );
                } else {
                    let msg = if self.frame_rx.is_some() {
                        "Loading video..."
                    } else {
                        "Failed to open video"
                    };
                    ui.painter().text(
                        video_area.center(),
                        egui::Align2::CENTER_CENTER,
                        msg,
                        egui::FontId::proportional(18.0),
                        Color32::from_white_alpha(150),
                    );
                }

                // ── Close button (X drawn with lines) ──
                let close_center =
                    egui::pos2(rect.max.x - 25.0, rect.min.y + 25.0);
                let cs = 10.0;
                ui.painter().line_segment(
                    [
                        close_center + Vec2::new(-cs, -cs),
                        close_center + Vec2::new(cs, cs),
                    ],
                    egui::Stroke::new(2.5, Color32::from_white_alpha(200)),
                );
                ui.painter().line_segment(
                    [
                        close_center + Vec2::new(cs, -cs),
                        close_center + Vec2::new(-cs, cs),
                    ],
                    egui::Stroke::new(2.5, Color32::from_white_alpha(200)),
                );

                // ── Progress bar ──
                let bar_left = rect.min.x + 50.0;
                let bar_right = rect.max.x - 50.0;
                let bar_y = rect.max.y - controls_height + 10.0;
                let bar_h = 5.0;
                let bar_rect = egui::Rect::from_min_size(
                    egui::pos2(bar_left, bar_y),
                    Vec2::new(bar_right - bar_left, bar_h),
                );

                // Bar background
                ui.painter().rect_filled(
                    bar_rect,
                    egui::Rounding::same(2.5),
                    Color32::from_white_alpha(40),
                );

                let pos = self.position();
                let dur = self.duration();
                if dur > 0.0 {
                    let frac = (pos / dur).clamp(0.0, 1.0) as f32;
                    let fill_w = (bar_right - bar_left) * frac;
                    // Filled portion
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(bar_left, bar_y),
                            Vec2::new(fill_w, bar_h),
                        ),
                        egui::Rounding::same(2.5),
                        Color32::from_rgb(60, 130, 240),
                    );
                    // Seek dot
                    ui.painter().circle_filled(
                        egui::pos2(bar_left + fill_w, bar_y + bar_h / 2.0),
                        6.0,
                        Color32::WHITE,
                    );
                }

                // ── Time display (left of controls) ──
                let time_text =
                    format!("{} / {}", format_time(pos), format_time(dur));
                ui.painter().text(
                    egui::pos2(rect.center().x, rect.max.y - 12.0),
                    egui::Align2::CENTER_CENTER,
                    &time_text,
                    egui::FontId::proportional(11.0),
                    Color32::from_white_alpha(160),
                );

                // ── Control row (real widgets via a child UI) ──
                // Sits between the seek bar and the time label.
                let row_y = rect.max.y - 50.0;
                let row_w = rect.width().min(720.0);
                let row_rect = egui::Rect::from_center_size(
                    egui::pos2(rect.center().x, row_y),
                    Vec2::new(row_w, 36.0),
                );

                // ── Interaction ──
                let close_hit =
                    egui::Rect::from_center_size(close_center, Vec2::splat(30.0));
                let seek_hit = egui::Rect::from_min_size(
                    egui::pos2(bar_left - 10.0, bar_y - 10.0),
                    Vec2::new(bar_right - bar_left + 20.0, bar_h + 20.0),
                );

                // Dedicated close-button interact rect at Foreground order
                // so it can't be stolen by other Areas.
                let close_resp = ui.interact(
                    close_hit,
                    egui::Id::new("video_close_btn"),
                    egui::Sense::click(),
                );
                if close_resp.clicked() {
                    keep_open = false;
                }
                if close_resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }

                if resp.clicked() {
                    if let Some(p) = resp.interact_pointer_pos() {
                        if seek_hit.contains(p) && dur > 0.0 {
                            let frac = ((p.x - bar_left) / (bar_right - bar_left))
                                .clamp(0.0, 1.0) as f64;
                            self.seek(frac * dur);
                        } else if row_rect.expand2(Vec2::new(0.0, 8.0)).contains(p) {
                            // Click was inside the controls row — let the child UI handle it
                        } else if video_area.contains(p) {
                            self.toggle_play_pause();
                        } else {
                            keep_open = false;
                        }
                    }
                }
            });

        // ── Control row in its OWN Area so it gets a normal horizontal
        // layout and renders on top of the overlay background. Doing this
        // inside the overlay's Area with allocate_ui_at_rect was clipping
        // the buttons because the inner UI defaulted to top-down layout.
        egui::Area::new(egui::Id::new("video_controls"))
            .fixed_pos(egui::pos2(screen.center().x - 360.0, screen.max.y - 56.0))
            .order(egui::Order::Tooltip)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(Color32::from_black_alpha(180))
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(egui::Margin::symmetric(10.0, 6.0))
                    .show(ui, |ui| {
                        ui.set_width(720.0);
                        let speed_btn_rect = ui.with_layout(
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui|
                        {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            let btn = Vec2::new(36.0, 28.0);
                            let cur_speed = self.speed;

                            // ── LEFT: speed picker ──
                            ui.label("⏩");
                            let menu_btn = ui.add_sized(
                                Vec2::new(64.0, 28.0),
                                egui::Button::new(format!("{:.2}x", cur_speed)),
                            );
                            if menu_btn.clicked() {
                                self.speed_popup_open = !self.speed_popup_open;
                            }
                            let captured_rect = menu_btn.rect;

                            // ── CENTER: transport, force-centered ──
                            // Reserve a flexible spacer that pushes transport
                            // to the visual center of the row. The volume
                            // section on the right uses the same trick.
                            let total_w = ui.available_width();
                            let transport_w = btn.x * 5.0 + 6.0 * 4.0; // 5 buttons + 4 gaps
                            let volume_w = 18.0 + 100.0 + 6.0;        // icon + slider + gap
                            let left_pad = ((total_w - transport_w - volume_w) / 2.0).max(8.0);
                            ui.add_space(left_pad);

                            if ui.add_sized(btn, egui::Button::new("⏮ 5s"))
                                .on_hover_text("Back 5 seconds").clicked()
                            {
                                let p = (self.position() - 5.0).max(0.0);
                                self.seek(p);
                            }
                            if ui.add_sized(btn, egui::Button::new("◀|"))
                                .on_hover_text("Previous frame").clicked()
                            {
                                self.step_frames(-1);
                            }
                            let pp_label = if self.playing { "⏸" } else { "▶" };
                            if ui.add_sized(btn, egui::Button::new(pp_label))
                                .on_hover_text("Play / Pause (Space)").clicked()
                            {
                                self.toggle_play_pause();
                            }
                            if ui.add_sized(btn, egui::Button::new("|▶"))
                                .on_hover_text("Next frame").clicked()
                            {
                                self.step_frames(1);
                            }
                            if ui.add_sized(btn, egui::Button::new("5s ⏭"))
                                .on_hover_text("Forward 5 seconds").clicked()
                            {
                                let p = (self.position() + 5.0).min(self.duration());
                                self.seek(p);
                            }

                            // ── RIGHT: volume, anchored to the row's end ──
                            let remaining = ui.available_width();
                            let right_pad = (remaining - volume_w).max(8.0);
                            ui.add_space(right_pad);
                            ui.label("🔊");
                            let mut vol = self.volume;
                            if ui.add_sized(
                                Vec2::new(100.0, 18.0),
                                egui::Slider::new(&mut vol, 0.0..=1.5).show_value(false),
                            )
                                .on_hover_text("Volume")
                                .changed()
                            {
                                self.set_volume(vol);
                            }

                            captured_rect
                        }).inner;

                        // ── Speed popup ──
                        // Drawn outside the layout closure so it can sit ABOVE
                        // the controls row without being clipped, and so
                        // outside-click dismissal can run after the inner
                        // widgets (slider, ComboBox) have already consumed
                        // their interactions.
                        if self.speed_popup_open {
                            let cur_speed = self.speed;
                            let popup_id = egui::Id::new("video_speed_popup_area");
                            let popup_resp = egui::Area::new(popup_id)
                                .order(egui::Order::Foreground)
                                // Pivot LEFT_BOTTOM so the popup grows upward
                                // from the position we anchor at — avoids the
                                // need to know its height in advance.
                                .pivot(egui::Align2::LEFT_BOTTOM)
                                .fixed_pos(egui::pos2(
                                    speed_btn_rect.left(),
                                    speed_btn_rect.top() - 6.0,
                                ))
                                .show(ctx, |ui| {
                                    egui::Frame::popup(ui.style())
                                        .show(ui, |ui| {
                                            ui.set_min_width(240.0);
                                            ui.label("Speed");
                                            let mut sp = self.speed;
                                            if ui.add(
                                                egui::Slider::new(&mut sp, 0.1..=2.0)
                                                    .step_by(0.05)
                                                    .suffix("x"),
                                            ).changed() {
                                                self.set_speed(sp);
                                            }

                                            ui.add_space(4.0);
                                            ui.horizontal(|ui| {
                                                ui.label("Preset:");
                                                let presets = [
                                                    0.25_f32, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0,
                                                ];
                                                let cur = self.speed;
                                                let cur_label = presets.iter()
                                                    .find(|&&p| (cur - p).abs() < 0.01)
                                                    .map(|p| format!("{:.2}x", p))
                                                    .unwrap_or_else(|| "—".to_string());
                                                egui::ComboBox::from_id_salt("video_speed_preset")
                                                    .selected_text(cur_label)
                                                    .width(80.0)
                                                    .show_ui(ui, |ui| {
                                                        for p in presets {
                                                            if ui.selectable_label(
                                                                (cur - p).abs() < 0.01,
                                                                format!("{:.2}x", p),
                                                            ).clicked() {
                                                                self.set_speed(p);
                                                            }
                                                        }
                                                    });
                                            });

                                            ui.add_space(4.0);
                                            ui.label("Boost");
                                            ui.horizontal(|ui| {
                                                for s in [3.0_f32, 4.0, 8.0] {
                                                    let active = (cur_speed - s).abs() < 0.01;
                                                    if ui.selectable_label(
                                                        active, format!("{:.0}x", s),
                                                    ).clicked() {
                                                        self.set_speed(s);
                                                    }
                                                }
                                            });
                                        });
                                });

                            // Outside-click dismissal: only on click (not on
                            // press) so that drags inside the popup — the
                            // slider thumb especially — can't dismiss it.
                            // Also explicitly allow the speed button itself,
                            // and any open ComboBox popup that egui registers
                            // separately in its memory.
                            let popup_rect = popup_resp.response.rect;
                            let combo_open = ctx.memory(|m| m.any_popup_open());
                            if !combo_open && ctx.input(|i| i.pointer.any_click()) {
                                if let Some(p) = ctx.input(|i| i.pointer.interact_pos()) {
                                    if !popup_rect.contains(p)
                                        && !speed_btn_rect.contains(p)
                                    {
                                        self.speed_popup_open = false;
                                    }
                                }
                            }
                        }
                    });
            });

        keep_open
    }
}

impl Drop for VideoPlayer {
    fn drop(&mut self) {
        self.close();
    }
}
