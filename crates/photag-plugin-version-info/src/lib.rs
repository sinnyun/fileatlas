//! Cordis facade for durable Lore metadata, backed by the shared Database service.
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use cordis::{Context, Plugin, Service, service::ServicePublication};
use photag_plugin_database::DatabaseService;
use photag_plugin_database::{DatabaseAccessError, DatabaseOpenError};
pub use photag_plugin_database::{
    PendingVersionMerge, VersionBranch, VersionBranchPoint, VersionGraph, VersionRecord,
};

const SERVICE_NAME: &str = "photag/version-info";

#[derive(Clone)]
pub struct VersionInfoService(Arc<DatabaseService>);

impl Service for VersionInfoService {
    const NAME: &'static str = SERVICE_NAME;
}

impl VersionInfoService {
    pub fn new(database: Arc<DatabaseService>) -> Self {
        Self(database)
    }

    /// Isolated typed-service constructor for tests and portable embedding.
    /// Product startup obtains the shared DatabaseService from Cordis instead.
    pub fn open_at(path: impl Into<PathBuf>) -> Result<Self, VersionInfoError> {
        Ok(Self(Arc::new(DatabaseService::open_at(path.into())?)))
    }

    pub fn list(&self, entity_id: &str) -> Result<Vec<VersionRecord>, VersionInfoError> {
        Ok(self.0.version_records(entity_id)?)
    }

    pub fn save(&self, record: &VersionRecord) -> Result<(), VersionInfoError> {
        Ok(self.0.save_version_record(record)?)
    }

    pub fn set_current(&self, entity_id: &str, version_id: &str) -> Result<(), VersionInfoError> {
        Ok(self.0.set_current_version(entity_id, version_id)?)
    }

    pub fn put_graph(&self, entity_id: &str, graph: &VersionGraph) -> Result<(), VersionInfoError> {
        Ok(self.0.put_version_graph(entity_id, graph)?)
    }

    pub fn graph(&self, entity_id: &str) -> Result<VersionGraph, VersionInfoError> {
        Ok(self.0.version_graph(entity_id)?)
    }
}

pub struct VersionInfoConfig {
    /// Legacy standalone metadata path. Existing rows are imported into the
    /// shared DatabaseService, while the original file is preserved for rollback.
    pub path: PathBuf,
}

pub struct VersionInfoInput {
    legacy_path: PathBuf,
    publication: Arc<Mutex<Option<ServicePublication<VersionInfoService>>>>,
}

pub struct VersionInfoPlugin;

