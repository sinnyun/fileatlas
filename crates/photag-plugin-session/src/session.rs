//! Adapted from PhoTag commit 36cee242a78c0a8cd9ac6b0d63b64265aa0add90.
//! Upstream license: `vendor/photag-source/LICENSE`.
//!
//! Session state persisted across app launches: which folders/tags are
//! expanded, what the user had selected, and where the gallery was scrolled.
//! Stored as JSON next to `settings.json`.

use photag_plugin_database::db::queries::SortOrder;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

/// Persisted media filter variants retain PhoTag JSON names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaTypeFilter {
    All,
    Photos,
    Videos,
    Audio,
}

/// Persisted tag click behavior retained from PhoTag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TagClickMode {
    Select,
    Exclude,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionState {
    #[serde(default)]
    pub expanded_subdirs: HashSet<String>,
    #[serde(default)]
    pub expanded_tag_ids: HashSet<i64>,
    #[serde(default)]
    pub selected_folder_id: Option<i64>,
    #[serde(default)]
    pub selected_path: Option<String>,
    /// "Folders" or "Tags". Free-form string so adding new tabs later
    /// doesn't break old session files on upgrade.
    #[serde(default)]
    pub sidebar_tab: String,
    #[serde(default)]
    pub gallery_scroll: f32,
    /// Gallery selection restored only when the saved IDs still match the active query.
    #[serde(default)]
    pub selected_media_ids: Vec<i64>,
    /// Reopen the image preview on startup when the source still exists.
    #[serde(default)]
    pub preview_path: Option<String>,
    /// Preview transform state is optional so existing PhoTag session files retain defaults.
    #[serde(default)]
    pub preview_quarter_turns: Option<u8>,
    #[serde(default)]
    pub preview_flip_horizontal: Option<bool>,
    #[serde(default)]
    pub preview_flip_vertical: Option<bool>,
    #[serde(default)]
    pub preview_fit_to_window: Option<bool>,
    #[serde(default)]
    pub preview_zoom_scale: Option<f32>,
    #[serde(default)]
    pub preview_pan_x: Option<f32>,
    #[serde(default)]
    pub preview_pan_y: Option<f32>,
    #[serde(default)]
    pub preview_zoom_at_cursor: Option<bool>,
    #[serde(default)]
    pub preview_show_info: Option<bool>,
    /// Whether the gallery groups date-sorted results by capture month.
    #[serde(default)]
    pub group_by_month: Option<bool>,
    /// Persisted left-sidebar width. 0 means "unset" → use the in-app default.
    #[serde(default)]
    pub sidebar_width: f32,

    // ── Filter state ──────────────────────────────────────────────────
    #[serde(default)]
    pub search_query: String,
    #[serde(default)]
    pub sort_order: Option<SortOrder>,
    #[serde(default)]
    pub media_type_filter: Option<MediaTypeFilter>,
    #[serde(default)]
    pub selected_extensions: Vec<String>,
    #[serde(default)]
    pub excluded_extensions: Vec<String>,
    #[serde(default)]
    pub date_from: Option<String>,
    #[serde(default)]
    pub date_to: Option<String>,
    #[serde(default)]
    pub rating_min: i32,
    #[serde(default)]
    pub rating_max: Option<i32>,
    #[serde(default)]
    pub selected_tag_ids: Vec<i64>,
    #[serde(default)]
    pub excluded_tag_ids: Vec<i64>,
    #[serde(default)]
    pub thumb_size: Option<f32>,

    // ── Gallery date groups ─────────────────────────────────────────────
    /// Month labels currently collapsed in date-grouped gallery view.
    #[serde(default)]
    pub collapsed_months: HashSet<String>,

    // ── Click modes ──────────────────────────────────────────────────
    #[serde(default)]
    pub tag_click_mode: Option<TagClickMode>,
    #[serde(default)]
    pub ext_click_mode: Option<TagClickMode>,

    // ── Tag dropdown expand/collapse ──────────────────────────────────
    /// Tag IDs collapsed in the tag-assign dropdown (preview + gallery).
    #[serde(default)]
    pub collapsed_dropdown_tag_ids: HashSet<i64>,
}

pub fn default_session_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("PhoTag")
        .join("session.json")
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
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).ok();
        }
        if let Ok(s) = serde_json::to_string_pretty(self) {
            std::fs::write(&path, s).ok();
        }
    }
}
