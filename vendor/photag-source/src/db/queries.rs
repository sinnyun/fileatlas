use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Folder {
    pub id: i64,
    pub path: String,
    pub last_scan: Option<i64>,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaItem {
    pub id: i64,
    pub folder_id: i64,
    pub filename: String,
    pub full_path: String,
    pub media_type: String,
    pub file_size: i64,
    pub modified_at: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub date_taken: Option<i64>,
    pub thumb_path: Option<String>,
    #[serde(skip)]
    pub micro_thumb: Option<Vec<u8>>,
    /// Rating 0..=5; 0 = unrated.
    #[serde(default)]
    pub rating: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub full_path: String,
    pub children: Vec<Tag>,
    pub media_count: i64,
    pub color: Option<String>,
    pub icon: Option<String>,
    /// When false, the tag is hidden from the tag chips drawn on
    /// gallery thumbnails. Still shown everywhere else (sidebar,
    /// info panel, picker dropdown). Defaults to true.
    #[serde(default = "default_true")]
    pub display: bool,
    /// Position among siblings (smaller = earlier). Renumbered when
    /// the user drags tags around in the sidebar.
    #[serde(default)]
    pub sort_order: i64,
}

fn default_true() -> bool { true }

/// Drop position for `move_tag`, as decided by the sidebar drag-zone hit-test.
#[derive(Debug, Clone, Copy)]
pub enum TagDropPos {
    /// Place `source` immediately before this sibling.
    Before(i64),
    /// Place `source` immediately after this sibling.
    After(i64),
    /// Make `source` a child of this tag (appended at end).
    Into(i64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SortOrder {
    NewestFirst,
    OldestFirst,
    BiggestFirst,
    SmallestFirst,
    NameAsc,
    NameDesc,
    HighestRated,
    LowestRated,
}

impl SortOrder {
    pub fn label(self) -> &'static str {
        match self {
            SortOrder::NewestFirst   => "Newest first",
            SortOrder::OldestFirst   => "Oldest first",
            SortOrder::BiggestFirst  => "Biggest first",
            SortOrder::SmallestFirst => "Smallest first",
            SortOrder::NameAsc       => "Name A→Z",
            SortOrder::NameDesc      => "Name Z→A",
            SortOrder::HighestRated  => "Highest rated",
            SortOrder::LowestRated   => "Lowest rated",
        }
    }
    /// SQL ORDER BY fragment (no leading "ORDER BY"). Uses COALESCE so items
    /// without `date_taken` fall back to `modified_at` for time-based sorts.
    fn order_by_sql(self) -> &'static str {
        match self {
            SortOrder::NewestFirst   => "COALESCE(m.date_taken, m.modified_at) DESC, m.id DESC",
            SortOrder::OldestFirst   => "COALESCE(m.date_taken, m.modified_at) ASC, m.id ASC",
            SortOrder::BiggestFirst  => "m.file_size DESC, m.id DESC",
            SortOrder::SmallestFirst => "m.file_size ASC, m.id ASC",
            SortOrder::NameAsc       => "m.filename ASC",
            SortOrder::NameDesc      => "m.filename DESC",
            // Rated sorts secondary-sort by date so unrated items group
            // together meaningfully rather than by id.
            SortOrder::HighestRated  => "COALESCE(m.rating, 0) DESC, COALESCE(m.date_taken, m.modified_at) DESC, m.id DESC",
            SortOrder::LowestRated   => "COALESCE(m.rating, 0) ASC, COALESCE(m.date_taken, m.modified_at) DESC, m.id DESC",
        }
    }
}

#[derive(Default, Clone)]
pub struct MediaQuery {
    pub folder_id: Option<i64>,
    pub tag_ids: Option<Vec<i64>>,
    /// Media that has ANY of these tags is excluded from results.
    pub excluded_tag_ids: Option<Vec<i64>>,
    pub path_prefix: Option<String>,
    /// Case-insensitive substring match. Which fields are matched is
    /// controlled by the `search_in_*` flags below; if every flag is false
    /// the search clause is dropped entirely.
    pub search: Option<String>,
    pub search_in_filenames: bool,
    pub search_in_paths: bool,
    pub search_in_tags: bool,
    /// "photo" | "video" — None means both.
    pub media_type: Option<String>,
    /// Lower-case extensions WITHOUT a leading dot (e.g. "jpg", "mp4"). Items
    /// must match ANY of these. None / empty disables the filter.
    pub extensions: Option<Vec<String>>,
    pub excluded_extensions: Option<Vec<String>>,
    /// Inclusive UNIX-second bounds on `COALESCE(date_taken, modified_at)`.
    pub date_from: Option<i64>,
    pub date_to: Option<i64>,
    /// Inclusive bounds on rating (0..=5).
    pub rating_min: Option<i32>,
    pub rating_max: Option<i32>,
    /// File-size bounds in bytes. Used by the bulk action filter.
    pub size_min: Option<i64>,
    pub size_max: Option<i64>,
    pub sort: Option<SortOrder>,
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

/// Returns `(sql_prefix_after_FROM, conditions, params)`. Used by both
/// `get_media` and `get_media_count` so the filter logic stays in one place.
fn build_filters(query: &MediaQuery) -> (String, Vec<String>, Vec<Box<dyn rusqlite::types::ToSql>>) {
    let mut sql_extra = String::new();
    let mut conditions: Vec<String> = Vec::new();
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref tag_ids) = query.tag_ids {
        if !tag_ids.is_empty() {
            let mut ph = Vec::with_capacity(tag_ids.len());
            for id in tag_ids {
                let i = param_values.len() + 1;
                ph.push(format!("?{}", i));
                param_values.push(Box::new(*id));
            }
            sql_extra.push_str(&format!(
                " JOIN media_tags mt ON m.id = mt.media_id AND mt.tag_id IN ({})",
                ph.join(",")
            ));
        }
    }
    if let Some(folder_id) = query.folder_id {
        let i = param_values.len() + 1;
        conditions.push(format!("m.folder_id = ?{}", i));
        param_values.push(Box::new(folder_id));
    }
    if let Some(ref prefix) = query.path_prefix {
        let i = param_values.len() + 1;
        conditions.push(format!("m.full_path LIKE ?{}", i));
        param_values.push(Box::new(format!("{}\\%", prefix)));
    }
    if let Some(ref ex_ids) = query.excluded_tag_ids {
        if !ex_ids.is_empty() {
            let mut ph = Vec::with_capacity(ex_ids.len());
            for id in ex_ids {
                let i = param_values.len() + 1;
                ph.push(format!("?{}", i));
                param_values.push(Box::new(*id));
            }
            conditions.push(format!(
                "m.id NOT IN (SELECT media_id FROM media_tags WHERE tag_id IN ({}))",
                ph.join(",")
            ));
        }
    }
    if let Some(ref s) = query.search {
        let trimmed = s.trim();
        // Drop the clause entirely when every scope is off — otherwise the
        // empty OR would match nothing and the user would see a blank gallery.
        let any_scope = query.search_in_filenames || query.search_in_paths || query.search_in_tags;
        if !trimmed.is_empty() && any_scope {
            let needle = format!("%{}%", trimmed.to_lowercase());
            let mut ors: Vec<String> = Vec::new();
            if query.search_in_filenames {
                let i = param_values.len() + 1;
                ors.push(format!("LOWER(m.filename) LIKE ?{}", i));
                param_values.push(Box::new(needle.clone()));
            }
            if query.search_in_paths {
                let i = param_values.len() + 1;
                ors.push(format!("LOWER(m.full_path) LIKE ?{}", i));
                param_values.push(Box::new(needle.clone()));
            }
            if query.search_in_tags {
                // EXISTS subquery: match if ANY tag assigned to this media
                // (or any of its ancestors, via the tag tree) has a name
                // containing the needle. Using a simple join to `tags`
                // keeps the clause cheap; ancestor matching could come
                // later if users ask for it.
                let i = param_values.len() + 1;
                ors.push(format!(
                    "EXISTS (SELECT 1 FROM media_tags mt2 \
                     JOIN tags t2 ON mt2.tag_id = t2.id \
                     WHERE mt2.media_id = m.id AND LOWER(t2.name) LIKE ?{})",
                    i
                ));
                param_values.push(Box::new(needle));
            }
            conditions.push(format!("({})", ors.join(" OR ")));
        }
    }
    if let Some(ref mt) = query.media_type {
        let i = param_values.len() + 1;
        conditions.push(format!("m.media_type = ?{}", i));
        param_values.push(Box::new(mt.clone()));
    }
    if let Some(ref exts) = query.extensions {
        if !exts.is_empty() {
            // Use suffix LIKE so multi-dot filenames (e.g. "img.2024.jpg")
            // are classified by their actual extension.
            let mut ors = Vec::with_capacity(exts.len());
            for e in exts {
                let i = param_values.len() + 1;
                ors.push(format!("LOWER(m.filename) LIKE ?{}", i));
                param_values.push(Box::new(format!("%.{}", e.to_lowercase())));
            }
            conditions.push(format!("({})", ors.join(" OR ")));
        }
    }
    if let Some(ref exts) = query.excluded_extensions {
        if !exts.is_empty() {
            let mut ors = Vec::with_capacity(exts.len());
            for e in exts {
                let i = param_values.len() + 1;
                ors.push(format!("LOWER(m.filename) LIKE ?{}", i));
                param_values.push(Box::new(format!("%.{}", e.to_lowercase())));
            }
            conditions.push(format!("NOT ({})", ors.join(" OR ")));
        }
    }
    if let Some(from) = query.date_from {
        let i = param_values.len() + 1;
        conditions.push(format!("COALESCE(m.date_taken, m.modified_at) >= ?{}", i));
        param_values.push(Box::new(from));
    }
    if let Some(to) = query.date_to {
        let i = param_values.len() + 1;
        conditions.push(format!("COALESCE(m.date_taken, m.modified_at) <= ?{}", i));
        param_values.push(Box::new(to));
    }
    if let Some(min) = query.rating_min {
        let i = param_values.len() + 1;
        conditions.push(format!("COALESCE(m.rating, 0) >= ?{}", i));
        param_values.push(Box::new(min as i64));
    }
    if let Some(max) = query.rating_max {
        let i = param_values.len() + 1;
        conditions.push(format!("COALESCE(m.rating, 0) <= ?{}", i));
        param_values.push(Box::new(max as i64));
    }
    if let Some(min) = query.size_min {
        let i = param_values.len() + 1;
        conditions.push(format!("m.file_size >= ?{}", i));
        param_values.push(Box::new(min));
    }
    if let Some(max) = query.size_max {
        let i = param_values.len() + 1;
        conditions.push(format!("m.file_size <= ?{}", i));
        param_values.push(Box::new(max));
    }
    (sql_extra, conditions, param_values)
}

pub fn add_folder(conn: &Connection, path: &str) -> rusqlite::Result<Folder> {
    conn.execute("INSERT OR IGNORE INTO folders (path) VALUES (?1)", params![path])?;
    let id = conn.query_row("SELECT id FROM folders WHERE path = ?1", params![path], |row| row.get::<_, i64>(0))?;
    let name = std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| path.to_string());
    Ok(Folder { id, path: path.to_string(), last_scan: None, name })
}

