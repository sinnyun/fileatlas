//! Adapted from PhoTag commit 36cee242a78c0a8cd9ac6b0d63b64265aa0add90.
//! Upstream license: `vendor/photag-source/LICENSE`.
//! PhoTag-compatible persisted user settings, separated from GPUI rendering.
//! Schema and serde defaults originate in `vendor/photag-source/src/settings.rs`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiTagGroup {
    pub name: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AiSettings {
    #[serde(default)]
    pub mandatory_groups: Vec<AiTagGroup>,
    /// Automatically run AI analysis on every photo opened in preview.
    #[serde(default = "default_true")]
    pub auto_analyze: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExcludeModifier {
    None,
    Ctrl,
    Shift,
    Alt,
}

impl ExcludeModifier {
    pub fn label(self) -> &'static str {
        match self {
            ExcludeModifier::None => "None",
            ExcludeModifier::Ctrl => "Ctrl",
            ExcludeModifier::Shift => "Shift",
            ExcludeModifier::Alt => "Alt",
        }
    }
    /// Match the same modifier semantics as PhoTag's original keyboard adapter.
    pub fn matches_modifiers(self, ctrl: bool, shift: bool, alt: bool) -> bool {
        match self {
            ExcludeModifier::None => false,
            ExcludeModifier::Ctrl => ctrl,
            ExcludeModifier::Shift => shift,
            ExcludeModifier::Alt => alt,
        }
    }
}

fn default_exclude_modifier() -> ExcludeModifier {
    ExcludeModifier::Ctrl
}

fn default_open_key() -> String {
    "Space".to_string()
}

fn default_accent_name() -> String {
    photag_ui_bridge::theme::DEFAULT_ACCENT_NAME.to_string()
}
/// Mirrors `theme::REGION_DEPTH_BORDER`; kept literal so this model stays UI-free.
fn default_region_separation() -> String {
    "border".to_string()
}
fn default_show_date_dividers() -> bool {
    true
}
fn default_auto_check_updates() -> bool {
    true
}

fn default_search_scopes() -> SearchScopes {
    SearchScopes::default()
}

fn default_tag_search_match() -> TagSearchMatch {
    TagSearchMatch::Contains
}
fn default_tag_search_persist() -> bool {
    true
}
fn default_tag_merge_children() -> TagMergeChildren {
    TagMergeChildren::Reparent
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagHotkey {
    pub key: String, // e.g. "F1", "F2"
    pub tag_id: i64,
}

/// How the Tags-tab search box matches tag names against the query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TagSearchMatch {
    Contains,
    StartsWith,
}

impl TagSearchMatch {
    pub fn label(self) -> &'static str {
        match self {
            TagSearchMatch::Contains => "Contains",
            TagSearchMatch::StartsWith => "Starts with",
        }
    }
    pub fn matches(self, name: &str, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        let n = name.to_lowercase();
        let q = query.to_lowercase();
        match self {
            TagSearchMatch::Contains => n.contains(&q),
            TagSearchMatch::StartsWith => n.starts_with(&q),
        }
    }
}

/// What happens to the children of the source tag when it's merged
/// into a target tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TagMergeChildren {
    /// Re-parent under target so the user's nested organisation is preserved.
    Reparent,
    /// Delete children (and their media-tag links) along with the source.
    Delete,
}

impl TagMergeChildren {
    pub fn label(self) -> &'static str {
        match self {
            TagMergeChildren::Reparent => "Re-parent under target",
            TagMergeChildren::Delete => "Delete",
        }
    }
}

/// Which fields the global search box matches against. Persisted so the
/// user's preferred scope set survives restarts. All three default to true
/// so a fresh launch behaves like "search everywhere".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchScopes {
    #[serde(default = "default_true")]
    pub filenames: bool,
    #[serde(default = "default_true")]
    pub paths: bool,
    #[serde(default = "default_true")]
    pub tags: bool,
    /// Apply the currently selected date range when searching.
    #[serde(default = "default_true")]
    pub time: bool,
}

