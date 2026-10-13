//! GPUI transport controls for the FFmpeg-backed player.

use std::sync::Arc;

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::slider::{Slider, SliderEvent, SliderState, SliderValue};
use gpui_kit::*;
use photag_ui_bridge::{UiViewContribution, theme};

use crate::MediaPlaybackService;

pub fn media_playback_view(service: Arc<MediaPlaybackService>) -> UiViewContribution {
    UiViewContribution::new(
        "media-playback/transport-view",
        "workspace.main",
        10,
        move |_, app| {
            let view_service = Arc::clone(&service);
            app.new(|cx| {
                let mut revision = service.snapshot().revision;
                let playhead = cx.new(|_| {
                    SliderState::new()
                        .min(0.0)
                        .max(1000.0)
                        .step(1.0)
                        .default_value(0.0)
                });
                let volume = cx.new(|_| {
                    SliderState::new()
                        .min(0.0)
                        .max(1.5)
                        .step(0.01)
                        .default_value(service.snapshot().volume)
                });
                let speed = cx.new(|_| {
                    SliderState::new()
                        .min(0.1)
                        .max(2.0)
                        .step(0.05)
                        .default_value(service.snapshot().speed.clamp(0.1, 2.0))
                });
                let playhead_subscription = cx.subscribe(
                    &playhead,
                    |view: &mut PlaybackView, _, event: &SliderEvent, cx| match event {
                        SliderEvent::Change(SliderValue::Single(value)) => {
                            view.playhead_value = Some(*value);
                            cx.notify();
                        }
                        SliderEvent::Release(SliderValue::Single(value)) => {
                            let duration = view.service.snapshot().duration_seconds;
                            let _ = view.service.seek(duration * (*value as f64 / 1000.0));
                            view.playhead_value = None;
                            cx.notify();
                        }
                        _ => {}
                    },
                );
                let volume_subscription = cx.subscribe(
                    &volume,
                    |view: &mut PlaybackView, _, event: &SliderEvent, cx| match event {
                        SliderEvent::Change(SliderValue::Single(value))
                        | SliderEvent::Release(SliderValue::Single(value)) => {
                            view.service.set_volume(*value);
                            cx.notify();
                        }
                        _ => {}
                    },
                );
                let speed_subscription = cx.subscribe(
                    &speed,
                    |view: &mut PlaybackView, _, event: &SliderEvent, cx| match event {
                        SliderEvent::Change(SliderValue::Single(value))
                        | SliderEvent::Release(SliderValue::Single(value)) => {
                            let _ = view.service.set_speed(*value);
                            cx.notify();
                        }
                        _ => {}
                    },
                );
                cx.spawn(async move |this, cx| {
                    loop {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(33))
                            .await;
                        let active = this
                            .update(cx, |view: &mut PlaybackView, cx| {
                                let snapshot = view.service.snapshot();
                                if snapshot.revision != revision || snapshot.playing {
                                    revision = snapshot.revision;
                                    cx.notify();
                                }
                            })
                            .is_ok();
                        if !active {
                            break;
                        }
                    }
                })
                .detach();
                PlaybackView {
                    service: view_service,
                    playhead,
                    volume,
                    speed,
                    playhead_value: None,
                    speed_popup_open: false,
                    _playhead_subscription: playhead_subscription,
                    _volume_subscription: volume_subscription,
                    _speed_subscription: speed_subscription,
                }
            })
            .into()
        },
    )
}

struct PlaybackView {
    service: Arc<MediaPlaybackService>,
    playhead: Entity<SliderState>,
    volume: Entity<SliderState>,
    speed: Entity<SliderState>,
    playhead_value: Option<f32>,
    speed_popup_open: bool,
    _playhead_subscription: Subscription,
    _volume_subscription: Subscription,
    _speed_subscription: Subscription,
}

