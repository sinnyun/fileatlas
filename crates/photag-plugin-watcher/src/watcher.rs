//! File system watcher for auto-scan: detects added/removed media files
//! in registered folders and reports them after a debounce period.

use notify::event::{ModifyKind, RenameMode};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

const MEDIA_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif", "svg", "ico", "heic", "heif",
    "avif", "mp4", "mov", "avi", "mkv", "wmv", "flv", "webm", "m4v", "mpg", "mpeg", "3gp",
];

/// Maximum number of events to drain from the channel per poll, to avoid
/// blocking the UI thread when a flood of FS events accumulates.
const MAX_DRAIN_PER_POLL: usize = 4000;
/// Maximum number of paths to return per poll, to spread DB work across frames.
const MAX_EMIT_PER_POLL: usize = 200;

fn is_media_ext(ext: &str) -> bool {
    MEDIA_EXTENSIONS.contains(&ext)
}

/// Decide whether an event kind is "interesting" for our purposes.
/// We accept Create, Remove, and Rename — but ignore generic Modify events,
/// which on Windows can fire spuriously for metadata/access-time changes
/// and overwhelm the channel during scans.
fn classify(kind: &EventKind) -> Option<EventClass> {
    match kind {
        EventKind::Create(_) => Some(EventClass::Add),
        EventKind::Remove(_) => Some(EventClass::Remove),
        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => Some(EventClass::Remove),
        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => Some(EventClass::Add),
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => Some(EventClass::Add),
        _ => None,
    }
}

#[derive(Copy, Clone)]
enum EventClass {
    Add,
    Remove,
}

pub struct FolderWatcher {
    /// The underlying watcher is shared with a background worker thread so
    /// that `watch()` and `unwatch()` calls never block the UI thread —
    /// `notify` on Windows can take noticeable time to register a directory
    /// (especially on slower drives or recursive trees).
    watcher: Arc<Mutex<RecommendedWatcher>>,
    rx: mpsc::Receiver<notify::Result<Event>>,
    pending_added: HashSet<String>,
    pending_removed: HashSet<String>,
    last_event: Option<Instant>,
}

impl FolderWatcher {
    pub fn new() -> Option<Self> {
        let (tx, rx) = mpsc::channel();
        let watcher = RecommendedWatcher::new(tx, Config::default()).ok()?;
        Some(Self {
            watcher: Arc::new(Mutex::new(watcher)),
            rx,
            pending_added: HashSet::new(),
            pending_removed: HashSet::new(),
            last_event: None,
        })
    }

    /// Non-blocking: actually registers the watch on a background thread.
    pub fn watch(&self, path: &str) {
        let watcher = self.watcher.clone();
        let path = path.to_string();
        std::thread::spawn(move || {
            if let Ok(mut w) = watcher.lock() {
                let _ = w.watch(Path::new(&path), RecursiveMode::Recursive);
            }
        });
    }

    /// Non-blocking: actually unregisters the watch on a background thread.
    pub fn unwatch(&self, path: &str) {
        let watcher = self.watcher.clone();
        let path = path.to_string();
        std::thread::spawn(move || {
            if let Ok(mut w) = watcher.lock() {
                let _ = w.unwatch(Path::new(&path));
            }
        });
    }

    /// Drain new events from the underlying channel into pending buffers.
    /// Caps work to MAX_DRAIN_PER_POLL to avoid blocking the UI thread.
    fn drain_events(&mut self) {
        let mut drained = 0;
        while drained < MAX_DRAIN_PER_POLL {
            let result = match self.rx.try_recv() {
                Ok(r) => r,
                Err(_) => break,
            };
            drained += 1;
            let event = match result {
                Ok(e) => e,
                Err(_) => continue,
            };
            let Some(class) = classify(&event.kind) else {
                continue;
            };

            for path in &event.paths {
                let ext = path
                    .extension()
                    .map(|e| e.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                if !is_media_ext(&ext) {
                    continue;
                }
                if path
                    .file_name()
                    .map(|n| n.to_string_lossy().starts_with('.'))
                    .unwrap_or(false)
                {
                    continue;
                }

                let path_str = path.to_string_lossy().to_string();
                match class {
                    EventClass::Add => {
                        self.pending_removed.remove(&path_str);
                        self.pending_added.insert(path_str);
                    }
                    EventClass::Remove => {
                        self.pending_added.remove(&path_str);
                        self.pending_removed.insert(path_str);
                    }
                }
            }
            self.last_event = Some(Instant::now());
        }
    }

    /// Returns (added_or_renamed, removed) once the debounce period has elapsed.
    /// During the debounce period, returns empty vectors. Caps the number of
    /// returned paths per call so that downstream DB work stays bounded per frame.
    pub fn poll_changes(&mut self) -> (Vec<String>, Vec<String>) {
        self.drain_events();

        // Wait 800ms after the last event before emitting (handles slow file copies)
        let Some(last) = self.last_event else {
            return (Vec::new(), Vec::new());
        };
        if last.elapsed() < Duration::from_millis(800) {
            return (Vec::new(), Vec::new());
        }

        let added: Vec<String> = self
            .pending_added
            .iter()
            .take(MAX_EMIT_PER_POLL)
            .cloned()
            .collect();
        for p in &added {
            self.pending_added.remove(p);
        }

        let remaining = MAX_EMIT_PER_POLL.saturating_sub(added.len());
        let removed: Vec<String> = self
            .pending_removed
            .iter()
            .take(remaining)
            .cloned()
            .collect();
        for p in &removed {
            self.pending_removed.remove(p);
        }

        // If buffers are empty, reset the debounce timer; otherwise keep firing
        // on subsequent frames until everything is drained.
        if self.pending_added.is_empty() && self.pending_removed.is_empty() {
            self.last_event = None;
        } else {
            self.last_event = Some(Instant::now() - Duration::from_secs(1));
        }

        (added, removed)
    }
}
