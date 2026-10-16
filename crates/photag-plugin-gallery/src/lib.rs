//! PhoTag gallery data access with its original skeleton-plus-window memory model.

pub use photag_plugin_grouping::GroupingMode;
mod browser;
mod live;
#[cfg(test)]
mod live_tests;
mod viewport;
pub use browser::{BrowserSnapshot, GalleryDirectory};
mod ui_requests;
mod view;
pub use ui_requests::{GalleryDialogReads, GalleryUiRequest, GalleryUiSnapshot};
pub use view::bulk_actions::{BulkActionKind, BulkActionSource};

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

pub mod filter;
pub mod selection;

use cordis::event::observer_sync;
use cordis::{Context, Plugin, Service};
use photag_contracts::{
    DirectoryPathSelected, FolderSelectionChanged, MediaIndexChanged, TagFilterSelection,
    TagFilterSelectionChanged, TagSelectionChanged,
};
use photag_plugin_database::DatabaseService;
use photag_plugin_database::db::queries::{
    self, MediaItem, MediaQuery, MediaSkeleton, SortOrder, Tag,
};
use photag_plugin_drag_drop::DragDropService;
use photag_plugin_file_operations::FileOperationsService;
use photag_plugin_media_playback::MediaPlaybackService;
use photag_plugin_preview::PreviewService;
use photag_plugin_session::SessionService;
use photag_ui_bridge::{
    ContributionKind, UiContribution, register_for_plugin, register_views_for_plugin,
};

/// The full media query result remains compact; full rows are loaded around the viewport.
pub const MEDIA_WINDOW_SIZE: usize = 200;

/// Current data needed to render and navigate the gallery.
#[derive(Clone)]
pub struct GallerySnapshot {
    pub browser: BrowserSnapshot,
    /// Stored folder rows used to resolve folder context in the inspector.
    pub folders: Vec<photag_plugin_database::db::queries::Folder>,
    pub skeletons: Vec<MediaSkeleton>,
    pub group_labels: Vec<Option<String>>,
    pub media_count: i64,
    pub window_offset: usize,
    pub media: Vec<MediaItem>,
    pub media_tags: HashMap<i64, Vec<Tag>>,
    pub available_extensions: Vec<String>,
    pub folder_counts: HashMap<i64, i64>,
    /// Full tag tree for naming active include/exclude filters.
    pub tag_tree: Vec<Tag>,
    pub groups: Vec<(String, usize)>,
}

/// Full-media window returned by viewport-driven prefetch.
#[derive(Clone)]
pub struct GalleryWindow {
    pub offset: usize,
    pub media: Vec<MediaItem>,
    pub media_tags: HashMap<i64, Vec<Tag>>,
}

pub struct GalleryService {
    wake: photag_ui_bridge::refresh::Wake,
    live: live::LiveBrowser,
    viewport_request: AtomicU64,
    navigation_revision: AtomicU64,
    thumbnails_disabled: std::sync::atomic::AtomicBool,
    watcher: Option<Arc<photag_plugin_watcher::WatcherService>>,
    database: Arc<DatabaseService>,
    grouping: Option<Arc<photag_plugin_grouping::GroupingService>>,
    indexing: Option<Arc<photag_plugin_indexing::IndexingService>>,
    ui_request: Mutex<Option<GalleryUiRequest>>,
    ui_snapshot: Mutex<Option<GalleryUiSnapshot>>,
    card_states: Mutex<Vec<GalleryCardState>>,
    card_frames: Mutex<std::collections::VecDeque<GalleryCardState>>,
}

/// Last rendered card state, for same-process MCP animation diagnostics.
#[derive(Clone, Copy)]
pub struct GalleryCardState {
    pub media_id: i64,
    pub hover: f32,
    pub selection: f32,
    pub timestamp_ms: u64,
}

#[derive(Default)]
struct SelectedFolder(Mutex<Option<i64>>);

impl Service for SelectedFolder {
    const NAME: &'static str = "photag/gallery-selected-folder";
}

#[derive(Default)]
struct SelectedTag(Mutex<TagFilterSelection>);

impl Service for SelectedTag {
    const NAME: &'static str = "photag/gallery-selected-tag";
}

#[derive(Default)]
struct SelectedPath(Mutex<Option<String>>);

impl Service for SelectedPath {
    const NAME: &'static str = "photag/gallery-selected-path";
}

#[derive(Default)]
struct IndexRevision(AtomicU64);

