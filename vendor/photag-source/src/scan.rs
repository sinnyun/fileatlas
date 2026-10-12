use crate::db::queries;
use crate::thumbs;
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};

/// Cap worker thread count so the UI thread can keep up.
/// Pegging every logical core with JPEG decoders starves the egui main thread
/// and the OS marks the window as "Not Responding".
fn worker_count() -> usize {
    let total = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
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

fn get_media_type(ext: &str) -> Option<&'static str> {
    let lower = ext.to_lowercase();
    if IMAGE_EXTENSIONS.contains(&lower.as_str()) { Some("image") }
    else if VIDEO_EXTENSIONS.contains(&lower.as_str()) { Some("video") }
    else if AUDIO_EXTENSIONS.contains(&lower.as_str()) { Some("audio") }
    else { None }
}

fn is_hidden(name: &std::ffi::OsStr) -> bool {
    name.to_string_lossy().starts_with('.')
}

#[derive(Clone)]
pub struct ScanProgress {
    pub scanned: usize,
    pub total: usize,
    pub phase: String,
}

struct MediaEntry {
    path: PathBuf,
    media_type: &'static str,
    thumb_image: Option<PathBuf>,
}

pub fn scan_folder_async(
    db: Arc<Mutex<Connection>>, folder_id: i64, folder_path: String,
    cancel: Arc<AtomicBool>,
) -> mpsc::Receiver<Option<ScanProgress>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        scan_folder_impl(&db, folder_id, &folder_path, &tx, &cancel);
        tx.send(None).ok();
    });
    rx
}

