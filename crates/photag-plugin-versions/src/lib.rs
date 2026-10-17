//! Lore-backed, per-file immutable version history.
mod branches;
mod commits;
mod state;
pub use state::{FileVersionState, FileVersionStatus};
use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use cordis::{Context, Plugin, Service, service::ServicePublication};
use lore_vm::{
    api::LoreApi,
    global::LoreGlobal,
    ops::{
        file::stage::{self, CaseChange, FileStageArgs},
        repository::{
            create::{self, CreateArgs},
            flush,
            status::{self, RepositoryStatusArgs},
        },
        revision::{
            commit::{self, CommitArgs},
            history::{self, RevisionHistoryArgs},
            info::{self, RevisionInfoArgs},
            sync::{self, RevisionSyncArgs},
        },
        shared_store::{create as create_store, create::SharedStoreCreateArgs},
    },
};
use photag_plugin_thumbnails::ThumbnailService;
pub use photag_plugin_version_info::{
    PendingVersionMerge, VersionBranch, VersionBranchPoint, VersionGraph,
};
use photag_plugin_version_info::{VersionInfoService, VersionRecord};
use serde::Serialize;
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionEntry {
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

#[derive(Debug, thiserror::Error)]
pub enum VersionsError {
    #[error("version engine error: {0}")]
    Lore(#[from] lore_vm::LoreError),
    #[error("file history operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("version metadata operation failed: {0}")]
    Metadata(#[from] photag_plugin_version_info::VersionInfoError),
    #[error("version operation lock is unavailable")]
    Lock,
    #[error("selected file does not exist or is not a regular file")]
    InvalidFile,
    #[error("unsaved file changes exist; create a version before switching")]
    UnsavedChanges,
    #[error("the requested version was not found in this file's history")]
    NotFound,
    #[error("version title must be between 1 and 240 printable characters")]
    InvalidTitle,
    #[error("service dependency is unavailable: {0}")]
    Dependency(String),
    #[error("service publication failed: {0}")]
    Publish(String),
    #[error("version engine task failed")]
    Worker,
    #[error(
        "branch name must be 1–120 characters and contain only letters, digits, '.', '_' or '-'"
    )]
    InvalidBranchName,
    #[error("the file has a pending branch merge that must be resolved or aborted first")]
    PendingMerge,
    #[error("cannot archive the current branch")]
    CurrentBranch,
    #[error("cannot merge a branch into itself")]
    MergeIntoSelf,
    #[error("merge strategy must be 'ours' or 'theirs'")]
    InvalidMergeStrategy,
    #[error("merge target must be the current branch")]
    MergeTargetNotCurrent,
}

pub struct VersionsService {
    root: PathBuf,
    info: Arc<VersionInfoService>,
    thumbs: Arc<ThumbnailService>,
    locks: Arc<Mutex<HashMap<String, Arc<AsyncMutex<()>>>>>,
}

struct FileOperationGuard {
    key: String,
    locks: Arc<Mutex<HashMap<String, Arc<AsyncMutex<()>>>>>,
    lock: Arc<AsyncMutex<()>>,
    guard: Option<OwnedMutexGuard<()>>,
}

impl Drop for FileOperationGuard {
    fn drop(&mut self) {
        self.guard.take();
        if let Ok(mut locks) = self.locks.lock()
            && locks
                .get(&self.key)
                .is_some_and(|lock| Arc::ptr_eq(lock, &self.lock) && Arc::strong_count(lock) == 2)
        {
            locks.remove(&self.key);
        }
    }
}
impl Service for VersionsService {
    const NAME: &'static str = "photag/versions";
}

impl VersionsService {
    /// Root directory containing Lore worktrees, shared stores, and previews.
    pub fn storage_path(&self) -> &Path {
        &self.root
    }

    fn key(path: &Path) -> Result<(String, PathBuf), VersionsError> {
        let path = fs::canonicalize(path)?;
        if !fs::symlink_metadata(&path)?.is_file() {
            return Err(VersionsError::InvalidFile);
        }
        let normalized = path.to_string_lossy().replace('\\', "/").to_lowercase();
        Ok((
            blake3::hash(normalized.as_bytes()).to_hex().to_string(),
            path,
        ))
    }
    async fn lock(&self, key: &str) -> Result<FileOperationGuard, VersionsError> {
        let lock = {
            let mut map = self.locks.lock().map_err(|_| VersionsError::Lock)?;
            map.entry(key.into())
                .or_insert_with(|| Arc::new(AsyncMutex::new(())))
                .clone()
        };
        let guard = lock.clone().lock_owned().await;
        Ok(FileOperationGuard {
            key: key.to_owned(),
            locks: self.locks.clone(),
            lock,
            guard: Some(guard),
        })
    }
    fn paths(&self, key: &str) -> (PathBuf, PathBuf, PathBuf) {
        let work = self.root.join("worktrees").join(key);
        (
            work.join("content"),
            work,
            self.root.join("stores").join(key).join("shared-store"),
        )
    }
    async fn open(&self, key: &str, work: &Path, store: &Path) -> Result<LoreApi, VersionsError> {
        let api = LoreApi::from_global(
            LoreGlobal::new(work.to_path_buf())
                .offline(true)
                .in_memory(false)
                .identity("photag-local"),
        );
        if work.join(".lore").exists() {
            Self::status(&api, false).await?;
            return Ok(api);
        }
        fs::create_dir_all(store.parent().ok_or(VersionsError::InvalidFile)?)?;
        if !store.exists() {
            create_store::create(
                &api,
                SharedStoreCreateArgs {
                    remote_url: String::new(),
                    path: Some(store.to_string_lossy().into_owned()),
                    make_default: false,
                },
            )
            .await?;
        }
        let name = blake3::hash(key.as_bytes()).to_hex().to_string();
        create::create(
            &api,
            CreateArgs {
                repository_url: format!("lore://localhost/{name}"),
                description: "PhoTag file history".into(),
                id: String::new(),
                use_shared_store: true,
                shared_store_path: store.to_string_lossy().into_owned(),
            },
        )
        .await?;
        flush::flush(&api).await?;
        Ok(api)
    }
    fn status<'a>(
        api: &'a LoreApi,
        scan: bool,
    ) -> impl std::future::Future<
        Output = Result<
            lore_vm::ops::repository::status::RepositoryStatusResult,
            lore_vm::LoreError,
        >,
    > + 'a {
        status::status(
            api,
            RepositoryStatusArgs {
                scan,
                staged: true,
                ..Default::default()
            },
        )
    }

    pub async fn list(&self, path: &str) -> Result<Vec<VersionEntry>, VersionsError> {
        let (key, _path) = Self::key(Path::new(path))?;
        let _guard = self.lock(&key).await?;
        let (_content, work, store) = self.paths(&key);
        if !work.join(".lore").exists() {
            return Ok(Vec::new());
        }
        let api = self.open(&key, &work, &store).await?;
        let head = Self::status(&api, true).await?.revision.map(|r| r.revision);
        let history = history::history(&api, RevisionHistoryArgs::default()).await?;
        let saved_records = self.info.list(&key)?;
        let saved_ids = saved_records
            .iter()
            .map(|record| record.version_id.as_str())
            .collect::<std::collections::HashSet<_>>();
        // Durable version metadata is authoritative for the user-facing list.
        // Lore history is only the current head's ancestry and cannot represent
        // saved revisions after activating an older version.
        let mut entries = saved_records
            .iter()
            .cloned()
            .map(|record| {
                let mut entry = to_entry(record);
                if let Some(head) = head.as_deref() {
                    entry.is_current = head == entry.version_id;
                }
                entry
            })
            .collect::<Vec<_>>();
        for item in history.entries {
            if saved_ids.contains(item.revision.as_str()) {
                continue;
            }
            let info = info::info(
                &api,
                RevisionInfoArgs {
                    revision: item.revision.clone(),
                    delta: true,
                    metadata: true,
                },
            )
            .await?;
            let matching = info
                .deltas
                .iter()
                .filter(|d| d.path.replace('\\', "/") == "content")
                .collect::<Vec<_>>();
            if matching.is_empty() {
                continue;
            }
            let created = info.timestamp().and_then(parse_time).unwrap_or_else(now);
            let size = matching.iter().map(|d| d.size).max().unwrap_or(0);
            entries.push(VersionEntry {
                version_id: item.revision.clone(),
                revision_number: item.revision_number,
                parent_revision_ids: item.parents.clone(),
                title: info
                    .message()
                    .map(str::to_owned)
                    .unwrap_or_else(|| "Version".into()),
                created_at: created,
                size_bytes: size,
                content_type: None,
                thumbnail_path: None,
                is_current: head.as_deref() == Some(item.revision.as_str()),
            });
        }
        Ok(entries)
    }

    pub async fn create_version(
        &self,
        path: &str,
        title: &str,
    ) -> Result<VersionEntry, VersionsError> {
        let title = title.trim();
        if title.is_empty() || title.chars().count() > 240 || title.chars().any(char::is_control) {
            return Err(VersionsError::InvalidTitle);
        }
        let (key, path) = Self::key(Path::new(path))?;
        let _guard = self.lock(&key).await?;
        let (content, work, store) = self.paths(&key);
        fs::create_dir_all(&work)?;
        let api = self.open(&key, &work, &store).await?;
        if self.info.graph(&key)?.pending_merge.is_some() {
            return Err(VersionsError::PendingMerge);
        }
        let before = Self::status(&api, true).await?;
        if before.files.is_empty() && content.exists() && same_file(&path, &content)? {
            return Err(VersionsError::UnsavedChanges);
        }
        self.prepare_snapshot_commit(&api, &content, &before)
            .await?;
        let source = path.clone();
        let dest = content.clone();
        tokio::task::spawn_blocking(move || copy_atomic(&source, &dest))
            .await
            .map_err(|_| VersionsError::Worker)??;
        let staged = stage::stage(
            &api,
            FileStageArgs {
                paths: vec![content.to_string_lossy().into_owned()],
                case_change: CaseChange::Error,
                scan: true,
            },
        )
        .await?;
        if staged.files.is_empty() {
            return Err(VersionsError::UnsavedChanges);
        }
        let commit = commit::commit(
            &api,
            CommitArgs {
                message: title.into(),
            },
        )
        .await?;
        flush::flush(&api).await?;
        let revision = info::info(
            &api,
            RevisionInfoArgs {
                revision: commit.revision.clone(),
                delta: true,
                metadata: true,
            },
        )
        .await?;
        let thumbs = self.thumbs.clone();
        let preview_path = path.clone();
        let preview =
            tokio::task::spawn_blocking(move || thumbs.generate_image(&preview_path).map(|v| v.0))
                .await
                .map_err(|_| VersionsError::Worker)?;
        let thumbnail_path = preview.and_then(|source| {
            let target = self
                .root
                .join("previews")
                .join(&key)
                .join(format!("{}.jpg", commit.revision));
            target
                .parent()
                .and_then(|p| fs::create_dir_all(p).ok())
                .and_then(|_| fs::copy(source, &target).ok())
                .map(|_| target.to_string_lossy().into_owned())
        });
        let parents = history::history(&api, RevisionHistoryArgs::default())
            .await?
            .entries
            .into_iter()
            .find(|entry| entry.revision == commit.revision)
            .map(|entry| entry.parents)
            .unwrap_or_default();
        let size = fs::metadata(&path)?.len();
        let record = VersionRecord {
            entity_id: key.clone(),
            version_id: commit.revision.clone(),
            revision_number: commit.revision_number,
            parent_revision_ids: parents,
            title: title.into(),
            created_at: revision
                .timestamp()
                .and_then(parse_time)
                .unwrap_or_else(now),
            size_bytes: size,
            content_type: mime_guess::from_path(&path).first_raw().map(str::to_owned),
            thumbnail_path,
            is_current: true,
        };
        self.info.save(&record)?;
        Ok(to_entry(record))
    }

    pub async fn activate(&self, path: &str, version_id: &str) -> Result<(), VersionsError> {
        let (key, path) = Self::key(Path::new(path))?;
        let _guard = self.lock(&key).await?;
        let (content, work, store) = self.paths(&key);
        if !work.join(".lore").exists() {
            return Err(VersionsError::NotFound);
        }
        if self.info.graph(&key)?.pending_merge.is_some() {
            return Err(VersionsError::PendingMerge);
        }
        let api = self.open(&key, &work, &store).await?;
        let status = Self::status(&api, true).await?;
        if !status.files.is_empty() || !same_file(&path, &content)? {
            return Err(VersionsError::UnsavedChanges);
        }
        if !self
            .info
            .list(&key)?
            .iter()
            .any(|v| v.version_id == version_id)
        {
            return Err(VersionsError::NotFound);
        }
        info::info(
            &api,
            RevisionInfoArgs {
                revision: version_id.into(),
                delta: false,
                metadata: false,
            },
        )
        .await?;
        sync::sync(
            &api,
            RevisionSyncArgs {
                revision: version_id.into(),
                reset: false,
                ..Default::default()
            },
        )
        .await?;
        flush::flush(&api).await?;
        let source = content.clone();
        let dest = path.clone();
        tokio::task::spawn_blocking(move || replace_atomic(&source, &dest))
            .await
            .map_err(|_| VersionsError::Worker)??;
        self.info.set_current(&key, version_id)?;
        Ok(())
    }
}