impl Plugin for VersionInfoPlugin {
    type Config = VersionInfoConfig;
    type Input = VersionInfoInput;
    type PrepareError = std::convert::Infallible;
    type ApplyError = VersionInfoError;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        "photag.version-info".into()
    }

    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(VersionInfoInput {
            legacy_path: config.path,
            publication: Arc::new(Mutex::new(None)),
        })
    }

    async fn apply(&self, ctx: Context, input: &Self::Input) -> Result<(), Self::ApplyError> {
        let database = ctx
            .try_service::<DatabaseService>()
            .map_err(|error| VersionInfoError::Dependency(error.to_string()))?;
        let legacy_path = input.legacy_path.clone();
        let migration_database = database.clone();
        tokio::task::spawn_blocking(move || {
            migration_database.import_legacy_version_info(&legacy_path)
        })
        .await
        .map_err(VersionInfoError::Worker)??;
        let publication = ctx
            .provide(Arc::new(VersionInfoService::new(database)))
            .map_err(|error| VersionInfoError::Publish(error.to_string()))?;
        *input
            .publication
            .lock()
            .map_err(|_| VersionInfoError::Lock)? = Some(publication);
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VersionInfoError {
    #[error("version metadata database operation failed: {0}")]
    Database(#[from] DatabaseAccessError),
    #[error("version metadata database could not be opened: {0}")]
    Open(#[from] DatabaseOpenError),
    #[error("version metadata service lock is unavailable")]
    Lock,
    #[error("version metadata record was not found")]
    NotFound,
    #[error("version metadata DatabaseService is unavailable: {0}")]
    Dependency(String),
    #[error("version metadata migration worker failed: {0}")]
    Worker(#[from] tokio::task::JoinError),
    #[error("version metadata service publication failed: {0}")]
    Publish(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{Connection, params};

    fn record(revision: &str, number: u64, current: bool) -> VersionRecord {
        VersionRecord {
            entity_id: "file-key".into(),
            version_id: revision.into(),
            revision_number: number,
            parent_revision_ids: Vec::new(),
            title: revision.into(),
            created_at: "2026-10-17T00:00:00Z".into(),
            size_bytes: 12,
            content_type: Some("image/jpeg".into()),
            thumbnail_path: None,
            is_current: current,
        }
    }

    #[test]
    fn typed_service_persists_versions_graph_and_current_marker()
    -> Result<(), Box<dyn std::error::Error>> {
        let path = std::env::temp_dir().join(format!(
            "photag-version-info-{}-{}.sqlite3",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        let database = Arc::new(DatabaseService::open_at(&path)?);
        let service = VersionInfoService::new(database.clone());
        service.save(&record("r1", 1, true))?;
        service.save(&record("r2", 2, true))?;
        let versions = service.list("file-key")?;
        assert_eq!(
            versions
                .iter()
                .filter(|version| version.is_current)
                .map(|version| version.version_id.as_str())
                .collect::<Vec<_>>(),
            vec!["r2"]
        );
        service.set_current("file-key", "r1")?;
        assert_eq!(
            service
                .list("file-key")?
                .iter()
                .find(|version| version.is_current)
                .map(|version| version.version_id.as_str()),
            Some("r1")
        );
        let graph = VersionGraph {
            graph_state: "ready".into(),
            ..Default::default()
        };
        service.put_graph("file-key", &graph)?;
        assert_eq!(service.graph("file-key")?, graph);
        drop(service);
        drop(database);

        let reopened = VersionInfoService::open_at(&path)?;
        assert_eq!(reopened.list("file-key")?.len(), 2);
        drop(reopened);
        let _ = std::fs::remove_file(path);
        Ok(())
    }

    #[test]
    fn legacy_database_import_is_idempotent_and_preserves_source()
    -> Result<(), Box<dyn std::error::Error>> {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "photag-version-migration-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root)?;
        let legacy = root.join("version-info.sqlite3");
        let old = Connection::open(&legacy)?;
        old.execute_batch("CREATE TABLE versions(entity_id TEXT NOT NULL,version_id TEXT NOT NULL,revision_number INTEGER NOT NULL,parents TEXT NOT NULL,title TEXT NOT NULL,created_at TEXT NOT NULL,size_bytes INTEGER NOT NULL,content_type TEXT,thumbnail_path TEXT,is_current INTEGER NOT NULL DEFAULT 0,PRIMARY KEY(entity_id,version_id)); CREATE TABLE version_graph(entity_id TEXT PRIMARY KEY,snapshot TEXT NOT NULL);")?;
        let saved = record("legacy-r1", 1, true);
        old.execute(
            "INSERT INTO versions VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                saved.entity_id,
                saved.version_id,
                saved.revision_number as i64,
                "[]",
                saved.title,
                saved.created_at,
                saved.size_bytes as i64,
                saved.content_type,
                saved.thumbnail_path,
                saved.is_current
            ],
        )?;
        old.execute(
            "INSERT INTO version_graph VALUES(?1,?2)",
            params![
                "file-key",
                serde_json::to_string(&VersionGraph {
                    graph_state: "legacy".into(),
                    ..Default::default()
                })?
            ],
        )?;
        drop(old);

        let database = DatabaseService::open_browser_at(root.join("photag-tags.db"))?;
        database.import_legacy_version_info(&legacy)?;
        database.import_legacy_version_info(&legacy)?;
        assert_eq!(database.version_records("file-key")?.len(), 1);
        assert_eq!(database.version_graph("file-key")?.graph_state, "legacy");
        assert!(legacy.exists(), "migration must leave the old file intact");
        drop(database);
        let reopened = DatabaseService::open_browser_at(root.join("photag-tags.db"))?;
        assert_eq!(reopened.version_records("file-key")?.len(), 1);
        assert_eq!(reopened.version_graph("file-key")?.graph_state, "legacy");
        let _ = std::fs::remove_dir_all(root);
        Ok(())
    }
}
