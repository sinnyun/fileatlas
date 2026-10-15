//! File system watcher for auto-scan: detects added/removed media files
//! in registered folders and reports them after a debounce period.

use notify::event::{ModifyKind, RenameMode};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

/// Maximum number of events to drain from the channel per poll, to avoid
/// blocking the UI thread when a flood of FS events accumulates.
const MAX_DRAIN_PER_POLL: usize = 4000;
/// Maximum number of paths to return per poll, to spread DB work across frames.
const MAX_EMIT_PER_POLL: usize = 200;

/// Decide whether an event kind is "interesting" for our purposes.
/// We accept Create, Remove, and Rename — but ignore generic Modify events,
/// which on Windows can fire spuriously for metadata/access-time changes
/// and overwhelm the channel during scans.
fn classify(kind: &EventKind) -> Option<EventClass> {
    match kind {
        EventKind::Modify(ModifyKind::Metadata(notify::event::MetadataKind::AccessTime)) => None,
        EventKind::Create(_) => Some(EventClass::Add),
        EventKind::Remove(_) => Some(EventClass::Remove),
        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => Some(EventClass::Remove),
        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => Some(EventClass::Add),
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => Some(EventClass::Add),
        EventKind::Modify(ModifyKind::Any)
        | EventKind::Modify(ModifyKind::Data(_))
        | EventKind::Modify(ModifyKind::Metadata(_))
        | EventKind::Any => Some(EventClass::Add),
        _ => None,
    }
}

#[cfg(test)]
mod registration_tests {
    use super::*;

