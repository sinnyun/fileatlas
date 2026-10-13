//! Source gallery.rs:1411-1453, 1509-1516, 1562-1590, 1750-1820.
use super::*;
use std::time::Instant;

pub(super) struct CardMotion {
    hover: f32,
    selection: f32,
    tick: Instant,
    targets: (bool, bool),
    selection_seconds: f32,
}

impl CardMotion {
    pub(super) fn new(selected: bool, now: Instant) -> Self {
        // egui's first animate_bool call snaps to the current state.
        Self {
            hover: 0.,
            selection: f32::from(selected),
            tick: now,
            targets: (false, selected),
            selection_seconds: 0.14,
        }
    }

    pub(super) fn with_selection_seconds(mut self, seconds: f32) -> Self {
        self.selection_seconds = seconds;
        self
    }

    pub(super) fn sample(
        &mut self,
        hovered: bool,
        selected: bool,
        now: Instant,
        reduce_motion: bool,
    ) -> (f32, f32, bool) {
        let dt = now.saturating_duration_since(self.tick).as_secs_f32();
        // Idle time before either target changes must not finish its new transition.
        let hover_dt = if self.targets.0 == hovered { dt } else { 0. };
        let selection_dt = if self.targets.1 == selected { dt } else { 0. };
        self.targets = (hovered, selected);
        self.tick = now;
        self.hover = approach(
            self.hover,
            hovered,
            if reduce_motion { 1. } else { hover_dt / 0.12 },
        );
        self.selection = approach(
            self.selection,
            selected,
            if reduce_motion {
                1.
            } else {
                selection_dt / self.selection_seconds
            },
        );
        let moving = self.hover != f32::from(hovered) || self.selection != f32::from(selected);
        (self.hover, self.selection, moving)
    }
}

fn approach(value: f32, target: bool, step: f32) -> f32 {
    if target {
        let value = (value + step).min(1.);
        if value >= 1. - f32::EPSILON {
            1.
        } else {
            value
        }
    } else {
        let value = (value - step).max(0.);
        if value <= f32::EPSILON { 0. } else { value }
    }
}

impl GalleryView {
    pub(super) fn set_card_hover(&mut self, id: i64, hovered: bool, cx: &mut Context<Self>) {
        if hovered {
            self.hovered_cards.insert(id);
        } else {
            self.hovered_cards.remove(&id);
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::{CardMotion, Instant};
    use std::time::Duration;
    #[test]
    fn transitions_reverse_without_jumps_and_finish_at_source_durations() {
        let start = Instant::now();
        let mut motion = CardMotion::new(false, start);
        motion.sample(true, true, start, false);
        let (hover, selection, moving) =
            motion.sample(true, true, start + Duration::from_millis(60), false);
        assert!((hover - 0.5).abs() < 0.001);
        assert!((selection - 60. / 140.).abs() < 0.001);
        assert!(moving);
        motion.sample(false, true, start + Duration::from_millis(60), false);
        let (hover, _, _) = motion.sample(false, true, start + Duration::from_millis(90), false);
        assert!((hover - 0.25).abs() < 0.001);
        let (hover, selection, moving) =
            motion.sample(false, true, start + Duration::from_millis(140), false);
        assert_eq!((hover, selection, moving), (0., 1., false));
        assert_eq!(
            CardMotion::new(true, start).sample(false, true, start, false),
            (0., 1., false)
        );
    }
}
