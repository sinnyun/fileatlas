use crate::{IndexingService, ffmpeg};
use fileatlas_plugin_database::{ScanFileRecord, ThumbnailRecord};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};

/// Cap worker thread count so the UI thread can keep up.
/// Pegging every logical core with JPEG decoders starves the egui main thread
/// and the OS marks the window as "Not Responding".
pub(super) fn worker_count() -> usize {
    let total = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    total.saturating_sub(1).max(2)
}

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
    thumb_image: Option<PathBuf>,
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
    let mut consumed_images: HashSet<PathBuf> = HashSet::new();

    for (_dir, items) in &dir_groups {
        let video_stems: HashSet<String> = items
            .iter()
            .filter(|(_, mt)| *mt == "video")
            .filter_map(|(p, _)| p.file_stem().map(|s| s.to_string_lossy().to_string()))
            .collect();
        if !video_stems.is_empty() {
            for (path, mt) in items {
                if *mt == "image" {
                    if let Some(stem) = path.file_stem() {
                        if video_stems.contains(&*stem.to_string_lossy()) {
                            consumed_images.insert(path.clone());
                        }
                    }
                }
            }
        }
    }

    for (_dir, items) in &dir_groups {
        for (path, media_type) in items {
            let thumb_image = if *media_type == "video" {
                let stem = path.file_stem().map(|s| s.to_string_lossy().to_string());
                stem.and_then(|s| {
                    consumed_images
                        .iter()
                        .find(|img| {
                            img.parent() == path.parent()
                                && img.file_stem().map(|is| is.to_string_lossy().to_string())
                                    == Some(s.clone())
                        })
                        .cloned()
                })
            } else {
                None
            };
            entries.push(MediaEntry {
                path: path.clone(),
                media_type,
                thumb_image,
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
            let ext = path
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let media_type = classify_media_type(&ext).to_string();
            let metadata = std::fs::metadata(path).ok();
            let file_size = metadata.as_ref().map(|m| m.len() as i64).unwrap_or(0);
            let modified_at = metadata
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let poster_path = entry
                .thumb_image
                .as_ref()
                .map(|p| p.to_string_lossy().to_string());
            files.push(ScanFileRecord {
                filename,
                full_path,
                media_type,
                file_size,
                modified_at,
                poster_path,
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

    // Phase 2: Generate image thumbnails + micro-thumbnails (parallel)
    let mut image_paths: Vec<(PathBuf, String)> = entries
        .iter()
        .filter(|e| e.media_type == "image")
        .map(|e| (e.path.clone(), e.path.to_string_lossy().to_string()))
        .collect();
    // Sort by parent directory then filename. HashMap iteration earlier
    // randomized the order; on HDD that causes a seek per file as workers
    // jump across the disk. Files in the same directory tend to live in
    // adjacent NTFS clusters, so directory-grouped reads are far more
    // sequential. This is the biggest single HDD-throughput win.
    image_paths.sort_by(|a, b| {
        let pa = a.0.parent();
        let pb = b.0.parent();
        pa.cmp(&pb)
            .then_with(|| a.0.file_name().cmp(&b.0.file_name()))
    });
    let img_total = image_paths.len();

    if img_total > 0 {
        let num_workers = worker_count();
        // Unbounded channel: backpressure (sync_channel) was throttling fast
        // workers to the orchestrator's drain rate even when the orchestrator
        // wasn't actually behind, which made peak throughput worse. Each item
        // is small (~1KB tuple + ~500B micro-thumb blob), so even a 50K-item
        // backlog is only ~75MB — well within OS limits.
        let (result_tx, result_rx) = mpsc::channel::<(String, String, u32, u32, Option<Vec<u8>>)>();
        // Shared counter: all threads pull the next file in sequence.
        // On HDD this keeps reads roughly sequential (files in the same
        // directory live on nearby sectors) instead of N threads seeking
        // to N different disk locations at once.
        let work_idx = Arc::new(AtomicUsize::new(0));

        std::thread::scope(|scope| -> Result<(), String> {
            for _ in 0..num_workers {
                let rtx = result_tx.clone();
                let db_worker = Arc::clone(db);
                let idx = work_idx.clone();
                let paths = &image_paths;
                let cancel = cancel.clone();
                scope.spawn(move || {
                    loop {
                        if cancel.load(Ordering::Relaxed) {
                            break;
                        }
                        let i = idx.fetch_add(1, Ordering::Relaxed);
                        if i >= paths.len() {
                            break;
                        }
                        let (path, full_path) = &paths[i];
                        if let Some((thumb_path, w, h, micro)) =
                            db_worker.thumbnails.generate_image(path)
                        {
                            rtx.send((full_path.clone(), thumb_path, w, h, micro)).ok();
                        }
                    }
                });
            }
            drop(result_tx);

            // Batch DB updates so we hold the write lock once per ~128 items.
            // UI reads go through read_db (separate connection), so larger
            // batches don't block them. Larger batches = fewer COMMIT/fsync
            // cycles = higher throughput on HDD.
            const BATCH_SIZE: usize = 128;
            let mut count = 0;
            let mut batch: Vec<(String, String, u32, u32, Option<Vec<u8>>)> =
                Vec::with_capacity(BATCH_SIZE);
            let flush = |batch: &mut Vec<(String, String, u32, u32, Option<Vec<u8>>)>,
                         db: &Arc<IndexingService>|
             -> Result<(), String> {
                if batch.is_empty() {
                    return Ok(());
                }
                let updates = batch
                    .iter()
                    .map(|(full_path, thumb_path, w, h, micro)| ThumbnailRecord {
                        full_path: full_path.clone(),
                        thumb_path: thumb_path.clone(),
                        width: (*w > 0).then_some(*w as i64),
                        height: (*h > 0).then_some(*h as i64),
                        micro_thumb: micro.clone(),
                    })
                    .collect::<Vec<_>>();
                db.database
                    .update_scanned_thumbnails(&updates)
                    .map_err(|error| error.to_string())?;
                batch.clear();
                Ok(())
            };
            while let Ok(item) = result_rx.recv() {
                batch.push(item);
                count += 1;
                if batch.len() >= BATCH_SIZE {
                    flush(&mut batch, &db)?;
                }
                if count % 50 == 0 || count == img_total {
                    tx.send(Some(ScanProgress {
                        scanned: count,
                        total: img_total,
                        phase: format!("Generating thumbnails... {}/{}", count, img_total),
                        error: None,
                    }))
                    .ok();
                }
            }
            flush(&mut batch, &db)?;
            Ok(())
        })?;
    }

    // Phase 3: Generate video thumbnails via ffmpeg (parallel)
    let video_paths: Vec<String> = entries
        .iter()
        .filter(|e| e.media_type == "video" && e.thumb_image.is_none())
        .map(|e| e.path.to_string_lossy().to_string())
        .collect();

    if !video_paths.is_empty() && ffmpeg::is_ffmpeg_available() {
        crate::maintenance::process_videos_parallel(
            db,
            &video_paths,
            cancel,
            tx,
            "Video thumbnails",
            0,
        )?;
    }

    crate::reconcile::finish_scan(db, folder_id, folder_path, cancel)
}
