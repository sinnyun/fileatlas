//! PhoTag gallery data access with its original skeleton-plus-window memory model.

pub use photag_plugin_grouping::GroupingMode;
mod browser;
mod live;
#[cfg(test)]
mod live_tests;
mod search;
mod viewport;
pub use browser::{BrowserSnapshot, GalleryDirectory};
mod ui_requests;
mod view;
pub use ui_requests::{
    GalleryDialogReads, GalleryUiRequest, GalleryUiSnapshot, GalleryVersionState,
};
pub use view::bulk_actions::{BulkActionKind, BulkActionSource};

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

mod filter;
pub use filter::{MediaTypeFilter, SearchScope, parse_date_input};
pub mod selection;

use cordis::event::observer_sync;
use cordis::{Context, Plugin, Service};
use photag_contracts::{
    DirectoryPathSelected, FolderSelectionChanged, MediaIndexChanged, MediaVersionHistoryChanged,
    TagFilterSelection, TagFilterSelectionChanged, TagSelectionChanged,
};
use photag_plugin_database::DatabaseService;
use photag_plugin_database::{self, MediaItem, MediaQuery, MediaSkeleton, SortOrder, Tag};
use photag_plugin_drag_drop::DragDropService;
use photag_plugin_file_operations::FileOperationsService;
use photag_plugin_quicklook::QuickLookService;
mod ai_suggestions;
use photag_plugin_session::SessionService;
use photag_ui_bridge::{
    ContributionKind, UiContribution, register_for_plugin, register_views_for_plugin,
};

/// The full media query result remains compact; full rows are loaded around the viewport.
pub const MEDIA_WINDOW_SIZE: usize = 200;

/// Current data needed to render and navigate the gallery.
#[derive(Clone, Default)]
pub struct GallerySnapshot {
    pub search_provider: &'static str,
    pub browser: BrowserSnapshot,
    /// Stored folder rows used to resolve folder context in the inspector.
    pub folders: Vec<photag_plugin_database::Folder>,
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
    search_provider: std::sync::atomic::AtomicU8,
    search_provider_detail: Mutex<Option<String>>,
    thumbnails_disabled: std::sync::atomic::AtomicBool,
    watcher: Option<Arc<photag_plugin_watcher::WatcherService>>,
    database: Arc<DatabaseService>,
    grouping: Option<Arc<photag_plugin_grouping::GroupingService>>,
    indexing: Option<Arc<photag_plugin_indexing::IndexingService>>,
    everything: Option<Arc<photag_plugin_everything_search::EverythingSearchService>>,
    ai_target: Arc<ai_suggestions::AiTarget>,
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
    /// Creates an independent browsing service over the same shared data providers.
    /// The live directory browser, viewport revision, transient UI requests and
    /// diagnostics are owned by the returned service.
    pub(crate) fn fork_for_column(&self) -> Self {
        Self {
            wake: Default::default(),
            live: Default::default(),
            viewport_request: AtomicU64::new(0),
            navigation_revision: AtomicU64::new(self.navigation_revision.load(Ordering::Acquire)),
            search_provider: std::sync::atomic::AtomicU8::new(0),
            search_provider_detail: Mutex::new(None),
            thumbnails_disabled: std::sync::atomic::AtomicBool::new(
                self.thumbnails_disabled.load(Ordering::Acquire),
            ),
            watcher: self.watcher.clone(),
            database: Arc::clone(&self.database),
            grouping: self.grouping.clone(),
            indexing: self.indexing.clone(),
            everything: self.everything.clone(),
            ai_target: Default::default(),
            ui_request: Mutex::new(None),
            ui_snapshot: Mutex::new(None),
            card_states: Mutex::new(Vec::new()),
            card_frames: Mutex::new(std::collections::VecDeque::new()),
        }
    }
    pub fn refresh_restored_file(&self, media_id: i64) -> Result<(), String> {
        self.indexing
            .as_ref()
            .ok_or_else(|| {
                "File indexing is unavailable to refresh the restored version.".to_string()
            })?
            .refresh_restored_file(media_id)
    }

