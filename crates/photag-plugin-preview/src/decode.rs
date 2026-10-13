//! Background decode queue for preview rasters and filmstrip thumbnails.

use std::collections::{HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Condvar, Mutex};

use image::ImageDecoder as _;

use super::PreviewService;

pub(super) struct WheelAnimation {
    pub target_zoom: f32,
    pub target_pan_x: f32,
    pub target_pan_y: f32,
    pub last_tick: std::time::Instant,
}

pub(super) struct DecodeJob {
    pub source: PathBuf,
    pub target: PathBuf,
    pub max_dim: u32,
}

#[derive(Default)]
pub(super) struct DecodeQueue {
    pub jobs: Mutex<VecDeque<DecodeJob>>,
    pub ready: Condvar,
    pub closed: Mutex<bool>,
}

impl DecodeQueue {
    pub(super) fn push(&self, job: DecodeJob) {
        if let Ok(mut jobs) = self.jobs.lock() {
            jobs.push_back(job);
            self.ready.notify_one();
        }
    }

    fn pop(&self) -> Option<DecodeJob> {
        let mut jobs = self.jobs.lock().ok()?;
        loop {
            if let Some(job) = jobs.pop_front() {
                return Some(job);
            }
            if self.closed.lock().map(|closed| *closed).unwrap_or(true) {
                return None;
            }
            jobs = self.ready.wait(jobs).ok()?;
        }
    }
}

/// Start the two background decoders that materialize preview rasters off the
/// render thread. A small queue keeps a filtered filmstrip from fanning out.
pub(super) fn start_decode_workers(
    decode_queue: Arc<DecodeQueue>,
    pending_decodes: Arc<Mutex<HashSet<(PathBuf, u32)>>>,
    decode_revision: Arc<std::sync::atomic::AtomicU64>,
) {
    for _ in 0..2 {
        let queue = Arc::clone(&decode_queue);
        let pending = Arc::clone(&pending_decodes);
        let revision = Arc::clone(&decode_revision);
        std::thread::spawn(move || {
            while let Some(job) = queue.pop() {
                let key = (job.source.clone(), job.max_dim);
                let decoded = PreviewService::open_oriented(&job.source)
                    .ok()
                    .map(|image| image.thumbnail(job.max_dim, job.max_dim));
                let written = decoded.is_some_and(|image| {
                    let temporary = job.target.with_extension(format!(
                        "{}.tmp.jpg",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_or(0, |elapsed| elapsed.as_nanos())
                    ));
                    image.save(&temporary).is_ok()
                        && if std::fs::rename(&temporary, &job.target).is_ok() {
                            true
                        } else {
                            let _ = std::fs::remove_file(temporary);
                            job.target.is_file()
                        }
                });
                if written {
                    revision.fetch_add(1, Ordering::Release);
                }
                if let Ok(mut pending) = pending.lock() {
                    pending.remove(&key);
                }
            }
        });
    }
}

impl Drop for PreviewService {
    fn drop(&mut self) {
        if let Ok(mut closed) = self.decode_queue.closed.lock() {
            *closed = true;
        }
        self.decode_queue.ready.notify_all();
    }
}

impl PreviewService {
    /// Return a brief low-resolution first frame while the native image element starts
    /// decoding the full source. The file is produced off the render thread.
    pub fn progressive_path(&self, source: &std::path::Path) -> Option<PathBuf> {
        let fast = self.decode_cache_path(source, 1600);
        if !fast.is_file() {
            return None;
        }
        let recent = fast
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age < std::time::Duration::from_millis(500));
        recent.then_some(fast)
    }

    pub fn filmstrip_thumbnail(&self, source: &std::path::Path) -> Option<PathBuf> {
        let path = self.decode_cache_path(source, 96);
        path.is_file().then_some(path)
    }

    pub(super) fn decode_cache_path(&self, source: &std::path::Path, max_dim: u32) -> PathBuf {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        self.cache_dir
            .join(format!("preview-{max_dim}-{:016x}.jpg", hasher.finish()))
    }

    pub(super) fn prewarm_image(&self, source: &std::path::Path, max_dim: u32) {
        let target = self.decode_cache_path(source, max_dim);
        if target.is_file() {
            return;
        }
        let source = source.to_path_buf();
        let key = (source.clone(), max_dim);
        let Ok(mut pending) = self.pending_decodes.lock() else {
            return;
        };
        if pending.insert(key.clone()) {
            self.decode_queue.push(DecodeJob {
                source,
                target,
                max_dim,
            });
        }
    }

    pub(super) fn open_oriented(path: &std::path::Path) -> Result<image::DynamicImage, String> {
        let reader = image::ImageReader::open(path).map_err(|error| error.to_string())?;
        let mut decoder = reader.into_decoder().map_err(|error| error.to_string())?;
        let orientation = decoder
            .orientation()
            .unwrap_or(image::metadata::Orientation::NoTransforms);
        let mut decoded =
            image::DynamicImage::from_decoder(decoder).map_err(|error| error.to_string())?;
        decoded.apply_orientation(orientation);
        Ok(decoded)
    }

    pub(super) fn read_orientation(path: &std::path::Path) -> image::metadata::Orientation {
        image::ImageReader::open(path)
            .ok()
            .and_then(|reader| reader.into_decoder().ok())
            .and_then(|mut decoder| decoder.orientation().ok())
            .unwrap_or(image::metadata::Orientation::NoTransforms)
    }
}
