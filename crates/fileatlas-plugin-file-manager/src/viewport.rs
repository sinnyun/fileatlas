//! Bounded viewport demand: no queries or decoding in a GPUI render callback.
use super::*;
impl FileManagerService {
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
    ) -> Result<FileManagerWindow, String> {
        let (mut media, _) = self
            .database
            .get_media_window(&ids)
            .map_err(|e| e.to_string())?;
        let mut thumbnail_ratios = HashMap::new();
        for item in &mut media {
            let browser = self.database.is_browser();
            let before = source_version(&item.full_path);
            if browser {
                if self.live.status().0 != generation
                    || self.viewport_request.load(Ordering::Acquire) != request
                {
                    return Err("Obsolete directory window".into());
                }
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
                // FileAtlas-generated historical cache entries are not valid
                // previews for ordinary cards. Resolve cards through Shell only.
                item.thumb_path = None;
                item.micro_thumb = None;
                hydrate_local(item);
                if item.missing {
                    self.database
                        .restore_record_preview(item)
                        .map_err(|e| e.to_string())?;
                }
            }
            if before.is_some()
                && self.thumbnails_enabled()
                && visible.contains(&item.id)
                && !item.missing
            {
                // Read source header dimensions on this bounded background worker.
                // Shell bitmap dimensions describe its transport image, not the original.
                if (item.width.is_none() || item.height.is_none())
                    && let Some((width, height)) = source_dimensions(&item.full_path)
                {
                    item.width = Some(i64::from(width));
                    item.height = Some(i64::from(height));
                }
                if let Some(indexing) = &self.indexing {
                    if let Some((thumb, width, height, _micro)) =
                        indexing.visible_thumbnail(&item.full_path, &item.media_type)
                    {
                        if source_version(&item.full_path) != before
                            || (browser
                                && (self.viewport_request.load(Ordering::Acquire) != request
                                    || self.live.status().0 != generation))
                        {
                            continue;
                        }
                        item.thumb_path = Some(thumb.clone());
                        if let (Some(width), Some(height)) = (width, height)
                            && width > 0
                            && height > 0
                        {
                            thumbnail_ratios.insert(item.id, width as f32 / height as f32);
                        }
                        self.database
                            .store_visible_thumbnail(
                                item.id,
                                &item.full_path,
                                &thumb,
                                item.width,
                                item.height,
                                None,
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
        let media_tags = self.tags_for_media(&ids).map_err(|e| e.to_string())?;
        Ok(FileManagerWindow {
            thumbnail_ratios,
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

fn source_dimensions(path: &str) -> Option<(u32, u32)> {
    // External thumbnail caches can contain PNG pixels in a file named .jpg.
    // Sniff the header before reading dimensions; no source pixels are decoded.
    image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn dimensions_follow_file_header_when_extension_is_wrong()
    -> Result<(), Box<dyn std::error::Error>> {
        let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.local/validation/profiles/thumbnail-header-test");
        std::fs::create_dir_all(&folder)?;
        let name = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let path = folder.join(format!("{name}.jpg"));
        image::RgbImage::new(30, 10).save_with_format(&path, image::ImageFormat::Png)?;
        let result = super::source_dimensions(&path.to_string_lossy());
        std::fs::remove_file(path)?;
        assert_eq!(result, Some((30, 10)));
        Ok(())
    }
}
