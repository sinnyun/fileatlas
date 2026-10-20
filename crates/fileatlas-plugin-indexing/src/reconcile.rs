//! Remove offline deletions only after successful traversal, preserving unavailable roots.
use crate::IndexingService;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) fn finish_scan(
    service: &IndexingService,
    folder_id: i64,
    root: &str,
    cancel: &AtomicBool,
) -> Result<(), String> {
    if cancel.load(Ordering::Acquire) {
        return Ok(());
    }
    if !Path::new(root).is_dir() {
        return Err(format!("Folder became unavailable: {root}"));
    }
    let rows = service
        .database
        .media_paths_for_folder(folder_id)
        .map_err(|error| error.to_string())?;
    let missing = rows
        .into_iter()
        .filter_map(|(id, path)| {
            // Do not erase permission failures or files created during this scan.
            match std::fs::metadata(path) {
                Ok(metadata) if !metadata.is_file() => Some(id),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Some(id),
                _ => None,
            }
        })
        .collect::<Vec<_>>();
    for chunk in missing.chunks(200) {
        if cancel.load(Ordering::Acquire) {
            return Ok(());
        }
        service
            .database
            .delete_media_for_folder(folder_id, chunk)
            .map_err(|error| error.to_string())?;
    }
    service
        .database
        .update_folder_scan_time(folder_id)
        .map_err(|error| error.to_string())
}
