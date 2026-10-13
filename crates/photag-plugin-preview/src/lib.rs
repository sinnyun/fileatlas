//! Source-compatible image preview state, non-destructive transforms, and a GPUI preview surface.
//!
//! The source preview owns transient view state rather than changing media files. This crate
//! follows that rule: transformed rasters are materialized under a cache directory and the
//! original path remains the identity used by callers.

mod ai_suggestions;
mod chrome;
mod info;
mod plugin;
mod view;

use std::collections::{HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use cordis::Service;
use image::ImageDecoder as _;
use photag_plugin_database::DatabaseService;
use photag_plugin_media_playback::MediaPlaybackService;
use photag_plugin_session::SessionService;

pub use plugin::{PreviewConfig, PreviewError, PreviewPlugin};
pub use view::preview_view;

/// Current preview item and the source transform controls.
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewSnapshot {
    pub source: Option<PathBuf>,
    pub media_kind: Option<PreviewMediaKind>,
    /// Database identity when opened by a caller that has an indexed media row.
    pub media_id: Option<i64>,
    /// Clockwise quarter-turn count, normalized to 0..=3.
    pub quarter_turns: u8,
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
    pub fit_to_window: bool,
    /// Percent zoom when fit-to-window is disabled; source presets are retained.
    pub zoom_percent: u16,
    /// Continuous zoom multiplier used by the source wheel interaction.
    pub zoom_scale: f32,
    /// Image-center offset from the viewport center in screen pixels.
    pub pan_x: f32,
    pub pan_y: f32,
    /// Source default is center-anchored wheel zoom; cursor mode preserves the point under the cursor.
    pub zoom_at_cursor: bool,
    pub show_info: bool,
    /// One-shot request from Gallery's AI Suggest Tags action.
    pub ai_request_media_id: Option<i64>,
    pub revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewMediaKind {
    Image,
    Video,
    Audio,
    Other,
}

pub fn classify_media_path(path: &std::path::Path) -> PreviewMediaKind {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if [
        "jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif", "svg", "ico", "heic", "heif",
        "avif",
    ]
    .contains(&extension.as_str())
    {
        PreviewMediaKind::Image
    } else if [
        "mp4", "mov", "avi", "mkv", "wmv", "flv", "webm", "m4v", "mpg", "mpeg", "3gp",
    ]
    .contains(&extension.as_str())
    {
        PreviewMediaKind::Video
    } else if [
        "mp3", "wav", "flac", "aac", "ogg", "wma", "m4a", "opus", "ape", "aiff", "aif",
    ]
    .contains(&extension.as_str())
    {
        PreviewMediaKind::Audio
    } else {
        PreviewMediaKind::Other
    }
}

impl Default for PreviewSnapshot {
    fn default() -> Self {
        Self {
            source: None,
            media_kind: None,
            media_id: None,
            quarter_turns: 0,
            flip_horizontal: false,
            flip_vertical: false,
            fit_to_window: true,
            zoom_percent: 100,
            zoom_scale: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            zoom_at_cursor: false,
            show_info: false,
            ai_request_media_id: None,
            revision: 0,
        }
    }
}

/// Cordis service for opening and controlling image previews.
pub struct PreviewService {
    state: Mutex<PreviewSnapshot>,
    cache_dir: PathBuf,
    session: Option<Arc<SessionService>>,
    filmstrip: Mutex<Vec<PathBuf>>,
    filmstrip_media_ids: Mutex<std::collections::HashMap<PathBuf, i64>>,
    decode_queue: Arc<DecodeQueue>,
    pending_decodes: Arc<Mutex<HashSet<(PathBuf, u32)>>>,
    decode_revision: Arc<AtomicU64>,
    database: Mutex<Option<Arc<DatabaseService>>>,
    last_opened_id: Mutex<Option<i64>>,
    playback: Mutex<Option<Arc<MediaPlaybackService>>>,
    wheel_animation: Mutex<WheelAnimation>,
    last_activity: Mutex<std::time::Instant>,
}

struct WheelAnimation {
    target_zoom: f32,
    target_pan_x: f32,
    target_pan_y: f32,
    last_tick: std::time::Instant,
}

struct DecodeJob {
    source: PathBuf,
    target: PathBuf,
    max_dim: u32,
}

#[derive(Default)]
struct DecodeQueue {
    jobs: Mutex<VecDeque<DecodeJob>>,
    ready: Condvar,
    closed: Mutex<bool>,
}

impl DecodeQueue {
    fn push(&self, job: DecodeJob) {
        if let Ok(mut jobs) = self.jobs.lock() {
            jobs.push_back(job);
            self.ready.notify_one();
        }
    }

    fn pop(&self) -> Option<DecodeJob> {
        let mut jobs = self.jobs.lock().ok()?;
        loop {
            if let Some(job) = jobs.pop_front() {
                return Some(job);
            }
            if self.closed.lock().map(|closed| *closed).unwrap_or(true) {
                return None;
            }
            jobs = self.ready.wait(jobs).ok()?;
        }
    }
}

impl Drop for PreviewService {
    fn drop(&mut self) {
        if let Ok(mut closed) = self.decode_queue.closed.lock() {
            *closed = true;
        }
        self.decode_queue.ready.notify_all();
    }
}

impl Service for PreviewService {
    const NAME: &'static str = "photag/preview";
}

impl PreviewService {
    pub fn open_at(cache_dir: impl Into<PathBuf>) -> std::io::Result<Self> {
        let cache_dir = cache_dir.into();
        std::fs::create_dir_all(&cache_dir)?;
        let decode_queue = Arc::new(DecodeQueue::default());
        let pending_decodes = Arc::new(Mutex::new(HashSet::new()));
        let decode_revision = Arc::new(AtomicU64::new(0));
        for _ in 0..2 {
            let queue = Arc::clone(&decode_queue);
            let pending = Arc::clone(&pending_decodes);
            let revision = Arc::clone(&decode_revision);
            std::thread::spawn(move || {
                while let Some(job) = queue.pop() {
                    let key = (job.source.clone(), job.max_dim);
                    let decoded = Self::open_oriented(&job.source)
                        .ok()
                        .map(|image| image.thumbnail(job.max_dim, job.max_dim));
                    let written = decoded.is_some_and(|image| {
                        let temporary = job.target.with_extension(format!(
                            "{}.tmp.jpg",
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map_or(0, |elapsed| elapsed.as_nanos())
                        ));
                        image.save(&temporary).is_ok()
                            && if std::fs::rename(&temporary, &job.target).is_ok() {
                                true
                            } else {
                                let _ = std::fs::remove_file(temporary);
                                job.target.is_file()
                            }
                    });
                    if written {
                        revision.fetch_add(1, Ordering::Release);
                    }
                    if let Ok(mut pending) = pending.lock() {
                        pending.remove(&key);
                    }
                }
            });
        }
        Ok(Self {
            state: Mutex::new(PreviewSnapshot::default()),
            cache_dir,
            session: None,
            filmstrip: Mutex::new(Vec::new()),
            filmstrip_media_ids: Mutex::new(std::collections::HashMap::new()),
            decode_queue,
            pending_decodes,
            decode_revision,
            database: Mutex::new(None),
            last_opened_id: Mutex::new(None),
            playback: Mutex::new(None),
            wheel_animation: Mutex::new(WheelAnimation {
                target_zoom: 1.0,
                target_pan_x: 0.0,
                target_pan_y: 0.0,
                last_tick: std::time::Instant::now(),
            }),
            last_activity: Mutex::new(std::time::Instant::now()),
        })
    }

    pub fn open_with_session_at(
        cache_dir: impl Into<PathBuf>,
        session: Arc<SessionService>,
    ) -> std::io::Result<Self> {
        let mut service = Self::open_at(cache_dir)?;
        service.session = Some(session);
        if let Some(path) = service
            .session
            .as_ref()
            .and_then(|session| session.snapshot().ok())
            .and_then(|state| state.preview_path)
            .map(PathBuf::from)
        {
            if path.is_file() {
                let _ = service.open(path);
            } else {
                service.save_preview_path(None);
            }
        }
        Ok(service)
    }

    pub fn snapshot(&self) -> PreviewSnapshot {
        self.state.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Queue an explicit AI suggestions request for the active preview consumer.
    pub fn request_ai_suggestions(&self, media_id: i64) {
        if let Ok(mut state) = self.state.lock() {
            state.ai_request_media_id = Some(media_id);
        }
    }

    /// Consume the one-shot AI suggestions request from the preview surface.
    pub fn take_ai_suggestion_request(&self) -> Option<i64> {
        self.state.lock().ok()?.ai_request_media_id.take()
    }

    pub fn decode_revision(&self) -> u64 {
        self.decode_revision.load(Ordering::Acquire)
    }

    pub fn configure_database(&self, database: Arc<DatabaseService>) {
        if let Ok(mut current) = self.database.lock() {
            *current = Some(database);
        }
    }

    pub fn configure_playback(&self, playback: Arc<MediaPlaybackService>) {
        if let Ok(mut current) = self.playback.lock() {
            *current = Some(playback);
        }
    }

    pub fn playback(&self) -> Option<Arc<MediaPlaybackService>> {
        self.playback.lock().ok()?.as_ref().map(Arc::clone)
    }

    pub fn database(&self) -> Option<Arc<DatabaseService>> {
        self.database.lock().ok()?.as_ref().map(Arc::clone)
    }

    pub fn open(&self, path: impl Into<PathBuf>) -> Result<(), PreviewError> {
        self.open_media(path, None)
    }

    /// Open a preview with its indexed media identity for detail and AI consumers.
    pub fn open_media(
        &self,
        path: impl Into<PathBuf>,
        media_id: Option<i64>,
    ) -> Result<(), PreviewError> {
        let path = path.into();
        if !path.is_file() {
            return Err(PreviewError::MissingSource(path));
        }
        let mut state = self.state.lock().map_err(|_| PreviewError::StatePoisoned)?;
        state.source = Some(path.clone());
        state.media_kind = Some(classify_media_path(&path));
        state.media_id = media_id;
        state.quarter_turns = 0;
        state.flip_horizontal = false;
        state.flip_vertical = false;
        state.fit_to_window = true;
        state.zoom_percent = 100;
        state.zoom_scale = 1.0;
        state.pan_x = 0.0;
        state.pan_y = 0.0;
        state.ai_request_media_id = None;
        state.revision = state.revision.wrapping_add(1);
        drop(state);
        self.restore_preview_transform(&path);
        self.sync_wheel_animation();
        self.save_preview_path(Some(path));
        self.save_preview_transform();
        self.mark_activity();
        if let (Some(media_id), Ok(mut last_id)) = (media_id, self.last_opened_id.lock()) {
            if *last_id != Some(media_id) {
                *last_id = Some(media_id);
                if let Some(database) = self.database() {
                    let _ = database.with_writer(|connection| {
                        photag_plugin_database::db::increment_open_count(connection, media_id);
                        Ok(())
                    });
                }
            }
        }
        if let Some(source) = self.snapshot().source {
            if classify_media_path(&source) == PreviewMediaKind::Image {
                self.prewarm_image(&source, 1600);
            }
            let paths = self.filmstrip();
            if let Some(index) = paths.iter().position(|path| path == &source) {
                for path in paths.iter().skip(index.saturating_sub(3)).take(7) {
                    if classify_media_path(path) == PreviewMediaKind::Image {
                        self.prewarm_image(path, 96);
                        if path != &source {
                            self.prewarm_image(path, 1600);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Set ordered media paths used by the preview filmstrip and arrow-key navigation.
    /// Only a small neighborhood is decoded ahead, so filtered libraries do not fan out work.
    pub fn set_filmstrip(&self, paths: impl IntoIterator<Item = PathBuf>) {
        let paths: Vec<_> = paths.into_iter().filter(|path| path.is_file()).collect();
        if let Ok(mut ids) = self.filmstrip_media_ids.lock() {
            ids.retain(|path, _| paths.contains(path));
        }
        if let Ok(mut strip) = self.filmstrip.lock() {
            *strip = paths.clone();
        }
        let current = self.snapshot().source;
        if let Some(index) =
            current.and_then(|path| paths.iter().position(|candidate| candidate == &path))
        {
            for path in paths.iter().skip(index.saturating_sub(3)).take(7) {
                if classify_media_path(path) == PreviewMediaKind::Image {
                    self.prewarm_image(path, 96);
                }
            }
        }
    }

    /// Set ordered filmstrip paths with their indexed row IDs for preview actions/navigation.
    pub fn set_filmstrip_media(&self, items: impl IntoIterator<Item = (PathBuf, i64)>) {
        let items: Vec<_> = items
            .into_iter()
            .filter(|(path, _)| path.is_file())
            .collect();
        if let Ok(mut ids) = self.filmstrip_media_ids.lock() {
            *ids = items.iter().cloned().collect();
        }
        self.set_filmstrip(items.into_iter().map(|(path, _)| path));
    }

    /// Return the filmstrip together with IDs, when supplied by an indexed caller.
    pub fn filmstrip_items(&self) -> Vec<(PathBuf, Option<i64>)> {
        let ids = self
            .filmstrip_media_ids
            .lock()
            .map(|ids| ids.clone())
            .unwrap_or_default();
        self.filmstrip()
            .into_iter()
            .map(|path| {
                let media_id = ids.get(&path).copied();
                (path, media_id)
            })
            .collect()
    }

    /// Keep the active filmstrip coherent after an indexed rename.
    pub fn replace_filmstrip_path(&self, old: &std::path::Path, new: PathBuf, media_id: i64) {
        if let Ok(mut paths) = self.filmstrip.lock() {
            for path in paths.iter_mut() {
                if path == old {
                    *path = new.clone();
                }
            }
        }
        if let Ok(mut ids) = self.filmstrip_media_ids.lock() {
            let had_identity = ids.remove(old).is_some();
            if had_identity {
                ids.insert(new, media_id);
            }
        }
    }

    /// Replace a renamed source path without resetting its view transforms or indexed identity.
    pub fn replace_media_path(&self, old: &std::path::Path, new: PathBuf, media_id: i64) {
        self.replace_filmstrip_path(old, new.clone(), media_id);
        if let Ok(mut state) = self.state.lock() {
            if state.source.as_deref() == Some(old) && state.media_id == Some(media_id) {
                state.source = Some(new.clone());
                state.revision = state.revision.wrapping_add(1);
            }
        }
        self.save_preview_path(Some(new.clone()));
        self.save_preview_transform();
        self.prewarm_image(&new, 1600);
        self.prewarm_image(&new, 96);
    }

    pub fn filmstrip(&self) -> Vec<PathBuf> {
        self.filmstrip
            .lock()
            .map(|paths| paths.clone())
            .unwrap_or_default()
    }

    pub fn navigate_filmstrip(&self, delta: isize) -> bool {
        let Some(current) = self.snapshot().source else {
            return false;
        };
        let paths = self.filmstrip();
        let Some(index) = paths.iter().position(|path| path == &current) else {
            return false;
        };
        let next =
            (index as isize + delta).clamp(0, paths.len().saturating_sub(1) as isize) as usize;
        if next == index {
            return false;
        }
        let media_id = self
            .filmstrip_media_ids
            .lock()
            .ok()
            .and_then(|ids| ids.get(&paths[next]).copied());
        self.open_media(paths[next].clone(), media_id).is_ok()
    }

    /// Return a brief low-resolution first frame while the native image element starts
    /// decoding the full source. The file is produced off the render thread.
    pub fn progressive_path(&self, source: &std::path::Path) -> Option<PathBuf> {
        let fast = self.decode_cache_path(source, 1600);
        if !fast.is_file() {
            return None;
        }
        let recent = fast
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age < std::time::Duration::from_millis(500));
        recent.then_some(fast)
    }

    pub fn filmstrip_thumbnail(&self, source: &std::path::Path) -> Option<PathBuf> {
        let path = self.decode_cache_path(source, 96);
        path.is_file().then_some(path)
    }

    fn decode_cache_path(&self, source: &std::path::Path, max_dim: u32) -> PathBuf {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        self.cache_dir
            .join(format!("preview-{max_dim}-{:016x}.jpg", hasher.finish()))
    }

    fn prewarm_image(&self, source: &std::path::Path, max_dim: u32) {
        let target = self.decode_cache_path(source, max_dim);
        if target.is_file() {
            return;
        }
        let source = source.to_path_buf();
        let key = (source.clone(), max_dim);
        let Ok(mut pending) = self.pending_decodes.lock() else {
            return;
        };
        if pending.insert(key.clone()) {
            self.decode_queue.push(DecodeJob {
                source,
                target,
                max_dim,
            });
        }
    }

    pub fn close(&self) {
        if let Ok(mut s) = self.state.lock() {
            s.source = None;
            s.media_id = None;
            s.media_kind = None;
            s.revision = s.revision.wrapping_add(1);
        }
        self.save_preview_path(None);
        self.mark_activity();
    }
    pub fn rotate_clockwise(&self) {
        self.change(|s| s.quarter_turns = (s.quarter_turns + 1) % 4);
    }
    pub fn rotate_counter_clockwise(&self) {
        self.change(|s| s.quarter_turns = (s.quarter_turns + 3) % 4);
    }
    pub fn flip_horizontal(&self) {
        self.change(|s| s.flip_horizontal = !s.flip_horizontal);
    }
    pub fn flip_vertical(&self) {
        self.change(|s| s.flip_vertical = !s.flip_vertical);
    }
    pub fn toggle_info(&self) {
        self.change(|s| s.show_info = !s.show_info);
    }
    pub fn set_fit(&self) {
        self.change(|s| {
            s.fit_to_window = true;
            s.zoom_percent = 100;
            s.zoom_scale = 1.0;
            s.pan_x = 0.0;
            s.pan_y = 0.0;
        });
    }
    pub fn set_zoom_percent(&self, percent: u16) {
        if [25, 50, 100, 125, 150, 200].contains(&percent) {
            self.change(|s| {
                s.fit_to_window = false;
                s.zoom_percent = percent;
                s.zoom_scale = percent as f32 / 100.0;
            });
        }
    }

    pub fn toggle_zoom_at_cursor(&self) {
        self.change(|s| s.zoom_at_cursor = !s.zoom_at_cursor);
    }

    /// Apply PhoTag's wheel multiplier and cursor-anchor pan calculation.
    /// `cursor_delta` is the cursor's offset from the viewport center.
    pub fn zoom_wheel(&self, scroll_delta: f32, fit_scale: f32, cursor_delta: (f32, f32)) {
        if scroll_delta == 0.0 || !scroll_delta.is_finite() || !fit_scale.is_finite() {
            return;
        }
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let Ok(mut animation) = self.wheel_animation.lock() else {
            return;
        };
        let old_target = if state.fit_to_window {
            state.zoom_scale = fit_scale;
            state.pan_x = 0.0;
            state.pan_y = 0.0;
            animation.target_zoom = fit_scale;
            animation.target_pan_x = 0.0;
            animation.target_pan_y = 0.0;
            fit_scale
        } else {
            animation.target_zoom
        };
        let new_target = (old_target * (1.0 + scroll_delta * 0.0025)).clamp(0.05, 16.0);
        if state.zoom_at_cursor {
            let ratio = new_target / old_target;
            animation.target_pan_x =
                cursor_delta.0 * (1.0 - ratio) + animation.target_pan_x * ratio;
            animation.target_pan_y =
                cursor_delta.1 * (1.0 - ratio) + animation.target_pan_y * ratio;
        }
        animation.target_zoom = new_target;
        animation.last_tick = std::time::Instant::now();
        state.fit_to_window = false;
        state.zoom_percent = (new_target * 100.0).round().clamp(5.0, 1600.0) as u16;
        state.revision = state.revision.wrapping_add(1);
        drop(animation);
        drop(state);
        self.mark_activity();
    }

    pub fn pan_by(&self, dx: f32, dy: f32) {
        if !dx.is_finite() || !dy.is_finite() {
            return;
        }
        self.change(|s| {
            s.pan_x += dx;
            s.pan_y += dy;
        });
    }

    /// Return the original path for the identity transform, otherwise make a cached oriented copy.
    pub fn display_path(&self) -> Result<Option<PathBuf>, PreviewError> {
        let s = self.snapshot();
        if matches!(
            s.media_kind,
            Some(PreviewMediaKind::Audio | PreviewMediaKind::Video)
        ) {
            return Ok(None);
        }
        let Some(source) = s.source else {
            return Ok(None);
        };
        let orientation = Self::read_orientation(&source);
        let user_transform = s.quarter_turns != 0 || s.flip_horizontal || s.flip_vertical;
        if !user_transform && orientation == image::metadata::Orientation::NoTransforms {
            return Ok(Some(source));
        }
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        if let Ok(metadata) = std::fs::metadata(&source) {
            metadata.len().hash(&mut hasher);
            metadata.modified().ok().hash(&mut hasher);
        }
        s.quarter_turns.hash(&mut hasher);
        s.flip_horizontal.hash(&mut hasher);
        s.flip_vertical.hash(&mut hasher);
        let target = self
            .cache_dir
            .join(format!("preview-{:016x}.png", hasher.finish()));
        if target.is_file() {
            return Ok(Some(target));
        }
        let mut image = Self::open_oriented(&source)
            .map_err(|error| PreviewError::Decode(source.clone(), error))?;
        image = match s.quarter_turns {
            1 => image.rotate90(),
            2 => image.rotate180(),
            3 => image.rotate270(),
            _ => image,
        };
        if s.flip_horizontal {
            image = image.fliph();
        }
        if s.flip_vertical {
            image = image.flipv();
        }
        image
            .save(&target)
            .map_err(|error| PreviewError::Write(target.clone(), error.to_string()))?;
        Ok(Some(target))
    }

    fn open_oriented(path: &std::path::Path) -> Result<image::DynamicImage, String> {
        let reader = image::ImageReader::open(path).map_err(|error| error.to_string())?;
        let mut decoder = reader.into_decoder().map_err(|error| error.to_string())?;
        let orientation = decoder
            .orientation()
            .unwrap_or(image::metadata::Orientation::NoTransforms);
        let mut decoded =
            image::DynamicImage::from_decoder(decoder).map_err(|error| error.to_string())?;
        decoded.apply_orientation(orientation);
        Ok(decoded)
    }

    fn read_orientation(path: &std::path::Path) -> image::metadata::Orientation {
        image::ImageReader::open(path)
            .ok()
            .and_then(|reader| reader.into_decoder().ok())
            .and_then(|mut decoder| decoder.orientation().ok())
            .unwrap_or(image::metadata::Orientation::NoTransforms)
    }

    fn change(&self, update: impl FnOnce(&mut PreviewSnapshot)) {
        if let Ok(mut state) = self.state.lock() {
            update(&mut state);
            state.revision = state.revision.wrapping_add(1);
            if let Ok(mut animation) = self.wheel_animation.lock() {
                animation.target_zoom = state.zoom_scale;
                animation.target_pan_x = state.pan_x;
                animation.target_pan_y = state.pan_y;
                animation.last_tick = std::time::Instant::now();
            }
        }
        self.save_preview_transform();
    }

    fn restore_preview_transform(&self, path: &std::path::Path) {
        let Some(session) = &self.session else { return };
        let Ok(saved) = session.snapshot() else {
            return;
        };
        let source_path = path.to_string_lossy().into_owned();
        if saved.preview_path.as_deref() != Some(source_path.as_str()) {
            return;
        }
        if let Ok(mut state) = self.state.lock() {
            state.quarter_turns = saved.preview_quarter_turns.unwrap_or(0).min(3);
            state.flip_horizontal = saved.preview_flip_horizontal.unwrap_or(false);
            state.flip_vertical = saved.preview_flip_vertical.unwrap_or(false);
            state.fit_to_window = saved.preview_fit_to_window.unwrap_or(true);
            state.zoom_scale = saved
                .preview_zoom_scale
                .filter(|v| v.is_finite())
                .unwrap_or(1.0)
                .clamp(0.05, 16.0);
            state.zoom_percent = (state.zoom_scale * 100.0).round() as u16;
            state.pan_x = saved.preview_pan_x.filter(|v| v.is_finite()).unwrap_or(0.0);
            state.pan_y = saved.preview_pan_y.filter(|v| v.is_finite()).unwrap_or(0.0);
            state.zoom_at_cursor = saved.preview_zoom_at_cursor.unwrap_or(false);
            state.show_info = saved.preview_show_info.unwrap_or(false);
        }
    }

    fn save_preview_transform(&self) {
        let Some(session) = &self.session else { return };
        let state = self.snapshot();
        let Ok(mut saved) = session.snapshot() else {
            return;
        };
        let current_path = state
            .source
            .as_deref()
            .map(|p| p.to_string_lossy().into_owned());
        if saved.preview_path != current_path {
            return;
        }
        saved.preview_quarter_turns = Some(state.quarter_turns);
        saved.preview_flip_horizontal = Some(state.flip_horizontal);
        saved.preview_flip_vertical = Some(state.flip_vertical);
        saved.preview_fit_to_window = Some(state.fit_to_window);
        saved.preview_zoom_scale = Some(state.zoom_scale);
        saved.preview_pan_x = Some(state.pan_x);
        saved.preview_pan_y = Some(state.pan_y);
        saved.preview_zoom_at_cursor = Some(state.zoom_at_cursor);
        saved.preview_show_info = Some(state.show_info);
        if let Err(error) = session.replace(saved) {
            eprintln!("preview transform session save failed: {error}");
        }
    }

    fn save_preview_path(&self, path: Option<PathBuf>) {
        let Some(session) = &self.session else { return };
        if let Ok(mut saved) = session.snapshot() {
            saved.preview_path = path.map(|path| path.to_string_lossy().into_owned());
            if let Err(error) = session.replace(saved) {
                eprintln!("preview session save failed: {error}");
            }
        }
    }
}
