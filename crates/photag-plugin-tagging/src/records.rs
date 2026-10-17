//! Explicit missing-file records and user-directed relinking; no move inference.
use super::*;
use photag_plugin_database::MediaItem;
use photag_plugin_database::{TaggedMediaSourceUpdate, ThumbnailRecord};

impl TaggingService {
    pub(super) fn capture_sources(&self, ids: &[i64]) -> Result<(), TaggingError> {
        if !self.database.is_browser() {
            return Ok(());
        }
        let items = self.database.get_media_by_ids(ids)?;
        for item in items {
            let metadata =
                std::fs::metadata(&item.full_path).map_err(|error| TaggingError::Io(error))?;
            let thumb = if item.media_type == "image" {
                self.thumbnails
                    .as_ref()
                    .and_then(|service| service.generate_image(&item.full_path))
            } else {
                None
            };
            let preview_source = thumb
                .as_ref()
                .map(|t| t.0.as_str())
                .or(item.thumb_path.as_deref());
            let preview = preview_source.and_then(|source| {
                let directory = self.database.path().parent()?.join("tag-previews");
                std::fs::create_dir_all(&directory).ok()?;
                let target = directory.join(std::path::Path::new(source).file_name()?);
                if !target.exists() {
                    std::fs::copy(source, &target).ok()?;
                }
                Some(target.to_string_lossy().into_owned())
            });
            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let thumbnail = thumb.map(|(thumb_path, w, h, micro)| ThumbnailRecord {
                full_path: item.full_path.clone(),
                thumb_path,
                width: (w > 0).then_some(w as i64),
                height: (h > 0).then_some(h as i64),
                micro_thumb: micro,
            });
            self.database
                .update_tagged_media_source(&TaggedMediaSourceUpdate {
                    media_id: item.id,
                    file_size: metadata.len() as i64,
                    modified_at: modified,
                    preview_path: preview,
                    thumbnail,
                })?;
        }
        Ok(())
    }

    pub fn missing_records(&self) -> Result<Vec<MediaItem>, String> {
        let items = self
            .database
            .annotated_media_with_previews()
            .map_err(|e| e.to_string())?;
        Ok(items
            .into_iter()
            .map(|(mut item, preview)| {
                if item.missing && self.database.is_browser() {
                    item.thumb_path = preview;
                }
                item
            })
            .filter(|item| {
                std::fs::metadata(&item.full_path)
                    .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
            })
            .collect())
    }

    /// The caller supplies the exact missing record and an explicitly chosen file.
    pub fn relink_record(&self, record_id: i64, destination: &str) -> Result<(), String> {
        let path = std::fs::canonicalize(destination).map_err(|e| e.to_string())?;
        let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?;
        if !metadata.is_file() {
            return Err("Relink target must be a file".into());
        }
        let full_path = path.to_string_lossy().into_owned();
        let filename = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let modified = metadata
            .modified()
            .map_err(|e| e.to_string())?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs() as i64;
        let kind = photag_plugin_indexing::classify_media_type(
            &path.extension().unwrap_or_default().to_string_lossy(),
        );
        let old = self
            .database
            .get_media_by_id(record_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Record no longer exists".to_string())?;
        let old_path_missing = std::fs::metadata(&old.full_path)
            .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound);
        self.database
            .relink_missing_media(
                record_id,
                old_path_missing,
                &filename,
                &full_path,
                kind,
                metadata.len() as i64,
                modified,
            )
            .map_err(|e| e.to_string())?;
        self.capture_sources(&[record_id])
            .map_err(|e| e.to_string())?;
        self.database
            .flush_annotations()
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Verify each candidate again at action time; never delete a live file.
    pub fn delete_missing_records(&self, record_ids: &[i64]) -> Result<usize, String> {
        let missing = self.missing_records()?;
        let paths = missing
            .into_iter()
            .filter(|item| record_ids.contains(&item.id))
            .map(|item| item.full_path)
            .collect::<Vec<_>>();
        self.database
            .forget_records(&paths)
            .map_err(|e| e.to_string())
    }
}
