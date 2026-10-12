use crate::db::queries::{self, Folder, MediaItem, MediaSkeleton, SortOrder, Tag};
use crate::scan::{self, ScanProgress};
use crate::ui::sidebar::SidebarTab;
use crate::watcher::FolderWatcher;
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TagClickMode {
    Select,
    Exclude,
}

/// Top-level action selected in the bulk action modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulkActionKind {
    Delete,
    CopyTo,
    MoveTo,
}

impl BulkActionKind {
    pub fn label(self) -> &'static str {
        match self {
            BulkActionKind::Delete => "Delete (recycle bin)",
            BulkActionKind::CopyTo => "Copy to…",
            BulkActionKind::MoveTo => "Move to…",
        }
    }
}

/// Source scope for the bulk action modal — which subset of the
/// library we even consider before applying the per-field filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulkActionSource {
    /// Every media row in every registered folder.
    WholeLibrary,
    /// Restrict to the folder currently selected in the sidebar.
    SelectedFolder,
    /// Restrict to media tagged with the currently-selected tag(s).
    SelectedTag,
    /// Just the items currently selected in the gallery.
    CurrentSelection,
}

impl BulkActionSource {
    pub fn label(self) -> &'static str {
        match self {
            BulkActionSource::WholeLibrary     => "Whole library",
            BulkActionSource::SelectedFolder   => "Selected folder",
            BulkActionSource::SelectedTag      => "Selected tag",
            BulkActionSource::CurrentSelection => "Current selection",
        }
    }
}

/// All configurable filter inputs in the bulk action modal. Empty /
/// `None` fields are treated as "no filter". AND-combined.
#[derive(Debug, Clone, Default)]
pub struct BulkActionFilters {
    pub date_from: Option<chrono::NaiveDate>,
    pub date_to:   Option<chrono::NaiveDate>,
    pub older_than_days: Option<i64>,
    pub newer_than_days: Option<i64>,
    pub rating_min: i32,
    pub rating_max: i32,
    pub name_contains: String,
    pub extension: String,
    pub size_min_mb: Option<f64>,
    pub size_max_mb: Option<f64>,
}

/// Full state of the bulk action modal — `Some` only while the modal
/// is open. Reset to defaults each time the modal opens.
#[derive(Debug, Clone)]
pub struct BulkActionState {
    pub action: BulkActionKind,
    pub source: BulkActionSource,
    pub filters: BulkActionFilters,
}

impl Default for BulkActionState {
    fn default() -> Self {
        Self {
            action: BulkActionKind::Delete,
            source: BulkActionSource::WholeLibrary,
            filters: BulkActionFilters {
                rating_max: 5,
                ..Default::default()
            },
        }
    }
}

/// Active rubber-band drag-selection state. Coordinates are in the
/// gallery's *content* space (i.e. accounting for the scroll offset)
/// so they remain valid as the user drags past the viewport edge and
/// the scroll area auto-scrolls.
#[derive(Debug, Clone)]
pub struct RubberbandState {
    pub start: egui::Pos2,
    pub end: egui::Pos2,
    /// Selection at the moment the drag started — restored when the
    /// drag is cancelled, and used as the base set when `mode == Add`.
    pub initial_selection: Vec<i64>,
    pub mode: RubberbandMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RubberbandMode {
    /// Plain drag → replace the existing selection with the rubber-banded items.
    Replace,
    /// Ctrl+drag → add to the existing selection.
    Add,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MediaTypeFilter {
    All,
    Photos,
    Videos,
    Audio,
}

impl MediaTypeFilter {
    pub fn label(self) -> &'static str {
        match self {
            MediaTypeFilter::All    => "All",
            MediaTypeFilter::Photos => "Photos",
            MediaTypeFilter::Videos => "Videos",
            MediaTypeFilter::Audio  => "Audio",
        }
    }
    pub fn as_db(self) -> Option<&'static str> {
        match self {
            MediaTypeFilter::All    => None,
            // The scanner stores image rows as "image" — match that exact string.
            MediaTypeFilter::Photos => Some("image"),
            MediaTypeFilter::Videos => Some("video"),
            MediaTypeFilter::Audio  => Some("audio"),
        }
    }
}

/// A single undoable operation. Each variant stores enough state to
/// reverse the action and re-apply it (redo). The stacks live on
/// `AppState`; Ctrl+Z pops from `undo_stack`, applies the inverse,
/// and pushes the forward entry onto `redo_stack` (and vice-versa).
#[derive(Debug, Clone)]
pub enum UndoEntry {
    /// Tags were assigned to media. Undo = remove them.
    AssignedTags { media_ids: Vec<i64>, tag_ids: Vec<i64> },
    /// Tags were removed from media. Undo = re-assign them.
    RemovedTags { media_ids: Vec<i64>, tag_ids: Vec<i64> },
    /// All tags were cleared from media. Undo = re-assign the snapshot.
    ClearedTags { media_ids: Vec<i64>, previous_tags: Vec<(i64, Vec<i64>)> },
    /// Rating was changed. Undo = restore previous ratings.
    RatingChanged { media_ids: Vec<i64>, old_rating: i32, new_rating: i32 },
    /// File was renamed. Undo = rename back.
    Renamed { media_id: i64, old_filename: String, new_filename: String },
}

pub enum PendingAction {
    AddFolder,
    RemoveFolder(i64),
    Rescan(i64, String),
    DeleteTag(i64),
    /// Delete a child tag and also remove its ancestor tags from affected media.
    DeleteTagAndCleanParents { tag_id: i64, parent_ids: Vec<i64> },
    AssignTags(Vec<i64>, Vec<i64>),
    RemoveTags(Vec<i64>, Vec<i64>),
    /// Merge `source` into `target`: re-tag all media, then drop the
    /// source. Children behave per `reparent_children`.
    MergeTag { source: i64, target: i64, reparent_children: bool },
    UpdateTagAppearance {
        id: i64,
        /// `Some(new_name)` if the user changed the name in the Edit
        /// dialog; `None` leaves the existing name untouched. Renames
        /// are applied to the tag and cascaded through every
        /// descendant's `full_path`.
        name: Option<String>,
        color: Option<String>,
        icon: Option<String>,
        /// Whether the tag's chip should appear on gallery thumbnails.
        display: bool,
        apply_to_children: bool,
        /// When true, only cascade to descendants whose color AND icon are
        /// both currently NULL (i.e. never had their appearance customized).
        exclude_edited: bool,
    },
    /// Drag-and-drop reorder/reparent of a tag inside the sidebar tree.
    MoveTag {
        source: i64,
        drop: crate::db::queries::TagDropPos,
    },
    /// Remove every tag assignment from the given media items.
    ClearMediaTags(Vec<i64>),
    /// Send a media file to the OS recycle bin and remove its DB row. After
    /// the action runs, the preview advances to the next item if `advance_preview`
    /// is true (used when called from within preview mode).
    DeleteMedia { media_id: i64, advance_preview: bool },
    /// Recycle-bin every file in `media_ids` and drop their DB rows. Used by
    /// the status-bar bulk delete button + the Delete keyboard shortcut on
    /// a multi-selection.
    BulkDeleteMedia { media_ids: Vec<i64> },
    /// Apply `rating` (0..=5) to every id.
    SetRating { media_ids: Vec<i64>, rating: i32 },
    /// Run the bulk action modal's chosen action over the rows
    /// returned by its filters. For Copy/Move the destination is
    /// resolved via a folder picker dialog inside the handler.
    ExecuteBulkAction { state: Box<BulkActionState> },
    /// Open a Save dialog seeded with the source filename and copy the file there.
    CopyMedia { media_id: i64 },
    /// Open a Save dialog and move the file there. Updates the DB so the entry
    /// keeps tracking the new path.
    MoveMedia { media_id: i64, advance_preview: bool },
    /// Move the file into one of the user-registered library folders.
    /// `dest_dir` must be an absolute directory path inside (or equal to) a
    /// folder added to PhoTag.
    MoveMediaTo { media_id: i64, dest_dir: String },
    /// Rename a media file on disk + update its DB row.
    RenameMedia { media_id: i64, new_filename: String },
    /// Create a new physical subfolder under `parent` using `name`. Used by the
    /// "+ New folder" button in the folders sidebar.
    CreateFolder { parent: String, name: String },
    /// Open the file in the system's default application.
    OpenExternal { media_id: i64 },
    /// Reveal the file in Windows Explorer (select it in the parent folder).
    ShowInExplorer { media_id: i64 },
    /// Export all media metadata (tags + ratings) to a JSON file.
    ExportMetadata,
    /// Apply AI-suggested tags to media.
    ApplyAiSuggestions { media_id: i64, labels: Vec<String> },
}

#[derive(Debug, Clone, Default)]
pub struct ExifData {
    pub camera_make: String,
    pub camera_model: String,
    pub lens: String,
    pub focal_length: String,
    pub aperture: String,
    pub shutter_speed: String,
    pub iso: String,
    pub gps: Option<(f64, f64)>,
    pub file_size: u64,
    pub dimensions: Option<(u32, u32)>,
    pub date_taken: String,
    pub color_space: String,
    pub software: String,
}

pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    /// Separate connection used by the UI thread for reads. Lets us run
    /// `load_media` etc. without contending on the write mutex held by the
    /// scan flusher — SQLite WAL allows concurrent readers + one writer.
    pub read_db: Arc<Mutex<Connection>>,
    pub folders: Vec<Folder>,
    pub tag_tree: Vec<Tag>,
    /// Bumped every time `tag_tree` is replaced. Per-frame consumers (gallery
    /// layout, chip rendering) cache derived data keyed on this counter so
    /// they only rebuild when the tree actually changes — not on every frame.
    pub tag_tree_gen: u64,
    /// Compact `(id, effective_date, is_video)` for every media row matching
    /// the current filters, in sort order. Drives the gallery layout (row
    /// count + date headers) without holding tens of thousands of full
    /// `MediaItem`s in memory — that was traced as the per-frame bottleneck:
    /// performance scaled inversely with `state.media.len()`, presumably via
    /// some egui/driver internal that's sensitive to live heap size. With
    /// pagination, only the windowed `media` Vec ever grows beyond a few
    /// hundred entries.
    pub media_skel: Vec<MediaSkeleton>,
    /// Windowed slice of full `MediaItem`s around the visible viewport.
    /// Indexed locally; map to global skeleton index via `media_window_offset`.
    pub media: Vec<MediaItem>,
    /// Global index in `media_skel` of `media[0]`. The window covers
    /// `media_skel[media_window_offset .. media_window_offset + media.len()]`.
    pub media_window_offset: usize,
    pub media_count: i64,
    /// Bumped on every `load_media`. Same purpose as `tag_tree_gen` but for
    /// the media list / layout caches.
    pub media_gen: u64,
    /// Cached query used by `load_media` so the windowed reload path can
    /// re-issue with a different (offset, limit) without re-deriving filters
    /// from the rest of `AppState`. Kept in sync with `load_media`.
    media_query: Option<queries::MediaQuery>,
    pub selected_folder_id: Option<i64>,
    pub selected_path: Option<String>,
    pub selected_tag_ids: Vec<i64>,
    pub excluded_tag_ids: Vec<i64>,
    /// Whether plain (no-modifier) clicks in the Tags tab select or exclude tags.
    pub tag_click_mode: TagClickMode,
    pub selected_media_ids: Vec<i64>,
    pub sidebar_tab: SidebarTab,

    // ── Filter / search state (top filter bar) ──────────────────────────
    pub search_query: String,
    /// Which fields the search box matches against. Mirrors
    /// `Settings::search_scopes`; main.rs keeps the two in sync.
    pub search_scopes: crate::settings::SearchScopes,
    pub sort_order: SortOrder,
    /// Photo / Video / Both
    pub media_type_filter: MediaTypeFilter,
    pub selected_extensions: Vec<String>,
    pub excluded_extensions: Vec<String>,
    pub ext_click_mode: TagClickMode,
    /// Inclusive bounds in local-calendar terms. `None` = unbounded.
    pub date_from: Option<chrono::NaiveDate>,
    pub date_to: Option<chrono::NaiveDate>,
    /// Rating filter — items with `rating < rating_min` or `rating >
    /// rating_max` are excluded. Default 0..=5 (no filter).
    pub rating_min: i32,
    pub rating_max: i32,
    /// Cached list of extensions present in the library, refreshed on
    /// load_media. Used by the filter bar to render its chip list.
    pub available_extensions: Vec<String>,
    /// Tag IDs whose children are currently SHOWN in the sidebar tree.
    /// Default is collapsed (empty set) so a fresh launch is uncluttered.
    pub expanded_tag_ids: HashSet<i64>,
    /// Folder/subdir paths whose children are currently SHOWN. Same default
    /// (empty = collapsed) — persisted across launches via session.json.
    pub expanded_subdirs: HashSet<String>,
    /// Pending gallery scroll offset to apply on the next frame. Set when
    /// restoring a session so the user lands where they left off.
    pub pending_gallery_scroll: Option<f32>,
    /// Last-observed gallery scroll offset, captured each frame so we can
    /// persist it on exit.
    pub gallery_scroll: f32,
    /// Month labels (e.g. "January 2025") currently collapsed in date-grouped view.
    pub collapsed_months: HashSet<String>,
    pub collapsed_months_gen: u64,
    pub thumb_size: f32,
    pub sidebar_width: f32,
    pub media_dirty: bool,
    pub preview_media: Option<MediaItem>,
    pub scan_progress: Option<ScanProgress>,
    pub pending_action: Option<PendingAction>,
    pub show_create_tag: bool,
    pub create_tag_name: String,
    pub create_tag_parent: Option<i64>,
    pub edit_tag: Option<i64>,
    pub edit_tag_name: String,
    pub edit_tag_color: [u8; 3],
    pub edit_tag_color_enabled: bool,
    pub edit_tag_icon: String,
    pub edit_tag_apply_children: bool,
    pub edit_tag_exclude_edited: bool,
    /// "Display on gallery thumbnails" — defaults to true; saved per-tag.
    pub edit_tag_display: bool,
    /// Associated tag IDs edited in the tag-edit dialog.
    pub edit_tag_associated: Vec<i64>,
    /// Live query string for the Tags-tab search input. Whether this
    /// persists across tab switches is controlled by `Settings::tag_search_persist`.
    pub tag_search_query: String,
    /// When `Some`, the merge-target picker modal is open with this
    /// tag id as the source. The picker filters `tag_tree` by
    /// `merge_tag_filter` and excludes the source + all its descendants.
    pub merge_tag_source: Option<i64>,
    pub merge_tag_filter: String,
    /// `Some` only while the bulk action modal is open — carries the
    /// in-flight action / source / filter configuration.
    pub bulk_actions: Option<BulkActionState>,
    /// When set, render a "Delete this tag?" confirmation with parent-cleanup option.
    /// Fields: (tag_id, tag_name, parent_ids, affected_media_count).
    pub confirm_delete_tag: Option<(i64, String, Vec<i64>, usize)>,
    /// When set, render a "Delete this file?" confirmation modal in main.rs.
    pub confirm_delete: Option<i64>,
    /// When set, render a "Delete N files?" confirmation modal for the
    /// status-bar bulk delete button / Delete key on a multi-selection.
    pub confirm_bulk_delete: Option<Vec<i64>>,
    /// "Anchor" media id used by Shift+arrow / Shift+click to extend a
    /// selection range. Plain click / plain arrow resets the anchor;
    /// Ctrl+click / Shift modifications preserve it so the range always
    /// extends from a stable origin.
    pub selection_anchor: Option<i64>,
    /// Active rubber-band selection — `Some` only while the user is
    /// dragging on empty gallery space. Stored in the gallery's content
    /// (scroll-offset-adjusted) coordinate space.
    pub rubberband: Option<RubberbandState>,
    /// One-shot global skeleton index that the gallery should scroll into
    /// view on the next frame. Set by keyboard navigation when the new
    /// selection lands outside the visible viewport.
    pub scroll_to_idx: Option<usize>,
    /// When set, render an inline rename text-field in the preview overlay.
    /// Carries the current edit buffer; commit on Enter, cancel on Esc.
    pub rename_media: Option<(i64, String)>,
    /// When set, show the "+ New folder" modal anchored under that parent path.
    pub new_folder_parent: Option<String>,
    pub new_folder_name: String,
    /// Undo stack — most recent action on top. Capped at 100 entries.
    pub undo_stack: Vec<UndoEntry>,
    /// Redo stack — cleared whenever a new undoable action is pushed.
    pub redo_stack: Vec<UndoEntry>,
    /// Transient toast message shown briefly after undo/redo.
    pub toast_message: Option<(String, std::time::Instant)>,
    /// Tag IDs currently **collapsed** in the tag-assign dropdown (preview
    /// and gallery status-bar). Default is expanded (absent from this set).
    /// Persisted across sessions via `SessionState`.
    pub collapsed_dropdown_tag_ids: HashSet<i64>,
    /// Filter text for the tag-assign dropdown (preview + gallery).
    pub tag_dropdown_filter: String,
    /// Recently assigned tag IDs (most recent first). Capped at 8.
    pub recent_tag_ids: Vec<i64>,
    /// Whether the EXIF metadata overlay panel is visible in preview mode.
    pub show_metadata_panel: bool,
    /// Cached EXIF data for the currently-previewed media (to avoid re-reading on every frame).
    pub cached_exif: Option<(i64, ExifData)>,
    /// Whether the statistics window is open.
    pub show_statistics: bool,
    // ── AI tag suggestions ──────────────────────────────────────────────
    /// Receiver for AI tag suggestion results (from a background thread).
    pub ai_suggestion_rx: Option<std::sync::mpsc::Receiver<Result<Vec<crate::ai::TagSuggestion>, String>>>,
    /// Latest AI suggestions for the popup. `None` = not requested yet.
    pub ai_suggestions: Option<Vec<crate::ai::TagSuggestion>>,
    /// The media ID the current AI suggestions belong to.
    pub ai_suggestions_for: Option<i64>,
    /// Whether the AI model is currently being downloaded.
    pub ai_downloading: bool,
    /// Receiver for model download completion.
    pub ai_download_rx: Option<std::sync::mpsc::Receiver<Result<(), String>>>,
    /// Whether to show the AI suggestions popup.
    pub show_ai_suggestions: bool,
    /// Deferred AI request (set from contexts where `&mut self` isn't available).
    pub ai_request_media_id: Option<i64>,
    /// Mandatory tag groups from AI settings, synced from Settings each frame.
    pub ai_mandatory_groups: Vec<crate::settings::AiTagGroup>,
    /// Media IDs that have already been AI-analyzed this session (avoids re-running).
    pub ai_analyzed_ids: HashSet<i64>,
    /// Whether auto-analyze is enabled (synced from Settings).
    pub ai_auto_analyze: bool,
    /// True when the current inference was triggered automatically (suppresses toasts).
    pub ai_auto_triggered: bool,
    pub preview_tags: Vec<Tag>,
    preview_tags_for: Option<i64>,
    /// Bulk-loaded tag assignments for the currently-listed `media`.
    /// Refreshed by `refresh_media_tags()` after load_media or any tag mutation.
    pub media_tags: HashMap<i64, Vec<Tag>>,
    subdir_cache: HashMap<String, Vec<(String, String, bool)>>,
    /// Total media count per registered root folder (id → count). Refreshed
    /// when the media list is reloaded so the sidebar can show "(N)" badges
    /// without re-querying every frame.
    pub folder_counts: HashMap<i64, i64>,
    /// Lazy per-subfolder media count (full_path → count). Populated on
    /// demand when the sidebar renders a subdir row, and invalidated whenever
    /// `load_media` runs.
    pub subdir_counts: HashMap<String, i64>,
    scan_rx: Option<mpsc::Receiver<Option<ScanProgress>>>,
    scan_cancel: Option<Arc<AtomicBool>>,
    last_scan_reload: usize, // track scanned count to avoid reloading every tick
    watcher: Option<FolderWatcher>,
}

impl AppState {
    /// Returns true when a background scan/thumbnail generation is in progress.
    pub fn is_scanning(&self) -> bool {
        self.scan_rx.is_some()
    }
}

