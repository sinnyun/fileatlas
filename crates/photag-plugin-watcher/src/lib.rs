//! Cordis-managed filesystem watcher preserving PhoTag's debounce and reconciliation behavior.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::thread::JoinHandle;
use std::time::Duration;

use cordis::{Context, Plugin, Service};
use photag_plugin_database::{DatabaseService, db::queries};
use photag_plugin_indexing::IndexingService;

mod watcher;

/// Counts of filesystem changes synchronized into the PhoTag database.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ReconcileSummary {
    pub added_or_updated: usize,
    pub removed: usize,
}

/// Shared watcher API. Poll from an application executor, not the GPUI render pass.
pub struct WatcherService {
    database: Arc<DatabaseService>,
    indexing: Arc<IndexingService>,
    watcher: Mutex<watcher::FolderWatcher>,
    stop: Arc<AtomicBool>,
    poller: Mutex<Option<JoinHandle<()>>>,
}

impl Service for WatcherService {
    const NAME: &'static str = "photag/watcher";
}

impl WatcherService {
    fn start_background_reconciliation(service: &Arc<Self>) -> std::io::Result<()> {
        let weak: Weak<Self> = Arc::downgrade(service);
        let stop = Arc::clone(&service.stop);
        let worker = std::thread::Builder::new()
            .name("photag-fs-reconcile".into())
            .spawn(move || {
                while !stop.load(Ordering::Acquire) {
                    if let Some(service) = weak.upgrade() {
                        if let Err(error) = service.poll_and_reconcile() {
                            eprintln!("PhoTag filesystem reconciliation failed: {error}");
                        }
                    } else {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            })?;
        *service
            .poller
            .lock()
            .map_err(|_| std::io::Error::other("watcher worker lock poisoned"))? = Some(worker);
        Ok(())
    }

    /// Schedule a recursive directory watch without blocking the caller.
    pub fn watch(&self, path: &str) {
        if let Ok(watcher) = self.watcher.lock() {
            watcher.watch(path);
        }
    }

    /// Schedule removal of a recursive directory watch without blocking.
    pub fn unwatch(&self, path: &str) {
        if let Ok(watcher) = self.watcher.lock() {
            watcher.unwatch(path);
        }
    }

    /// Drain debounced events and reconcile added/removed media in SQLite.
    /// Calls are bounded to 200 paths, matching the original per-frame budget.
    pub fn poll_and_reconcile(&self) -> Result<ReconcileSummary, WatcherError> {
        let (added, removed) = self
            .watcher
            .lock()
            .map_err(|_| WatcherError::Poisoned)?
            .poll_changes();
        if added.is_empty() && removed.is_empty() {
            return Ok(ReconcileSummary::default());
        }

        let removed_count = removed.len();
        let indexed = self.database.with_writer(|connection| {
            for path in &removed {
                connection.execute(
                    "DELETE FROM media WHERE full_path = ?1",
                    rusqlite::params![path],
                )?;
            }
            let folders = {
                let mut statement = connection.prepare("SELECT id, path FROM folders")?;
                statement
                    .query_map([], |row| {
                        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?
            };
            let mut indexed = Vec::new();
            for full_path in &added {
                let path = Path::new(full_path);
                let Some((folder_id, _)) = folders
                    .iter()
                    .filter(|(_, folder)| path.starts_with(folder))
                    .max_by_key(|(_, folder)| folder.len())
                else {
                    continue;
                };
                let Ok(metadata) = std::fs::metadata(path) else {
                    continue;
                };
                let filename = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let ext = path
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase();
                let media_type = classify_media_type(&ext);
                let file_size = metadata.len() as i64;
                let modified_at = metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|duration| duration.as_secs() as i64)
                    .unwrap_or(0);
                queries::watcher_upsert_media(
                    connection,
                    *folder_id,
                    &filename,
                    full_path,
                    media_type,
                    file_size,
                    modified_at,
                )?;
                indexed.push((full_path.clone(), media_type.to_owned()));
            }
            Ok(indexed)
        })?;

        if !indexed.is_empty() {
            let _thumbnail_progress = self
                .indexing
                .generate_missing_thumbnails(indexed.clone(), Arc::new(AtomicBool::new(false)));
        }
        Ok(ReconcileSummary {
            added_or_updated: indexed.len(),
            removed: removed_count,
        })
    }
}

impl Drop for WatcherService {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Ok(worker) = self.poller.get_mut() {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }
    }
}

fn classify_media_type(extension: &str) -> &'static str {
    match extension {
        "mp4" | "mov" | "avi" | "mkv" | "wmv" | "flv" | "webm" | "m4v" | "mpg" | "mpeg" | "3gp" => {
            "video"
        }
        _ => "image",
    }
}

#[derive(Debug, Clone, Default)]
pub struct WatcherConfig;

pub struct WatcherPlugin;

impl Plugin for WatcherPlugin {
    type Config = WatcherConfig;
    type Input = WatcherConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = WatcherPluginError;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        "photag.watcher".into()
    }

    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }

    async fn apply(&self, context: Context, _config: &Self::Input) -> Result<(), Self::ApplyError> {
        let database = context
            .try_service::<DatabaseService>()
            .map_err(|error| WatcherPluginError::Dependency(error.to_string()))?;
        let indexing = context
            .try_service::<IndexingService>()
            .map_err(|error| WatcherPluginError::Dependency(error.to_string()))?;
        let watcher = watcher::FolderWatcher::new().ok_or(WatcherPluginError::Unavailable)?;
        let folders = database
            .with_reader(|connection| {
                let mut statement = connection.prepare("SELECT path FROM folders")?;
                statement
                    .query_map([], |row| row.get::<_, String>(0))?
                    .collect::<rusqlite::Result<Vec<_>>>()
            })
            .map_err(|error| WatcherPluginError::Database(error.to_string()))?;
        for folder in folders {
            watcher.watch(&folder);
        }
        let service = Arc::new(WatcherService {
            database,
            indexing,
            watcher: Mutex::new(watcher),
            stop: Arc::new(AtomicBool::new(false)),
            poller: Mutex::new(None),
        });
        WatcherService::start_background_reconciliation(&service)
            .map_err(WatcherPluginError::Worker)?;
        let _publication = context
            .provide(service)
            .map_err(|error| WatcherPluginError::Publish(error.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WatcherPluginError {
    #[error("required watcher dependency unavailable: {0}")]
    Dependency(String),
    #[error("native filesystem notification watcher could not be initialized")]
    Unavailable,
    #[error("could not load registered folder watches: {0}")]
    Database(String),
    #[error("watcher service publication failed: {0}")]
    Publish(String),
    #[error("could not start filesystem reconciliation worker: {0}")]
    Worker(#[source] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum WatcherError {
    #[error("watcher service lock is poisoned")]
    Poisoned,
    #[error(transparent)]
    Database(#[from] photag_plugin_database::DatabaseAccessError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use photag_plugin_database::db::queries;
    use std::time::Duration;

    #[test]
    fn media_classification_matches_watched_extensions() {
        assert_eq!(classify_media_type("mp4"), "video");
        assert_eq!(classify_media_type("png"), "image");
    }

    #[test]
    fn native_file_create_and_remove_reconcile_database_rows() {
        let root = std::env::temp_dir().join(format!(
            "photag-watcher-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let database = Arc::new(DatabaseService::open_at(root.join("photag.sqlite")).unwrap());
        let folder_path = root.to_string_lossy().into_owned();
        database
            .with_writer(|connection| queries::add_folder(connection, &folder_path))
            .unwrap();
        let watcher = watcher::FolderWatcher::new().expect("native watcher");
        watcher.watch(&folder_path);
        std::thread::sleep(Duration::from_millis(150));
        let thumbnails = Arc::new(
            photag_plugin_thumbnails::ThumbnailService::open_at(root.join("thumbs")).unwrap(),
        );
        let indexing = Arc::new(IndexingService::new(Arc::clone(&database), thumbnails));
        let service = WatcherService {
            database: Arc::clone(&database),
            indexing,
            watcher: Mutex::new(watcher),
            stop: Arc::new(AtomicBool::new(true)),
            poller: Mutex::new(None),
        };

        let media = root.join("arriving.png");
        std::fs::write(&media, b"watcher fixture").unwrap();
        wait_until(|| {
            let _ = service.poll_and_reconcile();
            database
                .with_reader(|connection| {
                    connection.query_row(
                        "SELECT COUNT(*) FROM media WHERE full_path = ?1",
                        rusqlite::params![media.to_string_lossy()],
                        |row| row.get::<_, i64>(0),
                    )
                })
                .unwrap_or_default()
                == 1
        });

        std::fs::remove_file(&media).unwrap();
        wait_until(|| {
            let _ = service.poll_and_reconcile();
            database
                .with_reader(|connection| {
                    connection.query_row(
                        "SELECT COUNT(*) FROM media WHERE full_path = ?1",
                        rusqlite::params![media.to_string_lossy()],
                        |row| row.get::<_, i64>(0),
                    )
                })
                .unwrap_or_default()
                == 0
        });
        drop(service);
        drop(database);
        let _ = std::fs::remove_dir_all(root);
    }

    fn wait_until(mut condition: impl FnMut() -> bool) {
        for _ in 0..30 {
            if condition() {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        assert!(
            condition(),
            "filesystem event did not reconcile before timeout"
        );
    }
}
