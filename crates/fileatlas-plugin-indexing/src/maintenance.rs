//! Immediate directory reads and cache-only thumbnail demand handling.

use crate::IndexingService;
use crate::scan::ScanProgress;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, mpsc};

/// Finish the legacy watcher request without generating ordinary card artwork.
pub fn generate_missing_thumbnails_async(
    _db: Arc<IndexingService>,
    _items: Vec<(String, String)>,
    _cancel: Arc<AtomicBool>,
) -> mpsc::Receiver<Option<ScanProgress>> {
    let (tx, rx) = mpsc::channel();
    let _ = tx.send(Some(ScanProgress {
        scanned: 0,
        total: 0,
        phase: "File thumbnails are read from the Windows Shell cache when visible".into(),
        error: None,
    }));
    let _ = tx.send(None);
    rx
}

pub fn try_list_subdirectories(path: &str) -> std::io::Result<Vec<(String, String, bool)>> {
    read_subdirectories(path, true)
}

pub fn list_subdirectories(path: &str) -> Vec<(String, String, bool)> {
    read_subdirectories(path, false).unwrap_or_default()
}

fn read_subdirectories(path: &str, strict: bool) -> std::io::Result<Vec<(String, String, bool)>> {
    let mut subdirs = Vec::new();
    let entries = std::fs::read_dir(path)?;
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) if strict => return Err(error),
            Err(_) => continue,
        };
        let ft = match entry.file_type() {
            Ok(kind) => kind,
            Err(error) if strict => return Err(error),
            Err(_) => continue,
        };
        if !ft.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let full = entry.path().to_string_lossy().to_string();
        let has_children = std::fs::read_dir(entry.path())
            .map(|rd| {
                rd.flatten()
                    .any(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
            })
            .unwrap_or(false);
        subdirs.push((name, full, has_children));
    }
    subdirs.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    Ok(subdirs)
}
