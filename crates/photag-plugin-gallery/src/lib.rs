//! PhoTag gallery data access with its original skeleton-plus-window memory model.

mod ui_requests;
mod view;
pub use ui_requests::{GalleryUiRequest, GalleryUiSnapshot};
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
use photag_plugin_database::db::queries::{self, MediaItem, MediaQuery, MediaSkeleton, Tag};
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
    pub skeletons: Vec<MediaSkeleton>,
    pub media_count: i64,
    pub window_offset: usize,
    pub media: Vec<MediaItem>,
    pub media_tags: HashMap<i64, Vec<Tag>>,
    pub available_extensions: Vec<String>,
    pub folder_counts: HashMap<i64, i64>,
    /// Full tag tree for naming active include/exclude filters.
    pub tag_tree: Vec<Tag>,
    query: MediaQuery,
}

/// Full-media window returned by viewport-driven prefetch.
pub struct GalleryWindow {
    pub offset: usize,
    pub media: Vec<MediaItem>,
    pub media_tags: HashMap<i64, Vec<Tag>>,
}

pub struct GalleryService {
    database: Arc<DatabaseService>,
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
            indexing: None,
            ui_request: Mutex::new(None),
            ui_snapshot: Mutex::new(None),
            card_states: Mutex::new(Vec::new()),
            card_frames: Mutex::new(std::collections::VecDeque::new()),
        }
    }

    pub fn request_ui(&self, request: GalleryUiRequest) {
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
    pub fn load(&self, query: MediaQuery) -> Result<GallerySnapshot, GalleryError> {
        Ok(self.database.with_reader(|connection| {
            let skeletons = queries::get_media_skeletons(connection, &query)?;
            let media_count = skeletons.len() as i64;
            let mut window_query = query.clone();
            window_query.offset = Some(0);
            window_query.limit = Some(MEDIA_WINDOW_SIZE.min(skeletons.len()) as i64);
            let media = queries::get_media(connection, &window_query)?;
            let media_ids = media.iter().map(|item| item.id).collect::<Vec<_>>();
            let media_tags = queries::get_tags_for_media_batch(connection, &media_ids)?;
            let available_extensions = queries::get_available_extensions(connection)?;
            let folder_counts = queries::count_media_per_folder(connection)?;
            let tag_tree = queries::get_tag_tree(connection)?;
            Ok(GallerySnapshot {
                skeletons,
                media_count,
                window_offset: 0,
                media,
                media_tags,
                available_extensions,
                folder_counts,
                tag_tree,
                query,
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
        let mut query = snapshot.query.clone();
        query.offset = Some(offset as i64);
        query.limit = Some(window_size as i64);
        Ok(self.database.with_reader(|connection| {
            let media = queries::get_media(connection, &query)?;
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
mod tests {
    use super::*;
    use photag_plugin_database::db::queries;

    #[test]
    fn keeps_full_rows_windowed_and_loads_new_viewport_range() {
        let root = std::env::temp_dir().join(format!(
            "photag-gallery-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let database = Arc::new(DatabaseService::open_at(root.join("photag.sqlite")).unwrap());
        let folder_path = root.join("library").to_string_lossy().into_owned();
        std::fs::create_dir_all(&folder_path).unwrap();
        let folder = database
            .with_writer(|connection| queries::add_folder(connection, &folder_path))
            .unwrap();
        database
            .with_writer(|connection| {
                for index in 0..245 {
                    let filename = format!("image-{index:03}.png");
                    let path = format!("{folder_path}/{filename}");
                    queries::insert_media(
                        connection,
                        folder.id,
                        &filename,
                        &path,
                        "image",
                        index,
                        1_700_000_000 + index,
                        None,
                        None,
                        None,
                    )?;
                }
                Ok(())
            })
            .unwrap();

        let gallery = GalleryService::new(database);
        let snapshot = gallery.load(MediaQuery::default()).unwrap();
        assert_eq!(snapshot.media_count, 245);
        assert_eq!(snapshot.skeletons.len(), 245);
        assert_eq!(snapshot.media.len(), MEDIA_WINDOW_SIZE);
        assert_eq!(snapshot.window_offset, 0);
        assert_eq!(snapshot.media[0].filename, "image-000.png");
        assert!(gallery.ensure_window(&snapshot, 20..30).unwrap().is_none());

        let window = gallery.ensure_window(&snapshot, 220..235).unwrap().unwrap();
        assert_eq!(window.media.len(), MEDIA_WINDOW_SIZE);
        assert!(window.offset <= 220);
        assert!(window.offset + window.media.len() >= 235);
        assert_eq!(window.media[220 - window.offset].filename, "image-220.png");
        let page = gallery
            .load_page(MediaQuery::default(), MEDIA_WINDOW_SIZE)
            .unwrap();
        assert_eq!(page.offset, MEDIA_WINDOW_SIZE);
        assert_eq!(page.media.len(), 45);
        assert_eq!(page.media[0].filename, "image-200.png");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn applies_bulk_rating_with_source_range_clamping() {
        let root = std::env::temp_dir().join(format!(
            "photag-gallery-rating-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let database = Arc::new(DatabaseService::open_at(root.join("photag.sqlite")).unwrap());
        let media_id = database
            .with_writer(|connection| {
                let folder = queries::add_folder(connection, "/rating-test")?;
                queries::insert_media(
                    connection,
                    folder.id,
                    "sample.jpg",
                    "/rating-test/sample.jpg",
                    "image",
                    12,
                    1_700_000_000,
                    None,
                    None,
                    None,
                )
            })
            .unwrap();
        let gallery = GalleryService::new(Arc::clone(&database));
        gallery.apply_rating(&[media_id], 6).unwrap();
        assert_eq!(gallery.media_by_id(media_id).unwrap().unwrap().rating, 5);
        gallery.apply_rating(&[media_id], 0).unwrap();
        assert_eq!(gallery.media_by_id(media_id).unwrap().unwrap().rating, 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn plugin_registers_gallery_ui_and_cleans_it_on_disposal() {
        use cordis::{Context, Plugin, PreparedPlugin, Routing};
        use photag_contracts::{
            DirectoryPathSelected, FolderSelectionChanged, TagSelectionChanged,
        };
        use photag_ui_bridge::{UiBridgeService, UiCatalog};

        let root = std::env::temp_dir().join(format!(
            "photag-gallery-plugin-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let context = Context::new();
        let catalog = Arc::new(UiCatalog::default());
        let _ui = context
            .provide(Arc::new(UiBridgeService::new(Arc::clone(&catalog))))
            .unwrap();
        let database = Arc::new(DatabaseService::open_at(root.join("photag.sqlite")).unwrap());
        let _database = context.provide(Arc::clone(&database)).unwrap();
        let _file_operations = context
            .provide(Arc::new(FileOperationsService::new(
                Arc::clone(&database),
                context.clone(),
            )))
            .unwrap();
        let _drag_drop = context.provide(Arc::new(DragDropService)).unwrap();
        let _playback = context
            .provide(Arc::new(
                MediaPlaybackService::new(root.join("playback")).unwrap(),
            ))
            .unwrap();
        let _preview = context
            .provide(Arc::new(
                PreviewService::open_at(root.join("preview")).unwrap(),
            ))
            .unwrap();
        let _session = context
            .provide(Arc::new(SessionService::open_at(root.join("session.json"))))
            .unwrap();
        let plugin = GalleryPlugin;
        let input = plugin.prepare(GalleryConfig).unwrap();
        let fiber = context
            .spawn(PreparedPlugin::from_input(plugin, input))
            .await
            .unwrap();
        fiber.ready().await.unwrap();
        assert_eq!(catalog.snapshot().unwrap().len(), 3);
        context
            .emit::<FolderSelectionChanged>(Routing::Unscoped, Some(42))
            .await
            .unwrap();
        let selection = context.try_service::<SelectedFolder>().unwrap();
        assert_eq!(*selection.0.lock().unwrap(), Some(42));
        context
            .emit::<TagSelectionChanged>(Routing::Unscoped, Some(8))
            .await
            .unwrap();
        let selected_tag = context.try_service::<SelectedTag>().unwrap();
        assert_eq!(selected_tag.0.lock().unwrap().included, vec![8]);
        assert_eq!(
            *context
                .try_service::<SelectedFolder>()
                .unwrap()
                .0
                .lock()
                .unwrap(),
            None
        );
        context
            .emit::<DirectoryPathSelected>(Routing::Unscoped, Some("/library/trips".into()))
            .await
            .unwrap();
        assert_eq!(
            context
                .try_service::<SelectedPath>()
                .unwrap()
                .0
                .lock()
                .unwrap()
                .as_deref(),
            Some("/library/trips")
        );
        context
            .emit::<MediaIndexChanged>(Routing::Unscoped, ())
            .await
            .unwrap();
        assert_eq!(
            context
                .try_service::<IndexRevision>()
                .unwrap()
                .0
                .load(Ordering::Acquire),
            1
        );
        fiber.dispose().await.unwrap();
        assert!(catalog.snapshot().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(root);
    }
}