fn scan_folder_impl(
    db: &Arc<Mutex<Connection>>, folder_id: i64, folder_path: &str,
    tx: &mpsc::Sender<Option<ScanProgress>>, cancel: &Arc<AtomicBool>,
) {
    let mut dir_groups: HashMap<PathBuf, Vec<(PathBuf, &'static str)>> = HashMap::new();
    for entry in walkdir::WalkDir::new(folder_path)
        .follow_links(true).into_iter()
        .filter_entry(|e| e.depth() == 0 || !is_hidden(e.file_name()))
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() { continue; }
        if is_hidden(entry.file_name()) { continue; }
        if let Some(ext) = entry.path().extension() {
            if let Some(media_type) = get_media_type(&ext.to_string_lossy()) {
                let parent = entry.path().parent().unwrap_or(entry.path()).to_path_buf();
                dir_groups.entry(parent).or_default().push((entry.path().to_path_buf(), media_type));
            }
        }
    }

    let mut entries: Vec<MediaEntry> = Vec::new();
    let mut consumed_images: HashSet<PathBuf> = HashSet::new();

    for (_dir, items) in &dir_groups {
        let video_stems: HashSet<String> = items.iter()
            .filter(|(_, mt)| *mt == "video")
            .filter_map(|(p, _)| p.file_stem().map(|s| s.to_string_lossy().to_string()))
            .collect();
        if !video_stems.is_empty() {
            for (path, mt) in items {
                if *mt == "image" {
                    if let Some(stem) = path.file_stem() {
                        if video_stems.contains(&*stem.to_string_lossy()) { consumed_images.insert(path.clone()); }
                    }
                }
            }
        }
    }

    for (_dir, items) in &dir_groups {
        for (path, media_type) in items {
            if *media_type == "image" && consumed_images.contains(path) { continue; }
            let thumb_image = if *media_type == "video" {
                let stem = path.file_stem().map(|s| s.to_string_lossy().to_string());
                stem.and_then(|s| consumed_images.iter().find(|img| {
                    img.parent() == path.parent() && img.file_stem().map(|is| is.to_string_lossy().to_string()) == Some(s.clone())
                }).cloned())
            } else { None };
            entries.push(MediaEntry { path: path.clone(), media_type, thumb_image });
        }
    }

    let total = entries.len();
    if total == 0 { return; }

    // Phase 1: Index files into DB
    let chunk_size = 200;
    for (chunk_idx, chunk) in entries.chunks(chunk_size).enumerate() {
        if cancel.load(Ordering::Relaxed) { return; }
        let conn = db.lock().unwrap();
        conn.execute_batch("BEGIN").ok();
        for entry in chunk {
            let path = &entry.path;
            let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let full_path = path.to_string_lossy().to_string();
            let ext = path.extension().unwrap_or_default().to_string_lossy().to_string();
            let media_type = get_media_type(&ext).unwrap_or(entry.media_type);
            let metadata = std::fs::metadata(path).ok();
            let file_size = metadata.as_ref().map(|m| m.len() as i64).unwrap_or(0);
            let modified_at = metadata.as_ref().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0);
            let thumb_path_str = entry.thumb_image.as_ref().map(|p| p.to_string_lossy().to_string());
            queries::insert_media(&conn, folder_id, &filename, &full_path, media_type, file_size, modified_at, None, None, thumb_path_str.as_deref()).ok();
        }
        conn.execute_batch("COMMIT").ok();
        let scanned = ((chunk_idx + 1) * chunk_size).min(total);
        tx.send(Some(ScanProgress { scanned, total, phase: format!("Indexing files... {}/{}", scanned, total) })).ok();
    }

    // Phase 2: Generate image thumbnails + micro-thumbnails (parallel)
    let thumb_dir = thumbs::get_thumb_dir();
    let mut image_paths: Vec<(PathBuf, String)> = entries.iter()
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
        pa.cmp(&pb).then_with(|| a.0.file_name().cmp(&b.0.file_name()))
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

        std::thread::scope(|scope| {
            for _ in 0..num_workers {
                let rtx = result_tx.clone();
                let td = thumb_dir.clone();
                let idx = work_idx.clone();
                let paths = &image_paths;
                let cancel = cancel.clone();
                scope.spawn(move || {
                    loop {
                        if cancel.load(Ordering::Relaxed) { break; }
                        let i = idx.fetch_add(1, Ordering::Relaxed);
                        if i >= paths.len() { break; }
                        let (path, full_path) = &paths[i];
                        if let Some((thumb_path, w, h, micro)) = thumbs::generate_thumbnail_with_dims(path, &td) {
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
            let mut batch: Vec<(String, String, u32, u32, Option<Vec<u8>>)> = Vec::with_capacity(BATCH_SIZE);
            let flush = |batch: &mut Vec<(String, String, u32, u32, Option<Vec<u8>>)>, db: &Arc<Mutex<Connection>>| {
                if batch.is_empty() { return; }
                let conn = db.lock().unwrap();
                conn.execute_batch("BEGIN").ok();
                {
                    let mut stmt = conn.prepare_cached(
                        "UPDATE media SET thumb_path = ?1, width = ?2, height = ?3, micro_thumb = ?4 WHERE full_path = ?5"
                    ).unwrap();
                    for (full_path, thumb_path, w, h, micro) in batch.iter() {
                        let micro_ref: Option<&[u8]> = micro.as_deref();
                        let w_param = if *w > 0 { Some(*w as i64) } else { None };
                        let h_param = if *h > 0 { Some(*h as i64) } else { None };
                        stmt.execute(
                            rusqlite::params![thumb_path, w_param, h_param, micro_ref, full_path],
                        ).ok();
                    }
                }
                conn.execute_batch("COMMIT").ok();
                drop(conn);
                batch.clear();
            };
            while let Ok(item) = result_rx.recv() {
                batch.push(item);
                count += 1;
                if batch.len() >= BATCH_SIZE {
                    flush(&mut batch, &db);
                }
                if count % 50 == 0 || count == img_total {
                    tx.send(Some(ScanProgress {
                        scanned: count,
                        total: img_total,
                        phase: format!("Generating thumbnails... {}/{}", count, img_total),
                    })).ok();
                }
            }
            flush(&mut batch, &db);
        });
    }

    // Phase 3: Generate video thumbnails via ffmpeg (parallel)
    let video_paths: Vec<String> = entries
        .iter()
        .filter(|e| e.media_type == "video" && e.thumb_image.is_none())
        .map(|e| e.path.to_string_lossy().to_string())
        .collect();

    if !video_paths.is_empty() && crate::video::is_ffmpeg_available() {
        process_videos_parallel(db, &video_paths, &thumb_dir, cancel, tx, "Video thumbnails", 0);
    }

    let conn = db.lock().unwrap();
    queries::update_folder_scan_time(&conn, folder_id).ok();
}

/// Run ffmpeg-based video thumbnail extraction across multiple worker threads.
/// Each ffmpeg subprocess does its own seek+decode work; running 3-4 in
/// parallel saturates the disk on most systems while leaving CPU headroom.
/// `start_count` is the offset to add to progress.scanned (so callers that
/// already counted earlier work — like image thumbnails — get a continuous
/// progress bar).
fn process_videos_parallel(
    db: &Arc<Mutex<Connection>>,
    video_paths: &[String],
    thumb_dir: &std::path::Path,
    cancel: &Arc<AtomicBool>,
    tx: &mpsc::Sender<Option<ScanProgress>>,
    phase_name: &str,
    start_count: usize,
) {
    // 3 workers: ffmpeg seek+decode is mostly I/O-bound but each subprocess
    // also spins up briefly. More than ~3 concurrent ffmpeg invocations on
    // HDD just causes seek thrash without throughput gain.
    const VIDEO_WORKERS: usize = 3;
    let work_idx = Arc::new(AtomicUsize::new(0));
    let vid_total = video_paths.len();
    let (result_tx, result_rx) = mpsc::channel::<(String, String, Option<Vec<u8>>)>();

    std::thread::scope(|scope| {
        for _ in 0..VIDEO_WORKERS {
            let rtx = result_tx.clone();
            let td = thumb_dir.to_path_buf();
            let idx = work_idx.clone();
            let cancel = cancel.clone();
            let paths = video_paths;
            scope.spawn(move || {
                loop {
                    if cancel.load(Ordering::Relaxed) { break; }
                    let i = idx.fetch_add(1, Ordering::Relaxed);
                    if i >= paths.len() { break; }
                    let full_path = &paths[i];
                    let path = std::path::Path::new(full_path);
                    let vid_thumb = thumbs::thumb_path_for(path, &td);

                    if !vid_thumb.exists() {
                        let out_str = vid_thumb.to_string_lossy().to_string();
                        if !crate::video::extract_video_thumbnail(full_path, &out_str) {
                            continue;
                        }
                    }
                    let out_str = vid_thumb.to_string_lossy().to_string();
                    let micro = image::open(&vid_thumb)
                        .ok()
                        .map(|img| thumbs::generate_micro_thumb(&img));
                    rtx.send((full_path.clone(), out_str, micro)).ok();
                }
            });
        }
        drop(result_tx);

        // Batch DB updates same as image phase.
        const BATCH_SIZE: usize = 32;
        let mut batch: Vec<(String, String, Option<Vec<u8>>)> = Vec::with_capacity(BATCH_SIZE);
        let mut count = 0usize;
        let flush = |batch: &mut Vec<(String, String, Option<Vec<u8>>)>, db: &Arc<Mutex<Connection>>| {
            if batch.is_empty() { return; }
            let conn = db.lock().unwrap();
            conn.execute_batch("BEGIN").ok();
            {
                let mut stmt = conn.prepare_cached(
                    "UPDATE media SET thumb_path = ?1, micro_thumb = ?2 WHERE full_path = ?3"
                ).unwrap();
                for (full_path, thumb_path, micro) in batch.iter() {
                    let micro_ref: Option<&[u8]> = micro.as_deref();
                    stmt.execute(
                        rusqlite::params![thumb_path, micro_ref, full_path],
                    ).ok();
                }
            }
            conn.execute_batch("COMMIT").ok();
            drop(conn);
            batch.clear();
        };
        while let Ok(item) = result_rx.recv() {
            batch.push(item);
            count += 1;
            if batch.len() >= BATCH_SIZE {
                flush(&mut batch, db);
            }
            if count % 5 == 0 || count == vid_total {
                tx.send(Some(ScanProgress {
                    scanned: start_count + count,
                    total: start_count + vid_total,
                    phase: format!("{}... {}/{}", phase_name, count, vid_total),
                })).ok();
            }
        }
        flush(&mut batch, db);
    });
}

/// Generate thumbnails for items that are missing them (e.g. after interrupted scan).
pub fn generate_missing_thumbnails_async(
    db: Arc<Mutex<Connection>>,
    items: Vec<(String, String)>, // (full_path, media_type)
    cancel: Arc<AtomicBool>,
) -> mpsc::Receiver<Option<ScanProgress>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let thumb_dir = thumbs::get_thumb_dir();
        let total = items.len();
        if total == 0 { tx.send(None).ok(); return; }

        let mut image_paths: Vec<(PathBuf, String)> = items.iter()
            .filter(|(_, mt)| mt == "image")
            .map(|(fp, _)| (PathBuf::from(fp), fp.clone()))
            .collect();
        // Sort by directory then filename for HDD-friendly sequential reads.
        image_paths.sort_by(|a, b| {
            let pa = a.0.parent();
            let pb = b.0.parent();
            pa.cmp(&pb).then_with(|| a.0.file_name().cmp(&b.0.file_name()))
        });
        let video_paths: Vec<String> = items.iter()
            .filter(|(_, mt)| mt == "video")
            .map(|(fp, _)| fp.clone())
            .collect();
        let img_total = image_paths.len();
        let mut global_count = 0;

        // Image thumbnails in parallel (shared work queue for HDD-friendly sequential reads)
        if img_total > 0 {
            let num_workers = worker_count();
            // Unbounded: bounded sync_channel(256) was throttling fast workers
            // to the orchestrator's drain rate, hurting peak throughput. Each
            // result tuple is small (~1KB + ~500B micro-thumb), so backlog cost
            // is negligible compared to the throughput win.
            let (result_tx, result_rx) = mpsc::channel::<(String, String, u32, u32, Option<Vec<u8>>)>();
            let work_idx = Arc::new(AtomicUsize::new(0));

            std::thread::scope(|scope| {
                for _ in 0..num_workers {
                    let rtx = result_tx.clone();
                    let td = thumb_dir.clone();
                    let idx = work_idx.clone();
                    let paths = &image_paths;
                    let cancel = cancel.clone();
                    scope.spawn(move || {
                        loop {
                            if cancel.load(Ordering::Relaxed) { break; }
                            let i = idx.fetch_add(1, Ordering::Relaxed);
                            if i >= paths.len() { break; }
                            let (path, full_path) = &paths[i];
                            if let Some((tp, w, h, micro)) = thumbs::generate_thumbnail_with_dims(path, &td) {
                                rtx.send((full_path.clone(), tp, w, h, micro)).ok();
                            }
                        }
                    });
                }
                drop(result_tx);

                // Large batch — UI reads use read_db, so this doesn't block them.
                const BATCH_SIZE: usize = 128;
                let mut batch: Vec<(String, String, u32, u32, Option<Vec<u8>>)> = Vec::with_capacity(BATCH_SIZE);
                let flush = |batch: &mut Vec<(String, String, u32, u32, Option<Vec<u8>>)>, db: &Arc<Mutex<Connection>>| {
                    if batch.is_empty() { return; }
                    let conn = db.lock().unwrap();
                    conn.execute_batch("BEGIN").ok();
                    {
                        let mut stmt = conn.prepare_cached(
                            "UPDATE media SET thumb_path = ?1, width = ?2, height = ?3, micro_thumb = ?4 WHERE full_path = ?5"
                        ).unwrap();
                        for (full_path, thumb_path, w, h, micro) in batch.iter() {
                            let micro_ref: Option<&[u8]> = micro.as_deref();
                            let w_param = if *w > 0 { Some(*w as i64) } else { None };
                            let h_param = if *h > 0 { Some(*h as i64) } else { None };
                            stmt.execute(
                                rusqlite::params![thumb_path, w_param, h_param, micro_ref, full_path],
                            ).ok();
                        }
                    }
                    conn.execute_batch("COMMIT").ok();
                    drop(conn);
                    batch.clear();
                };
                while let Ok(item) = result_rx.recv() {
                    batch.push(item);
                    global_count += 1;
                    if batch.len() >= BATCH_SIZE {
                        flush(&mut batch, &db);
                    }
                    if global_count % 50 == 0 || global_count == total {
                        tx.send(Some(ScanProgress {
                            scanned: global_count, total,
                            phase: format!("Generating thumbnails... {}/{}", global_count, total),
                        })).ok();
                    }
                }
                flush(&mut batch, &db);
            });
        }

        // Video thumbnails (parallel)
        if !video_paths.is_empty() && crate::video::is_ffmpeg_available() {
            process_videos_parallel(
                &db, &video_paths, &thumb_dir, &cancel, &tx,
                "Generating video thumbnails", global_count,
            );
        }

        tx.send(None).ok();
    });
    rx
}

pub fn list_subdirectories(path: &str) -> Vec<(String, String, bool)> {
    let mut subdirs = Vec::new();
    let Ok(entries) = std::fs::read_dir(path) else { return subdirs };
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        if !ft.is_dir() { continue; }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') { continue; }
        let full = entry.path().to_string_lossy().to_string();
        // Mirror the dot-prefix filter used above so a folder containing
        // only `.git` (or similar) doesn't get an expand arrow that opens
        // to an empty list.
        let has_children = std::fs::read_dir(entry.path())
            .map(|rd| rd.flatten().any(|e| {
                let is_dir = e.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                let n = e.file_name().to_string_lossy().to_string();
                is_dir && !n.starts_with('.')
            }))
            .unwrap_or(false);
        subdirs.push((name, full, has_children));
    }
    subdirs.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    subdirs
}
