use rusqlite::{Connection, OptionalExtension, params};
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
    /// image/video/audio/file; `file` retains unsupported and extensionless files.
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

fn default_true() -> bool {
    true
}

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
            SortOrder::NewestFirst => "Newest first",
            SortOrder::OldestFirst => "Oldest first",
            SortOrder::BiggestFirst => "Biggest first",
            SortOrder::SmallestFirst => "Smallest first",
            SortOrder::NameAsc => "Name A→Z",
            SortOrder::NameDesc => "Name Z→A",
            SortOrder::HighestRated => "Highest rated",
            SortOrder::LowestRated => "Lowest rated",
        }
    }
    /// SQL ORDER BY fragment (no leading "ORDER BY"). Uses COALESCE so items
    /// without `date_taken` fall back to `modified_at` for time-based sorts.
    fn order_by_sql(self) -> &'static str {
        match self {
            SortOrder::NewestFirst => "COALESCE(m.date_taken, m.modified_at) DESC, m.id DESC",
            SortOrder::OldestFirst => "COALESCE(m.date_taken, m.modified_at) ASC, m.id ASC",
            SortOrder::BiggestFirst => "m.file_size DESC, m.id DESC",
            SortOrder::SmallestFirst => "m.file_size ASC, m.id ASC",
            SortOrder::NameAsc => "m.filename ASC",
            SortOrder::NameDesc => "m.filename DESC",
            // Rated sorts secondary-sort by date so unrated items group
            // together meaningfully rather than by id.
            SortOrder::HighestRated => {
                "COALESCE(m.rating, 0) DESC, COALESCE(m.date_taken, m.modified_at) DESC, m.id DESC"
            }
            SortOrder::LowestRated => {
                "COALESCE(m.rating, 0) ASC, COALESCE(m.date_taken, m.modified_at) DESC, m.id DESC"
            }
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
    /// "image" | "video" | "audio" | "file" — None means all.
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
fn build_filters(
    query: &MediaQuery,
) -> (String, Vec<String>, Vec<Box<dyn rusqlite::types::ToSql>>) {
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

pub struct LibraryStats {
    pub total_photos: i64,
    pub total_videos: i64,
    pub total_size_bytes: u64,
    pub rating_counts: [i64; 6],
    pub top_tags: Vec<(String, i64)>,
    pub monthly_counts: Vec<(String, i64)>,
}

#[derive(serde::Serialize, Default)]
pub struct MediaExport {
    pub filename: String,
    pub full_path: String,
    pub rating: i32,
    pub tags: Vec<String>,
}

mod library;
mod media_ops;
mod metadata;
mod tag_links;
mod tags;

pub use library::*;
pub use media_ops::*;
pub use metadata::*;
pub use tag_links::*;
pub use tags::*;
