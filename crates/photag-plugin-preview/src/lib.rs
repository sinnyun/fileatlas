//! Source-compatible image preview state, non-destructive transforms, and a GPUI preview surface.
//!
//! The source preview owns transient view state rather than changing media files. This crate
//! follows that rule: transformed rasters are materialized under a cache directory and the
//! original path remains the identity used by callers.

mod ai_suggestions;
mod chrome;
mod decode;
mod info;
mod plugin;
mod snapshot;
mod transform_session;
mod view;

use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use cordis::Service;
use photag_plugin_database::DatabaseService;
use photag_plugin_media_playback::MediaPlaybackService;
use photag_plugin_session::SessionService;

pub use plugin::{PreviewConfig, PreviewError, PreviewPlugin};
pub(crate) use snapshot::classify_media_path;
pub use snapshot::{PreviewMediaKind, PreviewSnapshot};
pub use view::preview_view;

/// Cordis service for opening and controlling image previews.
pub struct PreviewService {
    state: Mutex<PreviewSnapshot>,
    cache_dir: PathBuf,
    session: Option<Arc<SessionService>>,
    filmstrip: Mutex<Vec<PathBuf>>,
    filmstrip_media_ids: Mutex<std::collections::HashMap<PathBuf, i64>>,
    decode_queue: Arc<decode::DecodeQueue>,
    pending_decodes: Arc<Mutex<HashSet<(PathBuf, u32)>>>,
    decode_revision: Arc<AtomicU64>,
    database: Mutex<Option<Arc<DatabaseService>>>,
    last_opened_id: Mutex<Option<i64>>,
    playback: Mutex<Option<Arc<MediaPlaybackService>>>,
    wheel_animation: Mutex<decode::WheelAnimation>,
    last_activity: Mutex<std::time::Instant>,
}

impl Service for PreviewService {
    const NAME: &'static str = "photag/preview";
}

impl PreviewService {
    pub fn open_at(cache_dir: impl Into<PathBuf>) -> std::io::Result<Self> {
        let cache_dir = cache_dir.into();
        std::fs::create_dir_all(&cache_dir)?;
        let decode_queue = Arc::new(decode::DecodeQueue::default());
        let pending_decodes = Arc::new(Mutex::new(HashSet::new()));
        let decode_revision = Arc::new(AtomicU64::new(0));
        decode::start_decode_workers(
            Arc::clone(&decode_queue),
            Arc::clone(&pending_decodes),
            Arc::clone(&decode_revision),
        );
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
            wheel_animation: Mutex::new(decode::WheelAnimation {
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
            self.set_zoom_scale(percent as f32);
        }
    }

    /// Set a continuous toolbar zoom percentage while keeping the persisted
    /// transform and public percentage snapshot in sync.
    pub fn set_zoom_scale(&self, percent: f32) {
        if !percent.is_finite() {
            return;
        }
        let percent = percent.clamp(5.0, 1600.0);
        self.change(|snapshot| {
            snapshot.fit_to_window = false;
            snapshot.zoom_percent = percent.round() as u16;
            snapshot.zoom_scale = percent / 100.0;
        });
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
}