    #[test]
    fn registration_does_not_synthesize_a_change_but_real_creation_is_delivered() {
        let root =
            std::env::temp_dir().join(format!("photag-watch-registration-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.to_string_lossy();
        let mut watcher = FolderWatcher::new().expect("filesystem watcher");
        watcher.replace_watch(None, Some(&path), false);
        let deadline = Instant::now() + Duration::from_secs(10);
        while watcher.registration_pending() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(Duration::from_millis(950));
        assert_eq!(watcher.poll_changes(), (Vec::new(), Vec::new()));
        let file = root.join("actual.txt");
        std::fs::write(&file, "real change").unwrap();
        let expected = std::fs::canonicalize(&file)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if watcher.poll_changes().0.contains(&expected) {
                break;
            }
            assert!(Instant::now() < deadline, "real change was not delivered");
            std::thread::sleep(Duration::from_millis(50));
        }
        drop(watcher);
        std::fs::remove_dir_all(root).unwrap();
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
    watcher: Arc<Mutex<Box<dyn Watcher + Send>>>,
    generation: Arc<std::sync::atomic::AtomicU64>,
    registered_generation: Arc<std::sync::atomic::AtomicU64>,
    normalize_paths: std::sync::atomic::AtomicBool,
    rx: mpsc::Receiver<notify::Result<Event>>,
    sender: mpsc::SyncSender<notify::Result<Event>>,
    overflow: Arc<std::sync::atomic::AtomicBool>,
    watched_path: Arc<Mutex<Option<String>>>,
    registered_path: Arc<Mutex<Option<String>>>,
    pending_added: HashSet<String>,
    pending_removed: HashSet<String>,
    last_event: Option<Instant>,
}

impl FolderWatcher {
    pub fn new() -> Option<Self> {
        let (tx, rx) = mpsc::sync_channel(4096);
        let overflow = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let sender = tx.clone();
        let overflow_callback = overflow.clone();
        let handler = move |event| {
            if sender.try_send(event).is_err() {
                overflow_callback.store(true, std::sync::atomic::Ordering::Release);
            }
        };
        let watcher: Box<dyn Watcher + Send> =
            match RecommendedWatcher::new(handler.clone(), Config::default()) {
                Ok(watcher) => Box::new(watcher),
                Err(error) => {
                    tracing::warn!(%error,"native watcher unavailable, using polling");
                    Box::new(
                        notify::PollWatcher::new(
                            handler,
                            Config::default().with_poll_interval(Duration::from_secs(2)),
                        )
                        .ok()?,
                    )
                }
            };
        Some(Self {
            watcher: Arc::new(Mutex::new(watcher)),
            generation: Arc::default(),
            registered_generation: Arc::default(),
            normalize_paths: std::sync::atomic::AtomicBool::new(false),
            rx,
            sender: tx,
            overflow,
            watched_path: Arc::default(),
            registered_path: Arc::default(),
            pending_added: HashSet::new(),
            pending_removed: HashSet::new(),
            last_event: None,
        })
    }

    pub fn replace_watch(&self, _old: Option<&str>, new: Option<&str>, recursive: bool) {
        use std::sync::atomic::Ordering;
        self.normalize_paths.store(true, Ordering::Release);
        let ticket = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        let generation = self.generation.clone();
        let completed = self.registered_generation.clone();
        let watcher = self.watcher.clone();
        let sender = self.sender.clone();
        let overflow = self.overflow.clone();
        *self.watched_path.lock().unwrap() = new.map(str::to_owned);
        let registered = self.registered_path.clone();
        let new = new.map(str::to_owned);
        std::thread::spawn(move || {
            if let Ok(mut watcher) = watcher.lock() {
                if generation.load(Ordering::Acquire) != ticket {
                    return;
                }
                if let Some(old) = registered.lock().unwrap().take() {
                    let _ = watcher.unwatch(Path::new(&old));
                }
                if let Some(new) = new {
                    let mode = if recursive {
                        RecursiveMode::Recursive
                    } else {
                        RecursiveMode::NonRecursive
                    };
                    if let Err(error) = watcher.watch(Path::new(&new), mode) {
                        tracing::warn!(%error,path=%new,"directory watch failed, using polling");
                        let fallback_sender = sender.clone();
                        let fallback_overflow = overflow.clone();
                        let handler = move |event| {
                            if fallback_sender.try_send(event).is_err() {
                                fallback_overflow.store(true, Ordering::Release);
                            }
                        };
                        match notify::PollWatcher::new(
                            handler,
                            Config::default().with_poll_interval(Duration::from_secs(2)),
                        ) {
                            Ok(mut fallback) => match fallback.watch(Path::new(&new), mode) {
                                Ok(()) => {
                                    *registered.lock().unwrap() = Some(new.clone());
                                    *watcher = Box::new(fallback);
                                }
                                Err(error) => {
                                    tracing::error!(%error,path=%new,"polling directory watch failed")
                                }
                            },
                            Err(error) => {
                                tracing::error!(%error,path=%new,"polling watcher initialization failed")
                            }
                        }
                    } else {
                        *registered.lock().unwrap() = Some(new.clone());
                    }
                }
                completed.store(ticket, Ordering::Release);
            }
        });
    }

    pub fn registration_pending(&self) -> bool {
        use std::sync::atomic::Ordering;
        self.registered_generation.load(Ordering::Acquire)
            != self.generation.load(Ordering::Acquire)
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
        if self
            .overflow
            .swap(false, std::sync::atomic::Ordering::AcqRel)
        {
            if let Some(root) = self.watched_path.lock().unwrap().clone() {
                self.pending_added.insert(root);
                self.last_event = Some(Instant::now() - Duration::from_secs(1));
            }
        }
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
                let normalized = if self
                    .normalize_paths
                    .load(std::sync::atomic::Ordering::Acquire)
                {
                    std::fs::canonicalize(path)
                        .or_else(|_| {
                            let parent = path
                                .parent()
                                .ok_or_else(|| std::io::Error::other("no parent"))?;
                            Ok::<_, std::io::Error>(
                                std::fs::canonicalize(parent)?
                                    .join(path.file_name().unwrap_or_default()),
                            )
                        })
                        .unwrap_or_else(|_| path.clone())
                } else {
                    path.clone()
                };
                let path_str = normalized.to_string_lossy().to_string();
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
            if self.pending_added.len() + self.pending_removed.len() > 4096 {
                self.pending_added.clear();
                self.pending_removed.clear();
                if let Some(root) = self.watched_path.lock().unwrap().clone() {
                    self.pending_added.insert(root);
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
