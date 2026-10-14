//! Reconcile persisted roots before relying on future native notifications.
use super::*;

#[derive(Debug, Clone, Default)]
pub struct LibraryRefreshState {
    pub running: bool,
    pub total_folders: usize,
    pub completed_folders: usize,
    pub failures: Vec<(String, String)>,
}

pub(super) struct StartupRefresh {
    database: Arc<DatabaseService>,
    indexing: Arc<IndexingService>,
    stop: Arc<AtomicBool>,
    state: Arc<Mutex<LibraryRefreshState>>,
    cordis: Option<Context>,
    runtime: Option<tokio::runtime::Handle>,
}

impl StartupRefresh {
    pub(super) fn new(service: &WatcherService) -> Self {
        Self {
            database: service.database.clone(),
            indexing: service.indexing.clone(),
            stop: service.stop.clone(),
            state: service.refresh_state.clone(),
            cordis: service.cordis.clone(),
            runtime: service.runtime.clone(),
        }
    }

    // Own resources rather than Arc<WatcherService>: plugin stop can drop the
    // service, signal cancellation and join this worker even during a scan.
    pub(super) fn run(self) {
        let folders = match self.database.with_reader(queries::list_folders) {
            Ok(folders) => folders,
            Err(error) => {
                self.failure("", error.to_string());
                self.finish();
                return;
            }
        };
        if let Ok(mut state) = self.state.lock() {
            state.total_folders = folders.len();
        }
        for folder in folders {
            if self.stop.load(Ordering::Acquire) {
                break;
            }
            // The production scanner preserves existing identities/metadata and
            // reports inaccessible roots without clearing their cached records.
            let progress =
                self.indexing
                    .scan_folder(folder.id, folder.path.clone(), self.stop.clone());
            for message in progress {
                match message {
                    Some(progress) => {
                        if let Some(error) = progress.error {
                            self.failure(&folder.path, error);
                        }
                        // Index commits can be shown before thumbnail work finishes.
                        if progress.phase.starts_with("Indexing files") {
                            self.notify();
                        }
                    }
                    None => break,
                }
            }
            self.notify();
            if let Ok(mut state) = self.state.lock() {
                state.completed_folders += 1;
            }
        }
        self.finish();
    }

    fn failure(&self, path: &str, error: String) {
        eprintln!("PhoTag startup library refresh failed for {path}: {error}");
        if let Ok(mut state) = self.state.lock() {
            state.failures.push((path.into(), error));
        }
    }

    fn finish(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.running = false;
        }
    }

    fn notify(&self) {
        if self.stop.load(Ordering::Acquire) {
            return;
        }
        if let (Some(cordis), Some(runtime)) = (&self.cordis, &self.runtime) {
            if let Err(error) =
                runtime.block_on(cordis.emit::<MediaIndexChanged>(Routing::Unscoped, ()))
            {
                eprintln!("startup index notification failed: {error}");
            }
        }
    }
}
