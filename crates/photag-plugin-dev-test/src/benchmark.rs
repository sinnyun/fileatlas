use std::path::Path;
use std::time::{Duration, Instant};

use photag_plugin_database::{DatabaseService, db::queries};
use rusqlite::params;

use crate::{DevTestError, VIRTUAL_DIR};

/// Replace the plugin-owned synthetic rows in one transaction.
pub(crate) fn replace_virtual_records(
    database: &DatabaseService,
    root: &Path,
    preview: &Path,
    count: usize,
    mut progress: impl FnMut(usize),
) -> Result<(Duration, i64), DevTestError> {
    let root_string = root.to_string_lossy().into_owned();
    let marker = virtual_prefix(root);
    let preview = preview.to_string_lossy().into_owned();
    let started = Instant::now();
    let folder_id = database.with_writer(|connection| {
        let folder = queries::add_folder(connection, &root_string)?;
        let transaction = connection.unchecked_transaction()?;
        transaction.execute(
            "DELETE FROM media WHERE folder_id = ?1 AND substr(full_path, 1, length(?2)) = ?2",
            params![folder.id, marker],
        )?;
        {
            let mut insert = transaction.prepare_cached(
                "INSERT INTO media
                 (folder_id, filename, full_path, media_type, file_size, modified_at,
                  width, height, thumb_path, rating)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            )?;
            for index in 0..count {
                let extension = match index % 7 {
                    0 => "jpg",
                    1 => "png",
                    2 => "webp",
                    3 => "tif",
                    4 => "bmp",
                    5 => "mp4",
                    _ => "wav",
                };
                let media_type = match extension {
                    "mp4" => "video",
                    "wav" => "audio",
                    _ => "image",
                };
                let filename = format!("virtual-{index:06}.{extension}");
                let full_path = format!("{marker}{filename}");
                let thumb = (index % 4 == 0).then_some(preview.as_str());
                let side = 256 + (index % 3_841) as i64;
                let file_size = 512 + ((index * 7919) % (32 * 1024 * 1024)) as i64;
                insert.execute(params![
                    folder.id,
                    filename,
                    full_path,
                    media_type,
                    file_size,
                    1_700_000_000i64 + (index % 1_000_000) as i64,
                    side,
                    side * 3 / 4,
                    thumb,
                    (index % 6) as i64,
                ])?;
                if (index + 1) % 10_000 == 0 {
                    progress(index + 1);
                }
            }
        }
        transaction.commit()?;
        Ok(folder.id)
    })?;
    Ok((started.elapsed(), folder_id))
}

pub(crate) fn clear_virtual_records(
    database: &DatabaseService,
    root: &Path,
) -> Result<(usize, i64), DevTestError> {
    let root_string = root.to_string_lossy().into_owned();
    let marker = virtual_prefix(root);
    Ok(database.with_writer(|connection| {
        let folder = queries::add_folder(connection, &root_string)?;
        let removed = connection.execute(
            "DELETE FROM media WHERE folder_id = ?1 AND substr(full_path, 1, length(?2)) = ?2",
            params![folder.id, marker],
        )?;
        Ok((removed, folder.id))
    })?)
}

fn virtual_prefix(root: &Path) -> String {
    root.join(VIRTUAL_DIR).to_string_lossy().into_owned() + std::path::MAIN_SEPARATOR_STR
}
