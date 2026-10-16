//! File-manager-style projection for ungrouped directory views.
use super::*;
use photag_plugin_database::db::queries::Folder;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct GalleryDirectory {
    pub name: String,
    pub path: String,
    pub media_count: i64,
}
#[derive(Default, Clone)]
pub struct BrowserSnapshot {
    pub active: bool,
    pub directory: Option<String>,
    pub parent: Option<String>,
    pub directories: Vec<GalleryDirectory>,
    pub error: Option<String>,
}
impl GalleryService {
    pub(super) fn browser_snapshot(
        &self,
        query: &MediaQuery,
        mode: GroupingMode,
    ) -> Result<BrowserSnapshot, GalleryError> {
        if mode != GroupingMode::None && !self.database.is_browser() {
            return Ok(BrowserSnapshot::default());
        }
        let roots = self.database.with_reader(queries::list_folders)?;
        let directory = query.path_prefix.clone().or_else(|| {
            query
                .folder_id
                .and_then(|id| roots.iter().find(|f| f.id == id).map(|f| f.path.clone()))
        });
        let directory = if self.database.is_browser() {
            directory.map(|path| {
                std::fs::canonicalize(&path)
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or(path)
            })
        } else {
            directory
        };
        if self.database.is_browser() && directory.is_none() && self.live.reset() {
            if let Some(watcher) = &self.watcher {
                watcher.set_browse_directory(None, false);
            }
        }
        // Global search/tag views retain the complete matching file result.
        if directory.is_none()
            && (query.search.as_ref().is_some_and(|s| !s.trim().is_empty())
                || query.tag_ids.as_ref().is_some_and(|v| !v.is_empty())
                || query
                    .excluded_tag_ids
                    .as_ref()
                    .is_some_and(|v| !v.is_empty()))
        {
            return Ok(BrowserSnapshot::default());
        }
        let mut snapshot = BrowserSnapshot {
            active: true,
            directory,
            ..Default::default()
        };
        if let Some(path) = &snapshot.directory {
            if self.database.is_browser() {
                self.live
                    .ensure(path, &self.database, self.watcher.as_ref(), &self.wake);
                snapshot.parent = Path::new(path)
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .map(|p| p.to_string_lossy().into_owned());
                snapshot.directories = self.live.directories();
                snapshot.error = self.live.status().4;
            } else {
                snapshot.parent = parent_in_library(path, &roots);
                match self
                    .indexing
                    .as_ref()
                    .ok_or_else(|| "Directory listing service is unavailable".to_string())
                    .and_then(|indexing| {
                        indexing
                            .try_list_subdirectories(path)
                            .map_err(|error| format!("{path}: {error}"))
                    }) {
                    Ok(directories) => {
                        snapshot.directories = directories
                            .into_iter()
                            .map(|(name, path, _)| GalleryDirectory {
                                name,
                                path,
                                media_count: 0,
                            })
                            .collect()
                    }
                    Err(error) => snapshot.error = Some(error),
                }
            }
        } else {
            snapshot.directories = roots
                .into_iter()
                .map(|f| GalleryDirectory {
                    name: f.name,
                    path: f.path,
                    media_count: 0,
                })
                .collect();
        }
        if let Some(search) = query.search.as_ref().filter(|s| !s.trim().is_empty()) {
            let search = search.trim().to_lowercase();
            snapshot.directories.retain(|f| {
                (query.search_in_filenames && f.name.to_lowercase().contains(&search))
                    || (query.search_in_paths && f.path.to_lowercase().contains(&search))
            });
        }
        snapshot
            .directories
            .sort_by_cached_key(|entry| (entry.name.to_lowercase(), entry.path.clone()));
        if query.sort == Some(SortOrder::NameDesc) {
            snapshot.directories.reverse();
        }
        Ok(snapshot)
    }
}
fn parent_in_library(directory: &str, roots: &[Folder]) -> Option<String> {
    let parent = Path::new(directory).parent()?;
    roots
        .iter()
        .any(|root| parent.starts_with(&root.path))
        .then(|| parent.to_string_lossy().into_owned())
}