impl Service for IndexRevision {
    const NAME: &'static str = "photag/gallery-index-revision";
}

impl Service for GalleryService {
    const NAME: &'static str = "photag/gallery";
}

impl GalleryService {
    pub fn new(database: Arc<DatabaseService>) -> Self {
        Self {
            wake: Default::default(),
            database,
            live: Default::default(),
            viewport_request: AtomicU64::new(0),
            navigation_revision: AtomicU64::new(0),
            thumbnails_disabled: Default::default(),
            watcher: None,
            grouping: None,
            indexing: None,
            ui_request: Mutex::new(None),
            ui_snapshot: Mutex::new(None),
            card_states: Mutex::new(Vec::new()),
            card_frames: Mutex::new(std::collections::VecDeque::new()),
        }
    }

    pub fn request_ui(&self, request: GalleryUiRequest) {
        if !matches!(request, GalleryUiRequest::Hover(_)) {
            tracing::info!(?request, "gallery request");
        }
        *self.ui_request.lock().expect("Gallery UI request lock") = Some(request);
        self.wake.trigger();
    }

    pub fn card_states(&self) -> Vec<GalleryCardState> {
        self.card_states
            .lock()
            .map(|states| states.clone())
            .unwrap_or_default()
    }

    pub fn browser_enabled(&self) -> bool {
        self.database.is_browser()
    }
    pub fn thumbnails_enabled(&self) -> bool {
        !self.thumbnails_disabled.load(Ordering::Acquire)
    }
    pub fn set_thumbnails_enabled(&self, value: bool) {
        self.thumbnails_disabled.store(!value, Ordering::Release);
        self.live.touch();
    }
    pub fn show_descendants(&self) -> bool {
        self.live.recursive()
    }
    pub fn set_show_descendants(&self, enabled: bool) {
        self.live.set_recursive(enabled);
    }
    pub fn cancel_directory(&self) {
        self.live.cancel();
    }
    pub fn invalidate_directory(&self) {
        if let Some(watcher) = &self.watcher {
            watcher.retry_browse_watch();
        }
        self.live.invalidate();
    }
    pub fn reconcile_directory_event(&self) {
        if self
            .watcher
            .as_ref()
            .is_some_and(|watcher| watcher.take_directory_dirty())
        {
            self.live.invalidate();
        } else {
            self.live.touch();
        }
    }
    pub fn poll_directory(&self) -> bool {
        self.live.poll()
    }
    pub fn directory_worker_active(&self) -> bool {
        self.live.worker_active()
    }
    pub fn directory_status(&self) -> (u64, u64, bool, usize, Option<String>) {
        self.live.status()
    }

    pub fn scan_progress(&self) -> Option<photag_plugin_indexing::scan::ScanProgress> {
        self.indexing.as_ref()?.current_progress()
    }

    /// Bounded record of changed values produced by actual GPUI frames.
    pub fn card_animation_frames(&self) -> Vec<GalleryCardState> {
        self.card_frames
            .lock()
            .map(|frames| frames.iter().copied().collect())
            .unwrap_or_default()
    }

    fn publish_card_states(
        &self,
        states: Vec<GalleryCardState>,
        loaded: &std::collections::HashSet<i64>,
    ) {
        if let Ok(mut previous) = self.card_states.lock() {
            previous.retain(|old| loaded.contains(&old.media_id));
            if let Ok(mut frames) = self.card_frames.lock() {
                for state in &states {
                    if !previous.iter().any(|old| {
                        old.media_id == state.media_id
                            && old.hover == state.hover
                            && old.selection == state.selection
                    }) {
                        frames.push_back(*state);
                    }
                }
                while frames.len() > 256 {
                    frames.pop_front();
                }
            }
            for state in states {
                if let Some(old) = previous
                    .iter_mut()
                    .find(|old| old.media_id == state.media_id)
                {
                    *old = state;
                } else {
                    previous.push(state);
                }
            }
        }
    }

    fn has_ui_request(&self) -> bool {
        self.ui_request
            .lock()
            .is_ok_and(|request| request.is_some())
    }

    fn take_ui_request(&self) -> Option<GalleryUiRequest> {
        self.ui_request
            .lock()
            .ok()
            .and_then(|mut request| request.take())
    }