impl AppState {
    pub fn new(db: Connection) -> Self {
        let db = Arc::new(Mutex::new(db));
        // Open a second connection just for UI reads. If it fails, fall back
        // to using the same connection (mutex contention may cause stalls
        // during scan, but functionally correct).
        let read_db = match crate::db::open_read_conn() {
            Ok(c) => Arc::new(Mutex::new(c)),
            Err(_) => db.clone(),
        };
        let folders = { let conn = read_db.lock().unwrap(); queries::list_folders(&conn).unwrap_or_default() };
        let tag_tree = { let conn = read_db.lock().unwrap(); queries::get_tag_tree(&conn).unwrap_or_default() };

        let watcher = FolderWatcher::new();
        if let Some(ref w) = watcher {
            for folder in &folders { w.watch(&folder.path); }
        }

        let mut state = Self {
            db, read_db, folders, tag_tree,
            tag_tree_gen: 1,
            media_skel: Vec::new(),
            media: Vec::new(),
            media_window_offset: 0,
            media_count: 0,
            media_gen: 0,
            media_query: None,
            selected_folder_id: None, selected_path: None,
            selected_tag_ids: Vec::new(), excluded_tag_ids: Vec::new(),
            tag_click_mode: TagClickMode::Select,
            selected_media_ids: Vec::new(),
            sidebar_tab: SidebarTab::Folders,
            search_query: String::new(),
            search_scopes: crate::settings::SearchScopes::default(),
            sort_order: SortOrder::NewestFirst,
            media_type_filter: MediaTypeFilter::All,
            selected_extensions: Vec::new(),
            excluded_extensions: Vec::new(),
            ext_click_mode: TagClickMode::Select,
            date_from: None, date_to: None,
            rating_min: 0, rating_max: 5,
            available_extensions: Vec::new(),
            expanded_tag_ids: HashSet::new(),
            expanded_subdirs: HashSet::new(),
            pending_gallery_scroll: None,
            gallery_scroll: 0.0,
            collapsed_months: HashSet::new(),
            collapsed_months_gen: 0,
            thumb_size: 150.0, sidebar_width: 220.0,
            media_dirty: true, preview_media: None, scan_progress: None,
            pending_action: None, show_create_tag: false,
            create_tag_name: String::new(), create_tag_parent: None,
            edit_tag: None,
            edit_tag_name: String::new(),
            edit_tag_color: [200, 200, 200], edit_tag_color_enabled: false,
            edit_tag_icon: String::new(),
            edit_tag_apply_children: false, edit_tag_exclude_edited: true,
            edit_tag_display: true,
            edit_tag_associated: Vec::new(),
            tag_search_query: String::new(),
            merge_tag_source: None,
            merge_tag_filter: String::new(),
            bulk_actions: None,
            confirm_delete_tag: None,
            confirm_delete: None,
            confirm_bulk_delete: None,
            selection_anchor: None,
            rubberband: None,
            scroll_to_idx: None,
            rename_media: None,
            new_folder_parent: None,
            new_folder_name: String::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            toast_message: None,
            collapsed_dropdown_tag_ids: HashSet::new(),
            tag_dropdown_filter: String::new(),
            recent_tag_ids: Vec::new(),
            show_metadata_panel: false,
            cached_exif: None,
            show_statistics: false,
            ai_suggestion_rx: None,
            ai_suggestions: None,
            ai_suggestions_for: None,
            ai_downloading: false,
            ai_download_rx: None,
            show_ai_suggestions: false,
            ai_request_media_id: None,
            ai_mandatory_groups: Vec::new(),
            ai_analyzed_ids: HashSet::new(),
            ai_auto_analyze: true,
            ai_auto_triggered: false,
            preview_tags: Vec::new(), preview_tags_for: None,
            media_tags: HashMap::new(),
            subdir_cache: HashMap::new(),
            folder_counts: HashMap::new(),
            subdir_counts: HashMap::new(),
            scan_rx: None, scan_cancel: None, last_scan_reload: 0,
            watcher,
        };
        state.load_media();
        state.check_missing_thumbnails();
        state
    }

    pub fn has_watcher(&self) -> bool {
        self.watcher.is_some()
    }

    /// Refresh `preview_tags` when the previewed item changes or after a tag
    /// assign/remove has invalidated the cache (callers pass `force=true`).
    pub fn refresh_preview_tags(&mut self, force: bool) {
        let current_id = self.preview_media.as_ref().map(|m| m.id);
        if !force && current_id == self.preview_tags_for { return; }
        self.preview_tags_for = current_id;
        self.preview_tags = match current_id {
            Some(id) => {
                let conn = self.read_db.lock().unwrap();
                queries::get_media_tags(&conn, id).unwrap_or_default()
            }
            None => Vec::new(),
        };
    }

    /// Drain file system change events and apply them to the database.
    pub fn poll_watcher(&mut self) {
        // Don't interfere while a full scan is running
        if self.scan_rx.is_some() { return; }
        let Some(ref mut w) = self.watcher else { return };
        let (added, removed) = w.poll_changes();
        if added.is_empty() && removed.is_empty() { return; }

        let conn = self.db.lock().unwrap();
        // Removed files: drop from DB
        for path in &removed {
            conn.execute("DELETE FROM media WHERE full_path = ?1", rusqlite::params![path]).ok();
        }
        // Added/modified: insert with no thumbnail (check_missing_thumbnails will fill in)
        for path_str in &added {
            let path = Path::new(path_str);
            // Find which registered folder this file belongs to
            let folder_id = self.folders.iter()
                .find(|f| path_str.starts_with(&f.path))
                .map(|f| f.id);
            let Some(folder_id) = folder_id else { continue };
            let Ok(metadata) = std::fs::metadata(path) else { continue };
            let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let ext = path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
            let media_type = if matches!(ext.as_str(),
                "mp4" | "mov" | "avi" | "mkv" | "wmv" | "flv" | "webm" | "m4v" | "mpg" | "mpeg" | "3gp"
            ) { "video" } else if matches!(ext.as_str(),
                "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a" | "opus" | "ape" | "aiff" | "aif"
            ) { "audio" } else { "image" };
            let file_size = metadata.len() as i64;
            let modified_at = metadata.modified().ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            queries::watcher_upsert_media(&conn, folder_id, &filename, path_str, media_type, file_size, modified_at).ok();
        }
        drop(conn);
        self.media_dirty = true;
    }

    /// Cancel any in-flight scan and replace it with a new one.
    /// Workers periodically check the cancel flag and exit early so we don't
    /// end up with two scan generators competing for CPU and the DB lock.
    fn start_scan<F>(&mut self, spawn: F)
    where
        F: FnOnce(Arc<AtomicBool>) -> mpsc::Receiver<Option<ScanProgress>>,
    {
        if let Some(ref old) = self.scan_cancel {
            old.store(true, Ordering::Relaxed);
        }
        let cancel = Arc::new(AtomicBool::new(false));
        self.scan_rx = Some(spawn(cancel.clone()));
        self.scan_cancel = Some(cancel);
    }

    /// Start background thumbnail generation for items missing thumbnails.
    pub fn check_missing_thumbnails(&mut self) {
        if self.scan_rx.is_some() { return; }
        let missing: Vec<(String, String)> = self.media.iter()
            .filter(|m| m.thumb_path.is_none())
            .map(|m| (m.full_path.clone(), m.media_type.clone()))
            .collect();
        if !missing.is_empty() {
            let db = self.db.clone();
            self.start_scan(|cancel| {
                scan::generate_missing_thumbnails_async(db, missing, cancel)
            });
        }
    }

    /// Window size for the in-memory `state.media`. Anything in the gallery
    /// outside this window will reload on demand. Sized as ~5× a typical
    /// visible viewport (~30 thumbs) so scrolling within the window never
    /// triggers a DB hit.
    pub const MEDIA_WINDOW_SIZE: usize = 200;