/// Total media count for each registered folder, keyed by folder id.
/// Recursive — includes subfolders, since `media.folder_id` always points
/// at the root folder a subfolder lives under.
pub fn count_media_per_folder(conn: &Connection) -> rusqlite::Result<std::collections::HashMap<i64, i64>> {
    let mut stmt = conn.prepare("SELECT folder_id, COUNT(*) FROM media GROUP BY folder_id")?;
    let mut map = std::collections::HashMap::new();
    let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?;
    for row in rows {
        if let Ok((id, n)) = row { map.insert(id, n); }
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
        let name = std::path::Path::new(&path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| path.clone());
        Ok(Folder { id: row.get(0)?, path, last_scan: row.get(2)?, name })
    })?;
    rows.collect()
}

pub fn remove_folder(conn: &Connection, folder_id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM media WHERE folder_id = ?1", params![folder_id])?;
    conn.execute("DELETE FROM folders WHERE id = ?1", params![folder_id])?;
    Ok(())
}

pub fn update_folder_scan_time(conn: &Connection, folder_id: i64) -> rusqlite::Result<()> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    conn.execute("UPDATE folders SET last_scan = ?1 WHERE id = ?2", params![now, folder_id])?;
    Ok(())
}

pub fn insert_media(
    conn: &Connection, folder_id: i64, filename: &str, full_path: &str,
    media_type: &str, file_size: i64, modified_at: i64,
    width: Option<i64>, height: Option<i64>, thumb_path: Option<&str>,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT OR REPLACE INTO media (folder_id, filename, full_path, media_type, file_size, modified_at, width, height, thumb_path) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![folder_id, filename, full_path, media_type, file_size, modified_at, width, height, thumb_path],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_media(conn: &Connection, query: &MediaQuery) -> rusqlite::Result<Vec<MediaItem>> {
    let (joins, conditions, mut param_values) = build_filters(query);
    let mut sql = String::from(
        "SELECT DISTINCT m.id, m.folder_id, m.filename, m.full_path, m.media_type, \
         m.file_size, m.modified_at, m.width, m.height, m.date_taken, \
         m.thumb_path, m.micro_thumb, \
         COALESCE(m.rating, 0) FROM media m"
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

    let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|b| b.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_refs.as_slice(), |row| {
        Ok(MediaItem {
            id: row.get(0)?, folder_id: row.get(1)?, filename: row.get(2)?,
            full_path: row.get(3)?, media_type: row.get(4)?, file_size: row.get(5)?,
            modified_at: row.get(6)?, width: row.get(7)?, height: row.get(8)?,
            date_taken: row.get(9)?, thumb_path: row.get(10)?, micro_thumb: row.get(11)?,
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
pub fn get_media_skeletons(conn: &Connection, query: &MediaQuery) -> rusqlite::Result<Vec<MediaSkeleton>> {
    let (joins, conditions, param_values) = build_filters(query);
    let mut sql = String::from(
        "SELECT DISTINCT m.id, COALESCE(m.date_taken, m.modified_at), m.media_type FROM media m"
    );
    sql.push_str(&joins);
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    sql.push_str(" ORDER BY ");
    sql.push_str(query.sort.unwrap_or(SortOrder::NameAsc).order_by_sql());

    let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|b| b.as_ref()).collect();
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
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|b| b.as_ref()).collect();
    conn.query_row(&sql, params_refs.as_slice(), |row| row.get(0))
}

/// Returns the lower-case extensions (without leading dot) actually present
/// in the `media` table, sorted alphabetically. Done Rust-side because
/// SQLite has no rfind / regex without an extension.
pub fn get_available_extensions(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT filename FROM media")?;
    let names: Vec<String> = stmt.query_map([], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok()).collect();
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

pub fn create_tag(conn: &Connection, name: &str, parent_id: Option<i64>) -> rusqlite::Result<Tag> {
    let full_path = if let Some(pid) = parent_id {
        let parent_path: String = conn.query_row("SELECT full_path FROM tags WHERE id = ?1", params![pid], |row| row.get(0))?;
        format!("{}>{}", parent_path, name)
    } else { name.to_string() };
    // Append at the end of the chosen parent's child list — get the
    // current max sort_order and add 1. NULL parent_id is matched with
    // IS NULL since `?1 = NULL` is always NULL in SQL.
    let next_order: i64 = match parent_id {
        Some(pid) => conn.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM tags WHERE parent_id = ?1",
            params![pid], |r| r.get(0))?,
        None => conn.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM tags WHERE parent_id IS NULL",
            [], |r| r.get(0))?,
    };
    conn.execute(
        "INSERT INTO tags (name, parent_id, full_path, sort_order, display) VALUES (?1, ?2, ?3, ?4, 1)",
        params![name, parent_id, full_path, next_order],
    )?;
    Ok(Tag {
        id: conn.last_insert_rowid(), name: name.to_string(), parent_id, full_path,
        children: vec![], media_count: 0, color: None, icon: None,
        display: true, sort_order: next_order,
    })
}

pub fn update_tag_appearance(
    conn: &Connection,
    tag_id: i64,
    color: Option<&str>,
    icon: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE tags SET color = ?1, icon = ?2 WHERE id = ?3",
        params![color, icon, tag_id],
    )?;
    Ok(())
}

/// Toggle the "show on gallery thumbnail chips" flag.
pub fn set_tag_display(conn: &Connection, tag_id: i64, display: bool) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE tags SET display = ?1 WHERE id = ?2",
        params![display as i64, tag_id],
    )?;
    Ok(())
}