    fn scope_cache_query(&self, query: &mut MediaQuery) -> Result<(), GalleryError> {
        if !self.database.is_browser() {
            return Ok(());
        }
        if let Some(id) = query.folder_id.take() {
            if let Some(folder) = self
                .database
                .with_reader(queries::list_folders)?
                .into_iter()
                .find(|f| f.id == id)
            {
                query.directory_tree = Some(
                    std::fs::canonicalize(&folder.path)
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or(folder.path),
                );
            } else {
                return Err(photag_plugin_database::DatabaseAccessError::Sql(
                    rusqlite::Error::QueryReturnedNoRows,
                )
                .into());
            }
        }
        if !query.annotated_only
            && query.tag_ids.as_ref().is_none_or(Vec::is_empty)
            && query.excluded_tag_ids.as_ref().is_none_or(Vec::is_empty)
        {
            query.local_only = true;
        }
        Ok(())
    }

    /// Load compact rows for the whole result, plus the first 200 rich media rows.
    /// This keeps gallery layout memory small for libraries with many thousands of items.
    #[tracing::instrument(skip_all, err)]
    pub fn load(&self, mut query: MediaQuery) -> Result<GallerySnapshot, GalleryError> {
        self.scope_cache_query(&mut query)?;
        self.load_snapshot(query, photag_plugin_grouping::GroupingMode::None, false, 0)
    }

    #[tracing::instrument(skip_all, err)]
    pub fn load_grouped(
        &self,
        query: MediaQuery,
        mode: photag_plugin_grouping::GroupingMode,
    ) -> Result<GallerySnapshot, GalleryError> {
        self.load_snapshot(query, mode, true, 0)
    }

    pub(crate) fn load_grouped_window(
        &self,
        query: MediaQuery,
        mode: GroupingMode,
        offset: usize,
    ) -> Result<GallerySnapshot, GalleryError> {
        self.load_snapshot(query, mode, true, offset)
    }

    fn load_snapshot(
        &self,
        mut query: MediaQuery,
        mode: GroupingMode,
        browse: bool,
        window_offset: usize,
    ) -> Result<GallerySnapshot, GalleryError> {
        let browser = if browse {
            self.browser_snapshot(&query, mode)?
        } else {
            BrowserSnapshot::default()
        };
        if browser.active && self.database.is_browser() {
            query.folder_id = None;
            query.local_only = browser.directory.is_some();
            if self.live.recursive() {
                query.directory_tree = browser.directory.clone();
                query.direct_directory = None;
            } else {
                query.direct_directory = browser.directory.clone();
            }
            query.path_prefix = None;
        } else if browser.active {
            query.direct_directory = browser.directory.clone();
            query.path_prefix = None;
        }
        Ok(self.database.with_reader(|connection| {
            let folders = queries::list_folders(connection)?;
            let mut browser = browser;
            for directory in &mut browser.directories {
                directory.media_count =
                    queries::count_media_under_path(connection, &directory.path)?;
            }
            let status = self.live.status();
            let waiting_for_first_batch = self.database.is_browser()
                && browser.active
                && browser.directory.is_some()
                && status.2
                && status.3 == 0;
            let mut skeletons =
                if browser.active && browser.directory.is_none() || waiting_for_first_batch {
                    Vec::new()
                } else {
                    queries::get_media_skeletons(connection, &query)?
                };
            let group_labels = if let Some(grouping) = &self.grouping {
                let mut fields = if matches!(
                    mode,
                    photag_plugin_grouping::GroupingMode::None
                        | photag_plugin_grouping::GroupingMode::Month
                ) {
                    Vec::new()
                } else {
                    queries::get_media_grouping_fields(connection, &query)?
                };
                if self.database.is_browser() {
                    for field in &mut fields {
                        field.root_path = browser.directory.clone().unwrap_or_else(|| {
                            std::path::Path::new(&field.full_path)
                                .parent()
                                .map(|p| p.to_string_lossy().into_owned())
                                .unwrap_or_default()
                        });
                    }
                }
                grouping.arrange_in_scope(
                    &mut skeletons,
                    &fields,
                    mode,
                    query.sort.unwrap_or(SortOrder::NameAsc),
                    browser
                        .directory
                        .as_deref()
                        .or(query.path_prefix.as_deref()),
                )
            } else {
                vec![None; skeletons.len()]
            };
            let media_count = skeletons.len() as i64;
            let window_offset = window_offset.min(skeletons.len().saturating_sub(1));
            let mut media = skeletons
                .iter()
                .skip(window_offset)
                .take(MEDIA_WINDOW_SIZE)
                .map(|s| queries::get_media_by_id(connection, s.id))
                .collect::<rusqlite::Result<Vec<_>>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            if self.database.is_browser() && !browser.active {
                for item in &mut media {
                    hydrate_local(item);
                    if item.missing {
                        restore_record_preview(connection, item);
                    }
                    if !self.thumbnails_enabled() {
                        item.thumb_path = None;
                        item.micro_thumb = None;
                    }
                }
            }
            if self.database.is_browser() && !self.thumbnails_enabled() {
                for item in &mut media {
                    item.thumb_path = None;
                    item.micro_thumb = None;
                }
            }
            let media_ids = media.iter().map(|item| item.id).collect::<Vec<_>>();
            let media_tags = queries::get_tags_for_media_batch(connection, &media_ids)?;
            let available_extensions = queries::get_available_extensions(connection)?;
            let folder_counts = queries::count_media_per_folder(connection)?;
            let tag_tree = queries::get_tag_tree(connection)?;
            Ok(GallerySnapshot {
                browser,
                folders,
                skeletons,
                group_labels: group_labels.clone(),
                media_count,
                window_offset,
                media,
                media_tags,
                available_extensions,
                folder_counts,
                tag_tree,
                groups: {
                    let mut groups = Vec::<(String, usize)>::new();
                    for label in group_labels.iter().flatten() {
                        if let Some(last) = groups.last_mut().filter(|g| g.0 == *label) {
                            last.1 += 1;
                        } else {
                            groups.push((label.clone(), 1));
                        }
                    }
                    groups
                },
            })
        })?)
    }

