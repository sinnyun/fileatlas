//! Cordis-managed database plugin preserving FileAtlas's SQLite schema and query
//! implementation. Read and write connections stay separate to retain the
//! original WAL concurrency model.

mod browser_store;
#[cfg(test)]
mod browser_tests;
mod clipboard_api;
mod database_api;
mod db;
mod launcher_api;
#[cfg(test)]
mod launcher_tests;
mod search_cache;
mod version_metadata;
pub use clipboard_api::{ClipboardCategoryRecord, ClipboardHistoryRecord};
pub(crate) use db::get_open_count;
pub use db::queries::{
    Folder, LibraryStats, MediaExport, MediaGroupingFields, MediaItem, MediaQuery, MediaSkeleton,
    SortOrder, Tag, TagDropPos,
};
pub use launcher_api::{LauncherCategoryRecord, LauncherDiscoveredProgram, LauncherEntryRecord};
pub use version_metadata::{
    PendingVersionMerge, VersionBranch, VersionBranchPoint, VersionGraph, VersionRecord,
};

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::{Mutex, MutexGuard};

use cordis::{Context, Plugin, Service};
use db::queries;
use rusqlite::Connection;

/// FileAtlas database service available to scanner and feature plugins.
pub struct DatabaseService {
    write_revision: std::sync::atomic::AtomicU64,
    writer: Mutex<Connection>,
    reader: Mutex<Connection>,
    path: PathBuf,
    browser: bool,
}

impl Service for DatabaseService {
    const NAME: &'static str = "fileatlas/database";
}

impl DatabaseService {
    /// Open FileAtlas's normal `%LOCALAPPDATA%/FileAtlas/fileatlas.db` database.
    #[tracing::instrument(skip_all, err)]
    pub fn open_default() -> Result<Self, DatabaseOpenError> {
        let writer = db::open_db()?;
        let reader = db::open_read_conn()?;
        Ok(Self {
            write_revision: std::sync::atomic::AtomicU64::new(0),
            writer: Mutex::new(writer),
            reader: Mutex::new(reader),
            path: default_database_path(),
            browser: false,
        })
    }

