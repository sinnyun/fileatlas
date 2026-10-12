//! PhoTag folder indexing, thumbnail generation, and reconciliation helpers.

use std::sync::Arc;

use cordis::{Context, Plugin, Service};
use photag_plugin_database::DatabaseService;
use photag_plugin_thumbnails::ThumbnailService;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;

mod ffmpeg;
mod maintenance;
pub mod scan;

/// Cordis service exposing the migrated original scan workflows.
pub struct IndexingService {
    pub(crate) database: Arc<DatabaseService>,
    pub(crate) thumbnails: Arc<ThumbnailService>,
}

impl Service for IndexingService {
    const NAME: &'static str = "photag/indexing";
}

impl IndexingService {
    /// Construct the indexing API from the shared database and cache services.
    pub fn new(database: Arc<DatabaseService>, thumbnails: Arc<ThumbnailService>) -> Self {
        Self {
            database,
            thumbnails,
        }
    }

    /// Scan an already registered PhoTag folder and return its progress stream.
    pub fn scan_folder(
        self: &Arc<Self>,
        folder_id: i64,
        folder_path: String,
        cancel: Arc<AtomicBool>,
    ) -> mpsc::Receiver<Option<scan::ScanProgress>> {
        scan::scan_folder_async(Arc::clone(self), folder_id, folder_path, cancel)
    }

    /// Generate caches for indexed image/video rows that do not have thumbnails.
    pub fn generate_missing_thumbnails(
        self: &Arc<Self>,
        items: Vec<(String, String)>,
        cancel: Arc<AtomicBool>,
    ) -> mpsc::Receiver<Option<scan::ScanProgress>> {
        maintenance::generate_missing_thumbnails_async(Arc::clone(self), items, cancel)
    }

    /// List immediate child directories using the original PhoTag filtering.
    pub fn list_subdirectories(&self, path: &str) -> Vec<(String, String, bool)> {
        maintenance::list_subdirectories(path)
    }
}

/// Scanner startup configuration. Scans are started explicitly by feature consumers.
#[derive(Debug, Clone, Default)]
pub struct IndexingConfig;

/// Connects the scanner to the already running database and thumbnail plugins.
pub struct IndexingPlugin;

impl Plugin for IndexingPlugin {
    type Config = IndexingConfig;
    type Input = IndexingConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = IndexingPluginError;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        "photag.indexing".into()
    }

    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }

    async fn apply(&self, context: Context, _config: &Self::Input) -> Result<(), Self::ApplyError> {
        let database = context
            .try_service::<DatabaseService>()
            .map_err(|error| IndexingPluginError::Dependency(error.to_string()))?;
        let thumbnails = context
            .try_service::<ThumbnailService>()
            .map_err(|error| IndexingPluginError::Dependency(error.to_string()))?;
        let service = Arc::new(IndexingService::new(database, thumbnails));
        let _publication = context
            .provide(Arc::clone(&service))
            .map_err(|error| IndexingPluginError::Publish(error.to_string()))?;
        Ok(())
    }
}

/// Failure to connect or publish the indexing service.
#[derive(Debug, thiserror::Error)]
pub enum IndexingPluginError {
    #[error("required indexing dependency unavailable: {0}")]
    Dependency(String),
    #[error("indexing service publication failed: {0}")]
    Publish(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};
    use photag_plugin_database::db::queries;

    #[test]
    fn scan_indexes_images_and_pairs_video_posters() {
        let root = std::env::temp_dir().join(format!(
            "photag-indexing-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let library = root.join("library");
        let cache = root.join("cache");
        std::fs::create_dir_all(&library).unwrap();
        let image_path = library.join("photo.png");
        let poster_path = library.join("clip.png");
        ImageBuffer::from_pixel(32, 20, Rgb([42u8, 110, 205]))
            .save(&image_path)
            .unwrap();
        ImageBuffer::from_pixel(40, 30, Rgb([205u8, 110, 42]))
            .save(&poster_path)
            .unwrap();
        std::fs::write(
            library.join("clip.mp4"),
            b"fixture video; poster pairing skips ffmpeg",
        )
        .unwrap();
        std::fs::write(library.join(".hidden.png"), b"ignored hidden file").unwrap();

        let database = Arc::new(DatabaseService::open_at(root.join("photag.sqlite")).unwrap());
        let folder_path = library.to_string_lossy().into_owned();
        let folder = database
            .with_writer(|connection| queries::add_folder(connection, &folder_path))
            .unwrap();
        let thumbnails = Arc::new(ThumbnailService::open_at(cache).unwrap());
        let indexing = Arc::new(IndexingService::new(database, thumbnails));

        let progress =
            indexing.scan_folder(folder.id, folder_path, Arc::new(AtomicBool::new(false)));
        let mut finished = false;
        while let Ok(item) = progress.recv() {
            if item.is_none() {
                finished = true;
                break;
            }
        }
        assert!(finished, "scan should signal completion");

        let rows = indexing
            .database
            .with_reader(|connection| {
                let mut statement = connection.prepare(
                    "SELECT filename, media_type, thumb_path FROM media ORDER BY filename",
                )?;
                statement
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?,
                        ))
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()
            })
            .unwrap();
        assert_eq!(
            rows.len(),
            2,
            "hidden files and paired posters are not rows"
        );
        let video = rows.iter().find(|row| row.0 == "clip.mp4").unwrap();
        assert_eq!(video.1, "video");
        assert_eq!(
            video.2.as_deref(),
            Some(poster_path.to_string_lossy().as_ref())
        );
        let photo = rows.iter().find(|row| row.0 == "photo.png").unwrap();
        assert_eq!(photo.1, "image");
        assert!(
            photo
                .2
                .as_ref()
                .is_some_and(|path| std::path::Path::new(path).is_file())
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
