//! Adapted from PhoTag commit 36cee242a78c0a8cd9ac6b0d63b64265aa0add90.
//! Upstream license: `vendor/photag-source/LICENSE`.
//! Session state persisted across app launches: which folders/tags are
//! expanded in the sidebar, what the user had selected, and where the
//! gallery was scrolled to. Stored as JSON next to `settings.json`.
//!
//! Kept separate from `Settings` because this is "where I left off" data
//! rather than user preferences — it changes constantly and should not
//! show up in any settings UI.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use photag_plugin_database::db::queries::SortOrder;
use std::path::PathBuf;

/// Persisted media filter variants retain PhoTag JSON names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaTypeFilter { All, Photos, Videos, Audio }

/// Persisted tag click behavior retained from PhoTag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TagClickMode { Select, Exclude }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionState {
    #[serde(default)] pub expanded_subdirs: HashSet<String>,
    #[serde(default)] pub expanded_tag_ids: HashSet<i64>,
    #[serde(default)] pub selected_folder_id: Option<i64>,
    #[serde(default)] pub selected_path: Option<String>,
    /// "Folders" or "Tags". Free-form string so adding new tabs later
    /// doesn't break old session files on upgrade.
    #[serde(default)] pub sidebar_tab: String,
    #[serde(default)] pub gallery_scroll: f32,
    /// Persisted left-sidebar width. 0 means "unset" → use the in-app default.
    #[serde(default)] pub sidebar_width: f32,

    // ── Filter state ──────────────────────────────────────────────────
    #[serde(default)] pub search_query: String,
    #[serde(default)] pub sort_order: Option<SortOrder>,
    #[serde(default)] pub media_type_filter: Option<MediaTypeFilter>,
    #[serde(default)] pub selected_extensions: Vec<String>,
    #[serde(default)] pub excluded_extensions: Vec<String>,
    #[serde(default)] pub date_from: Option<String>,
    #[serde(default)] pub date_to: Option<String>,
    #[serde(default)] pub rating_min: i32,
    #[serde(default)] pub rating_max: Option<i32>,
    #[serde(default)] pub selected_tag_ids: Vec<i64>,
    #[serde(default)] pub excluded_tag_ids: Vec<i64>,
    #[serde(default)] pub thumb_size: Option<f32>,

    // ── Gallery date groups ─────────────────────────────────────────────
    /// Month labels currently collapsed in date-grouped gallery view.
    #[serde(default)] pub collapsed_months: HashSet<String>,

    // ── Click modes ──────────────────────────────────────────────────
    #[serde(default)] pub tag_click_mode: Option<TagClickMode>,
    #[serde(default)] pub ext_click_mode: Option<TagClickMode>,

    // ── Tag dropdown expand/collapse ──────────────────────────────────
    /// Tag IDs collapsed in the tag-assign dropdown (preview + gallery).
    #[serde(default)] pub collapsed_dropdown_tag_ids: HashSet<i64>,
}

pub fn default_session_path() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("PhoTag").join("session.json")
}

impl SessionState {
    pub fn load() -> Self {
        std::fs::read_to_string(default_session_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = default_session_path();
        if let Some(p) = path.parent() { std::fs::create_dir_all(p).ok(); }
        if let Ok(s) = serde_json::to_string_pretty(self) {
            std::fs::write(&path, s).ok();
        }
    }
}
