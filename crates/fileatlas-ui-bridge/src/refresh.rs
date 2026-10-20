//! Shared change detection and lifecycle-owned UI polling.
//!
//! A timer is a sampling opportunity, never evidence that the UI changed.
//! Pollers return a result; this module owns the single notification per tick.
use gpui_kit::Context;
use std::time::Duration;

/// Coalesced readiness hint. One permit, no queued payloads and no idle timer.
#[derive(Clone, Default)]
pub struct Wake(std::sync::Arc<tokio::sync::Notify>);
impl Wake {
    pub fn trigger(&self) {
        self.0.notify_one();
    }
    pub async fn wait(&self) {
        self.0.notified().await;
    }
}

/// Store the returned task in the owning view; dropping it cancels the listener.
/// A wake only asks the shared sampler to check for real changes.
pub fn listen<V: 'static>(
    cx: &mut Context<V>,
    wake: Wake,
    mut sample: impl FnMut(&mut V, &mut Context<V>) -> bool + 'static,
) -> gpui_kit::Task<()> {
    cx.spawn(async move |this, cx| {
        loop {
            wake.wait().await;
            if this
                .update(cx, |view, cx| {
                    if sample(view, cx) {
                        cx.notify();
                    }
                })
                .is_err()
            {
                break;
            }
        }
    })
}

#[derive(Debug)]
pub struct ChangeGate<T>(Option<T>);

impl<T> Default for ChangeGate<T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<T: PartialEq> ChangeGate<T> {
    pub fn new(value: T) -> Self {
        Self(Some(value))
    }

    /// The first sample changes an uninitialized gate. Repeated samples don't.
    pub fn changed(&mut self, next: T) -> bool {
        if self.0.as_ref() == Some(&next) {
            false
        } else {
            self.0 = Some(next);
            true
        }
    }

    pub fn current(&self) -> Option<&T> {
        self.0.as_ref()
    }
}

pub struct PollResult {
    changed: bool,
    active: bool,
}

impl PollResult {
    /// Report a finite operation's last sample, including any final UI change.
    pub fn until(changed: bool, active: bool) -> Self {
        Self { changed, active }
    }
}

impl From<bool> for PollResult {
    fn from(changed: bool) -> Self {
        Self::until(changed, true)
    }
}

/// The task holds a weak entity: disposal stops it automatically. Callbacks
/// update models and return changes instead of calling notify themselves.
/// Active playback/animation returns true only when its frame/position changes.
pub fn poll<V: 'static>(
    cx: &mut Context<V>,
    interval: Duration,
    mut sample: impl FnMut(&mut V, &mut Context<V>) -> PollResult + 'static,
) {
    cx.spawn(async move |this, cx| {
        loop {
            cx.background_executor().timer(interval).await;
            let active = this
                .update(cx, |view, cx| {
                    let result = sample(view, cx);
                    if result.changed {
                        cx.notify();
                    }
                    result.active
                })
                .unwrap_or(false);
            if !active {
                break;
            }
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn readiness_is_coalesced_and_stays_idle_without_new_work() {
        let wake = Wake::default();
        for _ in 0..1000 {
            wake.trigger();
        }
        wake.wait().await;
        assert!(
            tokio::time::timeout(Duration::from_millis(10), wake.wait())
                .await
                .is_err()
        );
        wake.trigger();
        wake.wait().await;
    }

    #[test]
    fn unchanged_samples_and_stationary_input_do_not_invalidate() {
        let mut gate = ChangeGate::default();
        assert!(gate.changed((4, Some((12, 42)))));
        for _ in 0..1000 {
            assert!(!gate.changed((4, Some((12, 42)))));
        }
        assert!(gate.changed((5, Some((12, 42)))));
        assert!(gate.changed((5, Some((13, 42)))));
    }

    #[test]
    fn playback_changes_advance_and_completion_keeps_final_notification() {
        let mut position = ChangeGate::new((true, 0));
        for frame in 1..30 {
            assert!(position.changed((true, frame)));
        }
        assert!(position.changed((false, 29)));
        assert!(!position.changed((false, 29)));
        let finished = PollResult::until(true, false);
        assert!(finished.changed);
        assert!(!finished.active);
    }
}