    /// Load the visible range plus its surrounding cache window. The window
    /// shifts only when the requested range leaves the current cached slice.
    pub fn ensure_window(
        &self,
        snapshot: &GallerySnapshot,
        requested: Range<usize>,
    ) -> Result<Option<GalleryWindow>, GalleryError> {
        let total = snapshot.skeletons.len();
        if total == 0 {
            return Ok(None);
        }
        let start = requested.start.min(total);
        let end = requested.end.min(total).max(start);
        let current_end = snapshot.window_offset + snapshot.media.len();
        if start >= snapshot.window_offset && end <= current_end {
            return Ok(None);
        }

        let window_size = MEDIA_WINDOW_SIZE.max(end - start).min(total);
        let midpoint = start + (end - start) / 2;
        let half = window_size / 2;
        let offset = midpoint
            .saturating_sub(half)
            .min(total.saturating_sub(window_size));
        Ok(self.database.with_reader(|connection| {
            let media = snapshot.skeletons[offset..(offset + window_size).min(total)]
                .iter()
                .map(|s| queries::get_media_by_id(connection, s.id))
                .collect::<rusqlite::Result<Vec<_>>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            let ids = media.iter().map(|item| item.id).collect::<Vec<_>>();
            let media_tags = queries::get_tags_for_media_batch(connection, &ids)?;
            Ok(Some(GalleryWindow {
                offset,
                media,
                media_tags,
            }))
        })?)
    }

    /// Load the next source-ordered page of full media rows and their tags.
    #[tracing::instrument(skip_all, err)]
    pub fn load_page(
        &self,
        mut query: MediaQuery,
        offset: usize,
    ) -> Result<GalleryWindow, GalleryError> {
        self.scope_cache_query(&mut query)?;
        query.offset = Some(offset as i64);
        query.limit = Some(MEDIA_WINDOW_SIZE as i64);
        Ok(self.database.with_reader(|connection| {
            let media = queries::get_media(connection, &query)?;
            let ids = media.iter().map(|item| item.id).collect::<Vec<_>>();
            let media_tags = queries::get_tags_for_media_batch(connection, &ids)?;
            Ok(GalleryWindow {
                offset,
                media,
                media_tags,
            })
        })?)
    }

    /// Resolve an item even when it is outside the active viewport window.
    pub fn media_by_id(&self, id: i64) -> Result<Option<MediaItem>, GalleryError> {
        let mut item = self
            .database
            .with_reader(|connection| queries::get_media_by_id(connection, id))?;
        if self.database.is_browser() {
            if let Some(item) = &mut item {
                hydrate_local(item);
                if item.missing {
                    self.database.with_reader(|conn| {
                        restore_record_preview(conn, item);
                        Ok(())
                    })?;
                }
            }
        }
        Ok(item)
    }

