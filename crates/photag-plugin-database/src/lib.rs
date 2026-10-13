//! Cordis-managed database plugin preserving PhoTag's SQLite schema and query
//! implementation. Read and write connections stay separate to retain the
//! original WAL concurrency model.

pub mod db;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::{Mutex, MutexGuard};

use cordis::{Context, Plugin, Service};
use rusqlite::Connection;

/// PhoTag database service available to scanner and feature plugins.
pub struct DatabaseService {
    writer: Mutex<Connection>,
    reader: Mutex<Connection>,
    path: PathBuf,
}

impl Service for DatabaseService {
    const NAME: &'static str = "photag/database";
}

impl DatabaseService {
    /// Open PhoTag's normal `%LOCALAPPDATA%/PhoTag/photag.db` database.
    pub fn open_default() -> Result<Self, DatabaseOpenError> {
        let writer = db::open_db()?;
        let reader = db::open_read_conn()?;
        Ok(Self {
            writer: Mutex::new(writer),
            reader: Mutex::new(reader),
            path: default_database_path(),
        })
    }

    /// Open or create a database at an explicit location, primarily for
    /// isolated tests and portable embedding.
    pub fn open_at(path: impl AsRef<Path>) -> Result<Self, DatabaseOpenError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let writer = db::open_db_at(&path)?;
        let reader = db::open_read_conn_at(&path)?;
        Ok(Self {
            writer: Mutex::new(writer),
            reader: Mutex::new(reader),
            path,
        })
    }

    /// Return the on-disk database path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Run a short read-only SQL operation on the independent UI/read
    /// connection. Callers should offload expensive work from the GUI thread.
    pub fn with_reader<T>(
        &self,
        operation: impl FnOnce(&Connection) -> rusqlite::Result<T>,
    ) -> Result<T, DatabaseAccessError> {
        let connection = lock(&self.reader)?;
        operation(&connection).map_err(DatabaseAccessError::Sql)
    }

    /// Run a short mutating SQL operation on the serialized writer connection.
    pub fn with_writer<T>(
        &self,
        operation: impl FnOnce(&Connection) -> rusqlite::Result<T>,
    ) -> Result<T, DatabaseAccessError> {
        let connection = lock(&self.writer)?;
        operation(&connection).map_err(DatabaseAccessError::Sql)
    }
}

fn lock(connection: &Mutex<Connection>) -> Result<MutexGuard<'_, Connection>, DatabaseAccessError> {
    connection.lock().map_err(|_| DatabaseAccessError::Poisoned)
}

fn default_database_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("PhoTag")
        .join("photag.db")
}

/// Failure while creating or opening a PhoTag database.
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
    /// SQLite rejected the operation.
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
}

/// Database plugin configuration. `None` selects PhoTag's original local path.
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
        "photag.database".into()
    }

    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }

    async fn apply(&self, context: Context, config: &Self::Input) -> Result<(), Self::ApplyError> {
        let path = config.path.clone();
        let service = tokio::task::spawn_blocking(move || match path {
            Some(path) => DatabaseService::open_at(path),
            None => DatabaseService::open_default(),
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
            "photag-plugin-db-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let database = DatabaseService::open_at(&path).unwrap();
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

        let folder = database
            .with_writer(|connection| db::queries::add_folder(connection, "D:/PhoTag-test"))
            .unwrap();
        assert_eq!(folder.path, "D:/PhoTag-test");

        drop(database);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn opening_an_existing_pre_migration_library_preserves_rows_and_tags() {
        let path = std::env::temp_dir().join(format!(
            "photag-plugin-legacy-{}-{}.sqlite",
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
                let item = db::queries::get_media_by_id(connection, 11)?.unwrap();
                assert_eq!(item.full_path, "D:/Archive/frame.jpg");
                assert_eq!(item.rating, 0);
                let tag_ids = db::queries::get_media_tags(connection, 11)?;
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
                    db::queries::get_media_count(connection, &db::queries::MediaQuery::default())?,
                    1
                );
                assert_eq!(db::queries::get_media_tags(connection, 11)?.len(), 1);
                Ok(())
            })
            .unwrap();
        drop(reopened);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn media_query_filters_and_sort_orders_match_the_source_contract() {
        use db::queries::{MediaQuery, SortOrder};

        let path = std::env::temp_dir().join(format!(
            "photag-plugin-query-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let database = DatabaseService::open_at(&path).unwrap();
        let folder_id = database
            .with_writer(|connection| {
                let folder = db::queries::add_folder(connection, r"D:\Archive")?;
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
                    Ok(db::queries::get_media_skeletons(connection, query)?
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

        query = MediaQuery {
            path_prefix: Some(r"D:\Archive".into()),
            ..MediaQuery::default()
        };
        assert_eq!(ids_for(&query), vec![13, 11, 12]);

        drop(database);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn database_service_lifetime_follows_its_cordis_plugin() {
        use cordis::PreparedPlugin;

        let path = std::env::temp_dir().join(format!(
            "photag-plugin-lifecycle-{}-{}.sqlite",
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
