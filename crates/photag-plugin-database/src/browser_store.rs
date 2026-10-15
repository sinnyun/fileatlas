//! Session-only file cache plus durable bookmarks and explicitly annotated files.
use super::*;

impl DatabaseService {
    pub fn open_browser_at(path: impl AsRef<Path>) -> Result<Self, DatabaseOpenError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let disk = db::open_db_at(&path)?;
        disk.execute_batch("CREATE TABLE IF NOT EXISTS favorite_meta(id INTEGER PRIMARY KEY REFERENCES folders(id) ON DELETE CASCADE,name TEXT NOT NULL,position INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS tag_previews(media_id INTEGER PRIMARY KEY REFERENCES media(id) ON DELETE CASCADE,thumb_path TEXT NOT NULL)")?;
        drop(disk);
        let cache = Connection::open_in_memory()?;
        db::initialize(&cache)?;
        db::migrate(&cache)?;
        cache.execute_batch("PRAGMA foreign_keys=OFF; CREATE TABLE favorite_meta(id INTEGER PRIMARY KEY REFERENCES folders(id) ON DELETE CASCADE,name TEXT NOT NULL,position INTEGER NOT NULL); CREATE TABLE tag_previews(media_id INTEGER PRIMARY KEY REFERENCES media(id) ON DELETE CASCADE,thumb_path TEXT NOT NULL)")?;
        cache.execute(
            "ATTACH DATABASE ?1 AS saved",
            [path.to_string_lossy().as_ref()],
        )?;
        for table in tables(&cache)? {
            cache.execute_batch(&format!(
                "INSERT INTO main.{table} SELECT * FROM saved.{table};"
            ))?;
        }
        cache.execute_batch("PRAGMA foreign_keys=ON")?;
        // Files are independent of bookmarks. This private owner is never shown in navigation.
        cache.execute("INSERT OR IGNORE INTO folders(id,path) VALUES(0,'')", [])?;
        cache.execute_batch("CREATE TABLE cache_scopes(path TEXT PRIMARY KEY,seen INTEGER NOT NULL); CREATE TABLE pinned_paths(full_path TEXT PRIMARY KEY); CREATE TABLE history_media_ids(id INTEGER PRIMARY KEY); CREATE TABLE live_paths(full_path TEXT PRIMARY KEY); CREATE TABLE file_versions(full_path TEXT PRIMARY KEY, version TEXT NOT NULL);")?;
        Ok(Self {
            write_revision: Default::default(),
            writer: Mutex::new(cache),
            reader: Mutex::new(Connection::open_in_memory()?),
            path,
            browser: true,
        })
    }

    pub fn is_browser(&self) -> bool {
        self.browser
    }

    /// Update derived session data without persisting a directory inventory.
    pub fn with_cache_writer<T>(
        &self,
        operation: impl FnOnce(&Connection) -> rusqlite::Result<T>,
    ) -> Result<T, DatabaseAccessError> {
        if !self.browser {
            return self.with_writer(operation);
        }
        let connection = lock(&self.writer)?;
        let result = operation(&connection);
        self.write_revision
            .fetch_add(1, std::sync::atomic::Ordering::Release);
        result.map_err(DatabaseAccessError::Sql)
    }

    /// Keep at most four recent directory scopes; prune older derived rows above the budget.
    pub fn begin_directory_cache(&self, root: &str) -> Result<(), DatabaseAccessError> {
        self.with_cache_writer(|conn| {
            let root=root.replace('\\',"/").trim_end_matches('/').to_owned();
            conn.execute("INSERT INTO cache_scopes(path,seen) VALUES(?1,(SELECT COALESCE(MAX(seen),0)+1 FROM cache_scopes)) ON CONFLICT(path) DO UPDATE SET seen=(SELECT COALESCE(MAX(seen),0)+1 FROM cache_scopes)",[&root])?;
            conn.execute("DELETE FROM cache_scopes WHERE path NOT IN(SELECT path FROM cache_scopes ORDER BY seen DESC LIMIT 4)",[])?;
            conn.execute("DELETE FROM live_paths",[])?;
            loop {
                conn.execute("DELETE FROM media WHERE COALESCE(rating,0)=0 AND NOT EXISTS(SELECT 1 FROM media_tags t WHERE t.media_id=media.id) AND id NOT IN(SELECT id FROM history_media_ids) AND full_path NOT IN(SELECT full_path FROM pinned_paths) AND NOT EXISTS(SELECT 1 FROM cache_scopes s WHERE substr(replace(media.full_path,char(92),'/'),1,length(s.path)+1)=s.path||'/')",[])?;
                let count:i64=conn.query_row("SELECT count(*) FROM media",[],|r|r.get(0))?;
                let scopes:i64=conn.query_row("SELECT count(*) FROM cache_scopes",[],|r|r.get(0))?;
                if count<=200_000 || scopes<=1 {break}
                conn.execute("DELETE FROM cache_scopes WHERE path IN(SELECT path FROM cache_scopes ORDER BY seen LIMIT 1)",[])?;
            }
            Ok(())
        })
    }

    /// Restore a cached file by path when an old filmstrip outlives its directory cache.
    pub fn pin_file(
        &self,
        path: &str,
        kind: &str,
        previous: Option<i64>,
    ) -> Result<i64, DatabaseAccessError> {
        let metadata = std::fs::metadata(path).map_err(|error| {
            DatabaseAccessError::Sql(rusqlite::Error::InvalidParameterName(error.to_string()))
        })?;
        let modified = metadata
            .modified()
            .ok()
            .and_then(|v| v.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let filename = Path::new(path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        self.with_cache_writer(|conn| {
            conn.execute("DELETE FROM pinned_paths",[])?;
            conn.execute("INSERT INTO pinned_paths VALUES(?1)",[path])?;
            let existing:Option<i64>=conn.query_row("SELECT id FROM media WHERE full_path=?1",[path],|r|r.get(0)).ok();
            if let Some(id)=existing {return Ok(id)}
            if let Some(id)=previous {
                let occupied:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM media WHERE id=?1)",[id],|r|r.get(0))?;
                if !occupied {conn.execute("INSERT INTO media(id,folder_id,filename,full_path,media_type,file_size,modified_at) VALUES(?1,0,?2,?3,?4,?5,?6)",rusqlite::params![id,filename,path,kind,metadata.len() as i64,modified])?;return Ok(id)}
            }
            db::queries::scan_upsert_media(conn,0,&filename,path,kind,metadata.len() as i64,modified,None)?;
            conn.query_row("SELECT id FROM media WHERE full_path=?1",[path],|r|r.get(0))
        })
    }

    pub fn pin_history(&self, ids: &[i64]) -> Result<(), DatabaseAccessError> {
        if !self.browser {
            return Ok(());
        }
        self.with_cache_writer(|conn| {
            conn.execute("DELETE FROM history_media_ids", [])?;
            for id in ids {
                conn.execute("INSERT OR IGNORE INTO history_media_ids VALUES(?1)", [id])?;
            }
            Ok(())
        })
    }

    pub fn flush_annotations(&self) -> Result<(), DatabaseAccessError> {
        self.with_writer(|_| Ok(()))
    }

    pub fn forget_records(&self, paths: &[String]) -> Result<usize, DatabaseAccessError> {
        self.with_writer(|conn| {
            let mut count = 0;
            for path in paths {
                if self.browser {
                    conn.execute("DELETE FROM saved.media WHERE full_path=?1", [path])?;
                }
                count += conn.execute("DELETE FROM media WHERE full_path=?1", [path])?;
            }
            Ok(count)
        })
    }
}

