//! Indexed filmstrip identities and existing thumbnail paths shared with Gallery.
use super::{PreviewMediaKind, PreviewService, classify_media_path};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct FilmstripMedia {
    pub path: PathBuf,
    pub media_id: i64,
    pub thumbnail: Option<PathBuf>,
}
impl From<&photag_plugin_database::db::queries::MediaItem> for FilmstripMedia {
    fn from(media: &photag_plugin_database::db::queries::MediaItem) -> Self {
        Self {
            path: media.full_path.clone().into(),
            media_id: media.id,
            thumbnail: media.thumb_path.as_ref().map(PathBuf::from),
        }
    }
}

impl PreviewService {
    /// Ordered indexed items; filesystem freshness checks happen in decode workers.
    pub fn set_indexed_filmstrip_items(&self, items: impl IntoIterator<Item = FilmstripMedia>) {
        let items: Vec<_> = items.into_iter().collect();
        let next: std::collections::HashMap<_, _> = items
            .iter()
            .filter_map(|m| m.thumbnail.clone().map(|p| (m.path.clone(), p)))
            .collect();
        let changed = if let Ok(mut thumbs) = self.indexed_thumbnails.lock() {
            let changed: std::collections::HashSet<_> = thumbs
                .keys()
                .chain(next.keys())
                .filter(|source| thumbs.get(*source) != next.get(*source))
                .cloned()
                .collect();
            *thumbs = next;
            changed
        } else {
            Default::default()
        };
        if let Ok(mut ready) = self.ready_decodes.lock() {
            ready.retain(|(source, dim), _| *dim != 96 || !changed.contains(source));
        }
        if let Ok(mut ids) = self.filmstrip_media_ids.lock() {
            *ids = items.iter().map(|m| (m.path.clone(), m.media_id)).collect();
        }
        if let Ok(mut strip) = self.filmstrip.lock() {
            *strip = items.into_iter().map(|m| m.path).collect();
        }
        self.prewarm_filmstrip_neighborhood();
    }

    pub(super) fn indexed_thumbnail(&self, source: &Path) -> Option<PathBuf> {
        self.indexed_thumbnails.lock().ok()?.get(source).cloned()
    }

    pub(super) fn prewarm_filmstrip_neighborhood(&self) {
        let Some(current) = self.snapshot().source else {
            return;
        };
        let paths = self.filmstrip();
        if let Some(index) = paths.iter().position(|p| p == &current) {
            for path in paths.iter().skip(index.saturating_sub(3)).take(7) {
                if classify_media_path(path) == PreviewMediaKind::Image {
                    self.prewarm_image(path, 96);
                }
            }
        } else if classify_media_path(&current) == PreviewMediaKind::Image {
            self.prewarm_image(&current, 96);
        }
    }

    pub(super) fn filmstrip_cache_status(&self) -> serde_json::Value {
        let current = self.snapshot().source;
        let paths = self.filmstrip();
        let nearby = current
            .as_ref()
            .and_then(|source| paths.iter().position(|p| p == source))
            .map(|index| {
                paths
                    .iter()
                    .skip(index.saturating_sub(3))
                    .take(7)
                    .map(|source| {
                        let supplied = self.indexed_thumbnail(source);
                        serde_json::json!({"source":source,"indexed_thumbnail":supplied,
                    "display_thumbnail":self.filmstrip_thumbnail(source)})
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        serde_json::json!({"progressive":"jpeg_if_pending_after_150ms", "nearby":nearby,
            "pending":self.pending_decodes.lock().map(|p|p.len()).unwrap_or(0)})
    }
}