    /// Open or create a database at an explicit location, primarily for
    /// isolated tests and portable embedding.
    #[tracing::instrument(skip_all, err)]
    pub fn open_at(path: impl AsRef<Path>) -> Result<Self, DatabaseOpenError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let writer = db::open_db_at(&path)?;
        let reader = db::open_read_conn_at(&path)?;
        Ok(Self {
            write_revision: std::sync::atomic::AtomicU64::new(0),
            writer: Mutex::new(writer),
            reader: Mutex::new(reader),
            path,
            browser: false,
        })
    }

    /// Return the on-disk database path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Memory-only invalidation clock. Conservative: every writer closure may
    /// have changed rows, including one returning an error after partial work.
    pub fn write_revision(&self) -> u64 {
        self.write_revision
            .load(std::sync::atomic::Ordering::Acquire)
    }

    /// Read the folder/tag navigation projection in one reader-lock scope.
    pub fn navigation_snapshot(&self) -> Result<NavigationDatabaseSnapshot, DatabaseAccessError> {
        self.with_reader(|connection| {
            let folders = queries::list_folders(connection)?;
            let folder_counts = queries::count_media_per_folder(connection)?;
            let tags = queries::get_tag_tree(connection)?;
            let favorite_meta = if self.browser {
                connection
                    .prepare("SELECT id,name,position FROM favorite_meta")?
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, i64>(2)?,
                        ))
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?
            } else {
                Vec::new()
            };
            Ok(NavigationDatabaseSnapshot {
                folders,
                folder_counts,
                tags,
                favorite_meta,
            })
        })
    }

    /// Clear a folder's indexed media before a source-compatible rescan.
    pub fn clear_folder_media(&self, folder_id: i64) -> Result<(), DatabaseAccessError> {
        self.with_writer(|connection| {
            connection.execute("DELETE FROM media WHERE folder_id = ?1", [folder_id])?;
            Ok(())
        })
    }

    /// Remove a registered folder; browser rows are reassigned to the virtual
    /// folder first, matching the browser-mode navigation behavior.
    pub fn remove_library_folder(&self, folder_id: i64) -> Result<(), DatabaseAccessError> {
        self.with_writer(|connection| {
            if self.browser {
                connection.execute(
                    "UPDATE media SET folder_id=0 WHERE folder_id=?1",
                    [folder_id],
                )?;
            }
            queries::remove_folder(connection, folder_id)
        })
    }

    /// Delete one tag and optionally remove its ancestors from affected media
    /// in the same serialized write operation.
    pub fn delete_tag_with_cleanup(
        &self,
        tag_id: i64,
        clean_parents: bool,
    ) -> Result<Vec<i64>, DatabaseAccessError> {
        self.with_writer(|connection| {
            let affected = queries::get_media_ids_with_tag(connection, tag_id)?;
            if clean_parents && !affected.is_empty() {
                let ancestors = queries::get_ancestor_ids(connection, tag_id)?;
                if !ancestors.is_empty() {
                    queries::remove_tags(connection, &affected, &ancestors)?;
                }
            }
            queries::delete_tag(connection, tag_id)?;
            Ok(affected)
        })
    }

    /// Set the display flag on a tag and its descendants atomically.
    pub fn set_tag_display_subtree(
        &self,
        tag_id: i64,
        display: bool,
    ) -> Result<bool, DatabaseAccessError> {
        self.with_writer(|connection| {
            let tree = queries::get_tag_tree(connection)?;
            fn collect(tags: &[queries::Tag], id: i64, result: &mut Vec<i64>) -> bool {
                for tag in tags {
                    if tag.id == id {
                        result.push(tag.id);
                        fn descendants(tags: &[queries::Tag], result: &mut Vec<i64>) {
                            for tag in tags {
                                result.push(tag.id);
                                descendants(&tag.children, result);
                            }
                        }
                        descendants(&tag.children, result);
                        return true;
                    }
                    if collect(&tag.children, id, result) {
                        return true;
                    }
                }
                false
            }
            let mut affected = Vec::new();
            if !collect(&tree, tag_id, &mut affected) {
                return Ok(false);
            }
            for id in affected {
                queries::set_tag_display(connection, id, display)?;
            }
            Ok(true)
        })
    }

    /// Run a short read-only SQL operation on the independent UI/read
    /// connection. Callers should offload expensive work from the GUI thread.
    #[track_caller]
    pub fn with_reader<T>(
        &self,
        operation: impl FnOnce(&Connection) -> rusqlite::Result<T>,
    ) -> Result<T, DatabaseAccessError> {
        let span = tracing::info_span!("with_reader",caller=%std::panic::Location::caller());
        let _entered = span.enter();
        let connection = lock(if self.browser {
            &self.writer
        } else {
            &self.reader
        })?;
        operation(&connection)
            .map_err(DatabaseAccessError::Sql)
            .inspect_err(|error| tracing::error!(%error,"database operation failed"))
    }

    /// Run a short mutating SQL operation on the serialized writer connection.
    #[track_caller]
    pub fn with_writer<T>(
        &self,
        operation: impl FnOnce(&Connection) -> rusqlite::Result<T>,
    ) -> Result<T, DatabaseAccessError> {
        let span = tracing::info_span!("with_writer",caller=%std::panic::Location::caller());
        let _entered = span.enter();
        let connection = lock(&self.writer)?;
        let result = operation(&connection);
        if self.browser {
            browser_store::persist(&connection)?;
        }
        self.write_revision
            .fetch_add(1, std::sync::atomic::Ordering::Release);
        result
            .map_err(DatabaseAccessError::Sql)
            .inspect_err(|error| tracing::error!(%error,"database operation failed"))
    }
}

/// Read-only source data needed to render the navigation plugin's tree.
pub struct NavigationDatabaseSnapshot {
    pub folders: Vec<queries::Folder>,
    pub folder_counts: std::collections::HashMap<i64, i64>,
    pub tags: Vec<queries::Tag>,
    pub favorite_meta: Vec<(i64, String, i64)>,
}