fn tables(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    // Parent tables before child tables for foreign keys.
    let mut tables = conn
        .prepare(
            "SELECT name FROM main.sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
        )?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    tables.sort_by_key(|name| match name.as_str() {
        "folders" => 0,
        "tags" => 1,
        "media" => 2,
        _ => 3,
    });
    Ok(tables)
}

pub(super) fn persist(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("SAVEPOINT persist_annotations; PRAGMA defer_foreign_keys=ON;")?;
    let result = (|| {
        for table in [
            "folders",
            "tags",
            "media",
            "tag_associations",
            "media_tags",
            "favorite_meta",
            "tag_previews",
        ] {
            let columns = conn
                .prepare(&format!("PRAGMA main.table_info({table})"))?
                .query_map([], |row| {
                    Ok((row.get::<_, String>(1)?, row.get::<_, i64>(5)?))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let names = columns
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>()
                .join(",");
            let keys = columns
                .iter()
                .filter(|(_, pk)| *pk > 0)
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>()
                .join(",");
            let assignments = columns
                .iter()
                .filter(|(_, pk)| *pk == 0)
                .map(|(name, _)| format!("{name}=excluded.{name}"))
                .collect::<Vec<_>>()
                .join(",");
            let filter = if table == "media" {
                "COALESCE(rating,0)<>0 OR EXISTS(SELECT 1 FROM main.media_tags mt WHERE mt.media_id=media.id)"
            } else if table == "tag_previews" {
                "EXISTS(SELECT 1 FROM main.media m WHERE m.id=tag_previews.media_id AND (COALESCE(m.rating,0)<>0 OR EXISTS(SELECT 1 FROM main.media_tags t WHERE t.media_id=m.id)))"
            } else {
                "1"
            };
            let conflict = if assignments.is_empty() {
                "DO NOTHING".to_owned()
            } else {
                let changed = columns
                    .iter()
                    .filter(|(_, pk)| *pk == 0)
                    .map(|(name, _)| format!("{table}.{name} IS NOT excluded.{name}"))
                    .collect::<Vec<_>>()
                    .join(" OR ");
                format!("DO UPDATE SET {assignments} WHERE {changed}")
            };
            conn.execute_batch(&format!("INSERT INTO saved.{table} ({names}) SELECT {names} FROM main.{table} WHERE {filter} ON CONFLICT({keys}) {conflict};"))?;
        }
        conn.execute_batch("DELETE FROM saved.media_tags WHERE NOT EXISTS(SELECT 1 FROM main.media_tags m WHERE m.media_id=saved.media_tags.media_id AND m.tag_id=saved.media_tags.tag_id);
            DELETE FROM saved.tag_associations WHERE NOT EXISTS(SELECT 1 FROM main.tag_associations a WHERE a.tag_id=saved.tag_associations.tag_id AND a.associated_tag_id=saved.tag_associations.associated_tag_id);
            DELETE FROM saved.tags WHERE id NOT IN(SELECT id FROM main.tags);
            DELETE FROM saved.favorite_meta WHERE id NOT IN(SELECT id FROM main.folders);
            DELETE FROM saved.folders WHERE id NOT IN(SELECT id FROM main.folders) AND id NOT IN(SELECT folder_id FROM saved.media);
            DELETE FROM saved.media WHERE id IN(SELECT id FROM main.media WHERE COALESCE(rating,0)=0 AND NOT EXISTS(SELECT 1 FROM main.media_tags t WHERE t.media_id=media.id));")?;
        Ok(())
    })();
    if result.is_ok() {
        conn.execute_batch("RELEASE persist_annotations;")?;
    } else {
        conn.execute_batch("ROLLBACK TO persist_annotations; RELEASE persist_annotations;")?;
    }
    result
}
