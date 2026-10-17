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
        if records.is_empty() {
            return Ok(state);
        }
        let current = records
            .iter()
            .find(|record| record.is_current)
            .ok_or(VersionsError::NotFound)?;
        state.current_version_id = Some(current.version_id.clone());
        state.current_title = Some(current.title.clone());
        let (_, work, store) = self.paths(&key);
        if !work.join(".lore").exists() {
            return Err(VersionsError::NotFound);
        }
        let api = self.open(&key, &work, &store).await?;
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
        for record in
            std::iter::once(current).chain(records.iter().filter(|record| !record.is_current))
        {
            let saved = file_info::info(
                &api,
                FileInfoArgs {
                    paths: vec!["content".into()],
                    revision: record.version_id.clone(),
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
                state.status = if record.is_current {
                    FileVersionStatus::Saved
                } else {
                    FileVersionStatus::MatchesSavedVersion
                };
                state.matching_version_id = Some(record.version_id.clone());
                state.matching_title = Some(record.title.clone());
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
            "photag-version-state-{}-{}",
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
            locks: Mutex::new(HashMap::new()),
        };
        assert_eq!(
            service.file_state(&path).await?.status,
            FileVersionStatus::Unversioned
        );
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
}
