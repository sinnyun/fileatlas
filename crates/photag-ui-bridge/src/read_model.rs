//! Keyed background reads shared by feature views. Render may request an
//! identity and read its snapshot; the model poller delivers completions.
//! One worker per model, latest request wins, success and errors are cached.
use std::sync::mpsc::{self, Receiver, TryRecvError};

type Reader<V> = Box<dyn FnOnce() -> Result<V, String> + Send>;

pub struct ReadModel<K, V> {
    key: Option<K>,
    value: Option<Result<V, String>>,
    pending: Option<(K, Receiver<Result<V, String>>)>,
    queued: Option<(K, Reader<V>)>,
    wake: Option<crate::refresh::Wake>,
}

impl<K, V> Default for ReadModel<K, V> {
    fn default() -> Self {
        Self {
            key: None,
            value: None,
            pending: None,
            queued: None,
            wake: None,
        }
    }
}

impl<K: Clone + PartialEq, V: Send + 'static> ReadModel<K, V> {
    pub fn set_wake(&mut self, wake: crate::refresh::Wake) {
        self.wake = Some(wake);
    }
    /// Request without consuming a completion or notifying from render.
    pub fn request(
        &mut self,
        key: K,
        read: impl FnOnce() -> Result<V, String> + Send + 'static,
    ) -> bool {
        let changed = self.key.as_ref() != Some(&key);
        if changed {
            self.key = Some(key.clone());
            self.value = None;
            self.queued = None;
        }
        if self.value.is_none()
            && self
                .pending
                .as_ref()
                .is_none_or(|(pending, _)| pending != &key)
            && self
                .queued
                .as_ref()
                .is_none_or(|(queued, _)| queued != &key)
        {
            self.queued = Some((key, Box::new(read)));
        }
        self.start_queued() | changed
    }

    /// Completion delivery only. Empty polls have no visible effect. The latest
    /// queued read starts after an obsolete worker finishes, without a repaint.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        if let Some((pending_key, receiver)) = &self.pending {
            let result = match receiver.try_recv() {
                Ok(value) => Some(value),
                Err(TryRecvError::Disconnected) => Some(Err(
                    "Background read stopped before returning a result.".into(),
                )),
                Err(TryRecvError::Empty) => None,
            };
            if let Some(result) = result {
                if self.key.as_ref() == Some(pending_key) {
                    self.value = Some(result);
                    // A -> B -> A can reuse the original A worker.
                    self.queued = None;
                    changed = true;
                }
                self.pending = None;
            }
        }
        self.start_queued() | changed
    }

    /// Model/event pollers can request and consume in one operation.
    pub fn update(
        &mut self,
        key: K,
        read: impl FnOnce() -> Result<V, String> + Send + 'static,
    ) -> bool {
        self.request(key, read) | self.poll()
    }

    fn start_queued(&mut self) -> bool {
        if self.pending.is_some() {
            return false;
        }
        let Some((key, read)) = self.queued.take() else {
            return false;
        };
        let (send, receive) = mpsc::channel();
        let wake = self.wake.clone();
        match std::thread::Builder::new()
            .name("photag-read-model".into())
            .spawn(move || {
                // Also deliver a disconnected-worker error promptly after panic.
                struct Ready(Option<crate::refresh::Wake>);
                impl Drop for Ready {
                    fn drop(&mut self) {
                        if let Some(wake) = &self.0 {
                            wake.trigger();
                        }
                    }
                }
                let _ready = Ready(wake);
                let sender = send; // Drop/disconnect before Ready on panic.
                let _ = sender.send(read());
            }) {
            Ok(_) => {
                self.pending = Some((key, receive));
                false
            }
            Err(error) => {
                self.value = Some(Err(error.to_string()));
                true
            }
        }
    }

    pub fn value(&self) -> Option<&Result<V, String>> {
        self.value.as_ref()
    }
    pub fn key(&self) -> Option<&K> {
        self.key.as_ref()
    }
    pub fn is_pending(&self) -> bool {
        self.pending.is_some() || self.queued.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn completion_and_panics_wake_the_consumer_without_periodic_polling() {
        let wake = crate::refresh::Wake::default();
        let mut model = ReadModel::<u64, usize>::default();
        model.set_wake(wake.clone());
        model.request(1, || Ok(7));
        tokio::time::timeout(std::time::Duration::from_secs(3), wake.wait())
            .await
            .unwrap();
        assert!(model.poll());
        assert_eq!(model.value().unwrap().as_ref().unwrap(), &7);
        assert!(!model.request(1, || panic!("completed reads must stay cached")));
        model.request(2, || panic!("simulated read failure"));
        tokio::time::timeout(std::time::Duration::from_secs(3), wake.wait())
            .await
            .unwrap();
        assert!(model.poll());
        assert!(model.value().unwrap().is_err());
    }
    fn finish(model: &mut ReadModel<u64, usize>, key: u64) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while model.value().is_none() {
            assert!(std::time::Instant::now() < deadline);
            model.update(key, || {
                panic!("a completed/current read must not be relaunched")
            });
            std::thread::yield_now();
        }
    }

    #[test]
    fn empty_polls_coalesce_and_errors_do_not_retry_on_every_paint() {
        let mut model = ReadModel::<u64, usize>::default();
        let (send, receive) = mpsc::channel();
        assert!(model.update(1, move || {
            receive.recv().unwrap();
            Err("missing".into())
        }));
        for _ in 0..20 {
            assert!(!model.update(1, || panic!("duplicate read")));
        }
        send.send(()).unwrap();
        finish(&mut model, 1);
        assert_eq!(model.value().unwrap().as_ref().unwrap_err(), "missing");
        assert!(!model.update(1, || panic!("error must be cached")));
    }

    #[test]
    fn stale_results_are_discarded_and_latest_request_waits_for_the_one_worker() {
        let mut model = ReadModel::default();
        let (send, receive) = mpsc::channel();
        model.update(1, move || {
            receive.recv().unwrap();
            Ok(1)
        });
        model.update(2, || Ok(2));
        assert!(model.value().is_none());
        send.send(()).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while model.value().is_none() {
            assert!(std::time::Instant::now() < deadline);
            model.update(2, || Ok(2));
            std::thread::yield_now();
        }
        assert_eq!(model.value().unwrap().as_ref().unwrap(), &2);
    }

    #[test]
    fn rapid_changes_run_only_the_latest_queued_read_and_cache_worker_failure() {
        let mut model = ReadModel::<u64, usize>::default();
        let (release, blocked) = mpsc::channel();
        let (started, observed) = mpsc::channel();
        model.request(1, move || {
            blocked.recv().unwrap();
            Ok(1)
        });
        let obsolete = started.clone();
        model.request(2, move || {
            obsolete.send(2).unwrap();
            Ok(2)
        });
        model.request(3, move || {
            started.send(3).unwrap();
            Ok(3)
        });
        for _ in 0..100 {
            assert!(!model.poll());
            assert!(observed.try_recv().is_err());
        }
        release.send(()).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while model.value().is_none() {
            assert!(std::time::Instant::now() < deadline);
            model.poll();
            std::thread::yield_now();
        }
        assert_eq!(observed.recv().unwrap(), 3);
        assert!(observed.try_recv().is_err());
        assert_eq!(model.value().unwrap().as_ref().unwrap(), &3);
        model.request(4, || panic!("simulated worker failure"));
        finish(&mut model, 4);
        assert!(model.value().unwrap().is_err());
        assert!(!model.update(4, || panic!("failure must not restart")));
    }
}
