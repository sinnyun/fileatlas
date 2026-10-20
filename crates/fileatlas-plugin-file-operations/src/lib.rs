//! FileAtlas file operations behind a Cordis service, including source-compatible
//! DB follow-through, cross-volume moves, and Recycle Bin deletion.

use std::path::{Path, PathBuf};
#[cfg(any(target_os = "windows", target_os = "macos"))]
use std::process::Command;
use std::sync::Arc;

use cordis::{Context, Routing, Service};
use fileatlas_contracts::MediaIndexChanged;
use fileatlas_plugin_database::DatabaseService;
use fileatlas_plugin_database::{Folder, MediaItem};

mod mcp;
mod model;
mod plugin;

pub use model::{
    BulkCopyOutcome, BulkDeleteOutcome, BulkMoveOutcome, DeleteResult, FileOperationError,
    PathOperationOutcome,
};
pub use plugin::{FileOperationsConfig, FileOperationsPlugin, FileOperationsPluginError};

pub struct FileOperationsService {
    database: Arc<DatabaseService>,
    cordis: Context,
}

impl Service for FileOperationsService {
    const NAME: &'static str = "fileatlas/file-operations";
}

impl FileOperationsService {
    pub fn new(database: Arc<DatabaseService>, cordis: Context) -> Self {
        Self { database, cordis }
    }

    /// Copy staged arbitrary files into a destination folder without replacing
    /// existing files. Indexed sources use their current database path.
    pub async fn copy_paths_to_folder(
        self: &Arc<Self>,
        paths: Vec<PathBuf>,
        destination_dir: PathBuf,
    ) -> Result<Vec<PathOperationOutcome>, FileOperationError> {
        let service = Arc::clone(self);
        Ok(tokio::task::spawn_blocking(move || {
            paths
                .into_iter()
                .map(|source| {
                    let result = (|| {
                        let source = service.current_source_path(&source)?;
                        let name = source
                            .file_name()
                            .ok_or(FileOperationError::InvalidParent)?;
                        let destination = destination_dir.join(name);
                        if destination.exists() {
                            return Ok(false);
                        }
                        copy_new_file(&source, &destination)?;
                        Ok(true)
                    })();
                    model::path_outcome(source, result)
                })
                .collect()
        })
        .await?)
    }

    /// Move staged arbitrary files through the existing file-operation service.
    /// Known indexed rows receive the same path/database follow-through as the
    /// normal FileManager move; unindexed files are observed by Watcher if needed.
    pub async fn move_paths_to_folder(
        self: &Arc<Self>,
        paths: Vec<PathBuf>,
        destination_dir: PathBuf,
    ) -> Result<Vec<PathOperationOutcome>, FileOperationError> {
        let service = Arc::clone(self);
        let (outcomes, changed) = tokio::task::spawn_blocking(move || {
            let mut changed = false;
            let outcomes = paths
                .into_iter()
                .map(|source| {
                    let result = (|| {
                        let source = service.current_source_path(&source)?;
                        let name = source
                            .file_name()
                            .ok_or(FileOperationError::InvalidParent)?
                            .to_owned();
                        let destination = destination_dir.join(name);
                        if destination.exists() {
                            return Ok(false);
                        }
                        if let Some(id) = service
                            .database
                            .ids_for_paths(&path_candidates(&source))?
                            .first()
                            .copied()
                        {
                            let moved =
                                service.move_into_folder_blocking(id, destination_dir.clone())?;
                            changed |= moved;
                            Ok(moved)
                        } else {
                            copy_new_file(&source, &destination)?;
                            std::fs::remove_file(&source)?;
                            Ok(true)
                        }
                    })();
                    model::path_outcome(source, result)
                })
                .collect();
            (outcomes, changed)
        })
        .await?;
        if changed {
            self.emit_index_changed().await?;
        }
        Ok(outcomes)
    }