pub fn get_tag_tree(conn: &Connection) -> rusqlite::Result<Vec<Tag>> {
    // ORDER BY groups children together (so build_tag_tree picks up
    // each sibling list in the right order) and within each group sorts
    // by the user-controlled `sort_order`. Falls back to name for
    // stability when sort_order values tie (e.g. pre-migration rows).
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, t.parent_id, t.full_path, COALESCE(c.cnt, 0), \
                t.color, t.icon, COALESCE(t.display, 1), COALESCE(t.sort_order, 0) \
         FROM tags t \
         LEFT JOIN (SELECT tag_id, COUNT(*) as cnt FROM media_tags GROUP BY tag_id) c \
           ON t.id = c.tag_id \
         ORDER BY COALESCE(t.parent_id, 0), COALESCE(t.sort_order, 0), t.name",
    )?;
    let all_tags: Vec<Tag> = stmt.query_map([], |row| {
        Ok(Tag {
            id: row.get(0)?, name: row.get(1)?, parent_id: row.get(2)?,
            full_path: row.get(3)?, children: vec![], media_count: row.get(4)?,
            color: row.get(5)?, icon: row.get(6)?,
            display: row.get::<_, i64>(7)? != 0,
            sort_order: row.get(8)?,
        })
    })?.collect::<rusqlite::Result<Vec<_>>>()?;
    build_tag_tree(&all_tags)
}

