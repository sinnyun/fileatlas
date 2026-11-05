//! Scoped invalidation for FileAtlas's Windows Shell thumbnail transport cache.
use crate::{DatabaseAccessError, DatabaseService};
use rusqlite::OptionalExtension;

impl DatabaseService {
    /// Clear only media thumbnail references stored under FileAtlas's
    /// `system-shell` transport cache. `media_ids=None` targets the whole
    /// indexed library; callers must pass the exact cache root owned by the
    /// thumbnail service.
    pub fn invalidate_system_shell_thumbnails(
        &self,
        cache_directory: &std::path::Path,
        media_ids: Option<&[i64]>,
    ) -> Result<Vec<String>, DatabaseAccessError> {
        let mut prefix = cache_directory
            .join("system-shell")
            .to_string_lossy()
            .replace('/', "\\")
            .to_lowercase();
        if !prefix.ends_with('\\') {
            prefix.push('\\');
        }
        self.with_cache_writer(|connection| {
            let candidates = if let Some(ids) = media_ids {
                let mut statement = connection.prepare(
                    "SELECT id,thumb_path FROM media WHERE id=?1 AND thumb_path IS NOT NULL AND substr(lower(replace(thumb_path,'/','\\')),1,length(?2))=?2",
                )?;
                ids.iter()
                    .map(|id| {
                        statement
                            .query_row(rusqlite::params![id, prefix], |row| {
                                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                            })
                            .optional()
                    })
                    .collect::<rusqlite::Result<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .collect()
            } else {
                let mut statement = connection.prepare(
                    "SELECT id,thumb_path FROM media WHERE thumb_path IS NOT NULL AND substr(lower(replace(thumb_path,'/','\\')),1,length(?1))=?1",
                )?;
                statement
                    .query_map([&prefix], |row| {
                        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?
            };
            for (id, _) in &candidates {
                connection.execute("UPDATE media SET thumb_path=NULL WHERE id=?1", [id])?;
            }
            Ok(candidates.into_iter().map(|(_, path)| path).collect())
        })
    }
}
