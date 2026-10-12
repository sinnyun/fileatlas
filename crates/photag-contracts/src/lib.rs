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

/// Toggle the selected library tag in the gallery; `None` clears tag filtering.
pub struct TagSelectionChanged;

impl Event for TagSelectionChanged {
    const NAME: &'static str = "photag/navigation/tag-selection-changed";
    type Args = Option<i64>;
    type Output = ();
}

/// Notify feature views that indexed media rows have changed.
pub struct MediaIndexChanged;

impl Event for MediaIndexChanged {
    const NAME: &'static str = "photag/indexing/media-index-changed";
    type Args = ();
    type Output = ();
}
