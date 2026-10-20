//! Observe production streams without consuming the caller's progress channel.
use crate::ScanProgress;
use std::collections::BTreeMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
    mpsc,
};

#[derive(Default)]
pub(super) struct ProgressRegistry {
    next: AtomicU64,
    active: Arc<Mutex<BTreeMap<u64, ScanProgress>>>,
}

impl ProgressRegistry {
    pub(super) fn current(&self) -> Option<ScanProgress> {
        self.active
            .lock()
            .ok()?
            .last_key_value()
            .map(|(_, progress)| progress.clone())
    }

    pub(super) fn track(
        &self,
        input: mpsc::Receiver<Option<ScanProgress>>,
    ) -> mpsc::Receiver<Option<ScanProgress>> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let active = self.active.clone();
        if let Ok(mut scans) = active.lock() {
            scans.insert(
                id,
                ScanProgress {
                    scanned: 0,
                    total: 0,
                    phase: "Scanning…".into(),
                    error: None,
                },
            );
        }
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            let mut last_phase = String::new();
            let mut failed = false;
            let mut completed = false;
            tracing::info!(scan_id = id, "scan/backfill started");
            while let Ok(message) = input.recv() {
                if let Some(p) = &message {
                    if p.phase != last_phase || p.error.is_some() {
                        tracing::info!(scan_id=id, phase=%p.phase, scanned=p.scanned, total=p.total, "indexing progress");
                        last_phase = p.phase.clone();
                    }
                    if let Some(error) = &p.error {
                        failed = true;
                        tracing::error!(scan_id=id,%error,"indexing error");
                    }
                } else {
                    completed = true;
                }
                if let Ok(mut scans) = active.lock() {
                    if let Some(progress) = &message {
                        scans.insert(id, progress.clone());
                    } else {
                        scans.remove(&id);
                    }
                }
                let done = message.is_none();
                // Keep observing even when the consumer closes its channel.
                let _ = tx.send(message);
                if done {
                    break;
                }
            }
            tracing::info!(
                scan_id = id,
                duration_ms = started.elapsed().as_secs_f64() * 1000.,
                failed,
                completed,
                "scan/backfill stream ended"
            );
            if let Ok(mut scans) = active.lock() {
                scans.remove(&id);
            }
        });
        rx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlapping_streams_keep_active_progress_and_forward_completion() {
        let registry = ProgressRegistry::default();
        let (first, input) = mpsc::channel();
        let output = registry.track(input);
        first
            .send(Some(ScanProgress {
                scanned: 2,
                total: 5,
                phase: "First".into(),
                error: None,
            }))
            .unwrap();
        assert_eq!(output.recv().unwrap().unwrap().scanned, 2);
        let (second, input) = mpsc::channel();
        let output2 = registry.track(input);
        second
            .send(Some(ScanProgress {
                scanned: 3,
                total: 8,
                phase: "Second".into(),
                error: None,
            }))
            .unwrap();
        output2.recv().unwrap();
        assert_eq!(registry.current().unwrap().phase, "Second");
        second.send(None).unwrap();
        assert!(output2.recv().unwrap().is_none());
        assert_eq!(registry.current().unwrap().phase, "First");
        first.send(None).unwrap();
        assert!(output.recv().unwrap().is_none());
        assert!(registry.current().is_none());
    }
}
