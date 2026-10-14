//! Remove offline deletions only after successful traversal, preserving unavailable roots.
use crate::IndexingService;
use photag_plugin_database::db::queries;
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
        .with_reader(|connection| {
            let mut statement =
                connection.prepare("SELECT id, full_path FROM media WHERE folder_id = ?1")?;
            statement
                .query_map([folder_id], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()
        })
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
            .with_writer(|connection| {
                for id in chunk {
                    connection.execute(
                        "DELETE FROM media WHERE id = ?1 AND folder_id = ?2",
                        rusqlite::params![id, folder_id],
                    )?;
                }
                Ok(())
            })
            .map_err(|error| error.to_string())?;
    }
    service
        .database
        .with_writer(|connection| queries::update_folder_scan_time(connection, folder_id))
        .map_err(|error| error.to_string())
}
