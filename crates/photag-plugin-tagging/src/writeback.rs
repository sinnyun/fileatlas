//! Observable results for best-effort metadata writeback jobs.
//!
//! PhoTag schedules these writes after the database mutation. Keep that
//! non-blocking behavior while retaining failures so UI consumers can report
//! them and users can retry after fixing the underlying file problem.

use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use photag_plugin_database::{DatabaseService, db::queries};

const MAX_RECORDED_FAILURES: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WritebackFailure {
    pub path: PathBuf,
    pub error: String,
}

#[derive(Default)]
struct State {
    generation: u64,
    current: BTreeMap<PathBuf, u64>,
    queue: VecDeque<Job>,
    worker_running: bool,
    failure_sequence: u64,
    failures: BTreeMap<PathBuf, (u64, WritebackFailure)>,
}

struct Job {
    key: PathBuf,
    path: PathBuf,
    tags: Vec<String>,
    generation: u64,
}

fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(State::default()))
}

/// Schedule writeback for media IDs after their tag mutation has committed.
/// Database lookup errors are returned to the caller instead of disappearing.
pub(crate) fn schedule(database: &DatabaseService, media_ids: &[i64]) -> Result<(), ScheduleError> {
    if media_ids.is_empty() {
        return Ok(());
    }
    let entries = database
        .with_reader(|connection| queries::get_paths_and_current_tags(connection, media_ids))?;
    if entries.is_empty() {
        return Ok(());
    }

    let start_worker = {
        let mut state = state().lock().map_err(|_| ScheduleError::StatePoisoned)?;
        for (path, tags) in entries {
            let path = PathBuf::from(path);
            let key = path_key(&path);
            state.generation = state.generation.wrapping_add(1).max(1);
            let generation = state.generation;
            state.current.insert(key.clone(), generation);
            state.failures.remove(&key);
            // Coalesce pending writes to the same path. An already-running
            // write may finish, but the single worker applies this newer job
            // afterward so stale data cannot be the final file contents.
            state.queue.retain(|job| job.key != key);
            state.queue.push_back(Job {
                key,
                path,
                tags,
                generation,
            });
        }
        if state.worker_running {
            false
        } else {
            state.worker_running = true;
            true
        }
    };

    if start_worker {
        if let Err(error) = std::thread::Builder::new()
            .name("photag-tag-writeback".into())
            .spawn(run_worker)
        {
            if let Ok(mut state) = state().lock() {
                state.worker_running = false;
                let jobs = state.queue.drain(..).collect::<Vec<_>>();
                for job in jobs {
                    if state.current.get(&job.key) == Some(&job.generation) {
                        state.current.remove(&job.key);
                        record_failure(
                            &mut state,
                            job.key,
                            job.path,
                            format!("could not start metadata writer: {error}"),
                        );
                    }
                }
            }
            return Err(ScheduleError::WorkerStart(error));
        }
    }
    Ok(())
}

fn run_worker() {
    loop {
        let job = {
            let Ok(mut state) = state().lock() else {
                return;
            };
            let mut next = None;
            while let Some(job) = state.queue.pop_front() {
                if state.current.get(&job.key) == Some(&job.generation) {
                    next = Some(job);
                    break;
                }
            }
            let Some(job) = next else {
                state.worker_running = false;
                return;
            };
            job
        };

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            super::xmp::write_metadata(&job.path.to_string_lossy(), &job.tags)
        }))
        .unwrap_or_else(|_| {
            Err(std::io::Error::other(
                "metadata writer panicked while processing this path",
            ))
        });
        if let Ok(mut state) = state().lock()
            && state.current.get(&job.key) == Some(&job.generation)
        {
            state.current.remove(&job.key);
            if let Err(error) = result {
                record_failure(&mut state, job.key, job.path, error.to_string());
            }
        }
    }
}

fn record_failure(state: &mut State, key: PathBuf, path: PathBuf, error: String) {
    state.failure_sequence = state.failure_sequence.wrapping_add(1).max(1);
    let sequence = state.failure_sequence;
    state
        .failures
        .insert(key, (sequence, WritebackFailure { path, error }));
    while state.failures.len() > MAX_RECORDED_FAILURES {
        if let Some(oldest) = state
            .failures
            .iter()
            .min_by_key(|(_, (sequence, _))| sequence)
            .map(|(path, _)| path.clone())
        {
            state.failures.remove(&oldest);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ScheduleError {
    #[error(transparent)]
    Database(#[from] photag_plugin_database::DatabaseAccessError),
    #[error("metadata writeback state lock is poisoned")]
    StatePoisoned,
    #[error("could not start metadata writeback worker: {0}")]
    WorkerStart(#[source] std::io::Error),
}

/// Return the failures currently awaiting UI acknowledgement.
pub fn failures() -> Vec<WritebackFailure> {
    let Ok(state) = state().lock() else {
        return Vec::new();
    };
    let mut failures = state
        .failures
        .values()
        .map(|(_, failure)| failure.clone())
        .collect::<Vec<_>>();
    failures.sort_by(|left, right| left.path.cmp(&right.path));
    failures
}

/// Acknowledge a displayed failure, or clear it after a caller retries.
pub fn clear_failure(path: impl AsRef<std::path::Path>) {
    if let Ok(mut state) = state().lock() {
        state.failures.remove(&path_key(path.as_ref()));
    }
}

#[cfg(windows)]
fn path_key(path: &std::path::Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().to_lowercase())
}

#[cfg(not(windows))]
fn path_key(path: &std::path::Path) -> PathBuf {
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::{clear_failure, failures, schedule};
    use photag_plugin_database::{DatabaseService, db::queries};
    use std::sync::Arc;
    use std::time::{Duration, SystemTime};

    #[test]
    fn failed_background_write_is_visible_and_can_be_cleared() {
        let root = std::env::temp_dir().join(format!(
            "photag-writeback-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let database = Arc::new(DatabaseService::open_at(root.join("photag.sqlite")).unwrap());
        let missing_photo = root.join("missing.jpg");
        let media_id = database
            .with_writer(|connection| {
                let folder = queries::add_folder(connection, root.to_str().unwrap())?;
                let media_id = queries::insert_media(
                    connection,
                    folder.id,
                    "missing.jpg",
                    missing_photo.to_str().unwrap(),
                    "image",
                    0,
                    1_700_000_000,
                    None,
                    None,
                    None,
                )?;
                let tag = queries::create_tag(connection, "Retry later", None)?;
                queries::assign_tags(connection, &[media_id], &[tag.id])?;
                Ok(media_id)
            })
            .unwrap();

        schedule(&database, &[media_id]).unwrap();
        let failure = (0..200)
            .find_map(|_| {
                let found = failures()
                    .into_iter()
                    .find(|item| item.path == missing_photo);
                if found.is_none() {
                    std::thread::sleep(Duration::from_millis(10));
                }
                found
            })
            .expect("worker should report the missing photo write failure");
        assert!(!failure.error.is_empty());

        clear_failure(&missing_photo);
        assert!(!failures().iter().any(|item| item.path == missing_photo));
        drop(database);
        let _ = std::fs::remove_dir_all(root);
    }
}
