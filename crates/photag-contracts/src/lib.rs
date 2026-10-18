#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
//! Shared, UI-framework-neutral contracts for PhoTag feature plugins.
//!
//! This crate deliberately contains no file-management behavior yet. The
//! contracts are introduced as migrations land, so their semantics can be
//! matched against the pinned source rather than guessed in advance.

use cordis::Event;

/// Change the active library folder in the gallery; `None` selects all media.
pub struct FolderSelectionChanged;

impl Event for FolderSelectionChanged {
    const NAME: &'static str = "photag/navigation/folder-selection-changed";
    type Args = Option<i64>;
    type Output = ();
}

/// Set a legacy single-tag gallery filter; `None` clears tag filtering.
pub struct TagSelectionChanged;

impl Event for TagSelectionChanged {
    const NAME: &'static str = "photag/navigation/tag-selection-changed";
    type Args = Option<i64>;
    type Output = ();
}

/// Replace the gallery's complete tag include/exclude filter from the sidebar.
/// `TagSelectionChanged` remains available for older single-tag callers.
pub struct TagFilterSelectionChanged;

impl Event for TagFilterSelectionChanged {
    const NAME: &'static str = "photag/navigation/tag-filter-selection-changed";
    type Args = TagFilterSelection;
    type Output = ();
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TagFilterSelection {
    pub included: Vec<i64>,
    pub excluded: Vec<i64>,
}

/// Settings shared with the tag sidebar without creating a settings/navigation
/// crate dependency cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TagInteractionPreferences {
    pub exclude_modifier: TagExcludeModifier,
    pub search_match: TagSearchMatch,
    pub search_persist: bool,
    pub merge_reparent_children: bool,
}

impl Default for TagInteractionPreferences {
    fn default() -> Self {
        Self {
            exclude_modifier: TagExcludeModifier::Ctrl,
            search_match: TagSearchMatch::Contains,
            search_persist: true,
            merge_reparent_children: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagExcludeModifier {
    None,
    Ctrl,
    Shift,
    Alt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagSearchMatch {
    Contains,
    StartsWith,
}

pub struct TagInteractionPreferencesChanged;

impl Event for TagInteractionPreferencesChanged {
    const NAME: &'static str = "photag/settings/tag-interaction-preferences-changed";
    type Args = TagInteractionPreferences;
    type Output = ();
}

/// Drag payload for assigning a sidebar tag to a gallery media card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TagDragPayload {
    pub source_id: i64,
}

/// Notify feature views that indexed media rows have changed.
pub struct MediaIndexChanged;

impl Event for MediaIndexChanged {
    const NAME: &'static str = "photag/indexing/media-index-changed";
    type Args = ();
    type Output = ();
}

/// Select a nested directory by source path prefix; `None` clears the filter.
pub struct DirectoryPathSelected;

impl Event for DirectoryPathSelected {
    const NAME: &'static str = "photag/navigation/directory-path-selected";
    type Args = Option<String>;
    type Output = ();
}

/// One user-pinned folder in the sidebar's Quick Access section.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct QuickAccessEntry {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// One folder entry inside a user-created favorites group.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FavoriteFolder {
    pub id: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// A user-created group in the sidebar favorites section.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FavoriteGroup {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub folders: Vec<FavoriteFolder>,
}

/// Sidebar drive/quick-access/favorites configuration owned by Navigation
/// and persisted by the Settings plugin without a crate dependency cycle.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SidebarFoldersConfig {
    #[serde(default)]
    pub quick_access: Vec<QuickAccessEntry>,
    #[serde(default)]
    pub favorite_groups: Vec<FavoriteGroup>,
    /// Normalized database bookmark paths already represented in unified groups. Removing a
    /// shortcut/group must not make legacy bookmarks reappear at next startup.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imported_library_paths: Vec<String>,
    #[serde(default = "default_true")]
    pub show_network_drives: bool,
}

impl Default for SidebarFoldersConfig {
    fn default() -> Self {
        Self {
            quick_access: Vec::new(),
            favorite_groups: Vec::new(),
            imported_library_paths: Vec::new(),
            show_network_drives: true,
        }
    }
}

fn default_true() -> bool {
    true
}

/// Revisioned sidebar write; stale asynchronous delivery must not overwrite newer preferences.
#[derive(Clone, Debug)]
pub struct SidebarFoldersUpdate {
    pub revision: u64,
    pub config: SidebarFoldersConfig,
}

/// Published by Navigation after a sidebar folders config mutation; the
/// Settings plugin persists the new value into its JSON document.
pub struct SidebarFoldersChanged;

impl Event for SidebarFoldersChanged {
    const NAME: &'static str = "photag/navigation/sidebar-folders-changed";
    type Args = SidebarFoldersUpdate;
    type Output = ();
}
