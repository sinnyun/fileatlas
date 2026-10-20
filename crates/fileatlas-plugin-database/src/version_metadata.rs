//! Typed durable metadata for Lore-backed file versions.
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::{DatabaseAccessError, DatabaseService};

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

impl DatabaseService {
    pub fn version_records(
        &self,
        entity_id: &str,
    ) -> Result<Vec<VersionRecord>, DatabaseAccessError> {
        let rows = self.with_reader(|connection| {
            let mut statement = connection.prepare("SELECT version_id,revision_number,parents,title,created_at,size_bytes,content_type,thumbnail_path,is_current FROM versions WHERE entity_id=?1 ORDER BY revision_number DESC")?;
            let rows = statement.query_map([entity_id], |row| {
                let parents: String = row.get(2)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    parents,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, bool>(8)?,
                ))
            })?;
            rows.collect::<Result<Vec<_>, _>>()
        })?;
        rows.into_iter()
            .map(
                |(
                    version_id,
                    revision_number,
                    parents,
                    title,
                    created_at,
                    size_bytes,
                    content_type,
                    thumbnail_path,
                    is_current,
                )| {
                    Ok(VersionRecord {
                        entity_id: entity_id.to_owned(),
                        version_id,
                        revision_number: revision_number.max(0) as u64,
                        parent_revision_ids: serde_json::from_str(&parents)?,
                        title,
                        created_at,
                        size_bytes: size_bytes.max(0) as u64,
                        content_type,
                        thumbnail_path,
                        is_current,
                    })
                },
            )
            .collect()
    }

    pub fn save_version_record(&self, record: &VersionRecord) -> Result<(), DatabaseAccessError> {
        let parents = serde_json::to_string(&record.parent_revision_ids)?;
        self.with_writer(|connection| {
            let transaction = connection.unchecked_transaction()?;
            if record.is_current {
                transaction.execute(
                    "UPDATE versions SET is_current=0 WHERE entity_id=?1",
                    [&record.entity_id],
                )?;
            }
            transaction.execute(
                "INSERT INTO versions(entity_id,version_id,revision_number,parents,title,created_at,size_bytes,content_type,thumbnail_path,is_current) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10) ON CONFLICT(entity_id,version_id) DO UPDATE SET revision_number=excluded.revision_number,parents=excluded.parents,title=excluded.title,created_at=excluded.created_at,size_bytes=excluded.size_bytes,content_type=excluded.content_type,thumbnail_path=COALESCE(excluded.thumbnail_path,versions.thumbnail_path),is_current=excluded.is_current",
                params![record.entity_id,record.version_id,record.revision_number as i64,parents,record.title,record.created_at,record.size_bytes as i64,record.content_type,record.thumbnail_path,record.is_current],
            )?;
            transaction.commit()
        })
    }

    pub fn set_current_version(
        &self,
        entity_id: &str,
        version_id: &str,
    ) -> Result<(), DatabaseAccessError> {
        self.with_writer(|connection| {
            let transaction = connection.unchecked_transaction()?;
            transaction.execute(
                "UPDATE versions SET is_current=0 WHERE entity_id=?1",
                [entity_id],
            )?;
            if transaction.execute(
                "UPDATE versions SET is_current=1 WHERE entity_id=?1 AND version_id=?2",
                params![entity_id, version_id],
            )? != 1
            {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            transaction.commit()
        })?;
        Ok(())
    }

    pub fn put_version_graph(
        &self,
        entity_id: &str,
        graph: &VersionGraph,
    ) -> Result<(), DatabaseAccessError> {
        let snapshot = serde_json::to_string(graph)?;
        self.with_writer(|connection| {
            connection.execute(
                "INSERT INTO version_graph(entity_id,snapshot) VALUES(?1,?2) ON CONFLICT(entity_id) DO UPDATE SET snapshot=excluded.snapshot",
                params![entity_id, snapshot],
            )?;
            Ok(())
        })
    }

    pub fn version_graph(&self, entity_id: &str) -> Result<VersionGraph, DatabaseAccessError> {
        let json = self.with_reader(|connection| {
            let json = connection
                .query_row(
                    "SELECT snapshot FROM version_graph WHERE entity_id=?1",
                    [entity_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            Ok(json)
        })?;
        json.map(|value| serde_json::from_str(&value).map_err(Into::into))
            .unwrap_or_else(|| Ok(VersionGraph::default()))
    }

    /// Import the former standalone metadata database once, without deleting it.
    /// Repeated startup imports are idempotent and never overwrite newer rows.
    pub fn import_legacy_version_info(&self, path: &Path) -> Result<(), DatabaseAccessError> {
        if !path.is_file() {
            return Ok(());
        }
        let legacy_path = path.to_string_lossy().into_owned();
        let already_imported = self.with_reader(|connection| {
            connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM version_metadata_imports WHERE legacy_path=?1)",
                [&legacy_path],
                |row| row.get::<_, bool>(0),
            )
        })?;
        if already_imported {
            return Ok(());
        }
        let source = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let has_versions: bool = source.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='versions')",
            [],
            |row| row.get(0),
        )?;
        if !has_versions {
            return Ok(());
        }
        let mut statement = source.prepare("SELECT entity_id,version_id,revision_number,parents,title,created_at,size_bytes,content_type,thumbnail_path,is_current FROM versions ORDER BY entity_id,revision_number")?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, bool>(9)?,
            ))
        })?;
        let mut records = Vec::new();
        for row in rows {
            let (
                entity_id,
                version_id,
                revision_number,
                parents,
                title,
                created_at,
                size_bytes,
                content_type,
                thumbnail_path,
                is_current,
            ) = row?;
            records.push(VersionRecord {
                entity_id,
                version_id,
                revision_number: revision_number.max(0) as u64,
                parent_revision_ids: serde_json::from_str(&parents)?,
                title,
                created_at,
                size_bytes: size_bytes.max(0) as u64,
                content_type,
                thumbnail_path,
                is_current,
            });
        }
        let has_graph: bool = source.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='version_graph')",
            [],
            |row| row.get(0),
        )?;
        let graphs = if has_graph {
            let mut statement = source.prepare("SELECT entity_id,snapshot FROM version_graph")?;
            statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()?
        } else {
            Vec::new()
        };
        let imported_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs() as i64);
        self.with_writer(|connection| {
            let transaction = connection.unchecked_transaction()?;
            for record in &records {
                transaction.execute(
                    "INSERT OR IGNORE INTO versions(entity_id,version_id,revision_number,parents,title,created_at,size_bytes,content_type,thumbnail_path,is_current) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                    params![record.entity_id,record.version_id,record.revision_number as i64,serde_json::to_string(&record.parent_revision_ids).map_err(|error|rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?,record.title,record.created_at,record.size_bytes as i64,record.content_type,record.thumbnail_path,record.is_current],
                )?;
            }
            for (entity_id, snapshot) in &graphs {
                transaction.execute(
                    "INSERT OR IGNORE INTO version_graph(entity_id,snapshot) VALUES(?1,?2)",
                    params![entity_id, snapshot],
                )?;
            }
            transaction.execute(
                "INSERT OR IGNORE INTO version_metadata_imports(legacy_path,imported_at) VALUES(?1,?2)",
                params![legacy_path, imported_at],
            )?;
            transaction.commit()
        })
    }
}
