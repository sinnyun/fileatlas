//! Thumbnail backfill and directory reconciliation routines from PhoTag's scanner.

use crate::scan::{self, ScanProgress};
use crate::{IndexingService, ffmpeg};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};

/// Run ffmpeg-based video thumbnail extraction across multiple worker threads.
/// Each ffmpeg subprocess does its own seek+decode work; running 3-4 in
/// parallel saturates the disk on most systems while leaving CPU headroom.
/// `start_count` is the offset to add to progress.scanned (so callers that
/// already counted earlier work — like image thumbnails — get a continuous
/// progress bar).
pub(super) fn process_videos_parallel(
    db: &Arc<IndexingService>,
    video_paths: &[String],
    thumb_dir: &std::path::Path,
    cancel: &Arc<AtomicBool>,
    tx: &mpsc::Sender<Option<ScanProgress>>,
    phase_name: &str,
    start_count: usize,
) -> Result<(), String> {
    // 3 workers: ffmpeg seek+decode is mostly I/O-bound but each subprocess
    // also spins up briefly. More than ~3 concurrent ffmpeg invocations on
    // HDD just causes seek thrash without throughput gain.
    const VIDEO_WORKERS: usize = 3;
    let work_idx = Arc::new(AtomicUsize::new(0));
    let vid_total = video_paths.len();
    let (result_tx, result_rx) = mpsc::channel::<(String, String, Option<Vec<u8>>)>();

    std::thread::scope(|scope| -> Result<(), String> {
        for _ in 0..VIDEO_WORKERS {
            let rtx = result_tx.clone();
            let td = thumb_dir.to_path_buf();
            let idx = work_idx.clone();
            let cancel = cancel.clone();
            let paths = video_paths;
            scope.spawn(move || {
                loop {
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    let i = idx.fetch_add(1, Ordering::Relaxed);
                    if i >= paths.len() {
                        break;
                    }
                    let full_path = &paths[i];
                    let path = std::path::Path::new(full_path);
                    let vid_thumb = photag_plugin_thumbnails::thumbs::thumb_path_for(path, &td);

                    if !vid_thumb.exists() {
                        let out_str = vid_thumb.to_string_lossy().to_string();
                        if !ffmpeg::extract_video_thumbnail(full_path, &out_str) {
                            continue;
                        }
                    }
                    let out_str = vid_thumb.to_string_lossy().to_string();
                    let micro = image::open(&vid_thumb)
                        .ok()
                        .map(|img| photag_plugin_thumbnails::thumbs::generate_micro_thumb(&img));
                    rtx.send((full_path.clone(), out_str, micro)).ok();
                }
            });
        }
        drop(result_tx);

        // Batch DB updates same as image phase.
        const BATCH_SIZE: usize = 32;
        let mut batch: Vec<(String, String, Option<Vec<u8>>)> = Vec::with_capacity(BATCH_SIZE);
        let mut count = 0usize;
        let flush = |batch: &mut Vec<(String, String, Option<Vec<u8>>)>,
                     db: &Arc<IndexingService>|
         -> Result<(), String> {
            if batch.is_empty() {
                return Ok(());
            }
            db.database
                .with_writer(|conn| {
                    conn.execute_batch("BEGIN")?;
                    let updates = (|| {
                        let mut stmt = conn.prepare_cached(
                        "UPDATE media SET thumb_path = ?1, micro_thumb = ?2 WHERE full_path = ?3"
                    )?;
                        for (full_path, thumb_path, micro) in batch.iter() {
                            let micro_ref: Option<&[u8]> = micro.as_deref();
                            stmt.execute(rusqlite::params![thumb_path, micro_ref, full_path])?;
                        }
                        Ok::<(), rusqlite::Error>(())
                    })();
                    match updates {
                        Ok(()) => conn.execute_batch("COMMIT"),
                        Err(error) => {
                            let _ = conn.execute_batch("ROLLBACK");
                            Err(error)
                        }
                    }
                })
                .map_err(|error| error.to_string())?;
            batch.clear();
            Ok(())
        };
        while let Ok(item) = result_rx.recv() {
            batch.push(item);
            count += 1;
            if batch.len() >= BATCH_SIZE {
                flush(&mut batch, db)?;
            }
            if count % 5 == 0 || count == vid_total {
                tx.send(Some(ScanProgress {
                    scanned: start_count + count,
                    total: start_count + vid_total,
                    phase: format!("{}... {}/{}", phase_name, count, vid_total),
                    error: None,
                }))
                .ok();
            }
        }
        flush(&mut batch, db)?;
        Ok(())
    })?;
    Ok(())
}