impl Default for SearchScopes {
    fn default() -> Self {
        Self {
            filenames: true,
            paths: true,
            tags: true,
            time: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub toolbar: ToolbarSettings,
    #[serde(default = "default_exclude_modifier")]
    pub exclude_modifier: ExcludeModifier,
    /// GPUI adapter key name (e.g. "Space", "Enter", "O") that opens the
    /// currently-selected gallery thumbnail in the preview view.
    #[serde(default = "default_open_key")]
    pub open_key: String,
    /// Name of the accent preset from `theme::ACCENT_PRESETS`. Falls back
    /// to the default if the stored value is unknown (e.g. after a rename).
    #[serde(default = "default_accent_name")]
    pub accent_name: String,
    /// How the UI separates its regions: `"border"` hairlines or `"shadow"`.
    /// Applied to `theme::set_region_depth_name` at startup and on change.
    #[serde(default = "default_region_separation")]
    pub region_separation: String,
    /// When true and the gallery is sorted by date, group thumbnails into
    /// month sections separated by `Month Year` headers. Off → flat grid.
    #[serde(default = "default_show_date_dividers")]
    pub show_date_dividers: bool,
    /// Which fields the global search box matches against. Persisted so the
    /// last-selected scope set survives restarts.
    #[serde(default = "default_search_scopes")]
    pub search_scopes: SearchScopes,
    /// Search all Everything-indexed locations instead of the selected folder tree.
    #[serde(default)]
    pub search_entire_computer: bool,
    /// How the Tags-tab search box matches names against the query.
    #[serde(default = "default_tag_search_match")]
    pub tag_search_match: TagSearchMatch,
    /// When true, the Tags-tab search query survives switching between
    /// the Folders / Tags tabs. When false, leaving the Tags tab clears
    /// it so the next visit starts fresh.
    #[serde(default = "default_tag_search_persist")]
    pub tag_search_persist: bool,
    /// Default behaviour for the children of the source tag when a
    /// merge is performed.
    #[serde(default = "default_tag_merge_children")]
    pub tag_merge_children: TagMergeChildren,
    /// Tag hotkeys: maps key name (e.g. "F1") to tag_id.
    #[serde(default)]
    pub tag_hotkeys: Vec<TagHotkey>,
    /// User overrides keyed by lifecycle-owned UI keybinding contribution ID.
    /// An empty value disables that binding; a missing value uses the plugin default.
    #[serde(default)]
    pub keybindings: BTreeMap<String, String>,
    /// Whether the preview-mode filmstrip is currently shown. Toggled
    /// by the toolbar button (when visible) and persisted across
    /// launches.
    #[serde(default)]
    pub show_filmstrip: bool,
    /// Persisted floating-toolbar position (relative to its default
    /// center-bottom anchor) and lock state.
    #[serde(default)]
    pub toolbar_pos: ToolbarPosition,
    #[serde(default)]
    pub ai: AiSettings,
    /// Ask GitHub for a newer release once per launch. The check is a single
    /// background request; nothing is downloaded or installed without the
    /// user confirming first.
    #[serde(default = "default_auto_check_updates")]
    pub auto_check_updates: bool,
    /// Sidebar drive/quick-access/favorites configuration owned by the
    /// Navigation plugin; persisted here so the JSON stays the single store.
    #[serde(default)]
    pub sidebar_folders: photag_contracts::SidebarFoldersConfig,
}

/// Floating preview-toolbar placement & interaction lock. Offsets are
/// in pixels from the toolbar's *default* anchor (center bottom of the
/// preview, lifted by the filmstrip if active). Storing offsets rather
/// than absolute coords means the toolbar stays sensibly placed when
/// the window is resized between launches.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct ToolbarPosition {
    #[serde(default)]
    pub offset_x: f32,
    #[serde(default)]
    pub offset_y: f32,
    #[serde(default)]
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolbarSettings {
    #[serde(default = "default_true")]
    pub zoom_mode_button: bool,
    #[serde(default = "default_true")]
    pub zoom_reset_button: bool,
    #[serde(default = "default_true")]
    pub zoom_presets: bool,
    #[serde(default = "default_true")]
    pub rotate_buttons: bool,
    #[serde(default = "default_true")]
    pub flip_buttons: bool,
    #[serde(default = "default_true")]
    pub tags_button: bool,
    #[serde(default = "default_true")]
    pub delete_button: bool,
    #[serde(default = "default_true")]
    pub copy_button: bool,
    #[serde(default = "default_true")]
    pub move_button: bool,
    /// Filmstrip toggle button (shown/hidden in the floating preview toolbar).
    #[serde(default = "default_true")]
    pub filmstrip_toggle: bool,
    #[serde(default = "default_true")]
    pub explorer_button: bool,
    #[serde(default = "default_true")]
    pub open_external_button: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            toolbar: ToolbarSettings::default(),
            exclude_modifier: ExcludeModifier::Ctrl,
            open_key: default_open_key(),
            accent_name: default_accent_name(),
            region_separation: default_region_separation(),
            show_date_dividers: default_show_date_dividers(),
            search_scopes: SearchScopes::default(),
            search_entire_computer: false,
            tag_search_match: default_tag_search_match(),
            tag_search_persist: default_tag_search_persist(),
            tag_merge_children: default_tag_merge_children(),
            tag_hotkeys: Vec::new(),
            keybindings: BTreeMap::new(),
            show_filmstrip: false,
            toolbar_pos: ToolbarPosition::default(),
            ai: AiSettings::default(),
            auto_check_updates: default_auto_check_updates(),
            sidebar_folders: photag_contracts::SidebarFoldersConfig::default(),
        }
    }
}

impl Default for ToolbarSettings {
    fn default() -> Self {
        Self {
            zoom_mode_button: true,
            zoom_reset_button: true,
            zoom_presets: true,
            rotate_buttons: true,
            flip_buttons: true,
            tags_button: true,
            delete_button: true,
            copy_button: true,
            move_button: true,
            filmstrip_toggle: true,
            explorer_button: true,
            open_external_button: true,
        }
    }
}

impl ToolbarSettings {
    pub fn zoom_category_visible(&self) -> bool {
        self.zoom_mode_button || self.zoom_reset_button || self.zoom_presets
    }
    pub fn manip_category_visible(&self) -> bool {
        self.rotate_buttons || self.flip_buttons
    }
    pub fn tags_category_visible(&self) -> bool {
        self.tags_button
    }
    pub fn file_category_visible(&self) -> bool {
        self.delete_button
            || self.copy_button
            || self.move_button
            || self.explorer_button
            || self.open_external_button
    }
    pub fn view_category_visible(&self) -> bool {
        self.filmstrip_toggle
    }
    pub fn any_visible(&self) -> bool {
        self.zoom_category_visible()
            || self.manip_category_visible()
            || self.tags_category_visible()
            || self.file_category_visible()
            || self.view_category_visible()
    }
}

pub fn default_settings_path() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("PhoTag").join("settings.json")
}

impl Settings {
    pub fn load() -> Self {
        std::fs::read_to_string(default_settings_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = default_settings_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if let Ok(s) = serde_json::to_string_pretty(self) {
            std::fs::write(&path, s).ok();
        }
    }
}
