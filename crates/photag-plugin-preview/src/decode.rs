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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn active_decode_precedes_queued_filmstrip_work_and_ready_frame_does_not_expire() {
        let queue = DecodeQueue::default();
        queue.push(DecodeJob {
            source: "neighbor".into(),
            target: "neighbor-cache".into(),
            max_dim: 96,
            urgent: false,
            queued_at: std::time::Instant::now(),
            generation: 0,
            request_cancelled: Arc::default(),
            full_ready: Arc::default(),
            reuse_thumbnail: None,
        });
        queue.push(DecodeJob {
            source: "active".into(),
            target: "active-cache".into(),
            max_dim: 1600,
            urgent: true,
            queued_at: std::time::Instant::now(),
            generation: 0,
            request_cancelled: Arc::default(),
            full_ready: Arc::default(),
            reuse_thumbnail: None,
        });
        assert_eq!(queue.pop().unwrap().source, PathBuf::from("active"));
        let directory =
            std::env::temp_dir().join(format!("photag-progressive-ready-{}", std::process::id()));
        let service = PreviewService::open_at(&directory).unwrap();
        let source = directory.join("source.jpg");
        image::RgbImage::from_pixel(64, 48, image::Rgb([20, 80, 180]))
            .save(&source)
            .unwrap();
        service.prewarm_image(&source, 1600);
        let started = std::time::Instant::now();
        let path = loop {
            if let Some(path) = service.progressive_path(&source) {
                break path;
            }
            assert!(started.elapsed() < std::time::Duration::from_secs(5));
            std::thread::sleep(std::time::Duration::from_millis(5));
        };
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(10))
            .unwrap();
        assert_eq!(service.progressive_path(&source), Some(path));
        drop(service);
        let _ = std::fs::remove_dir_all(directory);
    }
}

pub(super) struct DecodeJob {
    pub source: PathBuf,
    pub target: PathBuf,
    pub max_dim: u32,
    pub urgent: bool,
    pub queued_at: std::time::Instant,
    pub generation: u64,
    pub request_cancelled: Arc<std::sync::atomic::AtomicBool>,
    pub full_ready: Arc<std::sync::atomic::AtomicBool>,
    pub reuse_thumbnail: Option<PathBuf>,
}

impl DecodeJob {
    pub(super) fn cancellation_reason(&self) -> &'static str {
        if self.request_cancelled.load(Ordering::Acquire) {
            "request_changed_or_closed"
        } else if self.max_dim == 1600 && self.full_ready.load(Ordering::Acquire) {
            "full_image_ready"
        } else {
            "none"
        }
    }
    pub(super) fn cancelled(&self) -> bool {
        self.cancellation_reason() != "none"
    }
}

#[derive(Default)]
pub(super) struct DecodeQueue {
    pub jobs: Mutex<VecDeque<DecodeJob>>,
    pub ready: Condvar,
    pub closed: Mutex<bool>,
}

impl DecodeQueue {
    pub(super) fn push(&self, mut job: DecodeJob) {
        job.queued_at = std::time::Instant::now();
        if let Ok(mut jobs) = self.jobs.lock() {
            if job.urgent {
                jobs.push_front(job);
            } else {
                jobs.push_back(job);
            }
            self.ready.notify_one();
        }
    }

    pub(super) fn remove_cancelled(&self) -> Vec<DecodeJob> {
        let Ok(mut jobs) = self.jobs.lock() else {
            return Vec::new();
        };
        let mut removed = Vec::new();
        let mut retained = VecDeque::new();
        for job in jobs.drain(..) {
            if job.cancelled() {
                removed.push(job);
            } else {
                retained.push_back(job);
            }
        }
        *jobs = retained;
        removed
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
    pending_decodes: Arc<Mutex<HashSet<(PathBuf, u32, u64)>>>,
    ready_decodes: Arc<Mutex<std::collections::HashMap<(PathBuf, u32), PathBuf>>>,
    decode_revision: Arc<std::sync::atomic::AtomicU64>,
) {
    for _ in 0..2 {
        let queue = Arc::clone(&decode_queue);
        let pending = Arc::clone(&pending_decodes);
        let revision = Arc::clone(&decode_revision);
        let ready = Arc::clone(&ready_decodes);
        std::thread::spawn(move || {
            while let Some(job) = queue.pop() {
                let key = (job.source.clone(), job.max_dim);
                let result = super::decode_diagnostics::run(&job);
                if let Some(output) = result.filter(|_| !job.cancelled()) {
                    if let Ok(mut ready) = ready.lock() {
                        // Bound the in-memory index used by render-time lookups.
                        if ready.len() >= 64 {
                            ready.retain(|(source, _), _| source == &job.source);
                        }
                        ready.insert(key.clone(), output);
                    }
                    revision.fetch_add(1, Ordering::Release);
                }
                if let Ok(mut pending) = pending.lock() {
                    pending.remove(&(job.source.clone(), job.max_dim, job.generation));
                }
            }
        });
    }
}

impl Drop for PreviewService {
    fn drop(&mut self) {
        self.cancel_decode_request();
        if let Ok(mut closed) = self.decode_queue.closed.lock() {
            *closed = true;
        }
        self.decode_queue.ready.notify_all();
    }
}

impl PreviewService {
    /// Keep the ready progressive raster available until the native full image
    /// has painted. Age on disk is unrelated to image readiness.
    pub fn progressive_path(&self, source: &std::path::Path) -> Option<PathBuf> {
        self.ready_decodes
            .lock()
            .ok()?
            .get(&(source.to_owned(), 1600))
            .cloned()
    }

    pub fn filmstrip_thumbnail(&self, source: &std::path::Path) -> Option<PathBuf> {
        self.ready_decodes
            .lock()
            .ok()?
            .get(&(source.to_owned(), 96))
            .cloned()
            .or_else(|| self.indexed_thumbnail(source))
    }

    pub(super) fn decode_cache_path(&self, source: &std::path::Path, max_dim: u32) -> PathBuf {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        self.cache_dir
            .join(format!("preview-{max_dim}-{:016x}.jpg", hasher.finish()))
    }

    pub(super) fn prewarm_image(&self, source: &std::path::Path, max_dim: u32) {
        if max_dim == 1600 && !super::decode_policy::supports_fast_preview(source) {
            return;
        }
        let (generation, request_cancelled, full_ready) = {
            let Ok(control) = self.decode_control.lock() else {
                return;
            };
            (
                control.generation,
                Arc::clone(&control.cancelled),
                Arc::clone(&control.full_ready),
            )
        };
        if request_cancelled.load(Ordering::Acquire)
            || (max_dim == 1600 && full_ready.load(Ordering::Acquire))
        {
            return;
        }
        let reuse_thumbnail = (max_dim == 96)
            .then(|| self.indexed_thumbnail(source))
            .flatten();
        let target = self.decode_cache_path(source, max_dim);
        let source = source.to_path_buf();
        let key = (source.clone(), max_dim);
        if self
            .ready_decodes
            .lock()
            .is_ok_and(|ready| ready.contains_key(&key))
        {
            return;
        }
        let Ok(mut pending) = self.pending_decodes.lock() else {
            return;
        };
        if pending.insert((source.clone(), max_dim, generation)) {
            self.decode_queue.push(DecodeJob {
                source,
                target,
                max_dim,
                urgent: max_dim == 1600 && self.snapshot().source.as_ref() == Some(&key.0),
                queued_at: std::time::Instant::now(),
                generation,
                request_cancelled,
                full_ready,
                reuse_thumbnail,
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
