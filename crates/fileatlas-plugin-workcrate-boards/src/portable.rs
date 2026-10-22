use super::*;

impl WorkCrateBoardsService {
    pub(super) fn resolve_cache_paths(
        &self,
        project: &str,
        cache: &mut BoardCache,
    ) -> Result<(), BoardsError> {
        if let Some(folder) = self.projects.portable_folder(project)? {
            for board in &mut cache.boards {
                if let Some(path) = &mut board.info.first_image {
                    if !Path::new(path).is_absolute() {
                        *path = folder.join(&*path).to_string_lossy().into_owned();
                    }
                }
            }
        }
        Ok(())
    }
    pub(super) fn store_cache_paths(
        &self,
        project: &str,
        cache: &mut BoardCache,
    ) -> Result<(), BoardsError> {
        if let Some(folder) = self.projects.portable_folder(project)? {
            for board in &mut cache.boards {
                if let Some(path) = &mut board.info.first_image {
                    if let Ok(relative) = Path::new(&*path).strip_prefix(&folder) {
                        *path = relative.to_string_lossy().replace('\\', "/");
                    }
                }
            }
        }
        Ok(())
    }
    /// Reconnect moved metadata without trying to save back to its now-missing old folder.
    pub fn open_project_folder(
        &self,
        folder: &Path,
    ) -> Result<Vec<fileatlas_workcrate_contracts::WorkCrateProject>, BoardsError> {
        let _gate = self.gate.lock().map_err(|_| BoardsError::Poisoned)?;
        let mut canvas = self.canvas.lock().map_err(|_| BoardsError::Poisoned)?;
        if let Some(state) = canvas.as_mut() {
            state.prepare_relocation(None)?;
        }
        drop(canvas.take());
        drop(canvas);
        Ok(self.projects.open_project_folder(folder)?)
    }
    /// Relocate only FileAtlas metadata/cache. Native resources remain VectorCraft-owned.
    /// A cold migration opens no process and never rewrites the native document.
    pub fn relocate_project(&self, project: &str) -> Result<PathBuf, BoardsError> {
        let _gate = self.gate.lock().map_err(|_| BoardsError::Poisoned)?;
        let mut canvas = self.canvas.lock().map_err(|_| BoardsError::Poisoned)?;
        if let Some(state) = canvas.as_mut() {
            if state.prepare_relocation(Some(project))? {
                drop(canvas.take());
            }
        }
        drop(canvas);
        let folder = self.projects.relocate_project_data(project)?;
        if let Some(cache) = self.read_cache(project)? {
            self.write_cache(project, &cache)?;
        }
        Ok(folder)
    }
}