    /// Resolve filesystem paths in the exact order of a drag selection.
    pub fn media_paths_by_ids(&self, ids: &[i64]) -> Result<Vec<std::path::PathBuf>, GalleryError> {
        Ok(self.database.with_reader(|connection| {
            let items = ids
                .iter()
                .map(|id| {
                    queries::get_media_by_id(connection, *id)
                        .map(|media| media.map(|item| std::path::PathBuf::from(item.full_path)))
                })
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(items.into_iter().flatten().collect())
        })?)
    }

    /// Resolve surviving media identities and paths in caller order.
    pub fn media_path_pairs_by_ids(
        &self,
        ids: &[i64],
    ) -> Result<Vec<(std::path::PathBuf, i64)>, GalleryError> {
        Ok(self.database.with_reader(|connection| {
            ids.iter()
                .filter_map(|id| match queries::get_media_by_id(connection, *id) {
                    Ok(Some(media)) => Some(Ok((std::path::PathBuf::from(media.full_path), *id))),
                    Ok(None) => None,
                    Err(error) => Some(Err(error)),
                })
                .collect::<rusqlite::Result<Vec<_>>>()
        })?)
    }

    /// Ordered filmstrip identities plus existing thumbnail caches, resolved off the UI thread.
    pub fn preview_items_by_ids(
        &self,
        ids: &[i64],
    ) -> Result<Vec<photag_plugin_preview::FilmstripMedia>, GalleryError> {
        Ok(self.database.with_reader(|connection| {
            ids.iter()
                .filter_map(|id| match queries::get_media_by_id(connection, *id) {
                    Ok(Some(media)) => {
                        Some(Ok(photag_plugin_preview::FilmstripMedia::from(&media)))
                    }
                    Ok(None) => None,
                    Err(error) => Some(Err(error)),
                })
                .collect::<rusqlite::Result<Vec<_>>>()
        })?)
    }

    /// Read the informational open counter stored by PhoTag's original schema.
    pub fn open_count(&self, media_id: i64) -> Result<i64, GalleryError> {
        Ok(self.database.with_reader(|connection| {
            Ok(photag_plugin_database::db::get_open_count(
                connection, media_id,
            ))
        })?)
    }

    /// Resolve assigned tags even when the media row is outside the viewport window.
    pub fn tags_for_media(
        &self,
        media_ids: &[i64],
    ) -> Result<HashMap<i64, Vec<Tag>>, GalleryError> {
        Ok(self
            .database
            .with_reader(|connection| queries::get_tags_for_media_batch(connection, media_ids))?)
    }

    pub fn statistics(&self) -> Result<queries::LibraryStats, GalleryError> {
        Ok(self
            .database
            .with_reader(|conn| queries::get_statistics_scoped(conn, self.database.is_browser()))?)
    }

    pub fn data_revision(&self) -> u64 {
        self.database.write_revision()
    }

    /// Persist a selection rating using PhoTag's clamped bulk update query.
    pub fn apply_rating(&self, media_ids: &[i64], rating: u8) -> Result<(), GalleryError> {
        Ok(self.database.with_writer(|connection| {
            queries::set_rating_bulk(connection, media_ids, i32::from(rating))
        })?)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GalleryError {
    #[error(transparent)]
    Database(#[from] photag_plugin_database::DatabaseAccessError),
}

mod plugin;
pub use plugin::{GalleryConfig, GalleryPlugin, GalleryPluginError};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod grouping_tests;

#[cfg(test)]
mod browser_tests;

fn hydrate_local(item: &mut MediaItem) {
    match std::fs::metadata(&item.full_path) {
        Ok(metadata) => {
            item.file_size = metadata.len() as i64;
            if let Ok(modified) = metadata.modified().and_then(|v| {
                v.duration_since(std::time::UNIX_EPOCH)
                    .map_err(std::io::Error::other)
            }) {
                item.modified_at = modified.as_secs() as i64;
            }
            item.missing = false;
        }
        Err(error) => item.missing = error.kind() == std::io::ErrorKind::NotFound,
    }
}

fn restore_record_preview(conn: &rusqlite::Connection, item: &mut MediaItem) {
    if let Ok(path) = conn.query_row(
        "SELECT thumb_path FROM tag_previews WHERE media_id=?1",
        [item.id],
        |r| r.get::<_, String>(0),
    ) {
        item.thumb_path = Some(path);
    }
}
