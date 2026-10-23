//! Adapted from PhoTag commit 36cee242a78c0a8cd9ac6b0d63b64265aa0add90.
//! Upstream license: `vendor/photag-source/LICENSE`.
//! FileAtlas-compatible persisted user settings, separated from GPUI rendering.
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
    fileatlas_ui_bridge::theme::DEFAULT_ACCENT_NAME.to_string()
}
fn default_color_mode() -> String {
    "light".into()
}
fn default_ui_density() -> String {
    "standard".into()
}
fn default_show_date_dividers() -> bool {
    true
}
fn default_auto_check_updates() -> bool {
    true
}

/// How a feature opened from a global shortcut should present itself.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginLaunchMode {
    /// Keep the normal FileAtlas workspace behind the plugin panel.
    #[default]
    WorkspaceAndPanel,
    /// Hide the workspace while the plugin's shared floating panel is open.
    PanelOnly,
    /// Invoke the plugin's primary action without opening its management panel.
    Direct,
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

/// MCP permissions for this FileAtlas instance. High-impact tools still require
/// their own explicit confirmation parameter when Full is selected.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpAccessLevel {
    #[default]
    Off,
    Standard,
    Full,
}

impl McpAccessLevel {
    pub fn key(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Standard => "standard",
            Self::Full => "full",
        }
    }

    pub fn allows_standard(self) -> bool {
        matches!(self, Self::Standard | Self::Full)
    }

    pub fn allows_full(self) -> bool {
        self == Self::Full
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum McpAccessLevelValue {
    Level(McpAccessLevel),
    LegacyBool(bool),
}

fn deserialize_mcp_access_level<'de, D>(deserializer: D) -> Result<McpAccessLevel, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(match McpAccessLevelValue::deserialize(deserializer)? {
        McpAccessLevelValue::Level(level) => level,
        // The previous boolean opted into every mutating tool, so preserve
        // that effective permission when upgrading an existing settings file.
        McpAccessLevelValue::LegacyBool(true) => McpAccessLevel::Full,
        McpAccessLevelValue::LegacyBool(false) => McpAccessLevel::Off,
    })
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
    /// currently-selected file_manager thumbnail in the preview view.
    #[serde(default = "default_open_key")]
    pub open_key: String,
    /// Legacy accent name retained for JSON compatibility. Native ThemeConfig
    /// is now the only paint source; this field is not used as a palette.
    #[serde(default = "default_accent_name")]
    pub accent_name: String,
    /// Semantic workbench color mode: `dark` or `light`.
    #[serde(default = "default_color_mode")]
    pub color_mode: String,
    /// Shared geometry scale for the entire workbench, not a plugin override.
    #[serde(default = "default_ui_density")]
    pub ui_density: String,
    /// Show temporary colored outlines and collect shared geometry probes.
    /// Defaults on during the current user-requested audit phase.
    #[serde(default = "default_true")]
    pub show_ui_audit_frames: bool,
    /// Show temporary red/green/blue colors for title/body/assistant text tiers.
    /// Defaults on until the user finishes the typography audit.
    #[serde(default = "default_true")]
    pub show_typography_audit_colors: bool,
    /// Native GPUI ThemeSet JSON, None selects framework defaults.
    #[serde(default)]
    pub theme_file: Option<PathBuf>,
    /// When true and the file_manager is sorted by date, group thumbnails into
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
    pub sidebar_folders: fileatlas_contracts::SidebarFoldersConfig,
    /// Screenshot tool paths owned by the screenshot launcher feature.
    #[serde(default)]
    pub screenshot_tool: ScreenshotToolSettings,
    /// WorkCrate browsing and image-export defaults; existing settings own persistence.
    #[serde(default)]
    pub workcrate: WorkCrateSettings,
    /// Shortcut presentation preferences keyed by stable feature plugin ID.
    #[serde(default)]
    pub plugin_launch_modes: BTreeMap<String, PluginLaunchMode>,
    /// User enabled state for runtime-controllable feature plugins.
    #[serde(default)]
    pub plugin_enabled: BTreeMap<String, bool>,
    /// Live MCP permissions; legacy `mcp_allow_mutations` booleans are migrated on read.
    #[serde(
        default,
        alias = "mcp_allow_mutations",
        deserialize_with = "deserialize_mcp_access_level"
    )]
    pub mcp_access_level: McpAccessLevel,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScreenshotToolSettings {
    #[serde(default)]
    pub executable: Option<PathBuf>,
    #[serde(default)]
    pub save_directory: Option<PathBuf>,
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
            color_mode: default_color_mode(),
            ui_density: default_ui_density(),
            show_ui_audit_frames: default_true(),
            show_typography_audit_colors: default_true(),
            theme_file: None,
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
            sidebar_folders: fileatlas_contracts::SidebarFoldersConfig::default(),
            screenshot_tool: ScreenshotToolSettings::default(),
            workcrate: WorkCrateSettings::default(),
            plugin_launch_modes: BTreeMap::new(),
            plugin_enabled: BTreeMap::new(),
            mcp_access_level: McpAccessLevel::Off,
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
    base.join("FileAtlas").join("settings.json")
}