fn to_entry(v: VersionRecord) -> VersionEntry {
    VersionEntry {
        version_id: v.version_id,
        revision_number: v.revision_number,
        parent_revision_ids: v.parent_revision_ids,
        title: v.title,
        created_at: v.created_at,
        size_bytes: v.size_bytes,
        content_type: v.content_type,
        thumbnail_path: v.thumbnail_path,
        is_current: v.is_current,
    }
}
fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
fn parse_time(value: &str) -> Option<String> {
    let timestamp = value.parse::<i64>().ok()?;
    let parsed = if timestamp.unsigned_abs() >= 100_000_000_000_000_000 {
        chrono::DateTime::from_timestamp(
            timestamp.div_euclid(1_000_000_000),
            timestamp.rem_euclid(1_000_000_000) as u32,
        )
    } else if timestamp.unsigned_abs() >= 100_000_000_000 {
        chrono::DateTime::from_timestamp_millis(timestamp)
    } else {
        chrono::DateTime::from_timestamp(timestamp, 0)
    }?;
    Some(parsed.to_rfc3339())
}

#[cfg(test)]
mod tests {
    use super::parse_time;

    #[test]
    fn lore_millisecond_timestamps_are_rendered_as_real_dates() {
        let parsed = parse_time("1792168891000");
        assert!(
            parsed
                .as_deref()
                .is_some_and(|value| value.starts_with("2026-10-16T")),
            "unexpected timestamp: {parsed:?}"
        );
    }
}
fn same_file(a: &Path, b: &Path) -> Result<bool, std::io::Error> {
    if fs::metadata(a)?.len() != fs::metadata(b)?.len() {
        return Ok(false);
    }
    let mut left = fs::File::open(a)?;
    let mut right = fs::File::open(b)?;
    let mut left_buf = [0_u8; 64 * 1024];
    let mut right_buf = [0_u8; 64 * 1024];
    loop {
        let left_count = left.read(&mut left_buf)?;
        let right_count = right.read(&mut right_buf)?;
        if left_count != right_count || left_buf[..left_count] != right_buf[..right_count] {
            return Ok(false);
        }
        if left_count == 0 {
            return Ok(true);
        }
    }
}
fn copy_atomic(source: &Path, dest: &Path) -> Result<(), std::io::Error> {
    if source.starts_with(dest) || dest.starts_with(source) {
        return Err(std::io::Error::other(
            "version source overlaps its private snapshot path",
        ));
    }
    replace_atomic(source, dest)
}
fn replace_atomic(source: &Path, dest: &Path) -> Result<(), std::io::Error> {
    static TEMP_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let parent = dest
        .parent()
        .ok_or_else(|| std::io::Error::other("missing parent"))?;
    fs::create_dir_all(parent)?;
    let sequence = TEMP_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let stem = format!(".photag-version-{}-{sequence}", std::process::id());
    let temp = parent.join(format!("{stem}.tmp"));
    let backup = parent.join(format!("{stem}.bak"));
    fs::copy(source, &temp)?;
    if dest.exists() {
        fs::rename(dest, &backup)?
    }
    if let Err(error) = fs::rename(&temp, dest) {
        if backup.exists() {
            let _ = fs::rename(backup, dest);
        }
        return Err(error);
    }
    let _ = fs::remove_file(backup);
    Ok(())
}