/// Walk `tag_id`'s parent chain (excluding `tag_id` itself) and return the
/// list of ancestor IDs, nearest-parent first.
fn ancestors_of(conn: &Connection, tag_id: i64) -> rusqlite::Result<Vec<i64>> {
    let mut out = Vec::new();
    let mut current: Option<i64> = conn.query_row(
        "SELECT parent_id FROM tags WHERE id = ?1",
        params![tag_id], |r| r.get(0),
    )?;
    while let Some(p) = current {
        out.push(p);
        current = conn.query_row(
            "SELECT parent_id FROM tags WHERE id = ?1",
            params![p], |r| r.get(0),
        )?;
    }
    Ok(out)
}

/// Re-parent `source` and renumber siblings so its position matches `drop`.
/// Refuses no-op self moves and cycle-creating drops (source becoming a
/// descendant of itself). Recomputes `full_path` for the moved subtree.
///
/// After the move, scrubs auto-applied ancestor rows that are no longer
/// justified: for each ancestor A that source had BEFORE the move but
/// not AFTER, walk every media that still has A assigned with
/// `auto_applied=1` and remove the row when no other tag on that media
/// is a descendant of A. Manual assignments (`auto_applied=0`) are
/// always preserved.
///
/// Returns the IDs of media items whose tag set was modified by the
/// cleanup — caller uses this to drive XMP rewrites.
pub fn move_tag(conn: &Connection, source: i64, drop: TagDropPos) -> rusqlite::Result<Vec<i64>> {
    // Resolve target → (new_parent, neighbour) where `neighbour` is the
    // tag we want `source` placed adjacent to, or None for "append at
    // end" (Into).
    let (new_parent, neighbour, place_after): (Option<i64>, Option<i64>, bool) = match drop {
        TagDropPos::Into(t) => {
            let pid: Option<i64> = None;
            // For Into: new parent is `t` itself.
            let _ = pid;
            (Some(t), None, false)
        }
        TagDropPos::Before(t) => {
            let pid: Option<i64> = conn.query_row(
                "SELECT parent_id FROM tags WHERE id = ?1",
                params![t], |r| r.get(0))?;
            (pid, Some(t), false)
        }
        TagDropPos::After(t) => {
            let pid: Option<i64> = conn.query_row(
                "SELECT parent_id FROM tags WHERE id = ?1",
                params![t], |r| r.get(0))?;
            (pid, Some(t), true)
        }
    };

    if Some(source) == new_parent { return Ok(Vec::new()); }
    // Cycle check: source must not be (or contain) the new parent.
    if let Some(np) = new_parent {
        if is_descendant_of(conn, np, source)? || np == source {
            return Ok(Vec::new());
        }
    }
    // Trivial no-op: dropping a tag on itself.
    if Some(source) == neighbour { return Ok(Vec::new()); }

    // Capture the ancestor chain BEFORE the re-parent so we can diff
    // against the post-move chain and figure out which ancestors are
    // no longer justified for media that had `source` (or a descendant
    // of `source`) assigned.
    let old_ancestors: std::collections::HashSet<i64> =
        ancestors_of(conn, source)?.into_iter().collect();

    let tx = conn.unchecked_transaction()?;

    // 1. Re-parent the source row.
    tx.execute(
        "UPDATE tags SET parent_id = ?1 WHERE id = ?2",
        params![new_parent, source],
    )?;

    // 2. Build the ordered sibling list under the new parent, excluding source.
    let mut siblings: Vec<i64> = {
        let mut stmt = match new_parent {
            Some(_) => tx.prepare(
                "SELECT id FROM tags WHERE parent_id = ?1 AND id != ?2 \
                 ORDER BY COALESCE(sort_order, 0), name",
            )?,
            None => tx.prepare(
                "SELECT id FROM tags WHERE parent_id IS NULL AND id != ?2 \
                 ORDER BY COALESCE(sort_order, 0), name",
            )?,
        };
        let rows = match new_parent {
            Some(p) => stmt.query_map(params![p, source], |r| r.get::<_, i64>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?,
            None => stmt.query_map(params![0i64, source], |r| r.get::<_, i64>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?,
        };
        rows
    };

    // 3. Decide source's insertion index in the sibling list.
    let insert_at: usize = match neighbour {
        Some(n) => {
            let pos = siblings.iter().position(|&id| id == n).unwrap_or(siblings.len());
            if place_after { pos + 1 } else { pos }
        }
        None => siblings.len(), // Into → append.
    };
    let insert_at = insert_at.min(siblings.len());
    siblings.insert(insert_at, source);

    // 4. Renumber every sibling (cheap — bounded by direct children of one parent).
    for (i, sid) in siblings.iter().enumerate() {
        tx.execute(
            "UPDATE tags SET sort_order = ?1 WHERE id = ?2",
            params![i as i64, sid],
        )?;
    }

    // 5. Recompute full_path for the moved subtree (parent change ripples down).
    recompute_subtree_paths(&tx, source)?;

    // 6. Scrub auto-applied ancestors that the move just un-justified.
    let new_ancestors: std::collections::HashSet<i64> =
        ancestors_of(&tx, source)?.into_iter().collect();
    let lost: Vec<i64> = old_ancestors
        .into_iter()
        .filter(|a| !new_ancestors.contains(a))
        .collect();

    let mut affected: std::collections::HashSet<i64> = std::collections::HashSet::new();
    for &a in &lost {
        // Resolve the ancestor's current full_path so we can detect its
        // descendants via `LIKE 'path>%'`. (Paths were just recomputed,
        // so descendants of `source` no longer start with this prefix —
        // exactly what we want.)
        let a_path: String = tx.query_row(
            "SELECT full_path FROM tags WHERE id = ?1",
            params![a], |r| r.get(0),
        )?;
        let prefix = format!("{}>%", a_path);

        // Media to clean: those with `a` auto-applied AND no remaining
        // assigned tag whose full_path is a strict descendant of `a`.
        let to_remove: Vec<i64> = {
            let mut stmt = tx.prepare(
                "SELECT mt.media_id FROM media_tags mt \
                 WHERE mt.tag_id = ?1 \
                   AND COALESCE(mt.auto_applied, 0) = 1 \
                   AND NOT EXISTS ( \
                     SELECT 1 FROM media_tags mt2 \
                     JOIN tags t2 ON mt2.tag_id = t2.id \
                     WHERE mt2.media_id = mt.media_id \
                       AND mt2.tag_id != ?1 \
                       AND t2.full_path LIKE ?2 \
                   )",
            )?;
            let v: Vec<i64> = stmt.query_map(params![a, prefix], |r| r.get::<_, i64>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            v
        };

        if !to_remove.is_empty() {
            let mut del = tx.prepare(
                "DELETE FROM media_tags WHERE media_id = ?1 AND tag_id = ?2",
            )?;
            for mid in &to_remove {
                del.execute(params![mid, a])?;
                affected.insert(*mid);
            }
        }
    }

    tx.commit()?;
    Ok(affected.into_iter().collect())
}

/// True if `maybe_descendant` is found anywhere in the subtree rooted at `root`.
fn is_descendant_of(conn: &Connection, maybe_descendant: i64, root: i64) -> rusqlite::Result<bool> {
    let mut current = Some(maybe_descendant);
    while let Some(cid) = current {
        if cid == root { return Ok(true); }
        current = conn.query_row(
            "SELECT parent_id FROM tags WHERE id = ?1",
            params![cid], |r| r.get::<_, Option<i64>>(0),
        ).ok().flatten();
    }
    Ok(false)
}

fn build_tag_tree(tags: &[Tag]) -> rusqlite::Result<Vec<Tag>> {
    let mut roots = Vec::new();
    let mut children_map: std::collections::HashMap<i64, Vec<Tag>> = std::collections::HashMap::new();
    for tag in tags.iter().rev() { if let Some(pid) = tag.parent_id { children_map.entry(pid).or_default().push(tag.clone()); } }
    fn attach_children(tag: &Tag, children_map: &mut std::collections::HashMap<i64, Vec<Tag>>) -> Tag {
        let mut t = tag.clone();
        if let Some(children) = children_map.remove(&tag.id) { t.children = children.iter().rev().map(|c| attach_children(c, children_map)).collect(); }
        t
    }
    for tag in tags { if tag.parent_id.is_none() { roots.push(attach_children(tag, &mut children_map)); } }
    Ok(roots)
}

/// Update `media.rating` for every id in `media_ids` to `rating`.
/// Clamps to 0..=5; values outside that range are silently capped.
pub fn set_rating_bulk(conn: &Connection, media_ids: &[i64], rating: i32) -> rusqlite::Result<()> {
    if media_ids.is_empty() { return Ok(()); }
    let r = rating.clamp(0, 5);
    let tx = conn.unchecked_transaction()?;
    for &id in media_ids {
        tx.execute("UPDATE media SET rating = ?1 WHERE id = ?2", params![r, id])?;
    }
    tx.commit()
}

pub fn delete_tag(conn: &Connection, tag_id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM media_tags WHERE tag_id = ?1", params![tag_id])?;
    conn.execute("DELETE FROM tags WHERE id = ?1", params![tag_id])?;
    Ok(())
}

/// Rename a tag and recompute `full_path` for the tag and every
/// descendant. The full_path schema is `parent>child>grandchild>…`
/// joined by `>`; renaming a non-leaf changes every descendant's
/// canonical path.
pub fn rename_tag(conn: &Connection, tag_id: i64, new_name: &str) -> rusqlite::Result<()> {
    let new_name = new_name.trim();
    if new_name.is_empty() { return Ok(()); }
    let tx = conn.unchecked_transaction()?;
    tx.execute("UPDATE tags SET name = ?1 WHERE id = ?2", params![new_name, tag_id])?;
    recompute_subtree_paths(&tx, tag_id)?;
    tx.commit()?;
    Ok(())
}

/// Walk a subtree rooted at `tag_id` and rebuild every node's
/// `full_path` from its parent's path + its own name. Used after a
/// rename or a re-parent.
fn recompute_subtree_paths(conn: &Connection, tag_id: i64) -> rusqlite::Result<()> {
    fn recurse(conn: &Connection, tag_id: i64) -> rusqlite::Result<()> {
        let (name, parent_id): (String, Option<i64>) = conn.query_row(
            "SELECT name, parent_id FROM tags WHERE id = ?1",
            params![tag_id], |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let new_path = match parent_id {
            Some(pid) => {
                let pp: String = conn.query_row(
                    "SELECT full_path FROM tags WHERE id = ?1",
                    params![pid], |r| r.get(0),
                )?;
                format!("{}>{}", pp, name)
            }
            None => name.clone(),
        };
        conn.execute("UPDATE tags SET full_path = ?1 WHERE id = ?2",
            params![new_path, tag_id])?;
        let mut stmt = conn.prepare("SELECT id FROM tags WHERE parent_id = ?1")?;
        let child_ids: Vec<i64> = stmt
            .query_map(params![tag_id], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for cid in child_ids { recurse(conn, cid)?; }
        Ok(())
    }
    recurse(conn, tag_id)
}

/// Merge `source_id` into `target_id`. Every media item tagged with the
/// source is also given the target tag (skipping duplicates), then the
/// source's media-tag links are dropped. `reparent_children` controls
/// the source's children: when true they're re-parented under the
/// target (preserving their data); when false they're deleted via the
/// `ON DELETE CASCADE` triggered by removing the source.
///
/// Returns Err if the operation would create a duplicate `full_path`
/// (e.g. both source and target have a child of the same name and we're
/// re-parenting). Caller can surface that to the user.
pub fn merge_tag(
    conn: &Connection,
    source_id: i64,
    target_id: i64,
    reparent_children: bool,
) -> rusqlite::Result<()> {
    if source_id == target_id { return Ok(()); }
    let tx = conn.unchecked_transaction()?;
    // 1. Hand the source's media over to the target. INSERT OR IGNORE
    //    so items already tagged with both don't violate the PK.
    tx.execute(
        "INSERT OR IGNORE INTO media_tags (media_id, tag_id)
         SELECT media_id, ?1 FROM media_tags WHERE tag_id = ?2",
        params![target_id, source_id],
    )?;
    tx.execute("DELETE FROM media_tags WHERE tag_id = ?1", params![source_id])?;

    if reparent_children {
        // 2a. Re-parent direct children of source under target.
        tx.execute(
            "UPDATE tags SET parent_id = ?1 WHERE parent_id = ?2",
            params![target_id, source_id],
        )?;
        // 2b. Recompute full_paths for every former-source-child subtree.
        let mut stmt = tx.prepare("SELECT id FROM tags WHERE parent_id = ?1")?;
        let new_children: Vec<i64> = stmt
            .query_map(params![target_id], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);
        for cid in new_children {
            recompute_subtree_paths(&tx, cid)?;
        }
    }
    // (When !reparent_children, `ON DELETE CASCADE` on tags.parent_id
    // will recursively delete the source's descendants in the next step.)

    // 3. Delete the source tag itself.
    tx.execute("DELETE FROM tags WHERE id = ?1", params![source_id])?;
    tx.commit()?;
    Ok(())
}

/// Used by the file-system watcher: insert new media OR update timestamps for
/// existing entries without ever replacing the row (which would cascade-delete
/// media_tags and lose all tag assignments).
pub fn watcher_upsert_media(
    conn: &Connection,
    folder_id: i64,
    filename: &str,
    full_path: &str,
    media_type: &str,
    file_size: i64,
    modified_at: i64,
) -> rusqlite::Result<()> {
    // ON CONFLICT: refresh file_size only. We deliberately do NOT touch
    // `modified_at` for existing rows, so an XMP/IPTC tag write (which
    // bumps the file's mtime) doesn't appear to the user as a new
    // creation date. The first scan still records mtime for fresh files.
    conn.execute(
        "INSERT INTO media (folder_id, filename, full_path, media_type, file_size, modified_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(full_path) DO UPDATE SET
             file_size    = excluded.file_size",
        params![folder_id, filename, full_path, media_type, file_size, modified_at],
    )?;
    Ok(())
}

/// Return the IDs of every strict ancestor of `tag_id` (not including itself).
pub fn get_ancestor_ids(conn: &Connection, tag_id: i64) -> rusqlite::Result<Vec<i64>> {
    let mut result = Vec::new();
    let mut stmt = conn.prepare("SELECT parent_id FROM tags WHERE id = ?1")?;
    let mut current: Option<i64> = stmt
        .query_row(params![tag_id], |row| row.get::<_, Option<i64>>(0))
        .ok()
        .flatten();
    while let Some(pid) = current {
        result.push(pid);
        current = stmt
            .query_row(params![pid], |row| row.get::<_, Option<i64>>(0))
            .ok()
            .flatten();
    }
    Ok(result)
}

/// Walk each tag's parent chain and return the union of all tag IDs + ancestors.
/// Used so that assigning a child tag implicitly assigns its parents too.
fn expand_with_ancestors(conn: &Connection, tag_ids: &[i64]) -> rusqlite::Result<Vec<i64>> {
    let mut result: std::collections::HashSet<i64> = std::collections::HashSet::new();
    let mut stmt = conn.prepare("SELECT parent_id FROM tags WHERE id = ?1")?;
    for &id in tag_ids {
        let mut current = Some(id);
        while let Some(cid) = current {
            if !result.insert(cid) { break; } // already visited (cycle / shared ancestor)
            current = stmt
                .query_row(params![cid], |row| row.get::<_, Option<i64>>(0))
                .ok()
                .flatten();
        }
    }
    Ok(result.into_iter().collect())
}

pub fn assign_tags(conn: &Connection, media_ids: &[i64], tag_ids: &[i64]) -> rusqlite::Result<()> {
    let with_associated = expand_with_associated(conn, tag_ids)?;
    let expanded = expand_with_ancestors(conn, &with_associated)?;
    let manual: std::collections::HashSet<i64> = with_associated.iter().copied().collect();
    let tx = conn.unchecked_transaction()?;
    // For each (media, tag): insert if absent with the correct auto flag.
    // If already present, only upgrade auto→manual (manual→auto is never
    // a downgrade we want — a manual assignment is sticky).
    let mut stmt = tx.prepare(
        "INSERT INTO media_tags (media_id, tag_id, auto_applied) VALUES (?1, ?2, ?3) \
         ON CONFLICT(media_id, tag_id) DO UPDATE SET \
             auto_applied = CASE WHEN excluded.auto_applied = 0 THEN 0 \
                                 ELSE auto_applied END",
    )?;
    for &mid in media_ids {
        for &tid in &expanded {
            let flag: i64 = if manual.contains(&tid) { 0 } else { 1 };
            stmt.execute(params![mid, tid, flag])?;
        }
    }
    drop(stmt);
    tx.commit()
}

/// Bulk-fetch tag assignments for many media items at once.
/// Chunked to stay under SQLite's ~999-parameter limit.
pub fn get_tags_for_media_batch(
    conn: &Connection,
    media_ids: &[i64],
) -> rusqlite::Result<std::collections::HashMap<i64, Vec<Tag>>> {
    let mut result: std::collections::HashMap<i64, Vec<Tag>> = std::collections::HashMap::new();
    if media_ids.is_empty() { return Ok(result); }

    for chunk in media_ids.chunks(500) {
        let placeholders: Vec<String> =
            (1..=chunk.len()).map(|i| format!("?{}", i)).collect();
        let sql = format!(
            "SELECT mt.media_id, t.id, t.name, t.parent_id, t.full_path, t.color, t.icon, \
                    COALESCE(t.display, 1), COALESCE(t.sort_order, 0) \
             FROM media_tags mt JOIN tags t ON mt.tag_id = t.id \
             WHERE mt.media_id IN ({}) ORDER BY t.full_path",
            placeholders.join(",")
        );
        let mut stmt = conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            chunk.iter().map(|i| i as &dyn rusqlite::types::ToSql).collect();
        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            let media_id: i64 = row.get(0)?;
            let tag = Tag {
                id: row.get(1)?,
                name: row.get(2)?,
                parent_id: row.get(3)?,
                full_path: row.get(4)?,
                children: vec![],
                media_count: 0,
                color: row.get(5)?,
                icon: row.get(6)?,
                display: row.get::<_, i64>(7)? != 0,
                sort_order: row.get(8)?,
            };
            Ok((media_id, tag))
        })?;
        for r in rows {
            let (mid, tag) = r?;
            result.entry(mid).or_default().push(tag);
        }
    }
    Ok(result)
}

pub fn remove_tags(conn: &Connection, media_ids: &[i64], tag_ids: &[i64]) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare("DELETE FROM media_tags WHERE media_id = ?1 AND tag_id = ?2")?;
    for &mid in media_ids { for &tid in tag_ids { stmt.execute(params![mid, tid])?; } }
    Ok(())
}

/// Remove every tag assignment from the given media items in one shot.
/// Used by the preview toolbar's "Clear all tags" action.
pub fn remove_all_tags(conn: &Connection, media_ids: &[i64]) -> rusqlite::Result<()> {
    if media_ids.is_empty() { return Ok(()); }
    let mut stmt = conn.prepare("DELETE FROM media_tags WHERE media_id = ?1")?;
    for &mid in media_ids { stmt.execute(params![mid])?; }
    Ok(())
}

/// Get the IDs of all tags associated with the given tag.
pub fn get_associated_tags(conn: &Connection, tag_id: i64) -> rusqlite::Result<Vec<i64>> {
    let mut stmt = conn.prepare(
        "SELECT associated_tag_id FROM tag_associations WHERE tag_id = ?1",
    )?;
    let rows = stmt.query_map(params![tag_id], |row| row.get(0))?;
    rows.collect()
}

/// Replace the associated tags for `tag_id` with `associated_ids`.
pub fn set_associated_tags(conn: &Connection, tag_id: i64, associated_ids: &[i64]) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM tag_associations WHERE tag_id = ?1", params![tag_id])?;
    let mut stmt = conn.prepare(
        "INSERT OR IGNORE INTO tag_associations (tag_id, associated_tag_id) VALUES (?1, ?2)",
    )?;
    for &aid in associated_ids {
        stmt.execute(params![tag_id, aid])?;
    }
    Ok(())
}

/// Expand a set of tag IDs by adding each tag's associated tags (non-recursive,
/// one level only, one-directional).
pub fn expand_with_associated(conn: &Connection, tag_ids: &[i64]) -> rusqlite::Result<Vec<i64>> {
    let mut result: std::collections::HashSet<i64> = tag_ids.iter().copied().collect();
    let mut stmt = conn.prepare(
        "SELECT associated_tag_id FROM tag_associations WHERE tag_id = ?1",
    )?;
    for &id in tag_ids {
        let rows = stmt.query_map(params![id], |row| row.get::<_, i64>(0))?;
        for aid in rows { result.insert(aid?); }
    }
    Ok(result.into_iter().collect())
}

pub fn get_media_tags(conn: &Connection, media_id: i64) -> rusqlite::Result<Vec<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, t.parent_id, t.full_path, t.color, t.icon, \
                COALESCE(t.display, 1), COALESCE(t.sort_order, 0) \
         FROM tags t JOIN media_tags mt ON t.id = mt.tag_id \
         WHERE mt.media_id = ?1 ORDER BY t.full_path",
    )?;
    let rows = stmt.query_map(params![media_id], |row| {
        Ok(Tag {
            id: row.get(0)?, name: row.get(1)?, parent_id: row.get(2)?,
            full_path: row.get(3)?, children: vec![], media_count: 0,
            color: row.get(4)?, icon: row.get(5)?,
            display: row.get::<_, i64>(6)? != 0,
            sort_order: row.get(7)?,
        })
    })?;
    rows.collect()
}

