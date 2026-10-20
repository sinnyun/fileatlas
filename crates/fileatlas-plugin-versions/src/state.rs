//! Compare source bytes with immutable saved revisions, never a staged worktree.
use super::*;
use lore_vm::ops::file::{
    hash::{self, FileHashArgs},
    info::{self as file_info, FileInfoArgs},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileVersionStatus {
    Unversioned,
    Saved,
    MatchesSavedVersion,
    Modified,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileVersionState {
    pub status: FileVersionStatus,
    pub current_version_id: Option<String>,
    pub current_title: Option<String>,
    pub matching_version_id: Option<String>,
    pub matching_title: Option<String>,
}

impl VersionsService {
    /// Read-only content classification. Older selected revisions are valid
    /// baselines; branch.latest and timestamps never decide whether to warn.
    pub async fn file_state(&self, path: &str) -> Result<FileVersionState, VersionsError> {
        let (key, path) = Self::key(Path::new(path))?;
        let _guard = self.lock(&key).await?;
        let records = self.info.list(&key)?;
        let mut state = FileVersionState {
            status: FileVersionStatus::Unversioned,
            current_version_id: None,
            current_title: None,
            matching_version_id: None,
            matching_title: None,
        };
        let (_, work, store) = self.paths(&key);
        if !work.join(".lore").exists() {
            return if records.is_empty() {
                Ok(state)
            } else {
                Err(VersionsError::NotFound)
            };
        }
        let api = self.open(&key, &work, &store).await?;
        let head = VersionsService::status(&api, true)
            .await?
            .revision
            .map(|revision| revision.revision);
        let current = head
            .as_deref()
            .and_then(|id| records.iter().find(|record| record.version_id == id))
            .or_else(|| {
                head.is_none()
                    .then(|| records.iter().find(|record| record.is_current))
                    .flatten()
            });
        if let Some(current) = current {
            state.current_version_id = Some(current.version_id.clone());
            state.current_title = Some(current.title.clone());
        } else if let Some(head) = head.as_deref() {
            // Lore commit is authoritative if the following metadata write failed.
            let info = lore_vm::ops::revision::info::info(
                &api,
                lore_vm::ops::revision::info::RevisionInfoArgs {
                    revision: head.to_owned(),
                    delta: false,
                    metadata: false,
                },
            )
            .await?;
            state.current_version_id = Some(head.to_owned());
            state.current_title = Some(
                info.message()
                    .unwrap_or("Recovered saved version")
                    .to_owned(),
            );
        } else if records.is_empty() {
            return Ok(state);
        } else {
            return Err(VersionsError::NotFound);
        }
        let before = fs::metadata(&path)?;
        // Use the engine's own content hash, including its chunking semantics.
        let source = hash::hash(
            &api,
            FileHashArgs {
                paths: vec![path.to_string_lossy().into_owned()],
            },
        )
        .await?
        .files
        .into_iter()
        .next()
        .ok_or(VersionsError::NotFound)?;
        let current_id = state.current_version_id.as_deref();
        let mut revisions = records
            .iter()
            .map(|record| {
                (
                    record.version_id.as_str(),
                    record.title.as_str(),
                    current_id == Some(record.version_id.as_str()),
                )
            })
            .collect::<Vec<_>>();
        if let Some(head) = head
            .as_deref()
            .filter(|head| !records.iter().any(|record| record.version_id == *head))
        {
            revisions.insert(
                0,
                (
                    head,
                    state
                        .current_title
                        .as_deref()
                        .unwrap_or("Recovered saved version"),
                    true,
                ),
            );
        }
        if let Some(current_id) = current_id {
            revisions.sort_by_key(|(id, _, is_current)| {
                if *is_current || *id == current_id {
                    0
                } else {
                    1
                }
            });
        }
        for (version_id, title, is_current) in revisions {
            let saved = file_info::info(
                &api,
                FileInfoArgs {
                    paths: vec!["content".into()],
                    revision: version_id.to_owned(),
                    local: false,
                    filtered: false,
                },
            )
            .await?
            .entries
            .into_iter()
            .find(|entry| entry.is_file)
            .ok_or(VersionsError::NotFound)?;
            if source.hash == saved.hash && source.size == saved.size {
                state.status = if is_current {
                    FileVersionStatus::Saved
                } else {
                    FileVersionStatus::MatchesSavedVersion
                };
                state.matching_version_id = Some(version_id.to_owned());
                state.matching_title = Some(title.to_owned());
                self.verify_state_read(&path, &before)?;
                return Ok(state);
            }
        }
        self.verify_state_read(&path, &before)?;
        state.status = FileVersionStatus::Modified;
        Ok(state)
    }

    fn verify_state_read(&self, path: &Path, before: &fs::Metadata) -> Result<(), VersionsError> {
        let after = fs::metadata(path)?;
        if before.len() != after.len() || before.modified()? != after.modified()? {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "File changed while checking its saved versions; refresh to retry",
            )
            .into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn distinguishes_unsaved_bytes_from_old_and_other_saved_versions()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!(
            "fileatlas-version-state-{}-{}",
            std::process::id(),
            now().replace(':', "-")
        ));
        fs::create_dir_all(&root)?;
        let file = root.join("photo.png");
        fs::write(&file, b"one")?;
        let path = file.to_string_lossy().into_owned();
        let service = VersionsService {
            root: root.join("versions"),
            info: Arc::new(VersionInfoService::open_at(root.join("info.sqlite"))?),
            thumbs: Arc::new(ThumbnailService::open_at(root.join("thumbs"))?),
            locks: Arc::new(Mutex::new(HashMap::new())),
        };
        assert_eq!(
            service.file_state(&path).await?.status,
            FileVersionStatus::Unversioned
        );
        let locks = service.locks.lock().map_err(|_| "lock poisoned")?;
        assert!(locks.is_empty());
        drop(locks);
        let first = service.create_version(&path, "01").await?;
        assert_eq!(
            service.file_state(&path).await?.status,
            FileVersionStatus::Saved
        );
        fs::write(&file, b"two")?;
        assert_eq!(
            service.file_state(&path).await?.status,
            FileVersionStatus::Modified
        );
        let second = service.create_version(&path, "02").await?;
        service.activate(&path, &first.version_id).await?;
        assert_eq!(
            service.file_state(&path).await?.status,
            FileVersionStatus::Saved
        );
        fs::write(&file, b"one")?; // Timestamp-only change is not unsaved content.
        assert_eq!(
            service.file_state(&path).await?.status,
            FileVersionStatus::Saved
        );
        fs::write(&file, b"two")?;
        let matched = service.file_state(&path).await?;
        assert_eq!(matched.status, FileVersionStatus::MatchesSavedVersion);
        assert_eq!(
            matched.current_version_id.as_deref(),
            Some(first.version_id.as_str())
        );
        assert_eq!(
            matched.matching_version_id.as_deref(),
            Some(second.version_id.as_str())
        );
        fs::write(&file, b"new")?;
        // A failed commit can leave the private worktree holding unsaved bytes.
        // The comparison must still use immutable 01/02, not that staged file.
        let (key, _) = VersionsService::key(&file)?;
        fs::write(service.paths(&key).0, b"new")?;
        assert_eq!(
            service.file_state(&path).await?.status,
            FileVersionStatus::Modified
        );
        fs::remove_file(&file)?;
        assert!(service.file_state(&path).await.is_err());
        let _ = fs::remove_dir_all(root);
        Ok(())
    }

    #[tokio::test]
    async fn recognizes_lore_head_when_metadata_write_was_lost()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!(
            "fileatlas-version-recover-{}-{}",
            std::process::id(),
            now().replace(':', "-")
        ));
        fs::create_dir_all(&root)?;
        let file = root.join("photo.png");
        fs::write(&file, b"committed bytes")?;
        let path = file.to_string_lossy().into_owned();
        let metadata_path = root.join("info.sqlite");
        let service = VersionsService {
            root: root.join("versions"),
            info: Arc::new(VersionInfoService::open_at(&metadata_path)?),
            thumbs: Arc::new(ThumbnailService::open_at(root.join("thumbs"))?),
            locks: Arc::new(Mutex::new(HashMap::new())),
        };
        let saved = service.create_version(&path, "recovered").await?;
        rusqlite::Connection::open(&metadata_path)?.execute("DELETE FROM versions", [])?;
        let state = service.file_state(&path).await?;
        assert_eq!(state.status, FileVersionStatus::Saved);
        assert_eq!(
            state.current_version_id.as_deref(),
            Some(saved.version_id.as_str())
        );
        assert_eq!(state.current_title.as_deref(), Some("recovered"));
        let locks = service.locks.lock().map_err(|_| "lock poisoned")?;
        assert!(locks.is_empty());
        let _ = fs::remove_dir_all(root);
        Ok(())
    }
}