impl Settings {
    pub fn plugin_is_enabled(&self, plugin_id: &str) -> bool {
        self.plugin_enabled
            .get(plugin_id)
            .or_else(|| {
                (plugin_id == "fileatlas.project-tree")
                    .then(|| self.plugin_enabled.get("fileatlas.projects"))
                    .flatten()
            })
            .copied()
            .unwrap_or(true)
    }

    pub fn plugin_launch_mode(&self, plugin_id: &str) -> PluginLaunchMode {
        self.plugin_launch_modes
            .get(plugin_id)
            .or_else(|| {
                (plugin_id == "fileatlas.project-tree")
                    .then(|| self.plugin_launch_modes.get("fileatlas.projects"))
                    .flatten()
            })
            .copied()
            .unwrap_or_else(|| {
                if plugin_id == "fileatlas.screenshot" {
                    PluginLaunchMode::Direct
                } else {
                    PluginLaunchMode::WorkspaceAndPanel
                }
            })
    }

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

/// Defaults applied to a new WorkCrate panel; current browsing controls stay local.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkCrateSettings {
    pub sort_by_name: bool,
    pub image_export_jpeg: bool,
}

#[cfg(test)]
mod workcrate_tests {
    use super::*;

    #[test]
    fn new_settings_default_to_light_without_overriding_saved_mode()
    -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(Settings::default().color_mode, "light");
        assert!(Settings::default().show_ui_audit_frames);
        assert!(Settings::default().show_typography_audit_colors);

        let fresh: Settings = serde_json::from_str("{}")?;
        assert_eq!(fresh.color_mode, "light");
        assert!(fresh.show_ui_audit_frames);
        assert!(fresh.show_typography_audit_colors);

        let saved_dark: Settings = serde_json::from_str(r#"{"color_mode":"dark"}"#)?;
        assert_eq!(saved_dark.color_mode, "dark");
        Ok(())
    }

    #[test]
    fn old_settings_get_workcrate_defaults_and_new_choices_round_trip()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut settings: Settings = serde_json::from_str("{}")?;
        assert_eq!(settings.workcrate, WorkCrateSettings::default());
        assert_eq!(settings.ui_density, "standard");
        settings.workcrate.sort_by_name = true;
        settings.workcrate.image_export_jpeg = true;
        settings.show_ui_audit_frames = false;
        settings.show_typography_audit_colors = false;
        let reopened: Settings = serde_json::from_str(&serde_json::to_string(&settings)?)?;
        assert_eq!(reopened.workcrate, settings.workcrate);
        assert!(!reopened.show_ui_audit_frames);
        assert!(!reopened.show_typography_audit_colors);
        Ok(())
    }
}