    pub fn load_media(&mut self) {
        let conn = self.read_db.lock().unwrap();
        // In the Tags tab, the folder/path selection is hidden — ignore it so
        // tag filters apply across the whole library.
        let on_tags_tab = self.sidebar_tab == SidebarTab::Tags;
        let query = queries::MediaQuery {
            folder_id: if on_tags_tab || self.selected_path.is_some() { None } else { self.selected_folder_id },
            tag_ids: if self.selected_tag_ids.is_empty() { None } else { Some(self.selected_tag_ids.clone()) },
            excluded_tag_ids: if self.excluded_tag_ids.is_empty() { None } else { Some(self.excluded_tag_ids.clone()) },
            path_prefix: if on_tags_tab { None } else { self.selected_path.clone() },
            search: if self.search_query.trim().is_empty() { None } else { Some(self.search_query.clone()) },
            search_in_filenames: self.search_scopes.filenames,
            search_in_paths: self.search_scopes.paths,
            search_in_tags: self.search_scopes.tags,
            media_type: self.media_type_filter.as_db().map(String::from),
            extensions: if self.selected_extensions.is_empty() { None } else { Some(self.selected_extensions.clone()) },
            excluded_extensions: if self.excluded_extensions.is_empty() { None } else { Some(self.excluded_extensions.clone()) },
            // Convert calendar dates → UNIX seconds at the start of `date_from`
            // and end of `date_to` so both bounds are inclusive of full days.
            date_from: self.date_from.and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|dt| dt.and_utc().timestamp()),
            date_to: self.date_to.and_then(|d| d.and_hms_opt(23, 59, 59))
                .map(|dt| dt.and_utc().timestamp()),
            rating_min: if self.rating_min > 0 { Some(self.rating_min) } else { None },
            rating_max: if self.rating_max < 5 { Some(self.rating_max) } else { None },
            size_min: None,
            size_max: None,
            sort: Some(self.sort_order),
            offset: None,
            limit: None,
        };
        // Cheap query: just (id, date, is_video) for every row. Drives layout.
        self.media_skel = queries::get_media_skeletons(&conn, &query).unwrap_or_default();
        self.media_count = self.media_skel.len() as i64;
        // Initial window: first MEDIA_WINDOW_SIZE items. Subsequent windows
        // load lazily from the gallery as the user scrolls.
        let limit = Self::MEDIA_WINDOW_SIZE.min(self.media_skel.len());
        let mut win_query = query.clone();
        win_query.offset = Some(0);
        win_query.limit = Some(limit as i64);
        self.media = queries::get_media(&conn, &win_query).unwrap_or_default();
        self.media_window_offset = 0;
        let ids: Vec<i64> = self.media.iter().map(|m| m.id).collect();
        self.media_tags = queries::get_tags_for_media_batch(&conn, &ids).unwrap_or_default();
        self.available_extensions = queries::get_available_extensions(&conn).unwrap_or_default();
        // Drop any selected/excluded extensions that no longer exist (e.g. all
        // matching files were removed) so the UI doesn't keep stale chips.
        self.selected_extensions.retain(|e| self.available_extensions.contains(e));
        self.excluded_extensions.retain(|e| self.available_extensions.contains(e));
        // Refresh sidebar count badges. Subdir counts are populated lazily
        // (one query per visible subdir row) since they depend on the
        // currently-expanded tree.
        self.folder_counts = queries::count_media_per_folder(&conn).unwrap_or_default();
        self.subdir_counts.clear();
        drop(conn);
        self.media_query = Some(query);
        self.media_dirty = false;
        self.media_gen = self.media_gen.wrapping_add(1);
    }

    /// Slide the in-memory media window so it covers the global index
    /// range `[start, end)`. No-op when the requested range is already
    /// inside the current window.
    pub fn ensure_media_window(&mut self, start: usize, end: usize) {
        let total = self.media_skel.len();
        if total == 0 { return; }
        let end = end.min(total);
        let start = start.min(end);

        let win_size = Self::MEDIA_WINDOW_SIZE.max(end - start);
        let cur_start = self.media_window_offset;
        let cur_end = cur_start + self.media.len();
        // Hit: requested range already inside loaded window.
        if start >= cur_start && end <= cur_end { return; }

        // Center the new window roughly on the requested range so future
        // small scrolls in either direction don't immediately re-trigger.
        let span = end - start;
        let mid = start + span / 2;
        let half = win_size / 2;
        let new_start = mid.saturating_sub(half).min(total.saturating_sub(win_size.min(total)));
        let new_end = (new_start + win_size).min(total);

        let Some(query) = self.media_query.clone() else { return };
        let mut win_query = query;
        win_query.offset = Some(new_start as i64);
        win_query.limit = Some((new_end - new_start) as i64);
        let conn = self.read_db.lock().unwrap();
        let items = queries::get_media(&conn, &win_query).unwrap_or_default();
        let ids: Vec<i64> = items.iter().map(|m| m.id).collect();
        let tags = queries::get_tags_for_media_batch(&conn, &ids).unwrap_or_default();
        drop(conn);
        self.media = items;
        self.media_window_offset = new_start;
        // Merge in the new tags. Keep existing tags for items still in
        // memory elsewhere (e.g. the preview), but tags for items leaving
        // the window get dropped — they'll re-fetch when re-windowed in.
        self.media_tags = tags;
    }

    /// Look up a `MediaItem` by id, scanning the current window first and
    /// falling back to the database. Used by code paths (delete/move/rename,
    /// preview navigation) that have an id but the item may not be inside
    /// the active window.
    pub fn media_by_id(&self, id: i64) -> Option<MediaItem> {
        if let Some(item) = self.media.iter().find(|m| m.id == id) {
            return Some(item.clone());
        }
        let conn = self.read_db.lock().ok()?;
        queries::get_media_by_id(&conn, id).ok().flatten()
    }

    /// Translate a global skeleton index → local index in `state.media`,
    /// or `None` if the global index isn't currently in the window.
    pub fn local_media_idx(&self, global_idx: usize) -> Option<usize> {
        let off = self.media_window_offset;
        if global_idx < off { return None; }
        let local = global_idx - off;
        if local < self.media.len() { Some(local) } else { None }
    }

    /// Locate an id in the skeleton (global index). O(n) over `media_skel`.
    pub fn global_idx_of(&self, id: i64) -> Option<usize> {
        self.media_skel.iter().position(|s| s.id == id)
    }

    /// Replace the cached tag tree and bump the generation counter so per-frame
    /// caches (chip width lookup, flat lookup map) know to rebuild. Single
    /// chokepoint for every assignment so adding new mutation paths can't
    /// accidentally bypass cache invalidation.
    pub fn set_tag_tree(&mut self, tree: Vec<Tag>) {
        self.tag_tree = tree;
        self.tag_tree_gen = self.tag_tree_gen.wrapping_add(1);
    }

    /// Lazy lookup for the per-subfolder media count badge in the sidebar.
    /// First call for a given path hits the DB; subsequent calls hit the
    /// cache until `load_media` clears it.
    pub fn subdir_count(&mut self, path: &str) -> i64 {
        if let Some(&n) = self.subdir_counts.get(path) { return n; }
        let conn = self.read_db.lock().unwrap();
        let n = queries::count_media_under_path(&conn, path).unwrap_or(0);
        drop(conn);
        self.subdir_counts.insert(path.to_string(), n);
        n
    }

    /// Re-query tag assignments for the currently-listed `media`. Cheap enough
    /// to call after any tag mutation that may have changed assignments.
    pub fn refresh_media_tags(&mut self) {
        let conn = self.read_db.lock().unwrap();
        let ids: Vec<i64> = self.media.iter().map(|m| m.id).collect();
        self.media_tags = queries::get_tags_for_media_batch(&conn, &ids).unwrap_or_default();
    }

    pub fn get_subdirs(&mut self, path: &str) -> Vec<(String, String, bool)> {
        if let Some(cached) = self.subdir_cache.get(path) { return cached.clone(); }
        let subdirs = scan::list_subdirectories(path);
        self.subdir_cache.insert(path.to_string(), subdirs.clone());
        subdirs
    }

    /// Show a transient toast for 2 seconds.
    pub fn show_toast(&mut self, msg: String) {
        self.toast_message = Some((msg, std::time::Instant::now()));
    }

    /// Human-readable description of an undo entry.
    fn describe_undo(entry: &UndoEntry) -> String {
        match entry {
            UndoEntry::AssignedTags { media_ids, tag_ids } =>
                format!("Assigned {} tag{} to {} item{}",
                    tag_ids.len(), if tag_ids.len() == 1 { "" } else { "s" },
                    media_ids.len(), if media_ids.len() == 1 { "" } else { "s" }),
            UndoEntry::RemovedTags { media_ids, tag_ids } =>
                format!("Removed {} tag{} from {} item{}",
                    tag_ids.len(), if tag_ids.len() == 1 { "" } else { "s" },
                    media_ids.len(), if media_ids.len() == 1 { "" } else { "s" }),
            UndoEntry::ClearedTags { media_ids, .. } =>
                format!("Cleared tags from {} item{}", media_ids.len(),
                    if media_ids.len() == 1 { "" } else { "s" }),
            UndoEntry::RatingChanged { old_rating, new_rating, .. } =>
                format!("Rating {} → {}", old_rating, new_rating),
            UndoEntry::Renamed { old_filename, new_filename, .. } =>
                format!("Renamed \"{}\" → \"{}\"", old_filename, new_filename),
        }
    }

    /// Push an undo entry. Clears the redo stack and caps at 100.
    pub fn push_undo(&mut self, entry: UndoEntry) {
        self.redo_stack.clear();
        self.undo_stack.push(entry);
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
    }

    /// Common refresh after any tag assignment/removal mutation.
    fn post_tag_mutation(&mut self, media_ids: &[i64]) {
        let conn = self.read_db.lock().unwrap();
        let new_tree = queries::get_tag_tree(&conn).unwrap_or_default();
        let xmp_entries = queries::get_paths_and_current_tags(&conn, media_ids).unwrap_or_default();
        drop(conn);
        self.set_tag_tree(new_tree);
        self.refresh_preview_tags(true);
        self.refresh_media_tags();
        if !self.selected_tag_ids.is_empty() {
            self.media_dirty = true;
        }
        if !xmp_entries.is_empty() {
            std::thread::spawn(move || {
                for (path, tags) in xmp_entries {
                    crate::xmp::write_metadata(&path, &tags).ok();
                }
            });
        }
    }

    /// Execute undo: pop from undo_stack, apply inverse, push to redo_stack.
    pub fn execute_undo(&mut self) {
        let Some(entry) = self.undo_stack.pop() else { return };
        let desc = format!("Undo: {}", Self::describe_undo(&entry));
        self.show_toast(desc);
        match entry {
            UndoEntry::AssignedTags { ref media_ids, ref tag_ids } => {
                let conn = self.db.lock().unwrap();
                queries::remove_tags(&conn, media_ids, tag_ids).ok();
                drop(conn);
                self.post_tag_mutation(media_ids);
                self.redo_stack.push(entry);
            }
            UndoEntry::RemovedTags { ref media_ids, ref tag_ids } => {
                let conn = self.db.lock().unwrap();
                queries::assign_tags(&conn, media_ids, tag_ids).ok();
                drop(conn);
                self.post_tag_mutation(media_ids);
                self.redo_stack.push(entry);
            }
            UndoEntry::ClearedTags { ref media_ids, ref previous_tags } => {
                let conn = self.db.lock().unwrap();
                for (mid, tids) in previous_tags {
                    if !tids.is_empty() {
                        queries::assign_tags(&conn, &[*mid], tids).ok();
                    }
                }
                drop(conn);
                self.post_tag_mutation(media_ids);
                self.redo_stack.push(entry);
            }
            UndoEntry::RatingChanged { ref media_ids, old_rating, new_rating: _ } => {
                let conn = self.db.lock().unwrap();
                queries::set_rating_bulk(&conn, media_ids, old_rating).ok();
                drop(conn);
                for m in self.media.iter_mut() {
                    if media_ids.contains(&m.id) { m.rating = old_rating; }
                }
                if let Some(pm) = self.preview_media.as_mut() {
                    if media_ids.contains(&pm.id) { pm.rating = old_rating; }
                }
                self.redo_stack.push(entry);
            }
            UndoEntry::Renamed { media_id, ref old_filename, ref new_filename } => {
                // Rename back: new_filename → old_filename
                self.do_rename_media(media_id, old_filename);
                self.redo_stack.push(UndoEntry::Renamed {
                    media_id,
                    old_filename: new_filename.clone(),
                    new_filename: old_filename.clone(),
                });
            }
        }
    }

    /// Execute redo: pop from redo_stack, apply forward, push to undo_stack.
    pub fn execute_redo(&mut self) {
        let Some(entry) = self.redo_stack.pop() else { return };
        let desc = format!("Redo: {}", Self::describe_undo(&entry));
        self.show_toast(desc);
        match entry {
            UndoEntry::AssignedTags { ref media_ids, ref tag_ids } => {
                let conn = self.db.lock().unwrap();
                queries::assign_tags(&conn, media_ids, tag_ids).ok();
                drop(conn);
                self.post_tag_mutation(media_ids);
                self.undo_stack.push(entry);
            }
            UndoEntry::RemovedTags { ref media_ids, ref tag_ids } => {
                let conn = self.db.lock().unwrap();
                queries::remove_tags(&conn, media_ids, tag_ids).ok();
                drop(conn);
                self.post_tag_mutation(media_ids);
                self.undo_stack.push(entry);
            }
            UndoEntry::ClearedTags { ref media_ids, .. } => {
                let conn = self.db.lock().unwrap();
                queries::remove_all_tags(&conn, media_ids).ok();
                drop(conn);
                self.post_tag_mutation(media_ids);
                self.undo_stack.push(entry);
            }
            UndoEntry::RatingChanged { ref media_ids, old_rating: _, new_rating } => {
                let conn = self.db.lock().unwrap();
                queries::set_rating_bulk(&conn, media_ids, new_rating).ok();
                drop(conn);
                for m in self.media.iter_mut() {
                    if media_ids.contains(&m.id) { m.rating = new_rating; }
                }
                if let Some(pm) = self.preview_media.as_mut() {
                    if media_ids.contains(&pm.id) { pm.rating = new_rating; }
                }
                self.undo_stack.push(entry);
            }
            UndoEntry::Renamed { media_id, ref old_filename, ref new_filename } => {
                self.do_rename_media(media_id, new_filename);
                self.undo_stack.push(UndoEntry::Renamed {
                    media_id,
                    old_filename: old_filename.clone(),
                    new_filename: new_filename.clone(),
                });
            }
        }
    }

    /// Load EXIF metadata for the given media item. Caches the result to avoid re-reading.
    pub fn load_exif_for(&mut self, media_id: i64) {
        if self.cached_exif.as_ref().map(|(id, _)| *id) == Some(media_id) { return; }
        let path = self.media.iter()
            .find(|m| m.id == media_id)
            .map(|m| m.full_path.clone())
            .or_else(|| self.preview_media.as_ref()
                .filter(|m| m.id == media_id)
                .map(|m| m.full_path.clone()));
        let Some(path) = path else { self.cached_exif = None; return; };

        let mut data = ExifData::default();
        // File size
        if let Ok(meta) = std::fs::metadata(&path) {
            data.file_size = meta.len();
        }
        // EXIF
        if let Ok(file) = std::fs::File::open(&path) {
            let mut bufreader = std::io::BufReader::new(file);
            if let Ok(exif_data) = exif::Reader::new().read_from_container(&mut bufreader) {
                if let Some(f) = exif_data.get_field(exif::Tag::Make, exif::In::PRIMARY) {
                    data.camera_make = f.display_value().to_string();
                }
                if let Some(f) = exif_data.get_field(exif::Tag::Model, exif::In::PRIMARY) {
                    data.camera_model = f.display_value().to_string();
                }
                if let Some(f) = exif_data.get_field(exif::Tag::LensModel, exif::In::PRIMARY) {
                    data.lens = f.display_value().to_string();
                }
                if let Some(f) = exif_data.get_field(exif::Tag::FocalLength, exif::In::PRIMARY) {
                    data.focal_length = f.display_value().with_unit(&exif_data).to_string();
                }
                if let Some(f) = exif_data.get_field(exif::Tag::FNumber, exif::In::PRIMARY) {
                    data.aperture = f.display_value().with_unit(&exif_data).to_string();
                }
                if let Some(f) = exif_data.get_field(exif::Tag::ExposureTime, exif::In::PRIMARY) {
                    data.shutter_speed = f.display_value().with_unit(&exif_data).to_string();
                }
                if let Some(f) = exif_data.get_field(exif::Tag::PhotographicSensitivity, exif::In::PRIMARY) {
                    data.iso = f.display_value().to_string();
                }
                if let Some(f) = exif_data.get_field(exif::Tag::ColorSpace, exif::In::PRIMARY) {
                    data.color_space = f.display_value().to_string();
                }
                if let Some(f) = exif_data.get_field(exif::Tag::Software, exif::In::PRIMARY) {
                    data.software = f.display_value().to_string();
                }
                if let Some(f) = exif_data.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY) {
                    data.date_taken = f.display_value().to_string();
                }
                // GPS coordinates (basic extraction)
                let lat = exif_data.get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY);
                let lon = exif_data.get_field(exif::Tag::GPSLongitude, exif::In::PRIMARY);
                if lat.is_some() && lon.is_some() {
                    // Store raw display for now; full DMS parsing is complex
                    data.gps = Some((0.0, 0.0)); // placeholder indicating GPS is present
                }
            }
        }
        // Dimensions from the media record
        if let Some(item) = self.media.iter().find(|m| m.id == media_id)
            .or_else(|| self.preview_media.as_ref().filter(|m| m.id == media_id))
        {
            if let (Some(w), Some(h)) = (item.width, item.height) {
                data.dimensions = Some((w as u32, h as u32));
            }
        }

        self.cached_exif = Some((media_id, data));
    }

    /// Poll AI background tasks: model download, inference, and auto-trigger.
    pub fn poll_ai(&mut self) {
        // Poll model download
        if let Some(ref rx) = self.ai_download_rx {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok(()) => {
                        self.show_toast("AI model ready!".into());
                        self.ai_downloading = false;
                    }
                    Err(e) => {
                        self.show_toast(format!("Model download failed: {}", e));
                        self.ai_downloading = false;
                    }
                }
                self.ai_download_rx = None;
            }
        }
        // Process deferred AI request (set from gallery context menu)
        if let Some(mid) = self.ai_request_media_id.take() {
            self.request_ai_suggestions(mid, false);
        }
        // Poll inference results
        if let Some(ref rx) = self.ai_suggestion_rx {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok(suggestions) => {
                        if let Some(mid) = self.ai_suggestions_for {
                            self.ai_analyzed_ids.insert(mid);
                        }
                        self.ai_suggestions = Some(suggestions);
                        self.show_ai_suggestions = true;
                    }
                    Err(e) => {
                        // Don't toast on auto-analyze failures
                        if !self.ai_auto_triggered {
                            self.show_toast(format!("AI: {}", e));
                        }
                        self.ai_suggestions = None;
                    }
                }
                self.ai_suggestion_rx = None;
                self.ai_auto_triggered = false;
            }
        }
        // Auto-trigger: when previewing an image that hasn't been analyzed yet
        if self.ai_auto_analyze
            && self.ai_suggestion_rx.is_none()
            && !self.ai_downloading
            && crate::ai::model_exists()
        {
            if let Some(ref pm) = self.preview_media {
                let mid = pm.id;
                if !self.ai_analyzed_ids.contains(&mid)
                    && self.ai_suggestions_for != Some(mid)
                {
                    // Check it's an image, not a video or audio file
                    let ext = std::path::Path::new(&pm.full_path)
                        .extension().unwrap_or_default()
                        .to_string_lossy().to_lowercase();
                    if !matches!(ext.as_str(), "mp4" | "mov" | "avi" | "mkv" | "wmv" | "flv" | "webm" | "m4v" | "mpg" | "mpeg" | "3gp"
                        | "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a" | "opus" | "ape" | "aiff" | "aif") {
                        self.ai_suggestions_for = Some(mid);
                        self.ai_auto_triggered = true;
                        self.ai_suggestion_rx = Some(crate::ai::suggest_tags_async(
                            pm.full_path.clone(), 8, self.ai_mandatory_groups.clone(),
                        ));
                    }
                }
            }
        }
    }

    /// Trigger AI tag suggestion for a media item. `auto` = true when
    /// called from the auto-analyze path (suppresses toasts on failure).
    pub fn request_ai_suggestions(&mut self, media_id: i64, auto: bool) {
        if self.ai_downloading { return; }
        if self.ai_suggestion_rx.is_some() { return; } // already running

        let path = self.media.iter()
            .find(|m| m.id == media_id)
            .map(|m| m.full_path.clone())
            .or_else(|| self.preview_media.as_ref()
                .filter(|m| m.id == media_id)
                .map(|m| m.full_path.clone()));
        let Some(image_path) = path else { return };

        // Check if it's a video or audio — we can't do inference on those
        let ext = std::path::Path::new(&image_path)
            .extension().unwrap_or_default()
            .to_string_lossy().to_lowercase();
        if matches!(ext.as_str(), "mp4" | "mov" | "avi" | "mkv" | "wmv" | "flv" | "webm" | "m4v" | "mpg" | "mpeg" | "3gp"
            | "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a" | "opus" | "ape" | "aiff" | "aif") {
            if !auto { self.show_toast("AI suggestions are only available for images".into()); }
            return;
        }

        if !crate::ai::model_exists() {
            if auto { return; } // don't auto-download
            self.ai_downloading = true;
            self.ai_download_rx = Some(crate::ai::download_model_async());
            self.show_toast("Downloading AI model (~14 MB)...".into());
            self.ai_suggestions_for = Some(media_id);
            return;
        }

        self.ai_suggestions_for = Some(media_id);
        self.ai_auto_triggered = auto;
        self.ai_suggestion_rx = Some(crate::ai::suggest_tags_async(image_path, 8, self.ai_mandatory_groups.clone()));
        if !auto { self.show_toast("Analyzing image...".into()); }
    }

    pub fn process_pending(&mut self) {
        let action = self.pending_action.take();
        match action {
            Some(PendingAction::AddFolder) => {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    let path_str = path.to_string_lossy().to_string();
                    let conn = self.db.lock().unwrap();
                    if let Ok(folder) = queries::add_folder(&conn, &path_str) {
                        let folder_id = folder.id;
                        let folder_path = folder.path.clone();
                        self.folders.push(folder);
                        drop(conn);
                        if let Some(ref w) = self.watcher { w.watch(&folder_path); }
                        let db = self.db.clone();
                        self.start_scan(|cancel| {
                            scan::scan_folder_async(db, folder_id, folder_path, cancel)
                        });
                    }
                }
            }
            Some(PendingAction::RemoveFolder(id)) => {
                // Cancel any in-flight scan first. The scan may be writing
                // thumbnails for files in the folder we're about to delete —
                // both wasted work and a leak of orphan thumb files.
                if let Some(ref c) = self.scan_cancel { c.store(true, Ordering::Relaxed); }
                self.scan_cancel = None;
                self.scan_rx = None;
                self.scan_progress = None;
                self.last_scan_reload = 0;

                let folder_path = self.folders.iter().find(|f| f.id == id).map(|f| f.path.clone());
                let conn = self.db.lock().unwrap();
                queries::remove_folder(&conn, id).ok();
                drop(conn);
                if let (Some(ref w), Some(p)) = (self.watcher.as_ref(), folder_path) {
                    w.unwatch(&p);
                }
                self.folders.retain(|f| f.id != id);
                if self.selected_folder_id == Some(id) { self.selected_folder_id = None; self.selected_path = None; }
                self.media_dirty = true;
            }
            Some(PendingAction::Rescan(id, path)) => {
                { let conn = self.db.lock().unwrap(); conn.execute("DELETE FROM media WHERE folder_id = ?1", rusqlite::params![id]).ok(); }
                let db = self.db.clone();
                self.start_scan(|cancel| scan::scan_folder_async(db, id, path, cancel));
            }
            Some(PendingAction::DeleteTag(id)) => {
                let conn = self.db.lock().unwrap();
                // Collect affected media IDs before the cascade wipes media_tags
                let affected = queries::get_media_ids_with_tag(&conn, id).unwrap_or_default();
                queries::delete_tag(&conn, id).ok();
                let new_tree = queries::get_tag_tree(&conn).unwrap_or_default();
                // Re-query remaining tags for each affected item (deleted tag is now gone)
                let xmp_entries = queries::get_paths_and_current_tags(&conn, &affected).unwrap_or_default();
                drop(conn);
                self.set_tag_tree(new_tree);
                self.selected_tag_ids.retain(|&tid| tid != id);
                self.excluded_tag_ids.retain(|&tid| tid != id);
                self.media_dirty = true;
                self.refresh_preview_tags(true);
                if !xmp_entries.is_empty() {
                    std::thread::spawn(move || {
                        for (path, tags) in xmp_entries {
                            crate::xmp::write_metadata(&path, &tags).ok();
                        }
                    });
                }
            }
            Some(PendingAction::DeleteTagAndCleanParents { tag_id, parent_ids: _ }) => {
                let conn = self.db.lock().unwrap();
                let affected = queries::get_media_ids_with_tag(&conn, tag_id).unwrap_or_default();
                let ancestors = queries::get_ancestor_ids(&conn, tag_id).unwrap_or_default();
                if !affected.is_empty() && !ancestors.is_empty() {
                    queries::remove_tags(&conn, &affected, &ancestors).ok();
                }
                queries::delete_tag(&conn, tag_id).ok();
                let new_tree = queries::get_tag_tree(&conn).unwrap_or_default();
                let xmp_entries = queries::get_paths_and_current_tags(&conn, &affected).unwrap_or_default();
                drop(conn);
                self.set_tag_tree(new_tree);
                self.selected_tag_ids.retain(|&tid| tid != tag_id);
                self.excluded_tag_ids.retain(|&tid| tid != tag_id);
                self.media_dirty = true;
                self.refresh_preview_tags(true);
                self.refresh_media_tags();
                if !xmp_entries.is_empty() {
                    std::thread::spawn(move || {
                        for (path, tags) in xmp_entries {
                            crate::xmp::write_metadata(&path, &tags).ok();
                        }
                    });
                }
            }
            Some(PendingAction::AssignTags(media_ids, tag_ids)) => {
                let conn = self.db.lock().unwrap();
                queries::assign_tags(&conn, &media_ids, &tag_ids).ok();
                drop(conn);
                // Track recently-assigned tags (most recent first, cap 8)
                for &tid in tag_ids.iter().rev() {
                    self.recent_tag_ids.retain(|&id| id != tid);
                    self.recent_tag_ids.insert(0, tid);
                }
                self.recent_tag_ids.truncate(8);
                self.push_undo(UndoEntry::AssignedTags {
                    media_ids: media_ids.clone(), tag_ids,
                });
                self.post_tag_mutation(&media_ids);
            }
            Some(PendingAction::RemoveTags(media_ids, tag_ids)) => {
                let conn = self.db.lock().unwrap();
                queries::remove_tags(&conn, &media_ids, &tag_ids).ok();
                drop(conn);
                self.push_undo(UndoEntry::RemovedTags {
                    media_ids: media_ids.clone(), tag_ids: tag_ids.clone(),
                });
                // If we were filtering by any of the removed tags, the media
                // list may need to refresh too.
                if self.selected_tag_ids.iter().any(|t| tag_ids.contains(t)) {
                    self.media_dirty = true;
                }
                self.post_tag_mutation(&media_ids);
            }
            Some(PendingAction::MergeTag { source, target, reparent_children }) => {
                let conn = self.db.lock().unwrap();
                let xmp_targets = queries::get_media_ids_with_tag(&conn, source).unwrap_or_default();
                if let Err(e) = queries::merge_tag(&conn, source, target, reparent_children) {
                    eprintln!("merge_tag failed: {}", e);
                }
                let new_tree = queries::get_tag_tree(&conn).unwrap_or_default();
                let xmp_entries = queries::get_paths_and_current_tags(&conn, &xmp_targets).unwrap_or_default();
                drop(conn);
                self.set_tag_tree(new_tree);
                self.selected_tag_ids.retain(|&tid| tid != source);
                self.excluded_tag_ids.retain(|&tid| tid != source);
                self.media_dirty = true;
                self.refresh_preview_tags(true);
                self.refresh_media_tags();
                if !xmp_entries.is_empty() {
                    std::thread::spawn(move || {
                        for (path, tags) in xmp_entries {
                            crate::xmp::write_metadata(&path, &tags).ok();
                        }
                    });
                }
            }
            Some(PendingAction::MoveTag { source, drop: drop_pos }) => {
                let conn = self.db.lock().unwrap();
                let affected = match queries::move_tag(&conn, source, drop_pos) {
                    Ok(v) => v,
                    Err(e) => { eprintln!("move_tag failed: {}", e); Vec::new() }
                };
                let new_tree = queries::get_tag_tree(&conn).unwrap_or_default();
                let xmp_entries = if affected.is_empty() {
                    Vec::new()
                } else {
                    queries::get_paths_and_current_tags(&conn, &affected).unwrap_or_default()
                };
                drop(conn);
                self.set_tag_tree(new_tree);
                self.refresh_preview_tags(true);
                self.refresh_media_tags();
                if !affected.is_empty() {
                    // A tag-set change may shift this item in/out of an
                    // active tag filter; let the gallery requery.
                    self.media_dirty = true;
                }
                if !xmp_entries.is_empty() {
                    std::thread::spawn(move || {
                        for (path, tags) in xmp_entries {
                            crate::xmp::write_metadata(&path, &tags).ok();
                        }
                    });
                }
            }
            Some(PendingAction::ClearMediaTags(media_ids)) => {
                // Snapshot existing tags for undo
                let previous_tags: Vec<(i64, Vec<i64>)> = {
                    let conn = self.read_db.lock().unwrap();
                    let batch = queries::get_tags_for_media_batch(&conn, &media_ids).unwrap_or_default();
                    batch.into_iter()
                        .map(|(mid, tags)| (mid, tags.into_iter().map(|t| t.id).collect()))
                        .collect()
                };
                let conn = self.db.lock().unwrap();
                queries::remove_all_tags(&conn, &media_ids).ok();
                drop(conn);
                self.push_undo(UndoEntry::ClearedTags {
                    media_ids: media_ids.clone(),
                    previous_tags,
                });
                if !self.selected_tag_ids.is_empty() {
                    self.media_dirty = true;
                }
                self.post_tag_mutation(&media_ids);
            }
            Some(PendingAction::UpdateTagAppearance { id, name, color, icon, display, apply_to_children, exclude_edited }) => {
                let mut all_descendants = Vec::new();
                collect_descendants(&self.tag_tree, id, &mut all_descendants);

                let conn = self.db.lock().unwrap();
                if let Some(new_name) = name.as_deref() {
                    queries::rename_tag(&conn, id, new_name).ok();
                }
                queries::update_tag_appearance(&conn, id, color.as_deref(), icon.as_deref()).ok();
                queries::set_tag_display(&conn, id, display).ok();
                for child in &all_descendants {
                    queries::set_tag_display(&conn, child.id, display).ok();
                    if !apply_to_children {
                        continue;
                    }
                    if exclude_edited && (child.color.is_some() || child.icon.is_some()) {
                        continue;
                    }
                    queries::update_tag_appearance(&conn, child.id, color.as_deref(), icon.as_deref()).ok();
                }
                let new_tree = queries::get_tag_tree(&conn).unwrap_or_default();
                drop(conn);
                self.set_tag_tree(new_tree);
                self.refresh_preview_tags(true);
                self.refresh_media_tags();
            }
            Some(PendingAction::DeleteMedia { media_id, advance_preview }) => {
                self.do_delete_media(media_id, advance_preview);
            }
            Some(PendingAction::BulkDeleteMedia { media_ids }) => {
                self.do_bulk_delete_media(&media_ids);
            }
            Some(PendingAction::SetRating { media_ids, rating }) => {
                // Snapshot old rating for undo. Assumes bulk-rating sets
                // one value; capture any representative item's prior rating.
                let old_rating = self.media.iter()
                    .find(|m| media_ids.contains(&m.id))
                    .map(|m| m.rating)
                    .or_else(|| self.preview_media.as_ref()
                        .filter(|m| media_ids.contains(&m.id))
                        .map(|m| m.rating))
                    .unwrap_or(0);
                let conn = self.db.lock().unwrap();
                queries::set_rating_bulk(&conn, &media_ids, rating).ok();
                drop(conn);
                // Patch the in-memory windowed media so the UI reflects
                // the new rating immediately, without a full reload.
                for m in self.media.iter_mut() {
                    if media_ids.contains(&m.id) { m.rating = rating.clamp(0, 5); }
                }
                if let Some(pm) = self.preview_media.as_mut() {
                    if media_ids.contains(&pm.id) { pm.rating = rating.clamp(0, 5); }
                }
                if old_rating != rating {
                    self.push_undo(UndoEntry::RatingChanged {
                        media_ids, old_rating, new_rating: rating,
                    });
                }
            }
            Some(PendingAction::ExecuteBulkAction { state }) => {
                self.do_execute_bulk_action(*state);
            }
            Some(PendingAction::CopyMedia { media_id }) => {
                self.do_copy_media(media_id);
            }
            Some(PendingAction::MoveMedia { media_id, advance_preview }) => {
                self.do_move_media(media_id, advance_preview);
            }
            Some(PendingAction::MoveMediaTo { media_id, dest_dir }) => {
                self.do_move_media_to(media_id, &dest_dir);
            }
            Some(PendingAction::RenameMedia { media_id, new_filename }) => {
                // Snapshot old filename for undo
                let old_filename = self.media.iter()
                    .find(|m| m.id == media_id)
                    .map(|m| m.filename.clone())
                    .or_else(|| self.preview_media.as_ref()
                        .filter(|m| m.id == media_id)
                        .map(|m| m.filename.clone()));
                self.do_rename_media(media_id, &new_filename);
                if let Some(old_fn) = old_filename {
                    if old_fn != new_filename {
                        self.push_undo(UndoEntry::Renamed {
                            media_id,
                            old_filename: old_fn,
                            new_filename,
                        });
                    }
                }
            }
            Some(PendingAction::CreateFolder { parent, name }) => {
                self.do_create_folder(&parent, &name);
            }
            Some(PendingAction::OpenExternal { media_id }) => {
                let path = self.media.iter()
                    .find(|m| m.id == media_id)
                    .map(|m| m.full_path.clone())
                    .or_else(|| self.preview_media.as_ref()
                        .filter(|m| m.id == media_id)
                        .map(|m| m.full_path.clone()));
                if let Some(p) = path {
                    #[cfg(target_os = "windows")]
                    { let _ = std::process::Command::new("cmd").args(["/C", "start", "", &p]).spawn(); }
                    #[cfg(target_os = "linux")]
                    { let _ = std::process::Command::new("xdg-open").arg(&p).spawn(); }
                    #[cfg(target_os = "macos")]
                    { let _ = std::process::Command::new("open").arg(&p).spawn(); }
                }
            }
            Some(PendingAction::ShowInExplorer { media_id }) => {
                let path = self.media.iter()
                    .find(|m| m.id == media_id)
                    .map(|m| m.full_path.clone())
                    .or_else(|| self.preview_media.as_ref()
                        .filter(|m| m.id == media_id)
                        .map(|m| m.full_path.clone()));
                if let Some(p) = path {
                    #[cfg(target_os = "windows")]
                    { let _ = std::process::Command::new("explorer").args(["/select,", &p]).spawn(); }
                    #[cfg(target_os = "linux")]
                    {
                        if let Some(dir) = std::path::Path::new(&p).parent() {
                            let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
                        }
                    }
                    #[cfg(target_os = "macos")]
                    { let _ = std::process::Command::new("open").args(["-R", &p]).spawn(); }
                }
            }
            Some(PendingAction::ApplyAiSuggestions { media_id, labels }) => {
                // Create tags that don't exist yet, then assign them all.
                let conn = self.db.lock().unwrap();
                let mut tag_ids = Vec::new();
                for label in &labels {
                    // Try to find existing tag by name (case-insensitive)
                    let existing = queries::find_tag_by_name(&conn, label);
                    match existing {
                        Some(id) => tag_ids.push(id),
                        None => {
                            // Create as a top-level tag under an "AI" parent
                            let ai_parent = queries::find_or_create_tag(&conn, "AI Suggested");
                            if let Some(parent_id) = ai_parent {
                                if let Ok(tag) = queries::create_tag(&conn, label, Some(parent_id)) {
                                    tag_ids.push(tag.id);
                                }
                            } else if let Ok(tag) = queries::create_tag(&conn, label, None) {
                                tag_ids.push(tag.id);
                            }
                        }
                    }
                }
                if !tag_ids.is_empty() {
                    queries::assign_tags(&conn, &[media_id], &tag_ids).ok();
                    drop(conn);
                    for &tid in tag_ids.iter().rev() {
                        self.recent_tag_ids.retain(|&id| id != tid);
                        self.recent_tag_ids.insert(0, tid);
                    }
                    self.recent_tag_ids.truncate(8);
                    self.push_undo(UndoEntry::AssignedTags {
                        media_ids: vec![media_id], tag_ids,
                    });
                    self.post_tag_mutation(&[media_id]);
                    self.show_toast(format!("Applied {} AI tag{}", labels.len(),
                        if labels.len() == 1 { "" } else { "s" }));
                } else {
                    drop(conn);
                }
                self.show_ai_suggestions = false;
            }
            Some(PendingAction::ExportMetadata) => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Export metadata backup")
                    .add_filter("JSON", &["json"])
                    .set_file_name("photag_backup.json")
                    .save_file()
                {
                    let conn = self.read_db.lock().unwrap();
                    let export = queries::export_all_metadata(&conn).unwrap_or_default();
                    drop(conn);
                    if let Ok(json) = serde_json::to_string_pretty(&export) {
                        std::fs::write(path, json).ok();
                    }
                    self.show_toast("Metadata exported successfully".to_string());
                }
            }
            None => {}
        }
    }

    /// Send the file to the OS recycle bin, drop the DB row, and update preview
    /// state (advance to next item, or close if it was the last one).
    fn do_delete_media(&mut self, media_id: i64, advance_preview: bool) {
        // Look up the path before we wipe the row.
        let item = {
            let conn = self.read_db.lock().unwrap();
            queries::get_media_by_id(&conn, media_id).ok().flatten()
        };
        let Some(item) = item else { return };

        // Recycle-bin the file. If the file is already gone, still drop the row.
        let trash_result = trash::delete(&item.full_path);
        if let Err(ref e) = trash_result {
            eprintln!("trash::delete({}) failed: {}", item.full_path, e);
        }
        // Best-effort: clean up the cached thumbnail too.
        if let Some(ref tp) = item.thumb_path {
            std::fs::remove_file(tp).ok();
        }

        let conn = self.db.lock().unwrap();
        queries::delete_media(&conn, media_id).ok();
        drop(conn);

        // Pick the replacement preview item (next, else previous) using the
        // global skeleton — the deleted item may not have been in the
        // current window.
        let next_preview = if advance_preview {
            self.next_or_prev_in_skeleton(media_id)
        } else {
            None
        };

        self.media_dirty = true;
        if advance_preview {
            self.preview_media = next_preview;
            self.refresh_preview_tags(true);
        } else if self.preview_media.as_ref().map(|m| m.id) == Some(media_id) {
            self.preview_media = None;
        }
    }

    /// Resolve the bulk action modal's filter configuration into the
    /// set of media IDs it currently matches. Used for the live count
    /// shown in the modal AND for the actual execution path. Pure
    /// (no mutations to AppState).
    pub fn bulk_action_matching_ids(&self, ba: &BulkActionState) -> Vec<i64> {
        // Source = CurrentSelection short-circuits everything — the
        // user's already curated a list; ignore the per-field filters.
        if matches!(ba.source, BulkActionSource::CurrentSelection) {
            return self.selected_media_ids.clone();
        }
        let mut query = queries::MediaQuery::default();
        match ba.source {
            BulkActionSource::WholeLibrary => {}
            BulkActionSource::SelectedFolder => {
                query.folder_id = self.selected_folder_id;
                query.path_prefix = self.selected_path.clone();
            }
            BulkActionSource::SelectedTag => {
                if !self.selected_tag_ids.is_empty() {
                    query.tag_ids = Some(self.selected_tag_ids.clone());
                }
            }
            BulkActionSource::CurrentSelection => unreachable!(),
        }
        let f = &ba.filters;
        let now = chrono::Utc::now().timestamp();
        query.date_from = f.date_from.and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| dt.and_utc().timestamp());
        query.date_to = f.date_to.and_then(|d| d.and_hms_opt(23, 59, 59))
            .map(|dt| dt.and_utc().timestamp());
        if let Some(days) = f.older_than_days {
            let cutoff = now - days * 86400;
            query.date_to = Some(query.date_to.map_or(cutoff, |d| d.min(cutoff)));
        }
        if let Some(days) = f.newer_than_days {
            let cutoff = now - days * 86400;
            query.date_from = Some(query.date_from.map_or(cutoff, |d| d.max(cutoff)));
        }
        if f.rating_min > 0 { query.rating_min = Some(f.rating_min); }
        if f.rating_max < 5 { query.rating_max = Some(f.rating_max); }
        if !f.name_contains.trim().is_empty() {
            query.search = Some(f.name_contains.clone());
            query.search_in_filenames = true;
            query.search_in_paths = false;
            query.search_in_tags = false;
        }
        if !f.extension.trim().is_empty() {
            let ext = f.extension.trim_start_matches('.').to_lowercase();
            query.extensions = Some(vec![ext]);
        }
        if let Some(mb) = f.size_min_mb { query.size_min = Some((mb * 1024.0 * 1024.0) as i64); }
        if let Some(mb) = f.size_max_mb { query.size_max = Some((mb * 1024.0 * 1024.0) as i64); }
        let conn = match self.read_db.lock() { Ok(c) => c, Err(_) => return Vec::new() };
        queries::get_media_skeletons(&conn, &query)
            .unwrap_or_default()
            .into_iter().map(|s| s.id).collect()
    }

    /// Execute the user-confirmed bulk action against the IDs its
    /// filters currently match. Spawns native folder pickers for
    /// Copy/Move so this MUST run on the main thread.
    fn do_execute_bulk_action(&mut self, ba: BulkActionState) {
        let ids = self.bulk_action_matching_ids(&ba);
        if ids.is_empty() { return; }
        match ba.action {
            BulkActionKind::Delete => self.do_bulk_delete_media(&ids),
            BulkActionKind::CopyTo => self.do_bulk_copy_media(&ids),
            BulkActionKind::MoveTo => self.do_bulk_move_media(&ids),
        }
    }

    /// Copy every file in `media_ids` to a folder the user picks. The
    /// existing per-file copy path is reused so destination collisions
    /// are surfaced via the `rfd` dialog (we don't pre-check; first
    /// failing copy is logged and we move on).
    fn do_bulk_copy_media(&mut self, media_ids: &[i64]) {
        let Some(dest_dir) = rfd::FileDialog::new().pick_folder() else { return };
        for &id in media_ids {
            let Some(item) = self.media_by_id(id) else { continue };
            let dst = dest_dir.join(&item.filename);
            if let Err(e) = std::fs::copy(&item.full_path, &dst) {
                eprintln!("bulk copy {} → {:?} failed: {}", item.full_path, dst, e);
            }
        }
        // Destination might be inside a registered folder — let the
        // watcher pick that up. No DB writes here.
    }

    /// Move every file in `media_ids` to a folder the user picks.
    /// Updates DB rows that land inside a registered library folder;
    /// drops rows whose new location is outside the library.
    fn do_bulk_move_media(&mut self, media_ids: &[i64]) {
        let Some(dest_dir) = rfd::FileDialog::new().pick_folder() else { return };
        let dest_dir_str = dest_dir.to_string_lossy().to_string();
        let new_folder_id = self.folders.iter()
            .find(|f| dest_dir_str.starts_with(&f.path))
            .map(|f| f.id);
        for &id in media_ids {
            let Some(item) = self.media_by_id(id) else { continue };
            let dst = dest_dir.join(&item.filename);
            if dst.exists() { continue; } // don't clobber
            let move_ok = match std::fs::rename(&item.full_path, &dst) {
                Ok(_) => true,
                Err(_) => {
                    // Cross-volume rename fails on Windows — fall back to copy+delete.
                    if std::fs::copy(&item.full_path, &dst).is_ok() {
                        std::fs::remove_file(&item.full_path).ok();
                        true
                    } else { false }
                }
            };
            if !move_ok { continue; }
            let dst_str = dst.to_string_lossy().to_string();
            let conn = self.db.lock().unwrap();
            if new_folder_id.is_some() {
                queries::update_media_path(&conn, id, &item.filename, &dst_str, new_folder_id).ok();
            } else {
                queries::delete_media(&conn, id).ok();
            }
        }
        self.media_dirty = true;
    }

    /// Bulk-delete: send every file in `media_ids` to the OS recycle bin
    /// and drop their DB rows in a single transaction. We resolve paths
    /// first (so the file deletions don't race with the DB clears),
    /// then issue one bulk-delete query at the end. Best-effort — files
    /// that fail to trash are still removed from the DB so the gallery
    /// doesn't keep showing dangling rows.
    fn do_bulk_delete_media(&mut self, media_ids: &[i64]) {
        if media_ids.is_empty() { return; }
        // Resolve file paths + thumb paths from the read connection
        // before we mutate anything.
        let items: Vec<MediaItem> = {
            let conn = self.read_db.lock().unwrap();
            media_ids.iter()
                .filter_map(|&id| queries::get_media_by_id(&conn, id).ok().flatten())
                .collect()
        };

        for item in &items {
            if let Err(e) = trash::delete(&item.full_path) {
                eprintln!("trash::delete({}) failed: {}", item.full_path, e);
            }
            if let Some(ref tp) = item.thumb_path {
                std::fs::remove_file(tp).ok();
            }
        }

        // Drop DB rows in one transaction.
        let conn = self.db.lock().unwrap();
        for &id in media_ids {
            queries::delete_media(&conn, id).ok();
        }
        drop(conn);

        // If the previewed item was part of the deletion, close preview.
        if let Some(pm) = self.preview_media.as_ref() {
            if media_ids.contains(&pm.id) {
                self.preview_media = None;
            }
        }

        self.selected_media_ids.clear();
        self.selection_anchor = None;
        self.media_dirty = true;
    }

    /// Find the next item after `id` (or the previous if `id` is last)
    /// in the skeleton-ordered list, hydrating it from the DB.
    fn next_or_prev_in_skeleton(&self, id: i64) -> Option<MediaItem> {
        let pos = self.media_skel.iter().position(|s| s.id == id)?;
        let next_id = self.media_skel.get(pos + 1).map(|s| s.id);
        let prev_id = if pos > 0 { self.media_skel.get(pos - 1).map(|s| s.id) } else { None };
        let pick = next_id.or(prev_id)?;
        let conn = self.read_db.lock().ok()?;
        queries::get_media_by_id(&conn, pick).ok().flatten()
    }

    fn do_copy_media(&mut self, media_id: i64) {
        let item = {
            let conn = self.read_db.lock().unwrap();
            queries::get_media_by_id(&conn, media_id).ok().flatten()
        };
        let Some(item) = item else { return };
        let Some(dest) = rfd::FileDialog::new()
            .set_file_name(&item.filename)
            .save_file()
        else { return };
        if let Err(e) = std::fs::copy(&item.full_path, &dest) {
            eprintln!("copy {} → {:?} failed: {}", item.full_path, dest, e);
        } else {
            // If the destination falls inside a registered folder, the watcher
            // will pick it up — nothing else to do here.
        }
    }

    fn do_move_media(&mut self, media_id: i64, advance_preview: bool) {
        let item = {
            let conn = self.read_db.lock().unwrap();
            queries::get_media_by_id(&conn, media_id).ok().flatten()
        };
        let Some(item) = item else { return };
        let Some(dest) = rfd::FileDialog::new()
            .set_file_name(&item.filename)
            .save_file()
        else { return };
        let dest_str = dest.to_string_lossy().to_string();
        // std::fs::rename fails across volumes on Windows; fall back to copy+delete.
        let move_ok = match std::fs::rename(&item.full_path, &dest) {
            Ok(_) => true,
            Err(_) => {
                if std::fs::copy(&item.full_path, &dest).is_ok() {
                    std::fs::remove_file(&item.full_path).ok();
                    true
                } else { false }
            }
        };
        if !move_ok { return; }

        let new_filename = dest.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or(item.filename.clone());
        // Re-home into a registered folder if the destination is inside one;
        // otherwise drop the DB row (file no longer in the library).
        let new_folder_id = self.folders.iter()
            .find(|f| dest_str.starts_with(&f.path))
            .map(|f| f.id);

        let conn = self.db.lock().unwrap();
        if new_folder_id.is_some() {
            queries::update_media_path(&conn, media_id, &new_filename, &dest_str, new_folder_id).ok();
        } else {
            queries::delete_media(&conn, media_id).ok();
        }
        drop(conn);

        let next_preview = if advance_preview {
            self.next_or_prev_in_skeleton(media_id)
        } else {
            None
        };
        self.media_dirty = true;
        if advance_preview && new_folder_id.is_none() {
            self.preview_media = next_preview;
            self.refresh_preview_tags(true);
        }
    }

    /// Move the file into a known library folder (no dialog). The destination
    /// must be the path of, or a subdirectory under, one of `self.folders`.
    fn do_move_media_to(&mut self, media_id: i64, dest_dir: &str) {
        let item = {
            let conn = self.read_db.lock().unwrap();
            queries::get_media_by_id(&conn, media_id).ok().flatten()
        };
        let Some(item) = item else { return };
        let dest_path = Path::new(dest_dir).join(&item.filename);
        // Don't clobber: if a file with the same name already exists, bail.
        if dest_path.exists() { return; }

        let dest_str = dest_path.to_string_lossy().to_string();
        let move_ok = match std::fs::rename(&item.full_path, &dest_path) {
            Ok(_) => true,
            Err(_) => {
                if std::fs::copy(&item.full_path, &dest_path).is_ok() {
                    std::fs::remove_file(&item.full_path).ok();
                    true
                } else { false }
            }
        };
        if !move_ok { return; }

        let new_folder_id = self.folders.iter()
            .find(|f| dest_str.starts_with(&f.path))
            .map(|f| f.id);

        let conn = self.db.lock().unwrap();
        queries::update_media_path(&conn, media_id, &item.filename, &dest_str, new_folder_id).ok();
        drop(conn);
        self.media_dirty = true;
    }

    fn do_rename_media(&mut self, media_id: i64, new_filename: &str) {
        let new_filename = new_filename.trim();
        if new_filename.is_empty() { return; }
        let item = {
            let conn = self.read_db.lock().unwrap();
            queries::get_media_by_id(&conn, media_id).ok().flatten()
        };
        let Some(item) = item else { return };
        if new_filename == item.filename { return; }
        let parent = Path::new(&item.full_path).parent().map(|p| p.to_path_buf());
        let Some(parent) = parent else { return };
        let new_path = parent.join(new_filename);
        if new_path.exists() { return; }
        if std::fs::rename(&item.full_path, &new_path).is_err() { return; }
        let new_path_str = new_path.to_string_lossy().to_string();
        let conn = self.db.lock().unwrap();
        queries::update_media_path(&conn, media_id, new_filename, &new_path_str, None).ok();
        drop(conn);
        // Update preview_media in place so the filename label refreshes immediately.
        if let Some(pm) = self.preview_media.as_mut() {
            if pm.id == media_id {
                pm.filename = new_filename.to_string();
                pm.full_path = new_path_str;
            }
        }
        self.media_dirty = true;
    }

    fn do_create_folder(&mut self, parent: &str, name: &str) {
        let name = name.trim();
        if name.is_empty() || name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) { return; }
        let target = Path::new(parent).join(name);
        if target.exists() { return; }
        if std::fs::create_dir(&target).is_err() { return; }
        // Invalidate the subdir cache for the parent so the new folder appears.
        self.subdir_cache.remove(parent);
    }

    pub fn poll_scan(&mut self) {
        if let Some(rx) = &self.scan_rx {
            let mut latest_progress: Option<ScanProgress> = None;
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    Some(progress) => { latest_progress = Some(progress); }
                    None => {
                        // Scan complete — final reload
                        self.scan_progress = None;
                        self.scan_rx = None;
                        self.scan_cancel = None;
                        self.last_scan_reload = 0;
                        self.subdir_cache.clear();
                        self.media_dirty = true;
                        let new_tree = {
                            let conn = self.read_db.lock().unwrap();
                            queries::get_tag_tree(&conn).unwrap_or_default()
                        };
                        self.set_tag_tree(new_tree);
                        return;
                    }
                }
            }
            if let Some(progress) = latest_progress {
                // Detect phase change (counter resets between indexing/thumbnail phases)
                if progress.scanned < self.last_scan_reload {
                    self.last_scan_reload = 0;
                }
                // Reload every 200 items so newly-generated thumbnails appear
                // as the scan progresses. Reads use the separate read_db
                // connection so they don't block the scan's writes.
                let should_reload = progress.scanned >= self.last_scan_reload + 200
                    || progress.scanned == progress.total;
                if should_reload {
                    self.last_scan_reload = progress.scanned;
                    self.media_dirty = true;
                }
                self.scan_progress = Some(progress);
            }
        }
    }
}

/// Walk `tags` until the tag with `target_id` is found, then push every
/// descendant (depth-first) into `out`. Returns true once the target is found.
fn collect_descendants(tags: &[Tag], target_id: i64, out: &mut Vec<Tag>) -> bool {
    for tag in tags {
        if tag.id == target_id {
            for child in &tag.children { push_subtree(child, out); }
            return true;
        }
        if collect_descendants(&tag.children, target_id, out) { return true; }
    }
    false
}

fn push_subtree(tag: &Tag, out: &mut Vec<Tag>) {
    out.push(tag.clone());
    for child in &tag.children { push_subtree(child, out); }
}
