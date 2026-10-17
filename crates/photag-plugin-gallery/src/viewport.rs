//! Bounded viewport demand: no queries or decoding in a GPUI render callback.
use super::*;
impl GalleryService {
    pub fn ids_for_paths(&self, paths: &[String]) -> Result<Vec<i64>, String> {
        self.database
            .ids_for_paths(paths)
            .map_err(|e| e.to_string())
    }

    pub(crate) fn prepare_window(
        &self,
        ids: Vec<i64>,
        visible: Vec<i64>,
        offset: usize,
        generation: u64,
        request: u64,
    ) -> Result<GalleryWindow, String> {
        let (mut media, _) = self
            .database
            .get_media_window(&ids)
            .map_err(|e| e.to_string())?;
        for item in &mut media {
            if self.database.is_browser() {
                if self.live.status().0 != generation
                    || self.viewport_request.load(Ordering::Acquire) != request
                {
                    return Err("Obsolete directory window".into());
                }
                let before = source_version(&item.full_path);
                if let Some(version) = &before {
                    if self
                        .database
                        .file_version_changed(&item.full_path, version)
                        .map_err(|e| e.to_string())?
                    {
                        item.thumb_path = None;
                        item.width = None;
                        item.height = None;
                        item.micro_thumb = None;
                    }
                }
                if item
                    .thumb_path
                    .as_ref()
                    .is_some_and(|p| !std::path::Path::new(p).is_file())
                    || before.is_none()
                {
                    item.thumb_path = None;
                    item.micro_thumb = None;
                }
                hydrate_local(item);
                if item.missing {
                    self.database
                        .restore_record_preview(item)
                        .map_err(|e| e.to_string())?;
                }
                if before.is_some()
                    && self.thumbnails_enabled()
                    && visible.contains(&item.id)
                    && !item.missing
                    && item.thumb_path.is_none()
                {
                    if let Some(indexing) = &self.indexing {
                        if let Some((thumb, w, h, micro)) =
                            indexing.visible_thumbnail(&item.full_path, &item.media_type)
                        {
                            if source_version(&item.full_path) != before
                                || self.viewport_request.load(Ordering::Acquire) != request
                                || self.live.status().0 != generation
                            {
                                continue;
                            }
                            item.thumb_path = Some(thumb.clone());
                            item.width = w.map(i64::from);
                            item.height = h.map(i64::from);
                            item.micro_thumb = micro.clone();
                            self.database
                                .store_visible_thumbnail(
                                    item.id,
                                    &item.full_path,
                                    &thumb,
                                    item.width,
                                    item.height,
                                    micro.as_deref(),
                                    before.as_deref(),
                                )
                                .map_err(|e| e.to_string())?;
                        }
                    }
                }
                if !self.thumbnails_enabled() {
                    item.thumb_path = None;
                    item.micro_thumb = None;
                }
            }
        }
        let media_tags = self.tags_for_media(&ids).map_err(|e| e.to_string())?;
        Ok(GalleryWindow {
            offset,
            media,
            media_tags,
        })
    }
}

fn source_version(path: &str) -> Option<String> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified = metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?;
    Some(format!("{}:{}", metadata.len(), modified.as_nanos()))
}
