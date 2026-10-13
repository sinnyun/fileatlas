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
