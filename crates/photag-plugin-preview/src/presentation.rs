//! Tracks image paint completion, rather than just the open command returning.
use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use serde_json::{Value, json};

use super::PreviewService;

pub(super) struct LoadTracker {
    generation: u64,
    source: Option<PathBuf>,
    media_id: Option<i64>,
    started: Instant,
    thumbnail: Option<PathBuf>,
    first_frame_ms: Option<f64>,
    first_frame_quality: Option<&'static str>,
    full_frame_ms: Option<f64>,
    quality: Option<&'static str>,
    dimensions: Option<(u32, u32)>,
    error: Option<String>,
    renders: u64,
    poll_wakes: std::collections::BTreeMap<&'static str, u64>,
    activity_count: u64,
    activity_caller: Option<(&'static str, u32)>,
    pointer_position: photag_ui_bridge::refresh::ChangeGate<(f32, f32)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_full_completion_stops_deferred_work_and_old_requests_cannot_cancel_new_ones() {
        let directory =
            std::env::temp_dir().join(format!("photag-progressive-policy-{}", std::process::id()));
        let service = PreviewService::open_at(&directory).unwrap();
        service.begin_image_load(Path::new("first.jpg"), Some(1));
        let first = service.presentation_source().0;
        assert!(service.pending_progressive_source().is_none());
        let old_request = service.decode_control.lock().unwrap().cancelled.clone();
        service.begin_image_load(Path::new("second.jpg"), Some(2));
        assert!(old_request.load(std::sync::atomic::Ordering::Acquire));
        service.load_tracker.lock().unwrap().started -= std::time::Duration::from_secs(1);
        assert_eq!(
            service.pending_progressive_source(),
            Some("second.jpg".into())
        );
        service.image_presented(first, "full", (100, 100));
        assert!(
            !service
                .decode_control
                .lock()
                .unwrap()
                .full_ready
                .load(std::sync::atomic::Ordering::Acquire)
        );
        let second = service.presentation_source().0;
        service.image_presented(second, "full", (100, 100));
        assert!(service.pending_progressive_source().is_none());
        assert!(
            service
                .decode_control
                .lock()
                .unwrap()
                .full_ready
                .load(std::sync::atomic::Ordering::Acquire)
        );
        service.close();
        assert!(
            service
                .decode_control
                .lock()
                .unwrap()
                .cancelled
                .load(std::sync::atomic::Ordering::Acquire)
        );
        drop(service);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn late_paints_cannot_complete_a_different_media_request_or_downgrade_full() {
        let directory =
            std::env::temp_dir().join(format!("photag-paint-generation-{}", std::process::id()));
        let service = PreviewService::open_at(&directory).unwrap();
        service.begin_image_load(Path::new("first.jpg"), Some(1));
        let first = service.presentation_source().0;
        service.begin_image_load(Path::new("second.jpg"), Some(2));
        service.image_presented(first, "full", (100, 100));
        assert!(service.load_status()["first_frame_ms"].is_null());
        let second = service.presentation_source().0;
        service.image_presented(second, "thumbnail", (96, 64));
        service.image_presented(second, "full", (6000, 4000));
        let full = service.load_status();
        service.image_presented(second, "thumbnail", (96, 64));
        assert_eq!(service.load_status()["quality"], "full");
        assert_eq!(
            service.load_status()["full_frame_ms"],
            full["full_frame_ms"]
        );
        drop(service);
        let _ = std::fs::remove_dir_all(directory);
    }
}

impl Default for LoadTracker {
    fn default() -> Self {
        Self {
            generation: 0,
            source: None,
            media_id: None,
            started: Instant::now(),
            thumbnail: None,
            first_frame_ms: None,
            first_frame_quality: None,
            full_frame_ms: None,
            quality: None,
            dimensions: None,
            error: None,
            renders: 0,
            poll_wakes: Default::default(),
            activity_count: 0,
            activity_caller: None,
            pointer_position: Default::default(),
        }
    }
}

impl PreviewService {
    pub(super) fn begin_image_load(&self, source: &Path, media_id: Option<i64>) {
        if let Ok(mut ready) = self.ready_decodes.lock() {
            ready.retain(|(path, _), _| path != source);
        }
        if let Ok(mut tracker) = self.load_tracker.lock() {
            self.reset_decode_request();
            let generation = tracker.generation.wrapping_add(1);
            *tracker = LoadTracker {
                generation,
                source: Some(source.to_owned()),
                media_id,
                thumbnail: self.indexed_thumbnail(source),
                ..LoadTracker::default()
            };
        }
    }

    pub(super) fn pending_progressive_source(&self) -> Option<PathBuf> {
        let tracker = self.load_tracker.lock().ok()?;
        (tracker.full_frame_ms.is_none()
            && tracker.started.elapsed() >= std::time::Duration::from_millis(150))
        .then(|| tracker.source.clone())
        .flatten()
    }

    /// Supply the gallery's existing thumbnail before any original-image decode.
    pub fn open_indexed_media(
        &self,
        media: &photag_plugin_database::db::queries::MediaItem,
    ) -> Result<(), crate::PreviewError> {
        if let Ok(mut thumbs) = self.indexed_thumbnails.lock() {
            if let Some(path) = &media.thumb_path {
                thumbs.insert(media.full_path.clone().into(), path.into());
            } else {
                thumbs.remove(Path::new(&media.full_path));
            }
        }
        self.open_media(&media.full_path, Some(media.id))?;
        Ok(())
    }

    pub(super) fn presentation_source(&self) -> (u64, Option<PathBuf>) {
        let (generation, source, thumbnail) = self
            .load_tracker
            .lock()
            .map(|tracker| {
                (
                    tracker.generation,
                    tracker.source.clone(),
                    tracker.thumbnail.clone(),
                )
            })
            .unwrap_or_default();
        let ready = source
            .as_deref()
            .and_then(|path| self.filmstrip_thumbnail(path));
        (generation, ready.or(thumbnail))
    }

    pub(super) fn note_image_render(&self) {
        if let Ok(mut tracker) = self.load_tracker.lock() {
            tracker.renders += 1;
        }
    }

    pub(super) fn note_poll_wake(&self, reason: &'static str) {
        if let Ok(mut tracker) = self.load_tracker.lock() {
            *tracker.poll_wakes.entry(reason).or_default() += 1;
        }
    }

    pub(super) fn note_activity(&self, caller: &'static std::panic::Location<'static>) {
        if let Ok(mut tracker) = self.load_tracker.lock() {
            tracker.activity_count += 1;
            tracker.activity_caller = Some((caller.file(), caller.line()));
        }
    }

    #[track_caller]
    pub(super) fn mark_pointer_activity(&self, x: f32, y: f32) {
        let moved = self.load_tracker.lock().is_ok_and(|mut tracker| {
            if !tracker.pointer_position.changed((x, y)) {
                return false;
            }
            true
        });
        // Windows/GPUI can resend stationary coordinates after a hover/layout
        // change. Such observations must not restart the chrome fade timer.
        if moved {
            self.mark_activity();
        }
    }

    /// Called by a paint marker after the ready image element has painted.
    pub(super) fn image_presented(
        &self,
        generation: u64,
        quality: &'static str,
        dimensions: (u32, u32),
    ) {
        let Ok(mut tracker) = self.load_tracker.lock() else {
            return;
        };
        if tracker.generation != generation {
            return;
        }
        let elapsed = tracker.started.elapsed().as_secs_f64() * 1000.;
        if tracker.first_frame_ms.is_none() {
            tracker.first_frame_ms = Some(elapsed);
            tracker.first_frame_quality = Some(quality);
            tracing::info!(target: "photag_plugin_preview", action="first_frame_presented", source=?tracker.source, generation, dimensions=?dimensions, duration_scope="open_to_paint", duration_ms=elapsed, media_id=?tracker.media_id, quality, "preview image first paint");
        }
        if quality == "full" && tracker.full_frame_ms.is_none() {
            tracker.full_frame_ms = Some(elapsed);
            self.mark_full_image_ready();
            tracing::info!(target: "photag_plugin_preview", action="full_frame_presented", source=?tracker.source, generation, dimensions=?dimensions, duration_scope="open_to_paint", duration_ms=elapsed, media_id=?tracker.media_id, "preview image full paint");
        }
        if tracker.full_frame_ms.is_none() || quality == "full" {
            tracker.quality = Some(quality);
            tracker.dimensions = Some(dimensions);
        }
    }

    pub(super) fn image_load_failed(&self, generation: u64, error: String) {
        if let Ok(mut tracker) = self.load_tracker.lock() {
            if tracker.generation == generation && tracker.error.as_ref() != Some(&error) {
                tracing::error!(target: "photag_plugin_preview", source=?tracker.source, generation, media_id=?tracker.media_id, %error, "preview image loading failed");
                tracker.error = Some(error);
            }
        }
    }

    /// Read-only same-process diagnostics; paints and timings are independently visible.
    pub fn load_status(&self) -> Value {
        let mut status = self.load_tracker.lock().map(|tracker| json!({
            "generation": tracker.generation, "source": tracker.source,
            "media_id": tracker.media_id, "quality": tracker.quality,
            "first_frame_ms": tracker.first_frame_ms, "full_frame_ms": tracker.full_frame_ms,
            "first_frame_quality": tracker.first_frame_quality,
            "dimensions": tracker.dimensions, "error": tracker.error,
            "render_count": tracker.renders,
            "poll_wakes": tracker.poll_wakes,
            "activity": {"count": tracker.activity_count, "caller": tracker.activity_caller, "pointer": tracker.pointer_position.current()},
        })).unwrap_or(Value::Null);
        status["cache_policy"] = self.filmstrip_cache_status();
        status
    }
}
