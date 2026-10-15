//! Bounded viewport demand: no queries or decoding in a GPUI render callback.
use super::*;
impl GalleryService {
    pub fn ids_for_paths(&self, paths: &[String]) -> Result<Vec<i64>, String> {
        self.database
            .with_reader(|conn| {
                let mut ids = Vec::new();
                for path in paths {
                    if let Ok(id) =
                        conn.query_row("SELECT id FROM media WHERE full_path=?1", [path], |r| {
                            r.get::<_, i64>(0)
                        })
                    {
                        ids.push(id);
                    }
                }
                Ok(ids)
            })
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
        let mut media = self
            .database
            .with_reader(|conn| {
                ids.iter()
                    .map(|id| queries::get_media_by_id(conn, *id))
                    .collect::<rusqlite::Result<Vec<_>>>()
            })
            .map_err(|e| e.to_string())?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        for item in &mut media {
            if self.database.is_browser() {
                if self.live.status().0 != generation
                    || self.viewport_request.load(Ordering::Acquire) != request
                {
                    return Err("Obsolete directory window".into());
                }
                let before = source_version(&item.full_path);
                if let Some(version) = &before {
                    self.database
                        .with_reader(|conn| {
                            let previous: Option<String> = conn
                                .query_row(
                                    "SELECT version FROM file_versions WHERE full_path=?1",
                                    [&item.full_path],
                                    |r| r.get(0),
                                )
                                .ok();
                            if previous.as_ref() != Some(version) {
                                item.thumb_path = None;
                                item.width = None;
                                item.height = None;
                                item.micro_thumb = None;
                            }
                            Ok(())
                        })
                        .map_err(|e| e.to_string())?;
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
                        .with_reader(|conn| {
                            restore_record_preview(conn, item);
                            Ok(())
                        })
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
                            self.database.with_cache_writer(|conn| {
                                conn.execute("UPDATE media SET thumb_path=?1,width=?2,height=?3,micro_thumb=?4 WHERE id=?5 AND full_path=?6",rusqlite::params![thumb,item.width,item.height,micro,item.id,item.full_path])?;
                                if let Some(version)=&before {conn.execute("INSERT OR REPLACE INTO file_versions VALUES(?1,?2)",rusqlite::params![item.full_path,version])?;}
                                Ok(())
                            }).map_err(|e|e.to_string())?;
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
