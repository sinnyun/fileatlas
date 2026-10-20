//! Everything results are transient rows, independent of the live directory.
use crate::{BrowserFile, DatabaseAccessError, DatabaseService, db::queries};
use rusqlite::Connection;
use std::collections::HashMap;

impl DatabaseService {
    /// Materialize search hits in the browser session only. Preserve annotation
    /// identity across normal/extended Windows path spellings and repeated reads.
    /// Returns the actual row paths used by candidate filtering.
    pub fn cache_search_files(
        &self,
        files: &[BrowserFile],
    ) -> Result<Vec<String>, DatabaseAccessError> {
        if !self.is_browser() {
            return Ok(files.iter().map(|file| file.path.clone()).collect());
        }
        let (existing, previous) = self.with_reader(|connection| {
            let existing = connection.prepare("SELECT full_path FROM media")?
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?
                .into_iter().map(|path| (path_key(&path), path)).collect::<HashMap<_, _>>();
            let previous = connection.prepare("SELECT s.full_path,v.version FROM search_paths s JOIN file_versions v ON v.full_path=s.full_path")?
                .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?
                .collect::<rusqlite::Result<HashMap<_, _>>>()?;
            Ok((existing, previous))
        })?;
        let files = files
            .iter()
            .map(|file| {
                let mut file = file.clone();
                if let Some(path) = existing.get(&path_key(&file.path)) {
                    file.path.clone_from(path);
                }
                file
            })
            .collect::<Vec<_>>();
        let paths = files
            .iter()
            .map(|file| file.path.clone())
            .collect::<Vec<_>>();
        if previous.len() == files.len()
            && files
                .iter()
                .all(|file| previous.get(&file.path) == Some(&file.version))
        {
            return Ok(paths);
        }
        self.with_cache_writer(|connection| {
            let json = serde_json::to_string(&paths).map_err(|error| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(error))
            })?;
            connection.execute_batch("SAVEPOINT search_cache")?;
            let result = (|| {
                connection.execute("DELETE FROM media WHERE full_path IN (SELECT full_path FROM search_paths) AND full_path NOT IN (SELECT value FROM json_each(?1)) AND COALESCE(rating,0)=0 AND NOT EXISTS(SELECT 1 FROM media_tags t WHERE t.media_id=media.id) AND id NOT IN(SELECT id FROM history_media_ids) AND full_path NOT IN(SELECT full_path FROM pinned_paths) AND full_path NOT IN(SELECT full_path FROM live_paths) AND NOT EXISTS(SELECT 1 FROM cache_scopes s WHERE substr(replace(media.full_path,char(92),'/'),1,length(s.path)+1)=s.path||'/')", [&json])?;
                connection.execute("DELETE FROM search_paths", [])?;
                for file in &files {
                    cache_file(connection, file)?;
                    connection.execute("INSERT OR IGNORE INTO search_paths VALUES(?1)", [&file.path])?;
                }
                connection.execute("DELETE FROM file_versions WHERE full_path NOT IN(SELECT full_path FROM media)", [])?;
                Ok(paths)
            })();
            if result.is_ok() {
                connection.execute_batch("RELEASE search_cache")?;
            } else {
                connection.execute_batch("ROLLBACK TO search_cache; RELEASE search_cache")?;
            }
            result
        })
    }
}

pub(crate) fn cache_file(connection: &Connection, file: &BrowserFile) -> rusqlite::Result<()> {
    let previous: Option<String> = connection
        .query_row(
            "SELECT version FROM file_versions WHERE full_path=?1",
            [&file.path],
            |row| row.get(0),
        )
        .ok();
    if previous.as_ref() != Some(&file.version) {
        connection.execute("UPDATE media SET thumb_path=NULL,micro_thumb=NULL,width=NULL,height=NULL WHERE full_path=?1", [&file.path])?;
    }
    queries::scan_upsert_media(
        connection,
        0,
        &file.filename,
        &file.path,
        &file.media_type,
        file.file_size,
        file.modified_at,
        None,
    )?;
    connection.execute(
        "INSERT OR REPLACE INTO file_versions VALUES(?1,?2)",
        rusqlite::params![file.path, file.version],
    )?;
    Ok(())
}

fn path_key(path: &str) -> String {
    let path = path
        .strip_prefix(r"\\?\UNC\")
        .map(|tail| format!(r"\\{tail}"))
        .unwrap_or_else(|| path.strip_prefix(r"\\?\").unwrap_or(path).to_owned());
    path.replace('/', "\\").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_cache_preserves_annotations_identity_and_quiet_repeated_reads()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!(
            "fileatlas-search-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        std::fs::create_dir_all(&root)?;
        let database_path = root.join("tags.db");
        let database = DatabaseService::open_browser_at(&database_path)?;
        let file = |name: &str| BrowserFile {
            path: format!(r"C:\files\{name}.txt"),
            filename: format!("{name}.txt"),
            media_type: "file".into(),
            file_size: 10,
            modified_at: 100,
            version: "10:100".into(),
        };
        let first = file("one");
        let second = file("two");
        database.cache_search_files(&[first.clone(), second.clone()])?;
        let revision = database.write_revision();
        database.cache_search_files(&[first.clone(), second.clone()])?;
        assert_eq!(
            database.write_revision(),
            revision,
            "an identical search must not invalidate views"
        );
        let id: i64 = database.with_reader(|conn| {
            conn.query_row(
                "SELECT id FROM media WHERE full_path=?1",
                [&second.path],
                |row| row.get(0),
            )
        })?;
        database.with_writer(|conn| {
            let tag = queries::create_tag(conn, "Keep", None)?;
            queries::assign_tags(conn, &[id], &[tag.id])
        })?;
        let mut extended = second.clone();
        extended.path = format!(r"\\?\{}", second.path);
        assert_eq!(
            database.cache_search_files(&[extended])?,
            [second.path.clone()]
        );
        assert_eq!(
            database
                .get_media_by_id(id)?
                .ok_or("missing cached file")?
                .full_path,
            second.path
        );
        database.cache_search_files(&[file("three")])?;
        assert!(
            database.get_media_by_id(id)?.is_some(),
            "tagged rows survive cache eviction"
        );
        let count: i64 = database.with_reader(|conn| {
            conn.query_row("SELECT count(*) FROM media", [], |row| row.get(0))
        })?;
        assert_eq!(
            count, 2,
            "only the new result and annotated old result remain"
        );
        drop(database);
        let reopened = DatabaseService::open_browser_at(&database_path)?;
        let count: i64 = reopened.with_reader(|conn| {
            conn.query_row("SELECT count(*) FROM media", [], |row| row.get(0))
        })?;
        assert_eq!(
            count, 1,
            "search inventory is never saved to the annotation database"
        );
        assert_eq!(
            reopened
                .get_media_by_id(id)?
                .ok_or("missing cached file")?
                .full_path,
            second.path
        );
        drop(reopened);
        std::fs::remove_dir_all(root)?;
        Ok(())
    }
}
