use std::path::Path;
use std::time::{Duration, Instant};

use photag_plugin_database::DatabaseService;

use crate::{DevTestError, VIRTUAL_DIR};

/// Replace the plugin-owned synthetic rows in one database transaction.
pub(crate) fn replace_virtual_records(
    database: &DatabaseService,
    root: &Path,
    preview: &Path,
    count: usize,
    progress: impl FnMut(usize),
) -> Result<(Duration, i64), DevTestError> {
    let marker = virtual_prefix(root);
    let started = Instant::now();
    let folder_id = database.replace_virtual_benchmark(
        &root.to_string_lossy(),
        &marker,
        &preview.to_string_lossy(),
        count,
        progress,
    )?;
    Ok((started.elapsed(), folder_id))
}

pub(crate) fn clear_virtual_records(
    database: &DatabaseService,
    root: &Path,
) -> Result<(usize, i64), DevTestError> {
    Ok(database.clear_virtual_benchmark(&root.to_string_lossy(), &virtual_prefix(root))?)
}

fn virtual_prefix(root: &Path) -> String {
    root.join(VIRTUAL_DIR).to_string_lossy().into_owned() + std::path::MAIN_SEPARATOR_STR
}