/// Return the IDs of every media item that has the given tag assigned.
/// Call this *before* deleting the tag so the cascade hasn't happened yet.
pub fn get_media_ids_with_tag(conn: &Connection, tag_id: i64) -> rusqlite::Result<Vec<i64>> {
    let mut stmt = conn.prepare("SELECT media_id FROM media_tags WHERE tag_id = ?1")?;
    let rows = stmt.query_map(params![tag_id], |row| row.get(0))?;
    rows.collect()
}

/// For each media ID, return its file path and the full_path of every tag
/// currently assigned to it.  Used to build XMP sidecars after a tag change.
pub fn get_paths_and_current_tags(
    conn: &Connection,
    media_ids: &[i64],
) -> rusqlite::Result<Vec<(String, Vec<String>)>> {
    let mut result = Vec::new();
    let mut path_stmt = conn.prepare("SELECT full_path FROM media WHERE id = ?1")?;
    let mut tag_stmt = conn.prepare(
        "SELECT t.full_path FROM tags t \
         JOIN media_tags mt ON t.id = mt.tag_id \
         WHERE mt.media_id = ?1 ORDER BY t.full_path",
    )?;
    for &mid in media_ids {
        let full_path: String = path_stmt.query_row(params![mid], |row| row.get(0))?;
        let tags: Vec<String> = tag_stmt
            .query_map(params![mid], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        result.push((full_path, tags));
    }
    Ok(result)
}

/// Delete a media row by id. Cascade rules in the schema take care of
/// `media_tags` rows. Caller is responsible for deleting the file/thumbnail
/// on disk.
pub fn delete_media(conn: &Connection, media_id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM media WHERE id = ?1", params![media_id])?;
    Ok(())
}

/// Update both filename and full_path for a media row. Used after rename / move.
pub fn update_media_path(
    conn: &Connection,
    media_id: i64,
    new_filename: &str,
    new_full_path: &str,
    new_folder_id: Option<i64>,
) -> rusqlite::Result<()> {
    if let Some(fid) = new_folder_id {
        conn.execute(
            "UPDATE media SET filename = ?1, full_path = ?2, folder_id = ?3 WHERE id = ?4",
            params![new_filename, new_full_path, fid, media_id],
        )?;
    } else {
        conn.execute(
            "UPDATE media SET filename = ?1, full_path = ?2 WHERE id = ?3",
            params![new_filename, new_full_path, media_id],
        )?;
    }
    Ok(())
}

/// Look up a media item by id (used by the Delete/Copy/Move pending actions
/// when only an id is held).
pub fn get_media_by_id(conn: &Connection, media_id: i64) -> rusqlite::Result<Option<MediaItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, folder_id, filename, full_path, media_type, file_size, modified_at,
                width, height, date_taken, thumb_path, micro_thumb,
                COALESCE(rating, 0)
           FROM media WHERE id = ?1",
    )?;
    let mut rows = stmt.query(params![media_id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(MediaItem {
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
        }))
    } else {
        Ok(None)
    }
}

