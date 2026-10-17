//! Materialize Everything results without enumerating their parent directories.
use super::*;
use photag_plugin_database::BrowserFile;
use std::path::Path;
use std::time::UNIX_EPOCH;

impl GalleryService {
    /// Search scope is independent of the normal direct/recursive browse toggle.
    pub(super) fn scope_text_search(&self, query: &mut MediaQuery) -> Result<(), GalleryError> {
        if query
            .search
            .as_deref()
            .is_none_or(|text| text.trim().is_empty())
        {
            return Ok(());
        }
        let directory = if query.search_entire_computer {
            None
        } else {
            query
                .path_prefix
                .clone()
                .or(query.directory_tree.clone())
                .or(query.direct_directory.clone())
                .or(self
                    .database
                    .list_folders()?
                    .into_iter()
                    .find(|folder| Some(folder.id) == query.folder_id)
                    .map(|folder| folder.path))
        };
        query.directory_tree = directory.map(|path| {
            std::fs::canonicalize(&path)
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or(path)
        });
        query.folder_id = None;
        query.path_prefix = None;
        query.direct_directory = None;
        query.local_only = false;
        Ok(())
    }

    pub(super) fn cache_everything_paths(
        &self,
        paths: Vec<String>,
    ) -> Result<Vec<String>, GalleryError> {
        if !self.database.is_browser() {
            return Ok(paths);
        }
        let files = paths
            .into_iter()
            .filter_map(|path| {
                let metadata = match std::fs::metadata(&path) {
                    Ok(metadata) if metadata.is_file() => Some(metadata),
                    Ok(_) => return None,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
                    Err(_) => None,
                };
                let path = std::fs::canonicalize(&path)
                    .map(|path| path.to_string_lossy().into_owned())
                    .unwrap_or(path);
                let filename = Path::new(&path).file_name()?.to_string_lossy().into_owned();
                let extension = Path::new(&path)
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy();
                let media_type = photag_plugin_indexing::classify_media_type(&extension).to_owned();
                let modified = metadata
                    .as_ref()
                    .and_then(|m| m.modified().ok())
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .unwrap_or_default();
                let size = metadata.as_ref().map_or(0, std::fs::Metadata::len);
                let version = metadata.as_ref().map_or_else(
                    || "unreadable".to_owned(),
                    |_| format!("{}:{}", size, modified.as_nanos()),
                );
                Some(BrowserFile {
                    path,
                    filename,
                    media_type,
                    file_size: size as i64,
                    modified_at: modified.as_secs() as i64,
                    version,
                })
            })
            .collect::<Vec<_>>();
        Ok(self.database.cache_search_files(&files)?)
    }
}
