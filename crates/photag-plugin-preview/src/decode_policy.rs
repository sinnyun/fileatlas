//! Request ownership and demand-driven progressive work. No filesystem IO here.
use super::PreviewService;
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Default)]
pub(super) struct DecodeControl {
    pub generation: u64,
    pub cancelled: Arc<AtomicBool>,
    pub full_ready: Arc<AtomicBool>,
    progressive_requested: bool,
}

impl DecodeControl {
    fn claim_progressive(&mut self) -> bool {
        if self.progressive_requested
            || self.cancelled.load(Ordering::Acquire)
            || self.full_ready.load(Ordering::Acquire)
        {
            return false;
        }
        self.progressive_requested = true;
        true
    }
}

pub(super) fn supports_fast_preview(path: &Path) -> bool {
    path.extension()
        .and_then(|v| v.to_str())
        .is_some_and(|v| v.eq_ignore_ascii_case("jpg") || v.eq_ignore_ascii_case("jpeg"))
}

impl PreviewService {
    pub(super) fn reset_decode_request(&self) {
        if let Ok(mut control) = self.decode_control.lock() {
            control.cancelled.store(true, Ordering::Release);
            control.full_ready.store(true, Ordering::Release);
            *control = DecodeControl {
                generation: control.generation.wrapping_add(1),
                ..Default::default()
            };
        }
        self.discard_cancelled_jobs();
    }

    pub(super) fn cancel_decode_request(&self) {
        if let Ok(control) = self.decode_control.lock() {
            control.cancelled.store(true, Ordering::Release);
        }
        self.discard_cancelled_jobs();
    }

    pub(super) fn mark_full_image_ready(&self) {
        if let Ok(control) = self.decode_control.lock() {
            control.full_ready.store(true, Ordering::Release);
        }
        self.discard_cancelled_jobs();
    }

    fn discard_cancelled_jobs(&self) {
        let removed = self.decode_queue.remove_cancelled();
        if let Ok(mut pending) = self.pending_decodes.lock() {
            for job in removed {
                pending.remove(&(job.source.clone(), job.max_dim, job.generation));
                tracing::debug!(target: "photag_plugin_preview", source=%job.source.display(),
                    max_dim=job.max_dim, generation=job.generation, reason=job.cancellation_reason(),
                    "preview queued decode cancelled");
            }
        }
    }

    /// Sampled by the shared view poller. Native full images get a 150ms head start.
    pub(super) fn request_progressive_if_pending(&self) {
        if let Some(source) = self.pending_progressive_source() {
            if supports_fast_preview(&source) {
                let claimed = self
                    .decode_control
                    .lock()
                    .is_ok_and(|mut control| control.claim_progressive());
                if !claimed {
                    return;
                }
                self.prewarm_image(&source, 1600);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_progressive_work_is_not_retried_by_each_poll() {
        let mut request = DecodeControl::default();
        assert!(request.claim_progressive());
        for _ in 0..100 {
            assert!(!request.claim_progressive());
        }
        let mut completed = DecodeControl::default();
        completed.full_ready.store(true, Ordering::Release);
        assert!(!completed.claim_progressive());
        let mut closed = DecodeControl::default();
        closed.cancelled.store(true, Ordering::Release);
        assert!(!closed.claim_progressive());
        assert!(DecodeControl::default().claim_progressive());
    }
}
