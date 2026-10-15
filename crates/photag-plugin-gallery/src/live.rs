//! Local directory streaming adapted from Yazi VfsEntries/Refresher (MIT).
//! Copyright (c) 2023 - sxyazi. See vendor/yazi-reference/LICENSE.
//! Changes: synchronous Windows worker, bounded channel, latest request, session SQLite cache.
use super::*;
use std::path::{Path, PathBuf};
use std::sync::{Condvar, atomic::AtomicBool, mpsc};
use std::time::{Duration, Instant, UNIX_EPOCH};

#[derive(Default)]
pub(crate) struct LiveBrowser {
    state: Mutex<State>,
    queue: Arc<Queue>,
    worker: Mutex<Option<std::thread::JoinHandle<()>>>,
    recursive: AtomicBool,
}
#[derive(Default)]
struct Queue {
    request: Mutex<Option<Request>>,
    ready: Condvar,
    stop: AtomicBool,
    running: AtomicBool,
}
struct Request {
    root: String,
    recursive: bool,
    cancel: Arc<AtomicBool>,
    tx: mpsc::SyncSender<Message>,
    database: Arc<DatabaseService>,
    watcher: Option<Arc<photag_plugin_watcher::WatcherService>>,
    wake: photag_ui_bridge::refresh::Wake,
}
#[derive(Default)]
struct State {
    key: Option<(String, bool)>,
    cancel: Arc<AtomicBool>,
    rx: Option<mpsc::Receiver<Message>>,
    directories: Vec<GalleryDirectory>,
    generation: u64,
    revision: u64,
    scanned: usize,
    active: bool,
    error: Option<String>,
}
type LocalEntry = (PathBuf, std::io::Result<std::fs::Metadata>);

enum Message {
    Batch(Vec<GalleryDirectory>, usize),
    Done(Option<String>),
}

impl Drop for LiveBrowser {
    fn drop(&mut self) {
        if let Ok(state) = self.state.lock() {
            state.cancel.store(true, Ordering::Release);
        }
        self.queue.stop.store(true, Ordering::Release);
        self.queue.ready.notify_all();
        if let Ok(mut worker) = self.worker.lock() {
            worker.take();
        }
    }
}