/// Store a tiny micro-thumbnail BLOB for instant placeholders.
pub fn update_micro_thumb(conn: &Connection, full_path: &str, blob: &[u8]) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE media SET micro_thumb = ?1 WHERE full_path = ?2",
        params![blob, full_path],
    )?;
    Ok(())
}

// ── Statistics ────────────────────────────────────────────────────────────

pub struct LibraryStats {
    pub total_photos: i64,
    pub total_videos: i64,
    pub total_size_bytes: u64,
    pub rating_counts: [i64; 6],  // index 0-5
    pub top_tags: Vec<(String, i64)>,  // (tag_name, count) top 10
    pub monthly_counts: Vec<(String, i64)>,  // ("2024-03", count)
}

pub fn get_statistics(conn: &Connection) -> rusqlite::Result<LibraryStats> {
    let total_photos: i64 = conn.query_row(
        "SELECT COUNT(*) FROM media WHERE media_type = 'image'",
        [],
        |r| r.get(0),
    )?;
    let total_videos: i64 = conn.query_row(
        "SELECT COUNT(*) FROM media WHERE media_type = 'video'",
        [],
        |r| r.get(0),
    )?;
    let total_size_bytes: u64 = conn.query_row(
        "SELECT COALESCE(SUM(file_size), 0) FROM media",
        [],
        |r| r.get::<_, i64>(0).map(|v| v as u64),
    )?;

    // Rating distribution
    let mut rating_counts = [0i64; 6];
    {
        let mut stmt = conn.prepare(
            "SELECT COALESCE(rating, 0) as r, COUNT(*) FROM media GROUP BY r"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })?;
        for r in rows {
            if let Ok((rating, count)) = r {
                let idx = (rating as usize).min(5);
                rating_counts[idx] = count;
            }
        }
    }

    // Top 10 tags by assignment count
    let top_tags = {
        let mut stmt = conn.prepare(
            "SELECT t.name, COUNT(mt.media_id) as cnt
             FROM tags t JOIN media_tags mt ON mt.tag_id = t.id
             GROUP BY t.id ORDER BY cnt DESC LIMIT 10"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
    };

    // Photos per month (last 12 months)
    let monthly_counts = {
        let mut stmt = conn.prepare(
            "SELECT strftime('%Y-%m', datetime(COALESCE(date_taken, modified_at), 'unixepoch')) as month,
                    COUNT(*)
             FROM media
             GROUP BY month
             ORDER BY month DESC
             LIMIT 12"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
    };

    Ok(LibraryStats {
        total_photos,
        total_videos,
        total_size_bytes,
        rating_counts,
        top_tags,
        monthly_counts,
    })
}

// ── Metadata Export ───────────────────────────────────────────────────────

#[derive(serde::Serialize, Default)]
pub struct MediaExport {
    pub filename: String,
    pub full_path: String,
    pub rating: i32,
    pub tags: Vec<String>,
}

/// Find a tag by name (case-insensitive). Returns the tag ID if found.
pub fn find_tag_by_name(conn: &Connection, name: &str) -> Option<i64> {
    conn.query_row(
        "SELECT id FROM tags WHERE LOWER(name) = LOWER(?1) LIMIT 1",
        params![name],
        |row| row.get(0),
    ).ok()
}

/// Find or create a top-level tag by name. Returns the tag ID.
pub fn find_or_create_tag(conn: &Connection, name: &str) -> Option<i64> {
    if let Some(id) = find_tag_by_name(conn, name) {
        return Some(id);
    }
    create_tag(conn, name, None).ok().map(|t| t.id)
}

pub fn export_all_metadata(conn: &Connection) -> rusqlite::Result<Vec<MediaExport>> {
    let mut media_stmt = conn.prepare(
        "SELECT id, filename, full_path, COALESCE(rating, 0) FROM media ORDER BY full_path"
    )?;
    let mut tag_stmt = conn.prepare(
        "SELECT t.full_path FROM tags t
         JOIN media_tags mt ON mt.tag_id = t.id
         WHERE mt.media_id = ?1
         ORDER BY t.full_path"
    )?;

    let rows = media_stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i32>(3)?,
        ))
    })?;

    let mut result = Vec::new();
    for row in rows {
        let (id, filename, full_path, rating) = row?;
        let tags: Vec<String> = tag_stmt.query_map(params![id], |r| r.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .map(|fp| fp.replace('>', " > "))
            .collect();
        result.push(MediaExport { filename, full_path, rating, tags });
    }
    Ok(result)
}