    fn current_source_path(&self, source: &Path) -> Result<PathBuf, FileOperationError> {
        let id = self
            .database
            .ids_for_paths(&path_candidates(source))?
            .first()
            .copied();
        if let Some(id) = id {
            let indexed = self
                .media(id)?
                .map(|item| PathBuf::from(item.full_path))
                .ok_or(FileOperationError::MissingMedia(id));
            let indexed = indexed?;
            if !indexed.is_file() {
                return Err(FileOperationError::MissingMedia(id));
            }
            return Ok(indexed);
        }
        let source = source.canonicalize()?;
        if !source.is_file() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "source is not a file",
            )
            .into());
        }
        Ok(source)
    }

    #[tracing::instrument(skip_all, err)]
    pub async fn copy_to(
        self: &Arc<Self>,
        media_id: i64,
        destination: PathBuf,
    ) -> Result<u64, FileOperationError> {
        let service = Arc::clone(self);
        Ok(tokio::task::spawn_blocking(move || {
            let item = service
                .media(media_id)?
                .ok_or(FileOperationError::MissingMedia(media_id))?;
            Ok::<_, FileOperationError>(std::fs::copy(item.full_path, destination)?)
        })
        .await??)
    }

    /// Copy many items into a chosen directory. The source PhoTag bulk-copy
    /// action leaves indexing of in-library destinations to the watcher.
    #[tracing::instrument(skip_all, err)]
    pub async fn bulk_copy_to_folder(
        self: &Arc<Self>,
        media_ids: Vec<i64>,
        destination_dir: PathBuf,
    ) -> Result<Vec<BulkCopyOutcome>, FileOperationError> {
        let service = Arc::clone(self);
        tokio::task::spawn_blocking(move || {
            media_ids
                .into_iter()
                .map(|media_id| {
                    let result = (|| {
                        let item = service
                            .media(media_id)?
                            .ok_or(FileOperationError::MissingMedia(media_id))?;
                        std::fs::copy(&item.full_path, destination_dir.join(&item.filename))?;
                        Ok::<_, FileOperationError>(())
                    })();
                    match result {
                        Ok(()) => BulkCopyOutcome {
                            media_id,
                            copied: true,
                            error: None,
                        },
                        Err(error) => BulkCopyOutcome {
                            media_id,
                            copied: false,
                            error: Some(error.to_string()),
                        },
                    }
                })
                .collect()
        })
        .await
        .map_err(FileOperationError::from)
    }

    /// Move a file to an arbitrary save-dialog destination. Like FileAtlas, an
    /// item moved outside indexed library roots is removed from the DB.
    #[tracing::instrument(skip_all, err)]
    pub async fn move_to(
        self: &Arc<Self>,
        media_id: i64,
        destination: PathBuf,
    ) -> Result<(), FileOperationError> {
        self.move_to_with_outcome(media_id, destination)
            .await
            .map(|_| ())
    }

    /// Move to a caller-selected full path and report whether the file moved.
    /// `false` covers source-compatible collision or filesystem move failure.
    #[tracing::instrument(skip_all, err)]
    pub async fn move_to_with_outcome(
        self: &Arc<Self>,
        media_id: i64,
        destination: PathBuf,
    ) -> Result<bool, FileOperationError> {
        let service = Arc::clone(self);
        let changed =
            tokio::task::spawn_blocking(move || service.move_to_blocking(media_id, destination))
                .await??;
        if changed {
            self.emit_index_changed().await?;
        }
        Ok(changed)
    }

    /// Move into a registered library folder, keeping the existing name and
    /// rejecting destination collisions as the original app does.
    pub async fn move_into_folder(
        self: &Arc<Self>,
        media_id: i64,
        destination_dir: PathBuf,
    ) -> Result<(), FileOperationError> {
        self.move_into_folder_with_outcome(media_id, destination_dir)
            .await
            .map(|_| ())
    }

    /// Move into a registered folder and report collision or move failure.
    #[tracing::instrument(skip_all, err)]
    pub async fn move_into_folder_with_outcome(
        self: &Arc<Self>,
        media_id: i64,
        destination_dir: PathBuf,
    ) -> Result<bool, FileOperationError> {
        let service = Arc::clone(self);
        let changed = tokio::task::spawn_blocking(move || {
            service.move_into_folder_blocking(media_id, destination_dir)
        })
        .await??;
        if changed {
            self.emit_index_changed().await?;
        }
        Ok(changed)
    }

    /// Move many items to a folder, preserving FileAtlas's per-item collision and
    /// cross-volume behavior. Failures are recorded per item and do not stop
    /// later items, as in the original bulk action.
    #[tracing::instrument(skip_all, err)]
    pub async fn bulk_move_into_folder(
        self: &Arc<Self>,
        media_ids: Vec<i64>,
        destination_dir: PathBuf,
    ) -> Result<Vec<BulkMoveOutcome>, FileOperationError> {
        let service = Arc::clone(self);
        let outcomes = tokio::task::spawn_blocking(move || {
            media_ids
                .into_iter()
                .map(|media_id| {
                    let result =
                        service.move_into_folder_blocking(media_id, destination_dir.clone());
                    match result {
                        Ok(true) => BulkMoveOutcome {
                            media_id,
                            moved: true,
                            error: None,
                        },
                        Ok(false) => BulkMoveOutcome {
                            media_id,
                            moved: false,
                            error: None,
                        },
                        Err(error) => BulkMoveOutcome {
                            media_id,
                            moved: false,
                            error: Some(error.to_string()),
                        },
                    }
                })
                .collect::<Vec<_>>()
        })
        .await?;
        if outcomes.iter().any(|outcome| outcome.moved) {
            self.emit_index_changed().await?;
        }
        Ok(outcomes)
    }

    #[tracing::instrument(skip_all, err)]
    pub async fn rename(
        self: &Arc<Self>,
        media_id: i64,
        new_filename: String,
    ) -> Result<(), FileOperationError> {
        self.rename_with_outcome(media_id, new_filename)
            .await
            .map(|_| ())
    }

    /// Rename an item and report a no-op such as an unchanged or colliding name.
    #[tracing::instrument(skip_all, err)]
    pub async fn rename_with_outcome(
        self: &Arc<Self>,
        media_id: i64,
        new_filename: String,
    ) -> Result<bool, FileOperationError> {
        let service = Arc::clone(self);
        let changed =
            tokio::task::spawn_blocking(move || service.rename_blocking(media_id, new_filename))
                .await??;
        if changed {
            self.emit_index_changed().await?;
        }
        Ok(changed)
    }

    #[tracing::instrument(skip_all, err)]
    pub async fn create_folder(
        self: &Arc<Self>,
        parent: PathBuf,
        name: String,
    ) -> Result<bool, FileOperationError> {
        Ok(tokio::task::spawn_blocking(move || {
            let name = name.trim();
            if name.is_empty() || name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
                return Ok::<bool, FileOperationError>(false);
            }
            let target = parent.join(name);
            if target.exists() {
                return Ok(false);
            }
            std::fs::create_dir(target)?;
            Ok::<bool, FileOperationError>(true)
        })
        .await??)
    }

    /// Send a file to the OS Recycle Bin, then remove its FileAtlas row even if
    /// the recycle operation fails, matching the original single-delete path.
    #[tracing::instrument(skip_all, err)]
    pub async fn delete_to_recycle_bin(
        self: &Arc<Self>,
        media_id: i64,
    ) -> Result<DeleteResult, FileOperationError> {
        let service = Arc::clone(self);
        let result =
            tokio::task::spawn_blocking(move || service.delete_blocking(media_id)).await??;
        if result.removed_from_database {
            self.emit_index_changed().await?;
        }
        Ok(result)
    }

    /// Read an indexed media row after an operation returns an error. Event
    /// delivery can fail after the filesystem and database mutation committed.
    pub fn media_by_id(&self, media_id: i64) -> Result<Option<MediaItem>, FileOperationError> {
        self.media(media_id)
    }

    /// Bulk delete follows FileAtlas's best-effort contract: trash every
    /// available path, clean thumbnails, remove indexed rows and emit one
    /// refresh event. A broken item does not stop the rest of the batch.
    #[tracing::instrument(skip_all, err)]
    pub async fn bulk_delete_to_recycle_bin(
        self: &Arc<Self>,
        media_ids: Vec<i64>,
    ) -> Result<Vec<BulkDeleteOutcome>, FileOperationError> {
        let service = Arc::clone(self);
        let outcomes = tokio::task::spawn_blocking(move || {
            let items = media_ids
                .iter()
                .map(|&id| (id, service.media(id)))
                .collect::<Vec<_>>();
            let mut outcomes = Vec::with_capacity(items.len());
            let mut changed = false;
            for (media_id, item_result) in items {
                let outcome = match item_result {
                    Err(error) => BulkDeleteOutcome {
                        media_id,
                        removed_from_database: false,
                        recycled: false,
                        error: Some(error.to_string()),
                    },
                    Ok(None) => BulkDeleteOutcome {
                        media_id,
                        removed_from_database: false,
                        recycled: false,
                        error: None,
                    },
                    Ok(Some(item)) => {
                        let recycle_error = trash::delete(&item.full_path)
                            .err()
                            .map(|error| error.to_string());
                        if let Some(path) = &item.thumb_path {
                            std::fs::remove_file(path).ok();
                        }
                        match service.database.delete_media(media_id) {
                            Ok(()) => {
                                changed = true;
                                BulkDeleteOutcome {
                                    media_id,
                                    removed_from_database: true,
                                    recycled: recycle_error.is_none(),
                                    error: recycle_error,
                                }
                            }
                            Err(error) => BulkDeleteOutcome {
                                media_id,
                                removed_from_database: false,
                                recycled: recycle_error.is_none(),
                                error: Some(error.to_string()),
                            },
                        }
                    }
                };
                outcomes.push(outcome);
            }
            (outcomes, changed)
        })
        .await?;
        if outcomes.1 {
            self.emit_index_changed().await?;
        }
        Ok(outcomes.0)
    }

    #[tracing::instrument(skip_all, err)]
    pub async fn open_external(self: &Arc<Self>, media_id: i64) -> Result<(), FileOperationError> {
        let service = Arc::clone(self);
        tokio::task::spawn_blocking(move || {
            let item = service
                .media(media_id)?
                .ok_or(FileOperationError::MissingMedia(media_id))?;
            open::that(item.full_path)
                .map_err(|error| FileOperationError::Open(error.to_string()))?;
            Ok::<(), FileOperationError>(())
        })
        .await??;
        Ok(())
    }

    #[tracing::instrument(skip_all, err)]
    pub async fn reveal_in_file_manager(
        self: &Arc<Self>,
        media_id: i64,
    ) -> Result<(), FileOperationError> {
        let service = Arc::clone(self);
        tokio::task::spawn_blocking(move || {
            let item = service
                .media(media_id)?
                .ok_or(FileOperationError::MissingMedia(media_id))?;
            reveal_path(&item.full_path)
        })
        .await??;
        Ok(())
    }

    fn move_to_blocking(
        &self,
        media_id: i64,
        destination: PathBuf,
    ) -> Result<bool, FileOperationError> {
        let item = self
            .media(media_id)?
            .ok_or(FileOperationError::MissingMedia(media_id))?;
        if !move_file(&item.full_path, &destination) {
            return Ok(false);
        }
        let dest = destination.to_string_lossy().into_owned();
        let filename = destination
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| item.filename.clone());
        let folder_id = self.folder_for_path(&dest)?;
        if folder_id.is_some() || self.database.is_browser() {
            self.database
                .update_media_path(media_id, &filename, &dest, folder_id)?;
        } else {
            self.database.delete_media(media_id)?;
        }
        Ok(true)
    }

    fn move_into_folder_blocking(
        &self,
        media_id: i64,
        destination_dir: PathBuf,
    ) -> Result<bool, FileOperationError> {
        let item = self
            .media(media_id)?
            .ok_or(FileOperationError::MissingMedia(media_id))?;
        let destination = destination_dir.join(&item.filename);
        if destination.exists() {
            return Ok(false);
        }
        let dest = destination.to_string_lossy().into_owned();
        if !move_file(&item.full_path, &destination) {
            return Ok(false);
        }
        let folder_id = self.folder_for_path(&dest)?;
        if folder_id.is_some() || self.database.is_browser() {
            self.database
                .update_media_path(media_id, &item.filename, &dest, folder_id)?;
        } else {
            // FileAtlas removes the indexed row when a bulk move leaves all
            // registered library roots. Keep the moved file on disk, but
            // stop presenting it as library media.
            self.database.delete_media(media_id)?;
        }
        Ok(true)
    }

    fn rename_blocking(
        &self,
        media_id: i64,
        new_filename: String,
    ) -> Result<bool, FileOperationError> {
        let new_filename = new_filename.trim();
        if new_filename.is_empty() {
            return Ok(false);
        }
        let item = self
            .media(media_id)?
            .ok_or(FileOperationError::MissingMedia(media_id))?;
        if new_filename == item.filename {
            return Ok(false);
        }
        let parent = Path::new(&item.full_path)
            .parent()
            .ok_or(FileOperationError::InvalidParent)?;
        let destination = parent.join(new_filename);
        if destination.exists() {
            return Ok(false);
        }
        std::fs::rename(&item.full_path, &destination)?;
        let path = destination.to_string_lossy().into_owned();
        self.database
            .update_media_path(media_id, new_filename, &path, None)?;
        Ok(true)
    }

    fn delete_blocking(&self, media_id: i64) -> Result<DeleteResult, FileOperationError> {
        let Some(item) = self.media(media_id)? else {
            return Ok(DeleteResult::not_found());
        };
        let recycle_error = trash::delete(&item.full_path)
            .err()
            .map(|error| error.to_string());
        if let Some(path) = &item.thumb_path {
            std::fs::remove_file(path).ok();
        }
        if self.database.is_browser() {
            if recycle_error.is_none() {
                self.database.remove_live_path(&item.full_path)?;
            } else {
                return Ok(DeleteResult {
                    removed_from_database: false,
                    recycled: false,
                    recycle_error,
                });
            }
        } else {
            self.database.delete_media(media_id)?;
        }
        Ok(DeleteResult {
            removed_from_database: true,
            recycled: recycle_error.is_none(),
            recycle_error,
        })
    }

    fn media(&self, media_id: i64) -> Result<Option<MediaItem>, FileOperationError> {
        let item = self.database.get_media_by_id(media_id)?;
        if self.database.is_browser() {
            if let Some(item) = &item {
                std::fs::metadata(&item.full_path)?;
            }
        }
        Ok(item)
    }

    fn folder_for_path(&self, path: &str) -> Result<Option<i64>, FileOperationError> {
        if self.database.is_browser() {
            return Ok(Some(0));
        }
        let folders = self.database.list_folders()?;
        Ok(registered_folder_for_path(&folders, path))
    }

    async fn emit_index_changed(&self) -> Result<(), FileOperationError> {
        self.cordis
            .emit::<MediaIndexChanged>(Routing::Unscoped, ())
            .await
            .map_err(|error| FileOperationError::Event(error.to_string()))
    }
}