impl LiveBrowser {
    pub fn worker_active(&self) -> bool {
        self.queue.running.load(Ordering::Acquire)
    }
    pub fn recursive(&self) -> bool {
        self.recursive.load(Ordering::Acquire)
    }
    pub fn set_recursive(&self, value: bool) {
        if self.recursive.swap(value, Ordering::AcqRel) != value {
            self.invalidate();
        }
    }
    pub fn reset(&self) -> bool {
        let Ok(mut state) = self.state.lock() else {
            return false;
        };
        if state.key.is_none() {
            return false;
        }
        state.cancel.store(true, Ordering::Release);
        let generation = state.generation + 1;
        *state = State {
            generation,
            ..Default::default()
        };
        true
    }
    pub fn cancel(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.cancel.store(true, Ordering::Release);
            state.active = false;
            state.rx = None;
            state.error = Some("Loading cancelled".into());
            state.revision += 1;
        }
    }
    pub fn touch(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.revision += 1;
        }
    }
    pub fn invalidate(&self) {
        if let Ok(mut s) = self.state.lock() {
            s.cancel.store(true, Ordering::Release);
            s.key = None;
        }
    }
    pub fn status(&self) -> (u64, u64, bool, usize, Option<String>) {
        self.state
            .lock()
            .map(|s| {
                (
                    s.generation,
                    s.revision,
                    s.active,
                    s.scanned,
                    s.error.clone(),
                )
            })
            .unwrap_or_default()
    }
    pub fn poll(&self) -> bool {
        let Ok(mut s) = self.state.lock() else {
            return false;
        };
        let mut changed = false;
        for _ in 0..4 {
            let message = match s.rx.as_ref().map(|rx| rx.try_recv()) {
                Some(Ok(message)) => message,
                Some(Err(mpsc::TryRecvError::Disconnected)) if s.active => {
                    Message::Done(Some("Directory worker stopped".into()))
                }
                _ => break,
            };
            match message {
                Message::Batch(dirs, count) => {
                    s.directories.extend(dirs);
                    s.scanned += count;
                }
                Message::Done(error) => {
                    s.active = false;
                    s.error = error;
                    s.rx = None;
                }
            }
            s.revision += 1;
            changed = true;
        }
        changed
    }
    pub fn ensure(
        &self,
        root: &str,
        database: &Arc<DatabaseService>,
        watcher: Option<&Arc<photag_plugin_watcher::WatcherService>>,
        wake: &photag_ui_bridge::refresh::Wake,
    ) -> bool {
        let recursive = self.recursive();
        let Ok(mut s) = self.state.lock() else {
            return false;
        };
        if s.key.as_ref() == Some(&(root.to_owned(), recursive)) {
            return false;
        }
        if let Some(watcher) = watcher {
            watcher.set_browse_directory(Some(root), recursive);
        }
        s.cancel.store(true, Ordering::Release);
        let (tx, rx) = mpsc::sync_channel(4);
        let generation = s.generation + 1;
        *s = State {
            key: Some((root.into(), recursive)),
            cancel: Arc::new(AtomicBool::new(false)),
            rx: Some(rx),
            generation,
            active: true,
            ..Default::default()
        };
        let request = Request {
            root: root.into(),
            recursive,
            cancel: s.cancel.clone(),
            tx,
            database: database.clone(),
            watcher: watcher.cloned(),
            wake: wake.clone(),
        };
        if let Ok(mut pending) = self.queue.request.lock() {
            *pending = Some(request);
        }
        self.queue.ready.notify_one();
        if let Ok(mut worker) = self.worker.lock() {
            if worker.is_none() {
                let queue = self.queue.clone();
                *worker = Some(std::thread::spawn(move || {
                    loop {
                        let mut pending = queue.request.lock().unwrap();
                        while pending.is_none() && !queue.stop.load(Ordering::Acquire) {
                            pending = queue.ready.wait(pending).unwrap();
                        }
                        if queue.stop.load(Ordering::Acquire) {
                            break;
                        }
                        let request = pending.take().unwrap();
                        drop(pending);
                        if request.cancel.load(Ordering::Acquire) {
                            continue;
                        }
                        queue.running.store(true, Ordering::Release);
                        let result = enumerate(&request);
                        queue.running.store(false, Ordering::Release);
                        send(&request, Message::Done(result.err()));
                    }
                }));
            }
        }
        true
    }
    pub fn directories(&self) -> Vec<GalleryDirectory> {
        self.state
            .lock()
            .map(|s| s.directories.clone())
            .unwrap_or_default()
    }
}

fn send(request: &Request, mut message: Message) -> bool {
    loop {
        if request.cancel.load(Ordering::Acquire) {
            return false;
        }
        match request.tx.try_send(message) {
            Ok(()) => {
                request.wake.trigger();
                return true;
            }
            Err(mpsc::TrySendError::Disconnected(_)) => return false,
            Err(mpsc::TrySendError::Full(value)) => {
                message = value;
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }
}

fn enumerate(request: &Request) -> Result<(), String> {
    let started = Instant::now();
    while request
        .watcher
        .as_ref()
        .is_some_and(|w| w.browse_registration_pending())
    {
        if request.cancel.load(Ordering::Acquire) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    if request.cancel.load(Ordering::Acquire) {
        return Ok(());
    }
    let root = PathBuf::from(&request.root);
    let mut stack = vec![root.clone()];
    // Keep annotations even if the corresponding local file disappeared.
    request
        .database
        .begin_directory_cache(&request.root)
        .map_err(|e| e.to_string())?;
    let mut batch = Vec::with_capacity(256);
    let mut dirs = Vec::new();
    let mut first = true;
    let mut last_flush = Instant::now();
    let mut warnings = Vec::new();
    while let Some(directory) = stack.pop() {
        if request.cancel.load(Ordering::Acquire) {
            return Ok(());
        }
        let entries = match std::fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) if directory == root => {
                return Err(format!("{}: {error}", directory.display()));
            }
            Err(error) => {
                if warnings.len() < 8 {
                    warnings.push(format!("{}: {error}", directory.display()));
                }
                continue;
            }
        };
        for entry in entries {
            if request.cancel.load(Ordering::Acquire) {
                return Ok(());
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    if warnings.len() < 8 {
                        warnings.push(error.to_string());
                    }
                    continue;
                }
            };
            let path = entry.path();
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_dir() || kind.is_symlink() && path.is_dir() {
                if directory == root {
                    dirs.push(GalleryDirectory {
                        name: entry.file_name().to_string_lossy().into_owned(),
                        path: path.to_string_lossy().into_owned(),
                    });
                }
                if request.recursive && !kind.is_symlink() {
                    stack.push(path);
                }
            } else if kind.is_file() || kind.is_symlink() && path.is_file() {
                let metadata = if kind.is_symlink() {
                    std::fs::metadata(&path)
                } else {
                    entry.metadata()
                };
                batch.push((path, metadata));
            }
            let buffered = batch.len() + dirs.len();
            if buffered >= if first { 32 } else { 2048 }
                || buffered > 0 && last_flush.elapsed() >= Duration::from_millis(32)
            {
                let count = commit(request, &mut batch)?;
                if !send(request, Message::Batch(std::mem::take(&mut dirs), count)) {
                    return Ok(());
                }
                first = false;
                last_flush = Instant::now();
            }
        }
    }
    let count = commit(request, &mut batch)?;
    send(request, Message::Batch(dirs, count));
    tracing::info!(path=%request.root, recursive=request.recursive, duration_ms=started.elapsed().as_secs_f64()*1000., "local directory enumeration completed");
    if warnings.is_empty() {
        Ok(())
    } else {
        Err(warnings.join("; "))
    }
}

