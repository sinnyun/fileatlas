use super::*;

/// The saved native document's identity on disk. A preview catalogue records the stamp it
/// was built from, so an external save by the official program is detectable with a single
/// stat and never needs a running process.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentStamp {
    pub modified_ns: u64,
    pub len: u64,
}

/// What the panel needs to decide whether the visible catalogue still describes the file.
#[derive(Clone, Debug, Serialize)]
pub struct PreviewState {
    pub project_id: String,
    pub stamp: Option<DocumentStamp>,
    pub cache_stamp: Option<DocumentStamp>,
}

impl PreviewState {
    pub fn stale(&self) -> bool {
        self.stamp.is_some() && self.cache_stamp != self.stamp
    }
}

/// One background catalogue refresh. `synced == false` means the cache already described the
/// saved document, so no official process was started.
#[derive(Clone, Debug)]
pub struct AutoSync {
    pub synced: bool,
    pub boards: Vec<WorkCrateBoard>,
}

pub(crate) fn stamp_of(path: &Path) -> Option<DocumentStamp> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    Some(DocumentStamp {
        modified_ns: metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|elapsed| elapsed.as_nanos() as u64)
            .unwrap_or_default(),
        len: metadata.len(),
    })
}

impl WorkCrateBoardsService {
    /// A stat plus the cached source stamp; no official session and no preview decode.
    pub fn preview_state(&self, project: &str) -> Result<PreviewState, BoardsError> {
        let Some(path) = self.document_path(project)? else {
            return Ok(PreviewState {
                project_id: project.into(),
                stamp: None,
                cache_stamp: None,
            });
        };
        let stamp = stamp_of(&path);
        let cache_stamp = self.read_cache(project)?.and_then(|cache| cache.source);
        Ok(PreviewState {
            project_id: project.into(),
            stamp,
            cache_stamp,
        })
    }

    /// Rebuild the catalogue when the saved document changed since the cache was written.
    /// This never drives the user's GUI: a passive refresh reads the saved file through the
    /// silent official session, so an open VectorCraft window keeps its focus, document and
    /// unsaved state.
    pub fn auto_refresh_project(&self, project: &str) -> Result<AutoSync, BoardsError> {
        let _gate = self.gate.lock().map_err(|_| BoardsError::Poisoned)?;
        let path = self
            .document_path(project)?
            .ok_or(BoardsError::NoDocument)?;
        let Some(source) = stamp_of(&path) else {
            return Err(BoardsError::NoDocument);
        };
        let cached = self.read_cache(project)?;
        if let Some(cache) = &cached
            && cache.source == Some(source)
        {
            return Ok(AutoSync {
                synced: false,
                boards: cache.boards.clone(),
            });
        }
        let binding = self.projects.document_binding(project)?;
        let client = self.headless_client(std::slice::from_ref(&path), &[])?;
        let boards = self.rebuild_cache(project, &path, binding, Some(source), client)?;
        Ok(AutoSync {
            synced: true,
            boards,
        })
    }

    /// Read one native document with a ready-made official session and publish the whole
    /// catalogue. The source stamp is the one observed before the session opened: a file
    /// changing during the read stays stale and is picked up by the next sample.
    pub(super) fn rebuild_cache(
        &self,
        project: &str,
        path: &Path,
        binding: String,
        source: Option<DocumentStamp>,
        mut client: mcp::McpSession,
    ) -> Result<Vec<WorkCrateBoard>, BoardsError> {
        client.open_document(path)?;
        let boards = native::catalog(&mut client, project, &binding)?;
        let previews = self.make_previews(&mut client, &boards)?;
        self.write_cache(
            project,
            &BoardCache {
                version: 1,
                binding,
                boards: boards.clone(),
                previews,
                source,
            },
        )?;
        Ok(boards)
    }

    pub(super) fn source_stamp(&self, project: &str) -> Option<DocumentStamp> {
        self.document_path(project)
            .ok()
            .flatten()
            .and_then(|path| stamp_of(&path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamps_track_the_saved_file_and_ignore_missing_paths() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("page.vectorcraft");
        assert_eq!(stamp_of(&path), None);
        std::fs::write(&path, b"one").expect("write");
        let first = stamp_of(&path).expect("a written file has a stamp");
        assert_eq!(first.len, 3);
        std::fs::write(&path, b"a longer document").expect("rewrite");
        let second = stamp_of(&path).expect("a rewritten file has a stamp");
        assert_ne!(first, second);
        std::fs::remove_file(&path).expect("remove");
        assert_eq!(stamp_of(&path), None);
    }

    #[test]
    fn staleness_only_applies_to_a_present_document() {
        let stamp = DocumentStamp {
            modified_ns: 7,
            len: 3,
        };
        let state = |stamp: Option<DocumentStamp>, cache_stamp: Option<DocumentStamp>| {
            PreviewState {
                project_id: "p".into(),
                stamp,
                cache_stamp,
            }
            .stale()
        };
        assert!(!state(None, None));
        assert!(!state(None, Some(stamp)));
        assert!(state(Some(stamp), None));
        assert!(!state(Some(stamp), Some(stamp)));
        assert!(state(Some(stamp), Some(DocumentStamp { len: 4, ..stamp })));
    }
}