/// Generate thumbnails for items that are missing them (e.g. after interrupted scan).
pub fn generate_missing_thumbnails_async(
    db: Arc<IndexingService>,
    items: Vec<(String, String)>, // (full_path, media_type)
    cancel: Arc<AtomicBool>,
) -> mpsc::Receiver<Option<ScanProgress>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let worker_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let thumb_dir = db.thumbnails.directory().to_path_buf();
            let total = items.len();
            if total == 0 {
                return;
            }

            let mut image_paths: Vec<(PathBuf, String)> = items
                .iter()
                .filter(|(_, mt)| mt == "image")
                .map(|(fp, _)| (PathBuf::from(fp), fp.clone()))
                .collect();
            // Sort by directory then filename for HDD-friendly sequential reads.
            image_paths.sort_by(|a, b| {
                let pa = a.0.parent();
                let pb = b.0.parent();
                pa.cmp(&pb)
                    .then_with(|| a.0.file_name().cmp(&b.0.file_name()))
            });
            let video_paths: Vec<String> = items
                .iter()
                .filter(|(_, mt)| mt == "video")
                .map(|(fp, _)| fp.clone())
                .collect();
            let img_total = image_paths.len();
            let mut global_count = 0;

            // Image thumbnails in parallel (shared work queue for HDD-friendly sequential reads)
            if img_total > 0 {
                let num_workers = scan::worker_count();
                // Unbounded: bounded sync_channel(256) was throttling fast workers
                // to the orchestrator's drain rate, hurting peak throughput. Each
                // result tuple is small (~1KB + ~500B micro-thumb), so backlog cost
                // is negligible compared to the throughput win.
                let (result_tx, result_rx) =
                    mpsc::channel::<(String, String, u32, u32, Option<Vec<u8>>)>();
                let work_idx = Arc::new(AtomicUsize::new(0));

                let image_result = std::thread::scope(|scope| -> Result<(), String> {
                    for _ in 0..num_workers {
                        let rtx = result_tx.clone();
                        let db_worker = Arc::clone(&db);
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
                                if let Some((tp, w, h, micro)) =
                                    db_worker.thumbnails.generate_image(path)
                                {
                                    rtx.send((full_path.clone(), tp, w, h, micro)).ok();
                                }
                            }
                        });
                    }
                    drop(result_tx);

                    // Large batch — UI reads use read_db, so this doesn't block them.
                    const BATCH_SIZE: usize = 128;
                    let mut batch: Vec<(String, String, u32, u32, Option<Vec<u8>>)> =
                        Vec::with_capacity(BATCH_SIZE);
                    let flush = |batch: &mut Vec<(String, String, u32, u32, Option<Vec<u8>>)>,
                                 db: &Arc<IndexingService>|
                     -> Result<(), String> {
                        if batch.is_empty() {
                            return Ok(());
                        }
                        db.database.with_writer(|conn| {
                        conn.execute_batch("BEGIN")?;
                        let updates = (|| {
                            let mut stmt = conn.prepare_cached(
                                "UPDATE media SET thumb_path = ?1, width = ?2, height = ?3, micro_thumb = ?4 WHERE full_path = ?5"
                            )?;
                            for (full_path, thumb_path, w, h, micro) in batch.iter() {
                                let micro_ref: Option<&[u8]> = micro.as_deref();
                                let w_param = if *w > 0 { Some(*w as i64) } else { None };
                                let h_param = if *h > 0 { Some(*h as i64) } else { None };
                                stmt.execute(rusqlite::params![thumb_path, w_param, h_param, micro_ref, full_path])?;
                            }
                            Ok::<(), rusqlite::Error>(())
                        })();
                        match updates {
                            Ok(()) => conn.execute_batch("COMMIT"),
                            Err(error) => {
                                let _ = conn.execute_batch("ROLLBACK");
                                Err(error)
                            }
                        }
                    }).map_err(|error| error.to_string())?;
                        batch.clear();
                        Ok(())
                    };
                    while let Ok(item) = result_rx.recv() {
                        batch.push(item);
                        global_count += 1;
                        if batch.len() >= BATCH_SIZE {
                            flush(&mut batch, &db)?;
                        }
                        if global_count % 50 == 0 || global_count == total {
                            tx.send(Some(ScanProgress {
                                scanned: global_count,
                                total,
                                phase: format!(
                                    "Generating thumbnails... {}/{}",
                                    global_count, total
                                ),
                                error: None,
                            }))
                            .ok();
                        }
                    }
                    flush(&mut batch, &db)?;
                    Ok(())
                });
                if let Err(error) = image_result {
                    tx.send(Some(ScanProgress {
                        scanned: global_count,
                        total,
                        phase: "Thumbnail backfill failed".into(),
                        error: Some(error),
                    }))
                    .ok();
                    return;
                }
            }

            // Video thumbnails (parallel)
            if !video_paths.is_empty() && ffmpeg::is_ffmpeg_available() {
                if let Err(error) = process_videos_parallel(
                    &db,
                    &video_paths,
                    &thumb_dir,
                    &cancel,
                    &tx,
                    "Generating video thumbnails",
                    global_count,
                ) {
                    tx.send(Some(ScanProgress {
                        scanned: global_count,
                        total,
                        phase: "Thumbnail backfill failed".into(),
                        error: Some(error),
                    }))
                    .ok();
                    return;
                }
            }
        }));
        if worker_result.is_err() {
            tx.send(Some(ScanProgress {
                scanned: 0,
                total: items.len(),
                phase: "Thumbnail backfill failed".into(),
                error: Some("The thumbnail backfill worker stopped unexpectedly.".into()),
            }))
            .ok();
        }
        tx.send(None).ok();
    });
    rx
}

pub fn list_subdirectories(path: &str) -> Vec<(String, String, bool)> {
    let mut subdirs = Vec::new();
    let Ok(entries) = std::fs::read_dir(path) else {
        return subdirs;
    };
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        if !ft.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let full = entry.path().to_string_lossy().to_string();
        // Mirror the dot-prefix filter used above so a folder containing
        // only `.git` (or similar) doesn't get an expand arrow that opens
        // to an empty list.
        let has_children = std::fs::read_dir(entry.path())
            .map(|rd| {
                rd.flatten().any(|e| {
                    let is_dir = e.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                    let n = e.file_name().to_string_lossy().to_string();
                    is_dir && !n.starts_with('.')
                })
            })
            .unwrap_or(false);
        subdirs.push((name, full, has_children));
    }
    subdirs.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    subdirs
}