pub struct VersionsConfig {
    pub root: PathBuf,
}
pub struct VersionsInput {
    config: VersionsConfig,
    publication: Arc<Mutex<Option<ServicePublication<VersionsService>>>>,
}
pub struct VersionsPlugin;
impl Plugin for VersionsPlugin {
    type Config = VersionsConfig;
    type Input = VersionsInput;
    type PrepareError = std::io::Error;
    type ApplyError = VersionsError;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        "photag.versions".into()
    }
    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        fs::create_dir_all(&config.root)?;
        Ok(VersionsInput {
            config,
            publication: Arc::new(Mutex::new(None)),
        })
    }
    async fn apply(&self, ctx: Context, input: &Self::Input) -> Result<(), Self::ApplyError> {
        let info = ctx
            .try_service::<VersionInfoService>()
            .map_err(|e| VersionsError::Dependency(e.to_string()))?;
        let thumbs = ctx
            .try_service::<ThumbnailService>()
            .map_err(|e| VersionsError::Dependency(e.to_string()))?;
        let publication = ctx
            .provide(Arc::new(VersionsService {
                root: input.config.root.clone(),
                info,
                thumbs,
                locks: Arc::new(Mutex::new(HashMap::new())),
            }))
            .map_err(|e| VersionsError::Publish(e.to_string()))?;
        *input.publication.lock().map_err(|_| VersionsError::Lock)? = Some(publication);
        Ok(())
    }
}
