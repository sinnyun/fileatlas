//! PhoTag folder indexing, thumbnail generation, and reconciliation helpers.

use std::sync::Arc;

use cordis::{Context, Plugin, Service};
use photag_plugin_database::DatabaseService;
use photag_plugin_thumbnails::ThumbnailService;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;

mod ffmpeg;
mod maintenance;
mod progress;
mod reconcile;
mod scan;
mod visible;
pub use scan::{ScanProgress, classify_media_type};

/// Cordis service exposing the migrated original scan workflows.
pub struct IndexingService {
    pub(crate) database: Arc<DatabaseService>,
    pub(crate) thumbnails: Arc<ThumbnailService>,
    progress: progress::ProgressRegistry,
}

impl Service for IndexingService {
    const NAME: &'static str = "photag/indexing";
}

impl IndexingService {
    /// Rebuild cached file metadata after an explicit version restore.
    pub fn refresh_restored_file(&self, media_id: i64) -> Result<(), String> {
        let media = self
            .database
            .get_media_by_id(media_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "The restored file is no longer indexed.".to_string())?;
        let metadata = std::fs::metadata(&media.full_path).map_err(|e| e.to_string())?;
        let modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|time| time.as_secs().min(i64::MAX as u64) as i64)
            .unwrap_or(media.modified_at);
        let media_type = classify_media_type(
            std::path::Path::new(&media.filename)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or(""),
        );
        let thumbnail = if media_type == "image" {
            self.thumbnails
                .generate_image(&media.full_path)
                .map(|(path, width, height, micro)| {
                    let micro = micro.unwrap_or_else(|| vec![0; 8 * 8 * 3]);
                    (path, width as i64, height as i64, micro)
                })
        } else {
            None
        };
        let size = metadata.len().min(i64::MAX as u64) as i64;
        match thumbnail {
            Some((path, width, height, micro)) => self
                .database
                .refresh_restored_file(
                    media_id,
                    media_type,
                    size,
                    modified,
                    Some((&path, width, height, &micro)),
                )
                .map_err(|e| e.to_string()),
            None => self
                .database
                .refresh_restored_file(media_id, media_type, size, modified, None)
                .map_err(|e| e.to_string()),
        }
    }

    /// Construct the indexing API from the shared database and cache services.
    pub fn new(database: Arc<DatabaseService>, thumbnails: Arc<ThumbnailService>) -> Self {
        Self {
            database,
            thumbnails,
            progress: progress::ProgressRegistry::default(),
        }
    }

    /// Scan an already registered PhoTag folder and return its progress stream.
    #[tracing::instrument(skip_all)]
    pub fn scan_folder(
        self: &Arc<Self>,
        folder_id: i64,
        folder_path: String,
        cancel: Arc<AtomicBool>,
    ) -> mpsc::Receiver<Option<ScanProgress>> {
        self.progress.track(scan::scan_folder_async(
            Arc::clone(self),
            folder_id,
            folder_path,
            cancel,
        ))
    }

    /// Generate caches for indexed image/video rows that do not have thumbnails.
    #[tracing::instrument(skip_all)]
    pub fn generate_missing_thumbnails(
        self: &Arc<Self>,
        items: Vec<(String, String)>,
        cancel: Arc<AtomicBool>,
    ) -> mpsc::Receiver<Option<ScanProgress>> {
        self.progress
            .track(maintenance::generate_missing_thumbnails_async(
                Arc::clone(self),
                items,
                cancel,
            ))
    }

    /// Latest active scan/backfill, including scans started outside Navigation.
    pub fn current_progress(&self) -> Option<ScanProgress> {
        self.progress.current()
    }

    /// Checked immediate-directory listing for browser views that must report access errors.
    #[tracing::instrument(skip_all, err)]
    pub fn try_list_subdirectories(
        &self,
        path: &str,
    ) -> std::io::Result<Vec<(String, String, bool)>> {
        maintenance::try_list_subdirectories(path)
    }

    /// List immediate child directories including dot-prefixed directories.
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

    #[tracing::instrument(skip_all, err)]
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
    use photag_plugin_database::queries;

    #[test]
    fn scan_indexes_all_files_and_preserves_video_posters() {
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
        let photo_modified_at = std::fs::metadata(&image_path)
            .unwrap()
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        ImageBuffer::from_pixel(40, 30, Rgb([205u8, 110, 42]))
            .save(&poster_path)
            .unwrap();
        std::fs::write(
            library.join("clip.mp4"),
            b"fixture video; poster pairing skips ffmpeg",
        )
        .unwrap();
        std::fs::write(library.join(".hidden.png"), b"broken hidden image").unwrap();

        for name in [
            "document.PDF",
            "archive.zip",
            "special.custom",
            "LICENSE",
            "song.WAV",
        ] {
            std::fs::write(library.join(name), b"file fixture").unwrap();
        }
        std::fs::create_dir_all(library.join(".hidden-dir")).unwrap();
        std::fs::write(library.join(".hidden-dir/nested"), b"nested file").unwrap();
        let database = Arc::new(DatabaseService::open_at(root.join("photag.sqlite")).unwrap());
        let folder_path = library.to_string_lossy().into_owned();
        let folder = database
            .with_writer(|connection| queries::add_folder(connection, &folder_path))
            .unwrap();
        let thumbnails = Arc::new(ThumbnailService::open_at(cache).unwrap());
        let photo_id = database
            .with_writer(|connection| {
                let id = queries::insert_media(
                    connection,
                    folder.id,
                    "photo.png",
                    &image_path.to_string_lossy(),
                    "image",
                    1,
                    7,
                    None,
                    None,
                    None,
                )?;
                connection.execute(
                    "UPDATE media SET rating = 4, date_taken = 123 WHERE id = ?1",
                    [id],
                )?;
                connection.execute(
                    "INSERT INTO tags (id, name, full_path) VALUES (1, 'Kept', 'Kept')",
                    [],
                )?;
                connection.execute(
                    "INSERT INTO media_tags (media_id, tag_id) VALUES (?1, 1)",
                    [id],
                )?;
                Ok(id)
            })
            .unwrap();
        let indexing = Arc::new(IndexingService::new(database, thumbnails));

        let progress = indexing.scan_folder(
            folder.id,
            folder_path.clone(),
            Arc::new(AtomicBool::new(false)),
        );
        let mut finished = false;
        while let Ok(item) = progress.recv() {
            assert!(item.as_ref().is_none_or(|event| event.error.is_none()));
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
            10,
            "every ordinary file, including hidden, extensionless and poster files, is indexed"
        );
        for name in [
            "document.PDF",
            "archive.zip",
            "special.custom",
            "LICENSE",
            "nested",
        ] {
            let row = rows.iter().find(|row| row.0 == name).unwrap();
            assert_eq!(row.1, "file");
            assert!(
                row.2.is_none(),
                "unsupported files must not get fabricated thumbnails"
            );
        }
        assert_eq!(
            rows.iter().find(|row| row.0 == "song.WAV").unwrap().1,
            "audio"
        );
        assert!(rows.iter().any(|row| row.0 == "clip.png"));
        assert_eq!(
            indexing.list_subdirectories(&library.to_string_lossy())[0].0,
            ".hidden-dir"
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
        // Reconciliation refreshes filesystem metadata without resetting user data.
        std::fs::write(library.join("new.unique"), b"new file").unwrap();
        for item in indexing.scan_folder(folder.id, folder_path, Arc::new(AtomicBool::new(false))) {
            assert!(item.as_ref().is_none_or(|event| event.error.is_none()));
            if item.is_none() {
                break;
            }
        }
        indexing
            .database
            .with_reader(|connection| {
                let retained: (i64, i32, i64, i64, i64, i64, i64) = connection.query_row(
                    "SELECT id, rating, date_taken, modified_at,
                    (SELECT COUNT(*) FROM media_tags WHERE media_id = media.id), width, height
                 FROM media WHERE full_path = ?1",
                    [image_path.to_string_lossy().as_ref()],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                            row.get(6)?,
                        ))
                    },
                )?;
                assert_eq!(retained, (photo_id, 4, 123, photo_modified_at, 1, 32, 20));
                assert_eq!(
                    queries::get_media_count(connection, &Default::default())?,
                    11
                );
                Ok(())
            })
            .unwrap();

        // Replacing content at the same path must invalidate its path-hash cache.
        ImageBuffer::from_pixel(64, 40, Rgb([220u8, 16, 30]))
            .save(&image_path)
            .unwrap();
        for item in indexing.scan_folder(
            folder.id,
            library.to_string_lossy().into_owned(),
            Arc::new(AtomicBool::new(false)),
        ) {
            assert!(item.as_ref().is_none_or(|event| event.error.is_none()));
            if item.is_none() {
                break;
            }
        }
        let updated_file_size = std::fs::metadata(&image_path).unwrap().len() as i64;
        indexing
            .database
            .with_reader(|connection| {
                let updated: (i64, i32, i64, i64, i64, i64) = connection.query_row(
                    "SELECT id, rating, date_taken, file_size, width, height
                     FROM media WHERE full_path = ?1",
                    [image_path.to_string_lossy().as_ref()],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                        ))
                    },
                )?;
                assert_eq!(updated, (photo_id, 4, 123, updated_file_size, 64, 40));
                let tags = connection.query_row(
                    "SELECT COUNT(*) FROM media_tags WHERE media_id = ?1",
                    [photo_id],
                    |row| row.get::<_, i64>(0),
                )?;
                assert_eq!(tags, 1, "replacing file contents preserves tag links");
                Ok(())
            })
            .unwrap();
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn successful_startup_scan_removes_offline_deletions_but_keeps_unavailable_roots() {
        let root = std::env::temp_dir().join(format!(
            "photag-reconcile-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let library = root.join("library");
        std::fs::create_dir_all(&library).unwrap();
        std::fs::write(library.join("present.custom"), b"present").unwrap();
        let database = Arc::new(DatabaseService::open_at(root.join("photag.sqlite")).unwrap());
        let folder_path = library.to_string_lossy().into_owned();
        let folder = database
            .with_writer(|connection| queries::add_folder(connection, &folder_path))
            .unwrap();
        let indexing = Arc::new(IndexingService::new(
            Arc::clone(&database),
            Arc::new(ThumbnailService::open_at(root.join("cache")).unwrap()),
        ));
        let stale_path = library.join("deleted.jpg");
        let stale_id = database
            .with_writer(|connection| {
                queries::insert_media(
                    connection,
                    folder.id,
                    "deleted.jpg",
                    &stale_path.to_string_lossy(),
                    "image",
                    0,
                    0,
                    None,
                    None,
                    None,
                )
            })
            .unwrap();

        let errors = indexing
            .scan_folder(
                folder.id,
                folder_path.clone(),
                Arc::new(AtomicBool::new(false)),
            )
            .into_iter()
            .flatten()
            .filter_map(|progress| progress.error)
            .collect::<Vec<_>>();
        assert!(errors.is_empty());
        let stale_count = database
            .with_reader(|connection| {
                connection.query_row(
                    "SELECT COUNT(*) FROM media WHERE id = ?1",
                    [stale_id],
                    |row| row.get::<_, i64>(0),
                )
            })
            .unwrap();
        assert_eq!(stale_count, 0, "offline file deletions are removed");

        let missing_root = root.join("removed-library");
        let missing_folder = database
            .with_writer(|connection| {
                let folder = queries::add_folder(connection, &missing_root.to_string_lossy())?;
                let id = queries::insert_media(
                    connection,
                    folder.id,
                    "cached.jpg",
                    &missing_root.join("cached.jpg").to_string_lossy(),
                    "image",
                    0,
                    0,
                    None,
                    None,
                    None,
                )?;
                Ok((folder.id, id))
            })
            .unwrap();
        let errors = indexing
            .scan_folder(
                missing_folder.0,
                missing_root.to_string_lossy().into_owned(),
                Arc::new(AtomicBool::new(false)),
            )
            .into_iter()
            .flatten()
            .filter_map(|progress| progress.error)
            .collect::<Vec<_>>();
        assert!(!errors.is_empty(), "an unavailable folder reports failure");
        let cached_count = database
            .with_reader(|connection| {
                connection.query_row(
                    "SELECT COUNT(*) FROM media WHERE id = ?1",
                    [missing_folder.1],
                    |row| row.get::<_, i64>(0),
                )
            })
            .unwrap();
        assert_eq!(cached_count, 1, "unavailable-folder rows are retained");
        drop(indexing);
        drop(database);
        let _ = std::fs::remove_dir_all(root);
    }
}