fn commit(request: &Request, batch: &mut Vec<LocalEntry>) -> Result<usize, String> {
    // Reuse DirEntry metadata (cached native enumeration data on Windows),
    // avoiding a second path lookup for every ordinary file.
    let entries = batch
        .drain(..)
        .filter_map(|(path, metadata)| {
            let metadata = match metadata {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
                Err(_) => return Some((path, 0, 0, "unreadable".to_owned())),
            };
            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .unwrap_or_default();
            let version = format!("{}:{}", metadata.len(), modified.as_nanos());
            Some((
                path,
                metadata.len() as i64,
                modified.as_secs() as i64,
                version,
            ))
        })
        .collect::<Vec<_>>();
    let count = entries.len();
    request.database.with_cache_writer(|conn| {
        conn.execute_batch("SAVEPOINT local_batch")?;
        let mut previous_query=conn.prepare_cached("SELECT version FROM file_versions WHERE full_path=?1")?;
        let mut upsert=conn.prepare_cached("INSERT INTO media(folder_id,filename,full_path,media_type,file_size,modified_at) VALUES(0,?1,?2,?3,?4,?5) ON CONFLICT(full_path) DO UPDATE SET folder_id=0,filename=excluded.filename,media_type=excluded.media_type,file_size=excluded.file_size,modified_at=excluded.modified_at,thumb_path=CASE WHEN ?6 THEN NULL ELSE media.thumb_path END,width=CASE WHEN ?6 THEN NULL ELSE media.width END,height=CASE WHEN ?6 THEN NULL ELSE media.height END,micro_thumb=CASE WHEN ?6 THEN NULL ELSE media.micro_thumb END")?;
        let mut live=conn.prepare_cached("INSERT OR IGNORE INTO live_paths VALUES(?1)")?;
        let mut versions=conn.prepare_cached("INSERT OR REPLACE INTO file_versions VALUES(?1,?2)")?;
        for (path,size,mtime,version) in entries {
            if request.cancel.load(Ordering::Acquire) {break;}
            let path = path.to_string_lossy();
            let previous: Option<String> = previous_query.query_row([path.as_ref()],|row|row.get(0)).ok();
            let filename=Path::new(path.as_ref()).file_name().unwrap_or_default().to_string_lossy();
            let ext=Path::new(path.as_ref()).extension().unwrap_or_default().to_string_lossy();
            upsert.execute(rusqlite::params![filename,path.as_ref(),photag_plugin_indexing::scan::classify_media_type(&ext),size,mtime,previous.as_deref()!=Some(&version)])?;
            live.execute([path.as_ref()])?;
            versions.execute(rusqlite::params![path.as_ref(),version])?;
        }
        conn.execute_batch("RELEASE local_batch")?;
        Ok(())
    }).map_err(|e|e.to_string())?;
    Ok(count)
}
