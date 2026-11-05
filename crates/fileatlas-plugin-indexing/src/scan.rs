use crate::IndexingService;
use fileatlas_plugin_database::ScanFileRecord;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif", "svg", "ico", "heic", "heif", "avif",
];
const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "avi", "mkv", "wmv", "flv", "webm", "m4v", "mpg", "mpeg", "3gp",
];
const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "wav", "flac", "aac", "ogg", "wma", "m4a", "opus", "ape", "aiff", "aif",
];

/// Classify every extension; unsupported and extensionless files remain indexed as `file`.
pub fn classify_media_type(ext: &str) -> &'static str {
    let lower = ext.to_lowercase();
    if IMAGE_EXTENSIONS.contains(&lower.as_str()) {
        "image"
    } else if VIDEO_EXTENSIONS.contains(&lower.as_str()) {
        "video"
    } else if AUDIO_EXTENSIONS.contains(&lower.as_str()) {
        "audio"
    } else {
        "file"
    }
}

#[derive(Clone)]
pub struct ScanProgress {
    pub scanned: usize,
    pub total: usize,
    pub phase: String,
    /// Set only on the terminal progress event when the scan worker fails.
    pub error: Option<String>,
}

struct MediaEntry {
    path: PathBuf,
    media_type: &'static str,
}

pub fn scan_folder_async(
    db: Arc<IndexingService>,
    folder_id: i64,
    folder_path: String,
    cancel: Arc<AtomicBool>,
) -> mpsc::Receiver<Option<ScanProgress>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            scan_folder_impl(&db, folder_id, &folder_path, &tx, &cancel)
        }))
        .unwrap_or_else(|_| Err("The folder scan worker stopped unexpectedly.".into()));
        if let Err(error) = result {
            tx.send(Some(ScanProgress {
                scanned: 0,
                total: 0,
                phase: "Folder scan failed".into(),
                error: Some(error),
            }))
            .ok();
        }
        tx.send(None).ok();
    });
    rx
}

fn scan_folder_impl(
    db: &Arc<IndexingService>,
    folder_id: i64,
    folder_path: &str,
    tx: &mpsc::Sender<Option<ScanProgress>>,
    cancel: &Arc<AtomicBool>,
) -> Result<(), String> {
    let mut dir_groups: HashMap<PathBuf, Vec<(PathBuf, &'static str)>> = HashMap::new();
    for entry in walkdir::WalkDir::new(folder_path)
        .follow_links(true)
        .into_iter()
        .map(|entry| entry.map_err(|error| error.to_string()))
    {
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let ext = entry
            .path()
            .extension()
            .unwrap_or_default()
            .to_string_lossy();
        let media_type = classify_media_type(&ext);
        let parent = entry.path().parent().unwrap_or(entry.path()).to_path_buf();
        dir_groups
            .entry(parent)
            .or_default()
            .push((entry.path().to_path_buf(), media_type));
    }

    let mut entries: Vec<MediaEntry> = Vec::new();
    for (_dir, items) in &dir_groups {
        for (path, media_type) in items {
            entries.push(MediaEntry {
                path: path.clone(),
                media_type,
            });
        }
    }

    let total = entries.len();
    if total == 0 {
        return crate::reconcile::finish_scan(db, folder_id, folder_path, cancel);
    }

    // Phase 1: Index files into DB
    let chunk_size = 200;
    for (chunk_idx, chunk) in entries.chunks(chunk_size).enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        let mut files = Vec::with_capacity(chunk.len());
        for entry in chunk {
            let path = &entry.path;
            let filename = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let full_path = path.to_string_lossy().to_string();
            let media_type = entry.media_type.to_string();
            let metadata = std::fs::metadata(path).ok();
            let file_size = metadata.as_ref().map(|m| m.len() as i64).unwrap_or(0);
            let modified_at = metadata
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let (width, height) = source_dimensions(path, entry.media_type);
            files.push(ScanFileRecord {
                filename,
                full_path,
                media_type,
                file_size,
                modified_at,
                width,
                height,
                poster_path: None,
            });
        }
        let old_thumbnails = db
            .database
            .upsert_scanned_files(folder_id, &files)
            .map_err(|error| error.to_string())?;
        let invalidated_thumbnails = old_thumbnails
            .into_iter()
            .zip(files.iter())
            .filter_map(|(old, file)| {
                old.filter(|old| file.poster_path.as_deref() != Some(old.as_str()))
            })
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        for stale_thumbnail in invalidated_thumbnails {
            if let Err(error) = std::fs::remove_file(&stale_thumbnail)
                && error.kind() != std::io::ErrorKind::NotFound
            {
                tracing::error!(
                    "could not clear stale thumbnail {}: {error}",
                    stale_thumbnail.display()
                );
            }
        }
        let scanned = ((chunk_idx + 1) * chunk_size).min(total);
        tx.send(Some(ScanProgress {
            scanned,
            total,
            phase: format!("Indexing files... {}/{}", scanned, total),
            error: None,
        }))
        .ok();
    }

    // Ordinary file-card thumbnails are requested lazily from the Windows Shell cache.
    crate::reconcile::finish_scan(db, folder_id, folder_path, cancel)
}

/// Read image dimensions from the source header only. This does not decode
/// pixels or produce a thumbnail; visible cards still use Windows Shell cache.
fn source_dimensions(path: &std::path::Path, media_type: &str) -> (Option<i64>, Option<i64>) {
    if media_type != "image" {
        return (None, None);
    }
    let Some((width, height)) = image::ImageReader::open(path)
        .ok()
        .and_then(|reader| reader.into_dimensions().ok())
    else {
        return (None, None);
    };
    (Some(i64::from(width)), Some(i64::from(height)))
}
