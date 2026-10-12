//! PhoTag gallery data access with its original skeleton-plus-window memory model.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use cordis::{Context, Plugin, Service};
use photag_plugin_database::DatabaseService;
use photag_plugin_database::db::queries::{self, MediaItem, MediaQuery, MediaSkeleton, Tag};
use photag_ui_bridge::{ContributionKind, UiContribution, register_for_plugin};

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
}

impl Service for GalleryService {
    const NAME: &'static str = "photag/gallery";
}

impl GalleryService {
    pub fn new(database: Arc<DatabaseService>) -> Self {
        Self { database }
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
            Ok(GallerySnapshot {
                skeletons,
                media_count,
                window_offset: 0,
                media,
                media_tags,
                available_extensions,
                folder_counts,
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

    /// Resolve an item even when it is outside the active viewport window.
    pub fn media_by_id(&self, id: i64) -> Result<Option<MediaItem>, GalleryError> {
        Ok(self
            .database
            .with_reader(|connection| queries::get_media_by_id(connection, id))?)
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
        let _publication = context
            .provide(Arc::new(GalleryService::new(database)))
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
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn plugin_registers_gallery_ui_and_cleans_it_on_disposal() {
        use cordis::{Context, Plugin, PreparedPlugin};
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
        let _database = context
            .provide(Arc::new(
                DatabaseService::open_at(root.join("photag.sqlite")).unwrap(),
            ))
            .unwrap();
        let plugin = GalleryPlugin;
        let input = plugin.prepare(GalleryConfig).unwrap();
        let fiber = context
            .spawn(PreparedPlugin::from_input(plugin, input))
            .await
            .unwrap();
        fiber.ready().await.unwrap();
        assert_eq!(catalog.snapshot().unwrap().len(), 3);
        fiber.dispose().await.unwrap();
        assert!(catalog.snapshot().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(root);
    }
}
