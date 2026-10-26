//! Native ImageFn adapter: keyed background reads, linked sources, cached failures.
use crate::store::ImagePreviewCandidate;
use crate::store::{ImagePreview, WhiteboardService};
use fileatlas_canvas_engine::ImageRenderSources;
use fileatlas_plugin_eagle_links::{EagleImageSource, EagleLinkService};
use fileatlas_plugin_media_resources::MediaResourceService;
use fileatlas_plugin_thumbnails::ThumbnailService;
use fileatlas_ui_bridge::{read_model::ReadModel, refresh::Wake};
use gpui::RenderImage;
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

// Choose source quality by the image's full physical on-screen area. Viewport
// clipping controls whether it is painted, not which resolution is selected.
const ORIGINAL_UPGRADE_SCREEN_PIXELS: f32 = 500_000.0;
const THUMBNAIL_DOWNGRADE_SCREEN_PIXELS: f32 = 350_000.0;
const DOWNGRADE_SETTLE: Duration = Duration::from_millis(350);
const QUALITY_FADE: Duration = Duration::from_millis(180);
const ORIGINAL_CACHE_TTL: Duration = Duration::from_secs(30);
const ORIGINAL_IDLE_CACHE_BYTES: usize = 128 * 1024 * 1024;

#[derive(Clone, Debug, Default, Serialize)]
pub struct ImageCacheStats {
    pub board_id: Option<i64>,
    pub thumbnail_entries: usize,
    pub original_entries: usize,
    pub thumbnail_decoded_images: usize,
    pub original_decoded_images: usize,
    pub thumbnail_decoded_bytes: usize,
    pub original_decoded_bytes: usize,
    pub visible_images: usize,
    pub idle_original_bytes: usize,
    pub oldest_idle_original_age_ms: u64,
    pub thumbnail_quality_images: usize,
    pub original_quality_images: usize,
    pub active_transitions: usize,
    pub pending_downgrades: usize,
    pub original_requests: u64,
    pub original_memory_hits: u64,
    pub original_decode_requests: u64,
    pub original_ttl_evictions: u64,
    pub original_budget_evictions: u64,
    pub original_cache_ttl_ms: u64,
    pub idle_cache_budget_bytes: usize,
    pub downgrade_debounce_ms: u64,
    pub crossfade_ms: u64,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum Quality {
    Thumbnail,
    Original,
}
impl Default for Quality {
    fn default() -> Self {
        Self::Thumbnail
    }
}
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Key {
    source: String,
    quarter: i32,
    quality: Quality,
}
#[derive(Clone)]
struct Preview {
    path: PathBuf,
}
struct DecodedImage {
    image: Arc<RenderImage>,
    preview: Option<Preview>,
    byte_len: usize,
}
struct Transition {
    from: Arc<RenderImage>,
    to: Arc<RenderImage>,
    to_quality: Quality,
    target: Quality,
    started: Instant,
    start_opacity: f32,
    end_opacity: f32,
}
#[derive(Default)]
struct ImageState {
    stable: Quality,
    transition: Option<Transition>,
    downgrade_since: Option<Instant>,
    downgrade_ready: bool,
}
pub struct ImageCache {
    models: HashMap<Key, ReadModel<Key, Arc<DecodedImage>>>,
    original_last_used: HashMap<Key, Instant>,
    visible_originals: HashSet<(String, i32)>,
    thumbnails: Arc<ThumbnailService>,
    resources: Arc<MediaResourceService>,
    service: Arc<WhiteboardService>,
    board_id: Option<i64>,
    previews: HashMap<String, Preview>,
    states: HashMap<(String, i32), ImageState>,
    pub wake: Wake,
    stats: Arc<Mutex<ImageCacheStats>>,
}
impl ImageCache {
    pub fn new(
        thumbnails: Arc<ThumbnailService>,
        resources: Arc<MediaResourceService>,
        service: Arc<WhiteboardService>,
        stats: Arc<Mutex<ImageCacheStats>>,
    ) -> Self {
        Self {
            models: HashMap::new(),
            original_last_used: HashMap::new(),
            visible_originals: HashSet::new(),
            thumbnails,
            resources,
            service,
            board_id: None,
            previews: HashMap::new(),
            states: HashMap::new(),
            wake: Wake::default(),
            stats,
        }
    }
    pub fn set_board(&mut self, id: i64, previews: &[ImagePreview]) {
        self.clear();
        self.board_id = Some(id);
        self.update_stats();
        for item in previews {
            if let Some(path) = self.service.image_preview_path(id, item) {
                self.previews.insert(item.uri.clone(), Preview { path });
            }
        }
    }
    pub fn add_previews(&mut self, id: i64, previews: &[ImagePreview]) {
        if self.board_id != Some(id) {
            return;
        }
        for item in previews {
            if let Some(path) = self.service.image_preview_path(id, item) {
                self.previews.insert(item.uri.clone(), Preview { path });
            }
        }
    }
    pub fn get(
        &mut self,
        source: &str,
        rotation: f32,
        full_screen_pixels: [f32; 2],
    ) -> Option<ImageRenderSources> {
        // Upstream snaps images to quarter turns before calling ImageFn.
        let quarter = (rotation / std::f32::consts::FRAC_PI_2).round() as i32;
        let quarter = quarter.rem_euclid(4);
        let state_key = (source.to_owned(), quarter);
        let thumbnail = self.request(source, quarter, Quality::Thumbnail);
        let full_screen_area = full_screen_pixels[0].max(0.0) * full_screen_pixels[1].max(0.0);
        self.states.entry(state_key.clone()).or_default();
        let (stable_quality, transition_quality) = self
            .states
            .get(&state_key)
            .map(|state| {
                (
                    state.stable,
                    state
                        .transition
                        .as_ref()
                        .map(|transition| transition.target),
                )
            })
            .unwrap_or((Quality::Thumbnail, None));
        let current_quality = transition_quality.unwrap_or(stable_quality);
        let mut desired = match current_quality {
            Quality::Thumbnail if full_screen_area >= ORIGINAL_UPGRADE_SCREEN_PIXELS => {
                Quality::Original
            }
            quality => quality,
        };
        if current_quality == Quality::Original
            && full_screen_area <= THUMBNAIL_DOWNGRADE_SCREEN_PIXELS
        {
            let now = Instant::now();
            if let Some(state) = self.states.get_mut(&state_key) {
                match state.downgrade_since {
                    Some(since) if now.saturating_duration_since(since) >= DOWNGRADE_SETTLE => {
                        state.downgrade_ready = true;
                        desired = Quality::Thumbnail;
                    }
                    Some(_) => {}
                    None => {
                        state.downgrade_since = Some(now);
                        state.downgrade_ready = false;
                        self.wake.trigger();
                    }
                }
            }
        } else if let Some(state) = self.states.get_mut(&state_key) {
            state.downgrade_since = None;
            state.downgrade_ready = false;
        }
        let original = (thumbnail.is_some()
            && (desired == Quality::Original
                || stable_quality == Quality::Original
                || transition_quality == Some(Quality::Original)))
        .then(|| self.request(source, quarter, Quality::Original))
        .flatten();

        if let Some(state) = self.states.get_mut(&state_key) {
            if let Some(transition) = state.transition.as_mut() {
                let now = Instant::now();
                let progress = (now
                    .saturating_duration_since(transition.started)
                    .as_secs_f32()
                    / QUALITY_FADE.as_secs_f32())
                .clamp(0.0, 1.0);
                let opacity = transition.start_opacity
                    + (transition.end_opacity - transition.start_opacity) * smoothstep(progress);
                if desired != transition.target {
                    transition.start_opacity = opacity;
                    transition.end_opacity = if desired == transition.to_quality {
                        1.0
                    } else {
                        0.0
                    };
                    transition.target = desired;
                    transition.started = now;
                }
                return Some(ImageRenderSources {
                    base: transition.from.clone().into(),
                    overlay: Some((transition.to.clone().into(), opacity)),
                });
            }
        }

        let (stable, target) = match desired {
            Quality::Thumbnail => (thumbnail.clone(), thumbnail.clone()),
            Quality::Original => (
                original.clone().or_else(|| thumbnail.clone()),
                original.clone(),
            ),
        };
        if let (Some(target), Some(state)) = (target, self.states.get_mut(&state_key)) {
            if desired != state.stable {
                if let Some(from) = match state.stable {
                    Quality::Thumbnail => thumbnail.clone(),
                    Quality::Original => original.clone(),
                }
                .or(stable.clone())
                {
                    state.transition = Some(Transition {
                        from,
                        to: target,
                        to_quality: desired,
                        target: desired,
                        started: Instant::now(),
                        start_opacity: 0.0,
                        end_opacity: 1.0,
                    });
                    self.wake.trigger();
                    if let Some(transition) = &state.transition {
                        return Some(ImageRenderSources {
                            base: transition.from.clone().into(),
                            overlay: Some((transition.to.clone().into(), 0.0)),
                        });
                    }
                } else {
                    state.stable = desired;
                }
            }
        }
        if desired == Quality::Original {
            original.or(thumbnail)
        } else {
            thumbnail.or(original)
        }
        .map(|image| ImageRenderSources {
            base: image.into(),
            overlay: None,
        })
    }
    pub fn begin_visibility_frame(&mut self) {
        self.visible_originals.clear();
    }
    pub fn update_visibility(&mut self, source: &str, rotation: f32, visible: bool) {
        let quarter = ((rotation / std::f32::consts::FRAC_PI_2).round() as i32).rem_euclid(4);
        if visible {
            self.visible_originals.insert((source.to_owned(), quarter));
        }
    }
    fn request(
        &mut self,
        source: &str,
        quarter: i32,
        quality: Quality,
    ) -> Option<Arc<RenderImage>> {
        let key = Key {
            source: source.to_owned(),
            quarter,
            quality,
        };
        if quality == Quality::Original {
            self.original_last_used.insert(key.clone(), Instant::now());
            let exists = self.models.contains_key(&key);
            let hit = self
                .models
                .get(&key)
                .and_then(|model| model.value())
                .is_some_and(|result| result.is_ok());
            let mut stats = self
                .stats
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            stats.original_requests = stats.original_requests.saturating_add(1);
            if hit {
                stats.original_memory_hits = stats.original_memory_hits.saturating_add(1);
            } else if !exists {
                stats.original_decode_requests = stats.original_decode_requests.saturating_add(1);
            }
        }
        let cached_preview = self.previews.get(source).cloned();
        let board_id = self.board_id;
        let model = self.models.entry(key.clone()).or_insert_with(|| {
            let mut model = ReadModel::default();
            model.set_wake(self.wake.clone());
            model
        });
        let uri = source.to_owned();
        let thumbnails = Arc::clone(&self.thumbnails);
        let resources = Arc::clone(&self.resources);
        let service = Arc::clone(&self.service);
        let turn = quarter;
        model.request(key, move || {
            let source_path = resources
                .resolve(&uri)?
                .local_path
                .ok_or("image resource has no local preview source")?;
            let (path, preview) = if quality == Quality::Original {
                (source_path, None)
            } else if let Some(preview) = cached_preview.filter(|preview| preview.path.is_file()) {
                (preview.path.clone(), Some(preview))
            } else {
                let Some((cached_path, _, _, _)) = thumbnails.generate_image(&source_path) else {
                    return Err(format!("{uri}: could not create thumbnail"));
                };
                let path = PathBuf::from(cached_path);
                let (width, height) = image::image_dimensions(&path).map_err(|e| e.to_string())?;
                let mut preview = Preview { path: path.clone() };
                if let Some(id) = board_id {
                    let candidate = ImagePreviewCandidate {
                        uri: uri.clone(),
                        cached_path: path.clone(),
                        width,
                        height,
                    };
                    match service.persist_image_preview(id, candidate) {
                        Ok(Some(saved)) => {
                            if let Some(saved_path) = service.image_preview_path(id, &saved) {
                                preview = Preview { path: saved_path };
                            }
                        }
                        Ok(None) => {}
                        Err(error) => {
                            tracing::warn!(%error, %uri, "canvas thumbnail persistence failed")
                        }
                    }
                }
                (path, Some(preview))
            };
            let decoded = image::open(&path).map_err(|e| format!("{uri}: {e}"))?;
            let rotated = match turn {
                1 => decoded.rotate90(),
                2 => decoded.rotate180(),
                3 => decoded.rotate270(),
                _ => decoded,
            };
            let pixels = rotated.into_rgba8();
            let byte_len = pixels.as_raw().len();
            Ok(Arc::new(DecodedImage {
                image: device_image(pixels),
                preview,
                byte_len,
            }))
        });
        model
            .value()
            .and_then(|r| r.as_ref().ok())
            .map(|image| Arc::clone(&image.image))
    }
    pub fn poll(&mut self) -> bool {
        let mut changed = self
            .models
            .values_mut()
            .fold(false, |changed, model| model.poll() | changed);
        for (key, model) in &self.models {
            if key.quality != Quality::Thumbnail {
                continue;
            }
            if let Some(Ok(image)) = model.value()
                && let Some(preview) = &image.preview
            {
                self.previews.insert(key.source.clone(), preview.clone());
            }
        }
        let now = Instant::now();
        for (source, quarter) in &self.visible_originals {
            let key = Key {
                source: source.clone(),
                quarter: *quarter,
                quality: Quality::Original,
            };
            if self.models.contains_key(&key) {
                self.original_last_used.insert(key, now);
            }
        }
        for state in self.states.values_mut() {
            if !state.downgrade_ready
                && state
                    .downgrade_since
                    .is_some_and(|since| now.saturating_duration_since(since) >= DOWNGRADE_SETTLE)
            {
                state.downgrade_ready = true;
                changed = true;
            }
        }
        for state in self.states.values_mut() {
            let Some(transition) = &state.transition else {
                continue;
            };
            if now.saturating_duration_since(transition.started) >= QUALITY_FADE {
                state.stable = transition.target;
                state.transition = None;
                if state.stable == Quality::Thumbnail {
                    state.downgrade_since = None;
                    state.downgrade_ready = false;
                }
                changed = true;
            } else {
                changed = true;
            }
        }
        let mut idle_originals = Vec::new();
        for (key, model) in &self.models {
            if key.quality != Quality::Original {
                continue;
            }
            let last_used = self.original_last_used.get(key).copied().unwrap_or(now);
            if now.saturating_duration_since(last_used) >= ORIGINAL_CACHE_TTL {
                idle_originals.push((key.clone(), last_used, 0));
            } else if now.saturating_duration_since(last_used) >= Duration::from_secs(1) {
                let bytes = model
                    .value()
                    .and_then(|result| result.as_ref().ok())
                    .map_or(0, |image| image.byte_len);
                idle_originals.push((key.clone(), last_used, bytes));
            }
        }
        let mut idle_bytes = idle_originals
            .iter()
            .map(|(_, _, bytes)| bytes)
            .sum::<usize>();
        idle_originals.sort_by_key(|(_, last_used, _)| *last_used);
        let mut evict = Vec::new();
        for (key, last_used, bytes) in idle_originals {
            let expired = now.saturating_duration_since(last_used) >= ORIGINAL_CACHE_TTL;
            if expired || idle_bytes > ORIGINAL_IDLE_CACHE_BYTES {
                idle_bytes = idle_bytes.saturating_sub(bytes);
                evict.push((key, expired));
            }
        }
        for (key, expired) in evict {
            self.models.remove(&key);
            self.original_last_used.remove(&key);
            let mut stats = self
                .stats
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if expired {
                stats.original_ttl_evictions = stats.original_ttl_evictions.saturating_add(1);
            } else {
                stats.original_budget_evictions = stats.original_budget_evictions.saturating_add(1);
            }
            drop(stats);
            if let Some(state) = self.states.get_mut(&(key.source.clone(), key.quarter))
                && state.transition.is_none()
                && state.stable == Quality::Original
            {
                state.stable = Quality::Thumbnail;
                changed = true;
            }
        }
        self.update_stats();
        changed
    }
    pub fn has_active_transitions(&self) -> bool {
        self.states.values().any(|state| state.transition.is_some())
    }
    pub fn has_pending_downgrades(&self) -> bool {
        self.states
            .values()
            .any(|state| state.downgrade_since.is_some())
    }
    pub fn has_cached_originals(&self) -> bool {
        self.models
            .keys()
            .any(|key| key.quality == Quality::Original)
    }
    fn update_stats(&self) {
        let now = Instant::now();
        let mut snapshot = ImageCacheStats {
            board_id: self.board_id,
            visible_images: self.visible_originals.len(),
            active_transitions: self
                .states
                .values()
                .filter(|state| state.transition.is_some())
                .count(),
            pending_downgrades: self
                .states
                .values()
                .filter(|state| state.downgrade_since.is_some())
                .count(),
            original_cache_ttl_ms: ORIGINAL_CACHE_TTL.as_millis() as u64,
            idle_cache_budget_bytes: ORIGINAL_IDLE_CACHE_BYTES,
            downgrade_debounce_ms: DOWNGRADE_SETTLE.as_millis() as u64,
            crossfade_ms: QUALITY_FADE.as_millis() as u64,
            ..ImageCacheStats::default()
        };
        for state in self.states.values() {
            let quality = state
                .transition
                .as_ref()
                .map_or(state.stable, |transition| transition.target);
            match quality {
                Quality::Thumbnail => snapshot.thumbnail_quality_images += 1,
                Quality::Original => snapshot.original_quality_images += 1,
            }
        }
        for (key, model) in &self.models {
            let (entries, decoded_images, decoded_bytes) = if key.quality == Quality::Original {
                (
                    &mut snapshot.original_entries,
                    &mut snapshot.original_decoded_images,
                    &mut snapshot.original_decoded_bytes,
                )
            } else {
                (
                    &mut snapshot.thumbnail_entries,
                    &mut snapshot.thumbnail_decoded_images,
                    &mut snapshot.thumbnail_decoded_bytes,
                )
            };
            *entries += 1;
            if let Some(Ok(image)) = model.value() {
                *decoded_images += 1;
                *decoded_bytes = decoded_bytes.saturating_add(image.byte_len);
                if key.quality == Quality::Original
                    && !self
                        .visible_originals
                        .contains(&(key.source.clone(), key.quarter))
                    && let Some(last_used) = self.original_last_used.get(key)
                {
                    let age = now.saturating_duration_since(*last_used);
                    if age >= Duration::from_secs(1) {
                        snapshot.idle_original_bytes =
                            snapshot.idle_original_bytes.saturating_add(image.byte_len);
                        snapshot.oldest_idle_original_age_ms = snapshot
                            .oldest_idle_original_age_ms
                            .max(age.as_millis() as u64);
                    }
                }
            }
        }
        let mut stats = self
            .stats
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        snapshot.original_requests = stats.original_requests;
        snapshot.original_memory_hits = stats.original_memory_hits;
        snapshot.original_decode_requests = stats.original_decode_requests;
        snapshot.original_ttl_evictions = stats.original_ttl_evictions;
        snapshot.original_budget_evictions = stats.original_budget_evictions;
        *stats = snapshot;
    }
    pub fn errors(&self) -> Vec<String> {
        self.models
            .values()
            .filter_map(|m| m.value().and_then(|v| v.as_ref().err()).cloned())
            .collect()
    }
    pub fn clear(&mut self) {
        self.models.clear();
        self.original_last_used.clear();
        self.visible_originals.clear();
        self.states.clear();
        self.previews.clear();
        self.board_id = None;
        *self
            .stats
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = ImageCacheStats::default();
    }
}
fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}
/// GPUI 0.3.8's RenderImage device contract is BGRA; decoders return RGBA.
fn device_image(mut pixels: image::RgbaImage) -> Arc<RenderImage> {
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    Arc::new(RenderImage::new(vec![image::Frame::new(pixels)]))
}
pub fn linked_images(
    paths: Vec<PathBuf>,
    thumbnails: &ThumbnailService,
    resources: &MediaResourceService,
    eagle_links: Option<&EagleLinkService>,
) -> Result<Vec<LinkedImage>, String> {
    paths
        .into_iter()
        .map(|path| {
            let link = resources.link_file(&path)?;
            if link.kind != fileatlas_contracts::resources::ResourceKind::Image {
                return Err("请选择图片文件".into());
            }
            let eagle_source = eagle_links
                .map(|service| service.source_for_dropped_image(&path))
                .transpose()?
                .flatten();
            let source = resources
                .resolve(&link.uri)?
                .local_path
                .ok_or("image resource has no local source")?;
            let generated = thumbnails.generate_image(&source);
            let dimensions = generated
                .as_ref()
                .map(|(_, width, height, _)| (*width, *height))
                .or_else(|| image::image_dimensions(&source).ok())
                .ok_or_else(|| format!("{}: could not read image dimensions", source.display()))?;
            let preview = generated.and_then(|(cached_path, _, _, _)| {
                let cached_path = PathBuf::from(cached_path);
                image::image_dimensions(&cached_path)
                    .ok()
                    .map(|(width, height)| ImagePreviewCandidate {
                        uri: link.uri.clone(),
                        cached_path,
                        width,
                        height,
                    })
            });
            Ok(LinkedImage {
                uri: link.uri,
                width: dimensions.0,
                height: dimensions.1,
                eagle_source,
                preview,
            })
        })
        .collect()
}

pub struct LinkedImage {
    pub uri: String,
    pub width: u32,
    pub height: u32,
    pub eagle_source: Option<EagleImageSource>,
    pub preview: Option<ImagePreviewCandidate>,
}

#[cfg(test)]
mod tests {
    #[test]
    fn image_callback_supplies_native_device_channels_without_changing_alpha() {
        let source = image::RgbaImage::from_pixel(1, 1, image::Rgba([10, 130, 250, 77]));
        let rendered = super::device_image(source);
        assert_eq!(rendered.as_bytes(0), Some([250u8, 130, 10, 77].as_slice()));
    }
}
