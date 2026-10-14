use super::*;

pub fn add_folder(conn: &Connection, path: &str) -> rusqlite::Result<Folder> {
    conn.execute(
        "INSERT OR IGNORE INTO folders (path) VALUES (?1)",
        params![path],
    )?;
    let id = conn.query_row(
        "SELECT id FROM folders WHERE path = ?1",
        params![path],
        |row| row.get::<_, i64>(0),
    )?;
    let name = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());
    Ok(Folder {
        id,
        path: path.to_string(),
        last_scan: None,
        name,
    })
}

/// Total media count for each registered folder, keyed by folder id.
/// Recursive — includes subfolders, since `media.folder_id` always points
/// at the root folder a subfolder lives under.
pub fn count_media_per_folder(
    conn: &Connection,
) -> rusqlite::Result<std::collections::HashMap<i64, i64>> {
    let mut stmt = conn.prepare("SELECT folder_id, COUNT(*) FROM media GROUP BY folder_id")?;
    let mut map = std::collections::HashMap::new();
    let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?;
    for row in rows {
        if let Ok((id, n)) = row {
            map.insert(id, n);
        }
    }
    Ok(map)
}

/// Count media files whose `full_path` lies under `prefix` (recursive).
/// Used by the sidebar to show per-subfolder totals.
pub fn count_media_under_path(conn: &Connection, prefix: &str) -> rusqlite::Result<i64> {
    // Match both `prefix\foo` and `prefix/foo`. SQLite's LIKE is
    // case-insensitive by default for ASCII, which is fine on Windows.
    let pattern = format!("{}%", prefix.trim_end_matches(|c| c == '/' || c == '\\'));
    let pat_sep = if pattern.ends_with('/') || pattern.ends_with('\\') {
        pattern.clone()
    } else {
        format!("{}{}", pattern, std::path::MAIN_SEPARATOR)
    };
    conn.query_row(
        "SELECT COUNT(*) FROM media WHERE full_path = ?1 OR full_path LIKE ?2",
        params![prefix, pat_sep + "%"],
        |r| r.get::<_, i64>(0),
    )
}

pub fn list_folders(conn: &Connection) -> rusqlite::Result<Vec<Folder>> {
    let mut stmt = conn.prepare("SELECT id, path, last_scan FROM folders ORDER BY path")?;
    let rows = stmt.query_map([], |row| {
        let path: String = row.get(1)?;
        let name = std::path::Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        Ok(Folder {
            id: row.get(0)?,
            path,
            last_scan: row.get(2)?,
            name,
        })
    })?;
    rows.collect()
}

pub fn remove_folder(conn: &Connection, folder_id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM media WHERE folder_id = ?1", params![folder_id])?;
    conn.execute("DELETE FROM folders WHERE id = ?1", params![folder_id])?;
    Ok(())
}

pub fn update_folder_scan_time(conn: &Connection, folder_id: i64) -> rusqlite::Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    conn.execute(
        "UPDATE folders SET last_scan = ?1 WHERE id = ?2",
        params![now, folder_id],
    )?;
    Ok(())
}

