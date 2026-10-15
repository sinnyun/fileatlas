//! Explicit missing-file records and user-directed relinking; no move inference.
use super::*;
use photag_plugin_database::db::queries::MediaItem;

impl TaggingService {
    pub(super) fn capture_sources(&self, ids: &[i64]) -> Result<(), TaggingError> {
        if !self.database.is_browser() {
            return Ok(());
        }
        let items = self.database.with_reader(|conn| {
            ids.iter()
                .map(|id| queries::get_media_by_id(conn, *id))
                .collect::<rusqlite::Result<Vec<_>>>()
        })?;
        for item in items.into_iter().flatten() {
            let metadata = std::fs::metadata(&item.full_path).map_err(|error| {
                photag_plugin_database::DatabaseAccessError::Sql(
                    rusqlite::Error::InvalidParameterName(error.to_string()),
                )
            })?;
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
            self.database.with_cache_writer(|conn| {
                let modified = metadata.modified().ok().and_then(|t|t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d|d.as_secs() as i64).unwrap_or(0);
                conn.execute("UPDATE media SET file_size=?1,modified_at=?2 WHERE id=?3",rusqlite::params![metadata.len() as i64,modified,item.id])?;
                if let Some(preview)=&preview {conn.execute("INSERT OR REPLACE INTO tag_previews VALUES(?1,?2)",rusqlite::params![item.id,preview])?;}
                if let Some((path,w,h,micro))=thumb {
                    conn.execute("UPDATE media SET thumb_path=?1,width=COALESCE(?2,width),height=COALESCE(?3,height),micro_thumb=?4 WHERE id=?5",rusqlite::params![path,(w>0).then_some(w),(h>0).then_some(h),micro,item.id])?;
                }
                Ok(())
            })?;
        }
        Ok(())
    }

    pub fn missing_records(&self) -> Result<Vec<MediaItem>, String> {
        let items = self
            .database
            .with_reader(|conn| {
                let mut query = conn.prepare("SELECT DISTINCT media_id FROM media_tags")?;
                let ids = query
                    .query_map([], |row| row.get::<_, i64>(0))?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                ids.into_iter()
                    .map(|id| queries::get_media_by_id(conn, id))
                    .collect::<rusqlite::Result<Vec<_>>>()
            })
            .map_err(|e| e.to_string())?;
        Ok(items
            .into_iter()
            .flatten()
            .filter(|item| {
                std::fs::metadata(&item.full_path)
                    .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
            })
            .map(|mut item| {
                if self.database.is_browser() {
                    if let Ok(Some(path)) = self.database.with_reader(|conn| {
                        Ok(conn
                            .query_row(
                                "SELECT thumb_path FROM tag_previews WHERE media_id=?1",
                                [item.id],
                                |r| r.get::<_, String>(0),
                            )
                            .ok())
                    }) {
                        item.thumb_path = Some(path);
                    }
                }
                item
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
        let kind = photag_plugin_indexing::scan::classify_media_type(
            &path.extension().unwrap_or_default().to_string_lossy(),
        );
        self.database.with_writer(|conn| {
            let old = queries::get_media_by_id(conn,record_id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            if !std::fs::metadata(&old.full_path).is_err_and(|e|e.kind()==std::io::ErrorKind::NotFound) { return Err(rusqlite::Error::InvalidParameterName("Record is not missing".into())); }
            let existing: Option<i64> = conn.query_row("SELECT id FROM media WHERE full_path=?1",[&full_path],|r|r.get(0)).ok();
            if let Some(other) = existing.filter(|id| *id != record_id) {
                let assigned: i64 = conn.query_row("SELECT (SELECT count(*) FROM media_tags WHERE media_id=?1) + (SELECT COALESCE(rating,0) FROM media WHERE id=?1)",[other],|r|r.get(0))?;
                if assigned > 0 { return Err(rusqlite::Error::InvalidParameterName("Target already has annotations; choose another file".into())); }
                conn.execute("DELETE FROM media WHERE id=?1",[other])?;
            }
            queries::update_media_path(conn,record_id,&filename,&full_path,None)?;
            conn.execute("UPDATE media SET file_size=?1,modified_at=?2,thumb_path=NULL,micro_thumb=NULL,width=NULL,height=NULL,media_type=?4 WHERE id=?3",rusqlite::params![metadata.len() as i64,modified,record_id,kind])?;
            Ok(())
        }).map_err(|e|e.to_string())?;
        self.capture_sources(&[record_id])
            .map_err(|e| e.to_string())?;
        self.database
            .flush_annotations()
            .map_err(|e| e.to_string())?;
        self.schedule_xmp_sync(&[record_id]);
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
