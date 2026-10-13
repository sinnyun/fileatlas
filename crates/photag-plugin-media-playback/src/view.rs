//! GPUI transport controls for the FFmpeg-backed player.

use std::sync::Arc;

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::*;
use photag_ui_bridge::UiViewContribution;

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
                }
            })
            .into()
        },
    )
}

struct PlaybackView {
    service: Arc<MediaPlaybackService>,
}

impl Render for PlaybackView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.service.snapshot();
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
        let speed_half_service = Arc::clone(&self.service);
        let speed_half = cx.listener(move |_, _: &ClickEvent, _, cx| {
            let _ = speed_half_service.set_speed(0.5);
            cx.notify();
        });
        let speed_normal_service = Arc::clone(&self.service);
        let speed_normal = cx.listener(move |_, _: &ClickEvent, _, cx| {
            let _ = speed_normal_service.set_speed(1.0);
            cx.notify();
        });
        let speed_double_service = Arc::clone(&self.service);
        let speed_double = cx.listener(move |_, _: &ClickEvent, _, cx| {
            let _ = speed_double_service.set_speed(2.0);
            cx.notify();
        });
        let controls = div()
            .h_flex()
            .items_center()
            .gap_2()
            .child(button("−5s").on_click(back))
            .child(button("Prev frame").on_click(previous_frame))
            .child(button(if snapshot.playing { "Pause" } else { "Play" }).on_click(toggle))
            .child(button("Next frame").on_click(next_frame))
            .child(button("+5s").on_click(forward))
            .child(button("0.5×").on_click(speed_half))
            .child(button("1×").on_click(speed_normal))
            .child(button("2×").on_click(speed_double))
            .child(button("Close").on_click(close));

        let frame = match &snapshot.frame_path {
            Some(path) => img(path.clone())
                .max_w_full()
                .max_h_full()
                .object_fit(ObjectFit::Contain)
                .into_any_element(),
            None if snapshot.path.is_some() => div()
                .text_sm()
                .text_color(rgb(0x9aa0ad))
                .child("Decoding video frame…")
                .into_any_element(),
            None => div()
                .text_sm()
                .text_color(rgb(0x9aa0ad))
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
                    .bg(rgb(0x14161b))
                    .child(frame),
            )
            .child(
                div()
                    .h_flex()
                    .justify_between()
                    .text_xs()
                    .text_color(rgb(0x9aa0ad))
                    .child(progress)
                    .child(format!(
                        "{}  ·  rotation {}°",
                        audio,
                        snapshot.rotation_quarters * 90
                    )),
            )
    }
}

fn button(label: &'static str) -> Button {
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
