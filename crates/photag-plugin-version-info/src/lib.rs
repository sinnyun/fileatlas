//! Durable version metadata and graph projection, separate from Lore's object store.
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use cordis::{Context, Plugin, Service, service::ServicePublication};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

const SERVICE_NAME: &str = "photag/version-info";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VersionRecord {
    pub entity_id: String,
    pub version_id: String,
    pub revision_number: u64,
    pub parent_revision_ids: Vec<String>,
    pub title: String,
    pub created_at: String,
    pub size_bytes: u64,
    pub content_type: Option<String>,
    pub thumbnail_path: Option<String>,
    pub is_current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct VersionBranch {
    pub branch_id: String,
    pub name: String,
    #[serde(default)]
    pub category: String,
    pub head_revision_id: Option<String>,
    pub is_current: bool,
    pub is_archived: bool,
    #[serde(default)]
    pub created_at_unix: u64,
    #[serde(default)]
    pub stack: Vec<VersionBranchPoint>,
    pub revisions: Vec<VersionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct VersionBranchPoint {
    pub branch_id: String,
    pub revision_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PendingVersionMerge {
    pub source_branch: String,
    pub target_branch: String,
    pub conflict_files: Vec<String>,
    pub target_base_revision_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct VersionGraph {
    pub branches: Vec<VersionBranch>,
    pub graph_state: String,
    #[serde(default)]
    pub pending_merge: Option<PendingVersionMerge>,
}

#[derive(Clone)]
pub struct VersionInfoService(Arc<Mutex<Connection>>);

impl Service for VersionInfoService {
    const NAME: &'static str = SERVICE_NAME;
}

impl VersionInfoService {
    /// Open the durable version metadata store. The Cordis plugin uses this
    /// constructor during `prepare`; exposing it also enables isolated
    /// service-level integration tests.
    pub fn open_at(path: impl Into<PathBuf>) -> Result<Self, VersionInfoError> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let db = Connection::open(path)?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS versions(entity_id TEXT NOT NULL,version_id TEXT NOT NULL,revision_number INTEGER NOT NULL,parents TEXT NOT NULL,title TEXT NOT NULL,created_at TEXT NOT NULL,size_bytes INTEGER NOT NULL,content_type TEXT,thumbnail_path TEXT,is_current INTEGER NOT NULL DEFAULT 0,PRIMARY KEY(entity_id,version_id)); CREATE INDEX IF NOT EXISTS idx_versions_entity_revision ON versions(entity_id,revision_number DESC); CREATE UNIQUE INDEX IF NOT EXISTS idx_versions_current ON versions(entity_id) WHERE is_current=1; CREATE TABLE IF NOT EXISTS version_graph(entity_id TEXT PRIMARY KEY,snapshot TEXT NOT NULL);")?;
        Ok(Self(Arc::new(Mutex::new(db))))
    }

    pub fn list(&self, entity_id: &str) -> Result<Vec<VersionRecord>, VersionInfoError> {
        let db = self.lock()?;
        let mut query = db.prepare("SELECT version_id,revision_number,parents,title,created_at,size_bytes,content_type,thumbnail_path,is_current FROM versions WHERE entity_id=?1 ORDER BY revision_number DESC")?;
        query
            .query_map([entity_id], |row| {
                let parents: String = row.get(2)?;
                Ok(VersionRecord {
                    entity_id: entity_id.into(),
                    version_id: row.get(0)?,
                    revision_number: row.get::<_, i64>(1)? as u64,
                    parent_revision_ids: serde_json::from_str(&parents).unwrap_or_default(),
                    title: row.get(3)?,
                    created_at: row.get(4)?,
                    size_bytes: row.get::<_, i64>(5)?.max(0) as u64,
                    content_type: row.get(6)?,
                    thumbnail_path: row.get(7)?,
                    is_current: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn save(&self, record: &VersionRecord) -> Result<(), VersionInfoError> {
        let mut db = self.lock()?;
        let tx = db.transaction()?;
        if record.is_current {
            tx.execute(
                "UPDATE versions SET is_current=0 WHERE entity_id=?1",
                [&record.entity_id],
            )?;
        }
        tx.execute("INSERT INTO versions(entity_id,version_id,revision_number,parents,title,created_at,size_bytes,content_type,thumbnail_path,is_current) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10) ON CONFLICT(entity_id,version_id) DO UPDATE SET revision_number=excluded.revision_number,parents=excluded.parents,title=excluded.title,size_bytes=excluded.size_bytes,content_type=excluded.content_type,thumbnail_path=COALESCE(excluded.thumbnail_path,versions.thumbnail_path),is_current=excluded.is_current", params![record.entity_id,record.version_id,record.revision_number as i64,serde_json::to_string(&record.parent_revision_ids)?,record.title,record.created_at,record.size_bytes as i64,record.content_type,record.thumbnail_path,record.is_current])?;
        tx.commit()?;
        Ok(())
    }

    pub fn set_current(&self, entity_id: &str, version_id: &str) -> Result<(), VersionInfoError> {
        let mut db = self.lock()?;
        let tx = db.transaction()?;
        let updated = tx.execute(
            "UPDATE versions SET is_current=0 WHERE entity_id=?1",
            [entity_id],
        )?;
        let _ = updated;
        if tx.execute(
            "UPDATE versions SET is_current=1 WHERE entity_id=?1 AND version_id=?2",
            params![entity_id, version_id],
        )? != 1
        {
            return Err(VersionInfoError::NotFound);
        }
        tx.commit()?;
        Ok(())
    }

    pub fn put_graph(&self, entity_id: &str, graph: &VersionGraph) -> Result<(), VersionInfoError> {
        self.lock()?.execute("INSERT INTO version_graph(entity_id,snapshot) VALUES(?1,?2) ON CONFLICT(entity_id) DO UPDATE SET snapshot=excluded.snapshot", params![entity_id,serde_json::to_string(graph)?])?;
        Ok(())
    }

    pub fn graph(&self, entity_id: &str) -> Result<VersionGraph, VersionInfoError> {
        let db = self.lock()?;
        db.query_row(
            "SELECT snapshot FROM version_graph WHERE entity_id=?1",
            [entity_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .map(|json| serde_json::from_str(&json).map_err(Into::into))
        .unwrap_or_else(|| Ok(VersionGraph::default()))
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, VersionInfoError> {
        self.0.lock().map_err(|_| VersionInfoError::Lock)
    }
}

pub struct VersionInfoConfig {
    pub path: PathBuf,
}
pub struct VersionInfoInput {
    database: Arc<Mutex<Connection>>,
    publication: Arc<Mutex<Option<ServicePublication<VersionInfoService>>>>,
}
pub struct VersionInfoPlugin;
impl Plugin for VersionInfoPlugin {
    type Config = VersionInfoConfig;
    type Input = VersionInfoInput;
    type PrepareError = VersionInfoError;
    type ApplyError = VersionInfoError;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        "photag.version-info".into()
    }
    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        let database = VersionInfoService::open_at(config.path)?.0;
        Ok(VersionInfoInput {
            database,
            publication: Arc::new(Mutex::new(None)),
        })
    }
    async fn apply(&self, ctx: Context, input: &Self::Input) -> Result<(), Self::ApplyError> {
        let publication = ctx
            .provide(Arc::new(VersionInfoService(input.database.clone())))
            .map_err(|e| VersionInfoError::Publish(e.to_string()))?;
        *input
            .publication
            .lock()
            .map_err(|_| VersionInfoError::Lock)? = Some(publication);
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VersionInfoError {
    #[error("version metadata database error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("version metadata encode error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("version metadata file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("version metadata service lock is unavailable")]
    Lock,
    #[error("version metadata record was not found")]
    NotFound,
    #[error("version metadata service publication failed: {0}")]
    Publish(String),
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn versions_persist_and_current_revision_is_unique() {
        let path = std::env::temp_dir().join(format!(
            "photag-version-info-{}-{}.sqlite3",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos())
        ));
        let plugin = VersionInfoPlugin;
        let input = match plugin.prepare(VersionInfoConfig { path: path.clone() }) {
            Ok(input) => input,
            Err(error) => panic!("could not open metadata database: {error}"),
        };
        let service = VersionInfoService(input.database.clone());
        assert!(service.save(&record("r1", 1, true)).is_ok());
        assert!(service.save(&record("r2", 2, true)).is_ok());
        let first_read = match service.list("file-key") {
            Ok(versions) => versions,
            Err(error) => panic!("could not list saved versions: {error}"),
        };
        assert_eq!(
            first_read
                .iter()
                .filter(|version| version.is_current)
                .map(|version| version.version_id.as_str())
                .collect::<Vec<_>>(),
            vec!["r2"]
        );
        drop(service);
        drop(input);

        let reopened = match plugin.prepare(VersionInfoConfig { path: path.clone() }) {
            Ok(input) => input,
            Err(error) => panic!("could not reopen metadata database: {error}"),
        };
        let service = VersionInfoService(reopened.database.clone());
        let versions = match service.list("file-key") {
            Ok(versions) => versions,
            Err(error) => panic!("could not read persisted versions: {error}"),
        };
        assert_eq!(versions.len(), 2);
        assert!(service.set_current("file-key", "r1").is_ok());
        let versions = match service.list("file-key") {
            Ok(versions) => versions,
            Err(error) => panic!("could not read current revision: {error}"),
        };
        assert_eq!(
            versions
                .iter()
                .find(|version| version.is_current)
                .map(|version| version.version_id.as_str()),
            Some("r1")
        );
        drop(reopened);
        let _ = std::fs::remove_file(path);
    }
}
