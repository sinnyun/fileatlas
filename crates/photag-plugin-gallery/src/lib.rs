//! PhoTag gallery data access with its original skeleton-plus-window memory model.

pub use photag_plugin_grouping::GroupingMode;
mod browser;
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
pub struct GallerySnapshot {
    pub browser: BrowserSnapshot,
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
pub struct GalleryWindow {
    pub offset: usize,
    pub media: Vec<MediaItem>,
    pub media_tags: HashMap<i64, Vec<Tag>>,
}

pub struct GalleryService {
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
            database,
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
    }

    pub fn card_states(&self) -> Vec<GalleryCardState> {
        self.card_states
            .lock()
            .map(|states| states.clone())
            .unwrap_or_default()
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

    /// Load compact rows for the whole result, plus the first 200 rich media rows.
    /// This keeps gallery layout memory small for libraries with many thousands of items.
    #[tracing::instrument(skip_all, err)]
    pub fn load(&self, query: MediaQuery) -> Result<GallerySnapshot, GalleryError> {
        self.load_snapshot(query, photag_plugin_grouping::GroupingMode::None, false)
    }

    #[tracing::instrument(skip_all, err)]
    pub fn load_grouped(
        &self,
        query: MediaQuery,
        mode: photag_plugin_grouping::GroupingMode,
    ) -> Result<GallerySnapshot, GalleryError> {
        self.load_snapshot(query, mode, true)
    }

    fn load_snapshot(
        &self,
        mut query: MediaQuery,
        mode: GroupingMode,
        browse: bool,
    ) -> Result<GallerySnapshot, GalleryError> {
        let browser = if browse {
            self.browser_snapshot(&query, mode)?
        } else {
            BrowserSnapshot::default()
        };
        if browser.active {
            query.direct_directory = browser.directory.clone();
            query.path_prefix = None;
        }
        Ok(self.database.with_reader(|connection| {
            let mut skeletons = if browser.active && browser.directory.is_none() {
                Vec::new()
            } else {
                queries::get_media_skeletons(connection, &query)?
            };
            let group_labels = if let Some(grouping) = &self.grouping {
                let fields = if matches!(
                    mode,
                    photag_plugin_grouping::GroupingMode::None
                        | photag_plugin_grouping::GroupingMode::Month
                ) {
                    Vec::new()
                } else {
                    queries::get_media_grouping_fields(connection, &query)?
                };
                grouping.arrange_in_scope(
                    &mut skeletons,
                    &fields,
                    mode,
                    query.sort.unwrap_or(SortOrder::NameAsc),
                    query.path_prefix.as_deref(),
                )
            } else {
                vec![None; skeletons.len()]
            };
            let media_count = skeletons.len() as i64;
            let media = skeletons
                .iter()
                .take(MEDIA_WINDOW_SIZE)
                .map(|s| queries::get_media_by_id(connection, s.id))
                .collect::<rusqlite::Result<Vec<_>>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            let media_ids = media.iter().map(|item| item.id).collect::<Vec<_>>();
            let media_tags = queries::get_tags_for_media_batch(connection, &media_ids)?;
            let available_extensions = queries::get_available_extensions(connection)?;
            let folder_counts = queries::count_media_per_folder(connection)?;
            let tag_tree = queries::get_tag_tree(connection)?;
            Ok(GallerySnapshot {
                browser,
                skeletons,
                group_labels: group_labels.clone(),
                media_count,
                window_offset: 0,
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
        Ok(self
            .database
            .with_reader(|connection| queries::get_media_by_id(connection, id))?)
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
        Ok(self.database.with_reader(queries::get_statistics)?)
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

#[derive(Debug, Clone, Default)]
pub struct GalleryConfig;

pub struct GalleryPlugin;

impl Plugin for GalleryPlugin {
    type Config = GalleryConfig;
    type Input = GalleryConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = GalleryPluginError;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        "photag.gallery".into()
    }

    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }

    #[tracing::instrument(skip_all, err)]
    async fn apply(&self, context: Context, _config: &Self::Input) -> Result<(), Self::ApplyError> {
        let database = context
            .try_service::<DatabaseService>()
            .map_err(|error| GalleryPluginError::Dependency(error.to_string()))?;
        let file_operations = context
            .try_service::<FileOperationsService>()
            .map_err(|error| GalleryPluginError::Dependency(error.to_string()))?;
        let drag_drop = context
            .try_service::<DragDropService>()
            .map_err(|error| GalleryPluginError::Dependency(error.to_string()))?;
        let playback = context
            .try_service::<MediaPlaybackService>()
            .map_err(|error| GalleryPluginError::Dependency(error.to_string()))?;
        let preview = context
            .try_service::<PreviewService>()
            .map_err(|error| GalleryPluginError::Dependency(error.to_string()))?;
        preview.configure_playback(Arc::clone(&playback));
        let session = context
            .try_service::<SessionService>()
            .map_err(|error| GalleryPluginError::Dependency(error.to_string()))?;
        // History is optional so Gallery remains usable when the development
        // history plugin is not part of the active Cordis graph.
        let history = context
            .try_service::<photag_plugin_history::HistoryService>()
            .ok();
        let settings = context
            .try_service::<photag_plugin_settings::SettingsService>()
            .ok();
        let tagging = context
            .try_service::<photag_plugin_tagging::TaggingService>()
            .ok();
        let runtime = tokio::runtime::Handle::current();
        let mut service = GalleryService::new(database);
        service.grouping = Some(
            context
                .try_service::<photag_plugin_grouping::GroupingService>()
                .map_err(|e| GalleryPluginError::Dependency(e.to_string()))?,
        );
        service.indexing = context
            .try_service::<photag_plugin_indexing::IndexingService>()
            .ok();
        let service = Arc::new(service);
        let selected_folder = Arc::new(SelectedFolder::default());
        let selected_tag = Arc::new(SelectedTag::default());
        let selected_path = Arc::new(SelectedPath::default());
        let index_revision = Arc::new(IndexRevision::default());
        let _selection_publication = context
            .provide(Arc::clone(&selected_folder))
            .map_err(|error| GalleryPluginError::Publish(error.to_string()))?;
        let _tag_selection_publication = context
            .provide(Arc::clone(&selected_tag))
            .map_err(|error| GalleryPluginError::Publish(error.to_string()))?;
        let _path_selection_publication = context
            .provide(Arc::clone(&selected_path))
            .map_err(|error| GalleryPluginError::Publish(error.to_string()))?;
        let _index_revision_publication = context
            .provide(Arc::clone(&index_revision))
            .map_err(|error| GalleryPluginError::Publish(error.to_string()))?;
        let selection_state = Arc::clone(&selected_folder);
        let selection_tag_state = Arc::clone(&selected_tag);
        let selection_path_state = Arc::clone(&selected_path);
        let _selection_listener = context
            .on::<FolderSelectionChanged, _>(observer_sync(move |_, folder_id| {
                if let Ok(mut selected) = selection_state.0.lock() {
                    *selected = folder_id;
                }
                if let Ok(mut selected) = selection_tag_state.0.lock() {
                    *selected = TagFilterSelection::default();
                }
                if let Ok(mut selected) = selection_path_state.0.lock() {
                    *selected = None;
                }
                Ok::<_, std::convert::Infallible>(())
            }))
            .map_err(|error| GalleryPluginError::EventRegistration(error.to_string()))?;
        let tag_state = Arc::clone(&selected_tag);
        let tag_folder_state = Arc::clone(&selected_folder);
        let tag_path_state = Arc::clone(&selected_path);
        let _tag_selection_listener = context
            .on::<TagSelectionChanged, _>(observer_sync(move |_, tag_id: Option<i64>| {
                if let Ok(mut selected) = tag_state.0.lock() {
                    *selected = TagFilterSelection {
                        included: tag_id.into_iter().collect(),
                        excluded: Vec::new(),
                    };
                }
                if tag_id.is_some() {
                    if let Ok(mut selected) = tag_folder_state.0.lock() {
                        *selected = None;
                    }
                    if let Ok(mut selected) = tag_path_state.0.lock() {
                        *selected = None;
                    }
                }
                Ok::<_, std::convert::Infallible>(())
            }))
            .map_err(|error| GalleryPluginError::EventRegistration(error.to_string()))?;
        let tag_filter_state = Arc::clone(&selected_tag);
        let tag_filter_folder = Arc::clone(&selected_folder);
        let tag_filter_path = Arc::clone(&selected_path);
        let _tag_filter_selection_listener = context
            .on::<TagFilterSelectionChanged, _>(observer_sync(
                move |_, selection: TagFilterSelection| {
                    if let Ok(mut selected) = tag_filter_state.0.lock() {
                        *selected = selection.clone();
                    }
                    if !selection.included.is_empty() || !selection.excluded.is_empty() {
                        if let Ok(mut selected) = tag_filter_folder.0.lock() {
                            *selected = None;
                        }
                        if let Ok(mut selected) = tag_filter_path.0.lock() {
                            *selected = None;
                        }
                    }
                    Ok::<_, std::convert::Infallible>(())
                },
            ))
            .map_err(|error| GalleryPluginError::EventRegistration(error.to_string()))?;
        let path_state = Arc::clone(&selected_path);
        let path_folder_state = Arc::clone(&selected_folder);
        let path_tag_state = Arc::clone(&selected_tag);
        let _path_selection_listener = context
            .on::<DirectoryPathSelected, _>(observer_sync(move |_, path| {
                if let Ok(mut selected) = path_state.0.lock() {
                    *selected = path;
                }
                if let Ok(mut selected) = path_folder_state.0.lock() {
                    *selected = None;
                }
                if let Ok(mut selected) = path_tag_state.0.lock() {
                    *selected = TagFilterSelection::default();
                }
                Ok::<_, std::convert::Infallible>(())
            }))
            .map_err(|error| GalleryPluginError::EventRegistration(error.to_string()))?;
        let revision = Arc::clone(&index_revision);
        let _media_index_listener = context
            .on::<MediaIndexChanged, _>(observer_sync(move |_, ()| {
                revision.0.fetch_add(1, Ordering::Release);
                Ok::<_, std::convert::Infallible>(())
            }))
            .map_err(|error| GalleryPluginError::EventRegistration(error.to_string()))?;
        let _publication = context
            .provide(Arc::clone(&service))
            .map_err(|error| GalleryPluginError::Publish(error.to_string()))?;
        register_for_plugin(
            &context,
            "photag.gallery",
            [
                UiContribution::new(
                    "gallery/workspace",
                    ContributionKind::Layout {
                        label: "PhoTag workspace".into(),
                        order: 0,
                    },
                ),
                UiContribution::new(
                    "gallery/main-slot",
                    ContributionKind::Slot {
                        label: "gallery.content".into(),
                        order: 0,
                    },
                ),
                UiContribution::new(
                    "gallery/grid",
                    ContributionKind::Widget {
                        label: "Media Gallery".into(),
                        slot: "gallery.content".into(),
                        order: 0,
                    },
                ),
            ],
        )
        .map_err(|error| GalleryPluginError::UiRegistration(error.to_string()))?;
        let bridge = context
            .try_service::<photag_ui_bridge::UiBridgeService>()
            .map_err(|error| GalleryPluginError::Dependency(error.to_string()))?;
        register_views_for_plugin(
            &context,
            "photag.gallery",
            view::gallery(
                service,
                file_operations,
                drag_drop,
                preview,
                playback,
                runtime,
                context.clone(),
                selected_folder,
                selected_tag,
                selected_path,
                index_revision,
                session,
                history,
                settings,
                tagging,
                Arc::clone(&bridge.surfaces),
                Arc::clone(&bridge.views),
            ),
        )
        .map_err(|error| GalleryPluginError::UiRegistration(error.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GalleryPluginError {
    #[error("required gallery dependency unavailable: {0}")]
    Dependency(String),
    #[error("gallery service publication failed: {0}")]
    Publish(String),
    #[error("gallery UI contribution registration failed: {0}")]
    UiRegistration(String),
    #[error("gallery event registration failed: {0}")]
    EventRegistration(String),
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod grouping_tests;

#[cfg(test)]
mod browser_tests;