    pub fn new(database: Arc<DatabaseService>) -> Self {
        Self {
            wake: Default::default(),
            database,
            live: Default::default(),
            viewport_request: AtomicU64::new(0),
            navigation_revision: AtomicU64::new(0),
            search_provider: std::sync::atomic::AtomicU8::new(0),
            search_provider_detail: Mutex::new(None),
            thumbnails_disabled: Default::default(),
            watcher: None,
            grouping: None,
            indexing: None,
            everything: None,
            ai_target: Default::default(),
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
    pub fn search_provider(&self) -> &'static str {
        match self.search_provider.load(Ordering::Acquire) {
            2 => "everything",
            1 => "sqlite-fallback",
            _ => "sqlite",
        }
    }

    pub fn search_provider_detail(&self) -> Option<String> {
        self.search_provider_detail
            .lock()
            .ok()
            .and_then(|detail| detail.clone())
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

    pub fn scan_progress(&self) -> Option<photag_plugin_indexing::ScanProgress> {
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
                .list_folders()?
                .into_iter()
                .find(|f| f.id == id)
            {
                query.directory_tree = Some(
                    std::fs::canonicalize(&folder.path)
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or(folder.path),
                );
            } else {
                return Err(photag_plugin_database::DatabaseAccessError::NotFound.into());
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

    /// Replace filename/path matching with Everything candidate paths when its
    /// local IPC service is available. Remaining filters stay in PhoTag's DB.
    fn prepare_everything_search(&self, query: &mut MediaQuery) -> Result<(), GalleryError> {
        self.scope_text_search(query)?;
        query.external_search_paths = None;
        self.search_provider.store(1, Ordering::Release);
        if let Ok(mut detail) = self.search_provider_detail.lock() {
            *detail = None;
        }
        if query
            .search
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
            || !(query.search_in_filenames || query.search_in_paths)
        {
            if let Ok(mut detail) = self.search_provider_detail.lock() {
                *detail = Some(
                    "Everything skipped: empty query or filename/path scopes are disabled.".into(),
                );
            }
            return Ok(());
        }
        let Some(everything) = &self.everything else {
            if let Ok(mut detail) = self.search_provider_detail.lock() {
                *detail = Some("Everything adapter service is unavailable.".into());
            }
            return Ok(());
        };
        let roots = match if query.search_entire_computer {
            Ok(Vec::new())
        } else {
            self.database.folder_paths()
        } {
            Ok(roots) => roots,
            Err(error) => {
                if let Ok(mut detail) = self.search_provider_detail.lock() {
                    *detail = Some(format!("Couldn't read search roots: {error}"));
                }
                return Ok(());
            }
        };
        let roots = query
            .path_prefix
            .as_ref()
            .or(query.direct_directory.as_ref())
            .or(query.directory_tree.as_ref())
            .cloned()
            .map(|root| vec![root])
            .or_else(|| {
                query.folder_id.and_then(|id| {
                    self.database
                        .list_folders()
                        .ok()
                        .and_then(|folders| folders.into_iter().find(|folder| folder.id == id))
                        .map(|folder| vec![folder.path])
                })
            })
            .unwrap_or(roots);
        match everything.search_paths(query.search.as_deref().unwrap_or_default(), &roots) {
            Ok(paths) => {
                let paths = self.cache_everything_paths(paths)?;
                if self.database.is_browser() {
                    query.local_only = false;
                }
                if let Ok(mut detail) = self.search_provider_detail.lock() {
                    *detail = Some(format!(
                        "Everything SDK 3 returned {} file candidates.",
                        paths.len()
                    ));
                }
                query.external_search_paths = Some(paths);
                self.search_provider.store(2, Ordering::Release);
            }
            Err(error) => {
                if let Ok(mut detail) = self.search_provider_detail.lock() {
                    *detail = Some(error.to_string());
                }
                tracing::debug!(%error, "Everything unavailable; using PhoTag SQL search")
            }
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
        self.prepare_everything_search(&mut query)?;
        let browser = if browse {
            self.browser_snapshot(&query, mode)?
        } else {
            BrowserSnapshot::default()
        };
        if browser.active && self.database.is_browser() {
            query.folder_id = None;
            query.local_only = browser.directory.is_some() && query.external_search_paths.is_none();
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
        let mut browser = browser;
        let status = self.live.status();
        browser.loading = self.database.is_browser() && browser.active && status.2;
        let waiting_for_first_batch = self.database.is_browser()
            && browser.active
            && browser.directory.is_some()
            && status.2
            && status.3 == 0;
        let directory_paths = browser
            .directories
            .iter()
            .map(|item| item.path.clone())
            .collect::<Vec<_>>();
        let grouping_enabled = self.grouping.is_some()
            && !matches!(
                mode,
                photag_plugin_grouping::GroupingMode::None
                    | photag_plugin_grouping::GroupingMode::Month
            );
        let data = self.database.gallery_snapshot(
            &query,
            &directory_paths,
            grouping_enabled,
            browser.directory.as_deref(),
            browser.active && browser.directory.is_none() || waiting_for_first_batch,
            window_offset,
            MEDIA_WINDOW_SIZE,
            |skeletons, fields| {
                if let Some(grouping) = &self.grouping {
                    grouping.arrange_in_scope(
                        skeletons,
                        fields,
                        mode,
                        query.sort.unwrap_or(SortOrder::NameAsc),
                        browser
                            .directory
                            .as_deref()
                            .or(query.path_prefix.as_deref()),
                    )
                } else {
                    vec![None; skeletons.len()]
                }
            },
        )?;
        for directory in &mut browser.directories {
            directory.media_count = data
                .directory_counts
                .get(&directory.path)
                .copied()
                .unwrap_or(0);
        }
        let mut media = data.media;
        if self.database.is_browser() && !browser.active {
            for item in &mut media {
                hydrate_local(item);
            }
            self.database.restore_record_previews(&mut media)?;
        }
        if self.database.is_browser() && !self.thumbnails_enabled() {
            for item in &mut media {
                item.thumb_path = None;
                item.micro_thumb = None;
            }
        }
        let group_labels = data.group_labels;
        Ok(GallerySnapshot {
            search_provider: if query.external_search_paths.is_some() {
                "everything"
            } else {
                "sqlite-fallback"
            },
            browser,
            folders: data.folders,
            skeletons: data.skeletons,
            group_labels: group_labels.clone(),
            media_count: data.media_count,
            window_offset: data.window_offset,
            media,
            media_tags: data.media_tags,
            available_extensions: data.available_extensions,
            folder_counts: data.folder_counts,
            tag_tree: data.tag_tree,
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
        let ids = snapshot.skeletons[offset..(offset + window_size).min(total)]
            .iter()
            .map(|s| s.id)
            .collect::<Vec<_>>();
        let (media, media_tags) = self.database.get_media_window(&ids)?;
        Ok(Some(GalleryWindow {
            offset,
            media,
            media_tags,
        }))
    }

    /// Load the next source-ordered page of full media rows and their tags.
    #[tracing::instrument(skip_all, err)]
    pub fn load_page(
        &self,
        mut query: MediaQuery,
        offset: usize,
    ) -> Result<GalleryWindow, GalleryError> {
        self.scope_cache_query(&mut query)?;
        self.prepare_everything_search(&mut query)?;
        query.offset = Some(offset as i64);
        query.limit = Some(MEDIA_WINDOW_SIZE as i64);
        let (media, media_tags) = self.database.get_media_page_with_tags(&query)?;
        Ok(GalleryWindow {
            offset,
            media,
            media_tags,
        })
    }

    /// Resolve an item even when it is outside the active viewport window.
    pub fn media_by_id(&self, id: i64) -> Result<Option<MediaItem>, GalleryError> {
        let mut item = self.database.get_media_by_id(id)?;
        if self.database.is_browser() {
            if let Some(item) = &mut item {
                hydrate_local(item);
                if item.missing {
                    self.database.restore_record_preview(item)?;
                }
            }
        }
        Ok(item)
    }

    /// Resolve filesystem paths in the exact order of a drag selection.
    pub fn media_paths_by_ids(&self, ids: &[i64]) -> Result<Vec<std::path::PathBuf>, GalleryError> {
        Ok(self.database.media_paths_by_ids(ids)?)
    }

    /// Resolve surviving media identities and paths in caller order.
    pub fn media_path_pairs_by_ids(
        &self,
        ids: &[i64],
    ) -> Result<Vec<(std::path::PathBuf, i64)>, GalleryError> {
        Ok(self.database.media_path_pairs_by_ids(ids)?)
    }

    /// Read the informational open counter stored by PhoTag's original schema.
    pub fn open_count(&self, media_id: i64) -> Result<i64, GalleryError> {
        Ok(self.database.open_count(media_id))
    }

    /// Resolve assigned tags even when the media row is outside the viewport window.
    pub fn tags_for_media(
        &self,
        media_ids: &[i64],
    ) -> Result<HashMap<i64, Vec<Tag>>, GalleryError> {
        Ok(self.database.get_tags_for_media_batch(media_ids)?)
    }

    pub fn statistics(&self) -> Result<photag_plugin_database::LibraryStats, GalleryError> {
        Ok(self
            .database
            .get_statistics_scoped(self.database.is_browser())?)
    }

    pub fn data_revision(&self) -> u64 {
        self.database.write_revision()
    }

    /// Persist a selection rating using PhoTag's clamped bulk update query.
    pub fn apply_rating(&self, media_ids: &[i64], rating: u8) -> Result<(), GalleryError> {
        Ok(self
            .database
            .set_rating_bulk(media_ids, i32::from(rating))?)
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