impl Render for PlaybackView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.service.snapshot();
        let seek_value = self.playhead_value.unwrap_or_else(|| {
            if snapshot.duration_seconds > 0.0 {
                (snapshot.position_seconds / snapshot.duration_seconds * 1000.0) as f32
            } else {
                0.0
            }
        });
        let current_seek_value = match self.playhead.read(cx).value() {
            SliderValue::Single(value) => value,
            SliderValue::Range(_, end) => end,
        };
        if (current_seek_value - seek_value).abs() > 0.5 {
            self.playhead
                .update(cx, |state, cx| state.set_value(seek_value, window, cx));
        }
        let volume_value = snapshot.volume;
        let current_volume = match self.volume.read(cx).value() {
            SliderValue::Single(value) => value,
            SliderValue::Range(_, end) => end,
        };
        if (current_volume - volume_value).abs() > 0.005 {
            self.volume
                .update(cx, |state, cx| state.set_value(volume_value, window, cx));
        }
        let speed_value = snapshot.speed.clamp(0.1, 2.0);
        let current_speed = match self.speed.read(cx).value() {
            SliderValue::Single(value) => value,
            SliderValue::Range(_, end) => end,
        };
        if (current_speed - speed_value).abs() > 0.005 {
            self.speed
                .update(cx, |state, cx| state.set_value(speed_value, window, cx));
        }
        let toggle_service = Arc::clone(&self.service);
        let toggle = cx.listener(move |_, _: &ClickEvent, _, cx| {
            let _ = toggle_service.toggle_play_pause();
            cx.notify();
        });
        let back_service = Arc::clone(&self.service);
        let back = cx.listener(move |_, _: &ClickEvent, _, cx| {
            let s = back_service.snapshot();
            let _ = back_service.seek(s.position_seconds - 5.0);
            cx.notify();
        });
        let forward_service = Arc::clone(&self.service);
        let forward = cx.listener(move |_, _: &ClickEvent, _, cx| {
            let s = forward_service.snapshot();
            let _ = forward_service.seek(s.position_seconds + 5.0);
            cx.notify();
        });
        let prev_service = Arc::clone(&self.service);
        let previous_frame = cx.listener(move |_, _: &ClickEvent, _, cx| {
            let _ = prev_service.step_frames(-1);
            cx.notify();
        });
        let next_service = Arc::clone(&self.service);
        let next_frame = cx.listener(move |_, _: &ClickEvent, _, cx| {
            let _ = next_service.step_frames(1);
            cx.notify();
        });
        let close_service = Arc::clone(&self.service);
        let close = cx.listener(move |_, _: &ClickEvent, _, cx| {
            close_service.close();
            cx.notify();
        });
        let speed_popup = cx.listener(|view, _: &ClickEvent, _, cx| {
            view.speed_popup_open = !view.speed_popup_open;
            cx.notify();
        });
        let controls = div()
            .h_flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(button("−5s").on_click(back))
            .child(button("Prev frame").on_click(previous_frame))
            .child(button(if snapshot.playing { "Pause" } else { "Play" }).on_click(toggle))
            .child(button("Next frame").on_click(next_frame))
            .child(button("+5s").on_click(forward))
            .child(button(format!("{:.2}×", snapshot.speed)).on_click(speed_popup))
            .child(div().text_xs().child("Volume"))
            .child(Slider::new(&self.volume).w_32())
            .child(button("Close").on_click(close));

        let frame = match &snapshot.frame_path {
            Some(path) => img(path.clone())
                .max_w_full()
                .max_h_full()
                .object_fit(ObjectFit::Contain)
                .into_any_element(),
            None if snapshot.path.is_some() => div()
                .text_sm()
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child("Decoding video frame…")
                .into_any_element(),
            None => div()
                .text_sm()
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child("Open a video to play it.")
                .into_any_element(),
        };
        let title = snapshot
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "No media selected".into());
        let progress = format!(
            "{} / {}  ·  {:.2}×",
            format_time(snapshot.position_seconds),
            format_time(snapshot.duration_seconds),
            snapshot.speed
        );
        let speed_controls = if self.speed_popup_open {
            let presets = [0.25_f32, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0].into_iter().fold(
                div().h_flex().flex_wrap().items_center().gap_1(),
                |row, speed| {
                    let speed_service = Arc::clone(&self.service);
                    let choose_speed = cx.listener(move |view, _: &ClickEvent, _, cx| {
                        let _ = speed_service.set_speed(speed);
                        view.speed_popup_open = false;
                        cx.notify();
                    });
                    row.child(button(format!("{speed:.2}×")).on_click(choose_speed))
                },
            );
            let boosts = [3.0_f32, 4.0, 8.0].into_iter().fold(
                div().h_flex().items_center().gap_1(),
                |row, speed| {
                    let speed_service = Arc::clone(&self.service);
                    let choose_speed = cx.listener(move |view, _: &ClickEvent, _, cx| {
                        let _ = speed_service.set_speed(speed);
                        view.speed_popup_open = false;
                        cx.notify();
                    });
                    row.child(button(format!("{speed:.0}×")).on_click(choose_speed))
                },
            );
            let slower_service = Arc::clone(&self.service);
            let slower = cx.listener(move |_, _: &ClickEvent, _, cx| {
                let speed = slower_service.snapshot().speed - 0.05;
                let _ = slower_service.set_speed(speed);
                cx.notify();
            });
            let faster_service = Arc::clone(&self.service);
            let faster = cx.listener(move |_, _: &ClickEvent, _, cx| {
                let speed = faster_service.snapshot().speed + 0.05;
                let _ = faster_service.set_speed(speed);
                cx.notify();
            });
            div()
                .v_flex()
                .gap_1()
                .child(
                    div()
                        .h_flex()
                        .items_center()
                        .gap_2()
                        .child(div().text_xs().child("Speed"))
                        .child(Slider::new(&self.speed).w_48())
                        .child(format!("{:.2}×", snapshot.speed)),
                )
                .child(presets)
                .child(
                    div()
                        .h_flex()
                        .items_center()
                        .gap_1()
                        .child(button("−0.05×").on_click(slower))
                        .child(button("+0.05×").on_click(faster))
                        .child(boosts),
                )
        } else {
            div()
        };
        let audio = if snapshot.audio_available {
            "Audio enabled"
        } else {
            "Audio output unavailable"
        };
        div()
            .v_flex()
            .flex_1()
            .min_h_0()
            .gap_2()
            .p_3()
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_sm().font_medium().child(title))
                    .child(controls),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .items_center()
                    .justify_center()
                    .bg(rgba(theme::base_rgba(theme::SURFACE_BASE)))
                    .child(frame),
            )
            .child(Slider::new(&self.playhead).w_full().h_2())
            .child(speed_controls)
            .child(
                div()
                    .h_flex()
                    .justify_between()
                    .text_xs()
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child(progress)
                    .child(format!(
                        "{}  ·  rotation {}°",
                        audio,
                        snapshot.rotation_quarters * 90
                    )),
            )
    }
}

fn button(label: impl Into<String>) -> Button {
    let label = label.into();
    Button::new(format!("media-playback-{label}")).label(label)
}
fn format_time(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    if total >= 3600 {
        format!(
            "{:02}:{:02}:{:02}",
            total / 3600,
            total % 3600 / 60,
            total % 60
        )
    } else {
        format!("{:02}:{:02}", total / 60, total % 60)
    }
}