fn copy_new_file(source: &Path, destination: &Path) -> Result<(), FileOperationError> {
    use std::io::Write;
    let mut input = std::fs::File::open(source)?;
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    let result = (|| {
        std::io::copy(&mut input, &mut output)?;
        output.flush()?;
        Ok::<_, std::io::Error>(())
    })();
    drop(output);
    if let Err(error) = result {
        let _ = std::fs::remove_file(destination);
        return Err(error.into());
    }
    Ok(())
}

fn path_candidates(path: &Path) -> Vec<String> {
    let mut values = Vec::new();
    let mut push = |value: String| {
        if !values
            .iter()
            .any(|known: &String| known.eq_ignore_ascii_case(&value))
        {
            values.push(value);
        }
    };
    let raw = path.to_string_lossy().into_owned();
    for value in [
        Some(raw),
        path.canonicalize()
            .ok()
            .map(|p| p.to_string_lossy().into_owned()),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(unc) = value.strip_prefix(r"\\?\UNC\") {
            push(format!(r"\\{unc}"));
        } else if let Some(normal) = value.strip_prefix(r"\\?\") {
            push(normal.to_owned());
        } else {
            push(value.clone());
        }
        if value.starts_with(r"\\") && !value.starts_with(r"\\?\") {
            push(format!(r"\\?\UNC\{}", value.trim_start_matches('\\')));
        } else if value.len() >= 3
            && value.as_bytes().get(1) == Some(&b':')
            && !value.starts_with(r"\\?\")
        {
            push(format!(r"\\?\{value}"));
        }
    }
    values
}

/// FileAtlas's registered-root matching uses string-prefix comparison; retain the
/// same behavior so moved rows attach to the same root as the original app.
pub fn registered_folder_for_path(folders: &[Folder], path: &str) -> Option<i64> {
    folders
        .iter()
        .find(|folder| path.starts_with(&folder.path))
        .map(|folder| folder.id)
}

fn move_file(source: &str, destination: &Path) -> bool {
    match std::fs::rename(source, destination) {
        Ok(()) => true,
        Err(_) => {
            let source_path = Path::new(source);
            copy_new_file(source_path, destination).is_ok()
                && std::fs::remove_file(source_path).is_ok()
        }
    }
}

fn reveal_path(path: &str) -> Result<(), FileOperationError> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer.exe")
            .args(["/select,", path])
            .spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        let parent = Path::new(path)
            .parent()
            .ok_or(FileOperationError::InvalidParent)?;
        open::that(parent)?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").args(["-R", path]).spawn()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (PathBuf, Arc<FileOperationsService>, i64) {
        // Windows SystemTime granularity is ~100 ns, so parallel tests can read
        // the same timestamp and share a root; the counter keeps each one unique.
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "fileatlas-file-ops-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let library = root.join("library");
        std::fs::create_dir_all(&library).unwrap();
        let source = library.join("photo.jpg");
        std::fs::write(&source, b"temporary test media").unwrap();
        let database = Arc::new(DatabaseService::open_at(root.join("fileatlas.sqlite")).unwrap());
        let folder = database
            .add_folder(&library.to_string_lossy())
            .unwrap_or_else(|error| panic!("{error}"));
        let media_id = database
            .insert_media(
                folder.id,
                "photo.jpg",
                &source.to_string_lossy(),
                "image",
                20,
                1,
                None,
                None,
                None,
            )
            .unwrap_or_else(|error| panic!("{error}"));
        (
            root,
            Arc::new(FileOperationsService::new(database, Context::new())),
            media_id,
        )
    }

    #[test]
    fn rename_updates_the_file_and_indexed_path() {
        let (root, service, media_id) = setup();
        assert!(
            service
                .rename_blocking(media_id, "renamed.jpg".into())
                .unwrap()
        );
        let row = service.media(media_id).unwrap().unwrap();
        assert_eq!(row.filename, "renamed.jpg");
        assert!(Path::new(&row.full_path).is_file());
        assert!(!root.join("library/photo.jpg").exists());
        drop(service);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn move_within_registered_library_updates_index_and_external_move_removes_row() {
        let (root, service, media_id) = setup();
        let archive = root.join("library/archive");
        std::fs::create_dir_all(&archive).unwrap();
        assert!(
            service
                .move_into_folder_blocking(media_id, archive.clone())
                .unwrap()
        );
        let moved = archive.join("photo.jpg");
        assert!(moved.is_file());
        assert_eq!(
            service.media(media_id).unwrap().unwrap().full_path,
            moved.to_string_lossy()
        );

        let outside = root.join("export/photo.jpg");
        std::fs::create_dir_all(outside.parent().unwrap()).unwrap();
        assert!(service.move_to_blocking(media_id, outside.clone()).unwrap());
        assert!(outside.is_file());
        assert!(service.media(media_id).unwrap().is_none());
        drop(service);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn destination_collision_leaves_source_and_database_unchanged() {
        let (root, service, media_id) = setup();
        let destination = root.join("library/archive");
        std::fs::create_dir_all(&destination).unwrap();
        std::fs::write(destination.join("photo.jpg"), b"existing file").unwrap();
        assert!(
            !service
                .move_into_folder_blocking(media_id, destination.clone())
                .unwrap()
        );
        assert!(root.join("library/photo.jpg").is_file());
        assert_eq!(
            service.media(media_id).unwrap().unwrap().full_path,
            root.join("library").join("photo.jpg").to_string_lossy()
        );
        drop(service);
        let _ = std::fs::remove_dir_all(root);
    }
}