/// One lock-consistent read projection for FileManager's skeleton and visible row window.
pub struct FileManagerDatabaseSnapshot {
    pub folders: Vec<queries::Folder>,
    pub directory_counts: std::collections::HashMap<String, i64>,
    pub skeletons: Vec<queries::MediaSkeleton>,
    pub group_labels: Vec<Option<String>>,
    pub media_count: i64,
    pub window_offset: usize,
    pub media: Vec<queries::MediaItem>,
    pub media_tags: std::collections::HashMap<i64, Vec<queries::Tag>>,
    pub available_extensions: Vec<String>,
    pub folder_counts: std::collections::HashMap<i64, i64>,
    pub tag_tree: Vec<queries::Tag>,
}

/// Filesystem metadata verified by Watcher before it enters the database service.
#[derive(Debug, Clone)]
pub struct WatchedFile {
    pub path: String,
    pub filename: String,
    pub media_type: String,
    pub file_size: i64,
    pub modified_at: i64,
    pub version: Option<String>,
}

/// Files from the browser's live directory session.
#[derive(Debug, Clone)]
pub struct BrowserFile {
    pub path: String,
    pub filename: String,
    pub media_type: String,
    pub file_size: i64,
    pub modified_at: i64,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct ScanFileRecord {
    pub filename: String,
    pub full_path: String,
    pub media_type: String,
    pub file_size: i64,
    pub modified_at: i64,
    pub poster_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ThumbnailRecord {
    pub full_path: String,
    pub thumb_path: String,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub micro_thumb: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct TaggedMediaSourceUpdate {
    pub media_id: i64,
    pub file_size: i64,
    pub modified_at: i64,
    pub preview_path: Option<String>,
    pub thumbnail: Option<ThumbnailRecord>,
}

fn lock(connection: &Mutex<Connection>) -> Result<MutexGuard<'_, Connection>, DatabaseAccessError> {
    connection.lock().map_err(|_| DatabaseAccessError::Poisoned)
}

fn default_database_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("FileAtlas")
        .join("fileatlas.db")
}

/// Failure while creating or opening a FileAtlas database.
#[derive(Debug, thiserror::Error)]
pub enum DatabaseOpenError {
    /// The database parent directory could not be created.
    #[error("database directory could not be created: {0}")]
    Io(#[from] std::io::Error),
    /// SQLite rejected the database file or schema operation.
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
}

/// Failure from a serialized database operation.
#[derive(Debug, thiserror::Error)]
pub enum DatabaseAccessError {
    /// Another thread panicked while holding a connection lock.
    #[error("database connection lock is poisoned")]
    Poisoned,
    /// A requested persisted record no longer exists.
    #[error("database record was not found")]
    NotFound,
    /// SQLite rejected the operation.
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
    /// A typed database payload could not be serialized or parsed.
    #[error("database payload serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Database plugin configuration. `None` selects the source-compatible local path.
#[derive(Debug, Clone, Default)]
pub struct DatabaseConfig {
    /// Optional database file path override.
    pub path: Option<PathBuf>,
}

/// Opens the database and publishes its service for the lifetime of this
/// Cordis fiber. Cordis withdraws the service during plugin disposal.
pub struct DatabasePlugin;

impl Plugin for DatabasePlugin {
    type Config = DatabaseConfig;
    type Input = DatabaseConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = DatabasePluginError;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        "fileatlas.database".into()
    }

    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }

    #[tracing::instrument(skip_all, err)]
    async fn apply(&self, context: Context, config: &Self::Input) -> Result<(), Self::ApplyError> {
        let path = config.path.clone();
        let service = tokio::task::spawn_blocking(move || match path {
            Some(path) => {
                DatabaseService::open_browser_at(path.with_file_name("fileatlas-tags.db"))
            }
            None => DatabaseService::open_browser_at(
                default_database_path().with_file_name("fileatlas-tags.db"),
            ),
        })
        .await
        .map_err(DatabasePluginError::Worker)??;
        let _publication = context
            .provide(Arc::new(service))
            .map_err(|error| DatabasePluginError::Publish(error.to_string()))?;
        Ok(())
    }
}

/// Failure while opening or publishing the database service.
#[derive(Debug, thiserror::Error)]
pub enum DatabasePluginError {
    /// Background database initialization task failed.
    #[error("database initialization worker failed: {0}")]
    Worker(#[from] tokio::task::JoinError),
    /// SQLite schema or file initialization failed.
    #[error(transparent)]
    Open(#[from] DatabaseOpenError),
    /// Cordis rejected the service publication.
    #[error("database service publication failed: {0}")]
    Publish(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_schema_and_query_module_open_on_a_fresh_database() {
        let path = std::env::temp_dir().join(format!(
            "fileatlas-plugin-db-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let database = DatabaseService::open_at(&path).unwrap();
        assert_eq!(database.write_revision(), 0);
        let columns = database
            .with_reader(|connection| {
                let mut statement = connection.prepare("PRAGMA table_info(media)")?;
                statement
                    .query_map([], |row| row.get::<_, String>(1))?
                    .collect::<rusqlite::Result<Vec<_>>>()
            })
            .unwrap();
        assert!(columns.iter().any(|column| column == "micro_thumb"));
        assert!(columns.iter().any(|column| column == "rating"));
        assert_eq!(
            database.write_revision(),
            0,
            "reads must not invalidate caches"
        );

        let folder = database
            .with_writer(|connection| queries::add_folder(connection, "D:/FileAtlas-test"))
            .unwrap();
        assert_eq!(folder.path, "D:/FileAtlas-test");
        assert_eq!(database.write_revision(), 1);
        let failed: Result<(), _> = database.with_writer(|connection| {
            queries::create_tag(connection, "partial-write", None)?;
            Err(rusqlite::Error::InvalidQuery)
        });
        assert!(failed.is_err());
        assert_eq!(
            database.write_revision(),
            2,
            "partial writes must invalidate too"
        );

        drop(database);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn opening_an_existing_pre_migration_library_preserves_rows_and_tags() {
        let path = std::env::temp_dir().join(format!(
            "fileatlas-plugin-legacy-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        {
            let legacy = Connection::open(&path).unwrap();
            legacy
                .execute_batch(
                    "
                    CREATE TABLE folders (id INTEGER PRIMARY KEY, path TEXT NOT NULL UNIQUE, last_scan INTEGER);
                    CREATE TABLE media (
                        id INTEGER PRIMARY KEY,
                        folder_id INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
                        filename TEXT NOT NULL,
                        full_path TEXT NOT NULL UNIQUE,
                        media_type TEXT NOT NULL,
                        file_size INTEGER,
                        modified_at INTEGER,
                        width INTEGER,
                        height INTEGER,
                        date_taken INTEGER,
                        thumb_path TEXT,
                        metadata_dirty INTEGER DEFAULT 0
                    );
                    CREATE TABLE tags (
                        id INTEGER PRIMARY KEY,
                        name TEXT NOT NULL,
                        parent_id INTEGER REFERENCES tags(id) ON DELETE CASCADE,
                        full_path TEXT NOT NULL UNIQUE
                    );
                    CREATE TABLE media_tags (
                        media_id INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
                        tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
                        PRIMARY KEY (media_id, tag_id)
                    );
                    INSERT INTO folders VALUES (7, 'D:/Archive', 1700000000);
                    INSERT INTO media VALUES (11, 7, 'frame.jpg', 'D:/Archive/frame.jpg', 'image', 2048, 1700000000, 1920, 1080, NULL, NULL, 0);
                    INSERT INTO tags VALUES (13, 'Travel', NULL, 'Travel');
                    INSERT INTO media_tags VALUES (11, 13);
                    ",
                )
                .unwrap();
        }

        let database = DatabaseService::open_at(&path).unwrap();
        database
            .with_reader(|connection| {
                let item = queries::get_media_by_id(connection, 11)?.unwrap();
                assert_eq!(item.full_path, "D:/Archive/frame.jpg");
                assert_eq!(item.rating, 0);
                let tag_ids = queries::get_media_tags(connection, 11)?;
                assert_eq!(tag_ids.len(), 1);
                assert_eq!(tag_ids[0].full_path, "Travel");
                assert_eq!(db::get_open_count(connection, 11), 0);
                let migrated_columns = {
                    let mut statement = connection.prepare("PRAGMA table_info(media)")?;
                    statement
                        .query_map([], |row| row.get::<_, String>(1))?
                        .collect::<rusqlite::Result<Vec<_>>>()?
                };
                for expected in ["micro_thumb", "open_count", "rating", "pick_flag"] {
                    assert!(migrated_columns.iter().any(|column| column == expected));
                }
                let (pick_flag, auto_applied): (i64, i64) = connection.query_row(
                    "SELECT m.pick_flag, mt.auto_applied FROM media m JOIN media_tags mt ON mt.media_id = m.id WHERE m.id = 11",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?;
                assert_eq!(pick_flag, 0);
                assert_eq!(auto_applied, 0);
                Ok(())
            })
            .unwrap();
        drop(database);

        // Opening an already upgraded library again must be idempotent.
        let reopened = DatabaseService::open_at(&path).unwrap();
        reopened
            .with_reader(|connection| {
                assert_eq!(
                    queries::get_media_count(connection, &queries::MediaQuery::default())?,
                    1
                );
                assert_eq!(queries::get_media_tags(connection, 11)?.len(), 1);
                Ok(())
            })
            .unwrap();
        drop(reopened);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn media_query_filters_and_sort_orders_match_the_source_contract() {
        use queries::{MediaQuery, SortOrder};

        let path = std::env::temp_dir().join(format!(
            "fileatlas-plugin-query-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let database = DatabaseService::open_at(&path).unwrap();
        let folder_id = database
            .with_writer(|connection| {
                let folder = queries::add_folder(connection, r"D:\Archive")?;
                connection.execute_batch(
                    "INSERT INTO tags (id, name, full_path) VALUES (13, 'Travel', 'Travel'), (14, 'Family', 'Family');
                     INSERT INTO media (id, folder_id, filename, full_path, media_type, file_size, modified_at, date_taken, rating) VALUES
                       (11, 1, 'trip.jpg', 'D:\\Archive\\trip.jpg', 'image', 1000, 100, 200, 3),
                       (12, 1, 'trip.mp4', 'D:\\Archive\\trip.mp4', 'video', 3000, 300, NULL, 5),
                       (13, 1, 'portrait.jpg', 'D:\\Archive\\nested\\portrait.jpg', 'image', 2000, 400, 400, 0);
                     INSERT INTO media_tags (media_id, tag_id) VALUES (11, 13), (12, 14), (13, 14);",
                )?;
                Ok(folder.id)
            })
            .unwrap();

        let ids_for = |query: &MediaQuery| {
            database
                .with_reader(|connection| {
                    Ok(queries::get_media_skeletons(connection, query)?
                        .into_iter()
                        .map(|row| row.id)
                        .collect::<Vec<_>>())
                })
                .unwrap()
        };
        let mut query = MediaQuery {
            folder_id: Some(folder_id),
            sort: Some(SortOrder::NewestFirst),
            ..MediaQuery::default()
        };
        assert_eq!(ids_for(&query), vec![13, 12, 11]);

        query.sort = Some(SortOrder::HighestRated);
        assert_eq!(ids_for(&query), vec![12, 11, 13]);

        query = MediaQuery {
            extensions: Some(vec!["jpg".into()]),
            date_from: Some(200),
            date_to: Some(400),
            rating_min: Some(3),
            rating_max: Some(5),
            sort: Some(SortOrder::NameAsc),
            ..MediaQuery::default()
        };
        assert_eq!(ids_for(&query), vec![11]);

        query = MediaQuery {
            tag_ids: Some(vec![14]),
            ..MediaQuery::default()
        };
        assert_eq!(ids_for(&query), vec![13, 12]);

        query = MediaQuery {
            excluded_tag_ids: Some(vec![14]),
            ..MediaQuery::default()
        };
        assert_eq!(ids_for(&query), vec![11]);

        query = MediaQuery {
            search: Some("TRIP".into()),
            search_in_filenames: true,
            ..MediaQuery::default()
        };
        assert_eq!(ids_for(&query), vec![11, 12]);

        query.external_search_paths = Some(vec![r"D:\Archive\trip.jpg".into()]);
        assert_eq!(ids_for(&query), vec![11]);
        query.external_search_paths = Some(Vec::new());
        assert!(ids_for(&query).is_empty());

        query = MediaQuery {
            path_prefix: Some(r"D:\Archive".into()),
            ..MediaQuery::default()
        };
        assert_eq!(ids_for(&query), vec![13, 11, 12]);

        drop(database);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn folder_and_tag_foreign_keys_cascade_through_related_rows() {
        let path = std::env::temp_dir().join(format!(
            "fileatlas-plugin-cascade-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let database = DatabaseService::open_at(&path).unwrap();
        let (media_id, parent_id, child_id, associated_id) = database
            .with_writer(|connection| {
                let folder = queries::add_folder(connection, "D:/Cascade")?;
                let media_id = queries::insert_media(
                    connection,
                    folder.id,
                    "frame.jpg",
                    "D:/Cascade/frame.jpg",
                    "image",
                    128,
                    1_700_000_000,
                    None,
                    None,
                    None,
                )?;
                let parent = queries::create_tag(connection, "Parent", None)?;
                let child = queries::create_tag(connection, "Child", Some(parent.id))?;
                let associated = queries::create_tag(connection, "Associated", None)?;
                queries::set_associated_tags(connection, child.id, &[associated.id])?;
                queries::assign_tags(connection, &[media_id], &[child.id])?;
                Ok((media_id, parent.id, child.id, associated.id))
            })
            .unwrap();

        database
            .with_writer(|connection| {
                connection.execute("DELETE FROM tags WHERE id = ?1", [parent_id])?;
                let remaining_tags: i64 = connection.query_row(
                    "SELECT COUNT(*) FROM tags WHERE id IN (?1, ?2, ?3)",
                    [parent_id, child_id, associated_id],
                    |row| row.get(0),
                )?;
                let remaining_links: i64 = connection.query_row(
                    "SELECT (SELECT COUNT(*) FROM media_tags WHERE media_id = ?1) + (SELECT COUNT(*) FROM tag_associations WHERE tag_id = ?2 OR associated_tag_id = ?2)",
                    [media_id, child_id],
                    |row| row.get(0),
                )?;
                assert_eq!(remaining_tags, 1, "unrelated associated tag remains");
                assert_eq!(
                    remaining_links, 1,
                    "the unrelated tag's media assignment remains until its folder is deleted"
                );

                connection.execute("DELETE FROM folders WHERE path = ?1", ["D:/Cascade"])?;
                let remaining_media: i64 = connection.query_row(
                    "SELECT COUNT(*) FROM media WHERE id = ?1",
                    [media_id],
                    |row| row.get(0),
                )?;
                assert_eq!(remaining_media, 0);
                Ok(())
            })
            .unwrap();

        drop(database);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn database_service_lifetime_follows_its_cordis_plugin() {
        use cordis::PreparedPlugin;

        let path = std::env::temp_dir().join(format!(
            "fileatlas-plugin-lifecycle-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let context = Context::new();
        let plugin = DatabasePlugin;
        let input = plugin
            .prepare(DatabaseConfig {
                path: Some(path.clone()),
            })
            .unwrap();
        let fiber = context
            .spawn(PreparedPlugin::from_input(plugin, input))
            .await
            .unwrap();
        fiber.ready().await.unwrap();
        assert!(context.try_service::<DatabaseService>().is_ok());
        fiber.dispose().await.unwrap();
        assert!(context.try_service::<DatabaseService>().is_err());
        let _ = std::fs::remove_file(path);
    }
}
