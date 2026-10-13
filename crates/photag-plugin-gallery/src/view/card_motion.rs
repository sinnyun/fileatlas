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

pub(super) fn hover_overlay(
    media: &photag_plugin_database::db::queries::MediaItem,
    image_size: f32,
    hover: f32,
) -> Div {
    let height = (image_size * 0.45).clamp(20., 54.);
    let cap = theme::THUMB_ROUND.min(height * 0.5);
    let alpha = 200. * hover;
    let seam = (alpha * (1. - cap / height)) as u8;
    let mut metadata = Vec::new();
    if let (Some(w), Some(h)) = (media.width, media.height) {
        metadata.push(format!("{w}×{h}"));
    }
    let timestamp = media.date_taken.unwrap_or(media.modified_at);
    if timestamp > 0 {
        use chrono::{Datelike, Local};
        if let Some(date) = chrono::DateTime::from_timestamp(timestamp, 0) {
            let date = date.with_timezone(&Local);
            metadata.push(format!(
                "{} {}, {}",
                date.format("%b"),
                date.day(),
                date.year()
            ));
        }
    }
    let mut overlay = div()
        .absolute()
        .left_0()
        .right_0()
        .bottom_0()
        .h(px(height))
        .child(
            div()
                .absolute()
                .inset_0()
                .bottom(px(cap))
                .bg(linear_gradient(
                    180.,
                    linear_color_stop(rgba(0), 0.),
                    linear_color_stop(rgba(u32::from(seam)), 1.),
                )),
        )
        .child(
            div()
                .absolute()
                .bottom_0()
                .left_0()
                .right_0()
                .h(px(cap))
                .bg(rgba(u32::from((u16::from(seam) + alpha as u16) / 2))),
        );
    let has_metadata = height >= 36. && !metadata.is_empty();
    overlay = overlay.child(
        div()
            .absolute()
            .left(px(8.))
            .right(px(8.))
            .bottom(px(if has_metadata { 22. } else { 8. }))
            .text_size(px(12.))
            .text_color(rgba(0xffffff00 | (250. * hover) as u32))
            .overflow_hidden()
            .text_ellipsis()
            .child(media.filename.clone()),
    );
    if has_metadata {
        overlay = overlay.child(
            div()
                .absolute()
                .left(px(8.))
                .right(px(8.))
                .bottom(px(8.))
                .text_size(px(10.))
                .text_color(rgba(0xdcdcdc00 | (200. * hover) as u32))
                .overflow_hidden()
                .text_ellipsis()
                .child(metadata.join(" · ")),
        );
    }
    overlay
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