pub fn insert_media(
    conn: &Connection,
    folder_id: i64,
    filename: &str,
    full_path: &str,
    media_type: &str,
    file_size: i64,
    modified_at: i64,
    width: Option<i64>,
    height: Option<i64>,
    thumb_path: Option<&str>,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT OR REPLACE INTO media (folder_id, filename, full_path, media_type, file_size, modified_at, width, height, thumb_path) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![folder_id, filename, full_path, media_type, file_size, modified_at, width, height, thumb_path],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Index an ordinary file without replacing its identity or user metadata.
/// Refresh filesystem metadata while keeping identity, rating, date_taken,
/// tag links and cached dimensions.
pub fn scan_upsert_media(
    conn: &Connection,
    folder_id: i64,
    filename: &str,
    full_path: &str,
    media_type: &str,
    file_size: i64,
    modified_at: i64,
    poster_path: Option<&str>,
) -> rusqlite::Result<Option<String>> {
    let previous: Option<(i64, i64, String, Option<String>)> = conn
        .query_row(
            "SELECT file_size, modified_at, media_type, thumb_path FROM media WHERE full_path = ?1",
            [full_path],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?;
    let changed = previous
        .as_ref()
        .is_some_and(|(old_size, old_mtime, old_type, _)| {
            *old_size != file_size || *old_mtime != modified_at || old_type != media_type
        });
    let old_thumb_path = changed
        .then(|| previous.as_ref().and_then(|(_, _, _, path)| path.clone()))
        .flatten();
    conn.execute(
        "INSERT INTO media (folder_id, filename, full_path, media_type, file_size, modified_at, thumb_path)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(full_path) DO UPDATE SET
             folder_id = excluded.folder_id,
             filename = excluded.filename,
             media_type = excluded.media_type,
             file_size = excluded.file_size,
             modified_at = excluded.modified_at,
             thumb_path = CASE WHEN ?8 THEN excluded.thumb_path ELSE COALESCE(excluded.thumb_path, media.thumb_path) END,
             width = CASE WHEN ?8 THEN NULL ELSE media.width END,
             height = CASE WHEN ?8 THEN NULL ELSE media.height END,
             micro_thumb = CASE WHEN ?8 THEN NULL ELSE media.micro_thumb END",
        params![folder_id, filename, full_path, media_type, file_size, modified_at, poster_path, changed],
    )?;
    Ok(old_thumb_path)
}

pub fn get_media(conn: &Connection, query: &MediaQuery) -> rusqlite::Result<Vec<MediaItem>> {
    let (joins, conditions, mut param_values) = build_filters(query);
    let mut sql = String::from(
        "SELECT DISTINCT m.id, m.folder_id, m.filename, m.full_path, m.media_type, \
         m.file_size, m.modified_at, m.width, m.height, m.date_taken, \
         m.thumb_path, m.micro_thumb, \
         COALESCE(m.rating, 0) FROM media m",
    );
    sql.push_str(&joins);
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    sql.push_str(" ORDER BY ");
    sql.push_str(query.sort.unwrap_or(SortOrder::NameAsc).order_by_sql());
    if let Some(limit) = query.limit {
        let i = param_values.len() + 1;
        sql.push_str(&format!(" LIMIT ?{}", i));
        param_values.push(Box::new(limit));
    }
    if let Some(offset) = query.offset {
        let i = param_values.len() + 1;
        sql.push_str(&format!(" OFFSET ?{}", i));
        param_values.push(Box::new(offset));
    }

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|b| b.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_refs.as_slice(), |row| {
        Ok(MediaItem {
            id: row.get(0)?,
            folder_id: row.get(1)?,
            filename: row.get(2)?,
            full_path: row.get(3)?,
            media_type: row.get(4)?,
            file_size: row.get(5)?,
            modified_at: row.get(6)?,
            width: row.get(7)?,
            height: row.get(8)?,
            date_taken: row.get(9)?,
            thumb_path: row.get(10)?,
            micro_thumb: row.get(11)?,
            rating: row.get::<_, i64>(12).unwrap_or(0) as i32,
        })
    })?;
    rows.collect()
}

/// Compact per-item record used to build the gallery layout (row count,
/// date-section headers) without loading the full `MediaItem` for every
/// row in the library. With ~17 bytes per entry it's cheap enough to keep
/// for the entire result set even when there are tens of thousands of
/// items, while the full `state.media` is kept as a small windowed slice
/// around the currently-visible viewport.
#[derive(Debug, Clone, Copy)]
pub struct MediaSkeleton {
    pub id: i64,
    /// `COALESCE(date_taken, modified_at)` in UNIX seconds — the value used
    /// for date-based grouping. Pre-computed in SQL so date-header build
    /// doesn't have to walk every item's `Option`s.
    pub effective_date: i64,
    /// `true` if the row is a video, used to draw the small "VIDEO" badge.
    /// We only need a one-bit fact at layout time.
    pub is_video: bool,
    /// `true` if the row is an audio file, used to draw the "AUDIO" badge.
    pub is_audio: bool,
}

/// Fetch only `(id, effective_date, is_video)` for every media row matching
/// `query`, in the same sort order as `get_media`. Drives the gallery's
/// layout (which needs to know totals and date boundaries) without forcing
/// every item's `String` fields into memory.
pub fn get_media_skeletons(
    conn: &Connection,
    query: &MediaQuery,
) -> rusqlite::Result<Vec<MediaSkeleton>> {
    let (joins, conditions, param_values) = build_filters(query);
    let mut sql = String::from(
        "SELECT DISTINCT m.id, COALESCE(m.date_taken, m.modified_at), m.media_type FROM media m",
    );
    sql.push_str(&joins);
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    sql.push_str(" ORDER BY ");
    sql.push_str(query.sort.unwrap_or(SortOrder::NameAsc).order_by_sql());

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|b| b.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_refs.as_slice(), |row| {
        let media_type: String = row.get(2)?;
        Ok(MediaSkeleton {
            id: row.get(0)?,
            effective_date: row.get(1)?,
            is_video: media_type == "video",
            is_audio: media_type == "audio",
        })
    })?;
    rows.collect()
}

pub fn get_media_count(conn: &Connection, query: &MediaQuery) -> rusqlite::Result<i64> {
    let (joins, conditions, param_values) = build_filters(query);
    let mut sql = String::from("SELECT COUNT(DISTINCT m.id) FROM media m");
    sql.push_str(&joins);
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|b| b.as_ref()).collect();
    conn.query_row(&sql, params_refs.as_slice(), |row| row.get(0))
}

/// Returns the lower-case extensions (without leading dot) actually present
/// in the `media` table, sorted alphabetically. Done Rust-side because
/// SQLite has no rfind / regex without an extension.
pub fn get_available_extensions(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT filename FROM media")?;
    let names: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();
    let mut set = std::collections::BTreeSet::new();
    for n in names {
        if let Some(idx) = n.rfind('.') {
            let ext = n[idx + 1..].to_lowercase();
            if !ext.is_empty() && ext.chars().all(|c| c.is_ascii_alphanumeric()) {
                set.insert(ext);
            }
        }
    }
    Ok(set.into_iter().collect())
}
