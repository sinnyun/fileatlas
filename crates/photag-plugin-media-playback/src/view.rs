//! GPUI transport controls for the FFmpeg-backed player.

use std::sync::Arc;

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::Icon;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::slider::{Slider, SliderEvent, SliderState, SliderValue};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use photag_plugin_database::db::queries::Tag;
use photag_plugin_file_operations::FileOperationsService;
use photag_plugin_tagging::TaggingService;
use photag_ui_bridge::{UiViewContribution, theme};

use crate::MediaPlaybackService;

pub fn media_playback_view(
    service: Arc<MediaPlaybackService>,
    tagging: Option<Arc<TaggingService>>,
    file_operations: Option<Arc<FileOperationsService>>,
    surfaces: Arc<photag_ui_bridge::UiSurfaceState>,
) -> UiViewContribution {
    UiViewContribution::new(
        "media-playback/transport-view",
        "workspace.modal",
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
                                if snapshot.revision != revision
                                    || snapshot.playing
                                    || view.tags_open != view.surfaces.is_open("playback.tags")
                                    || view.more_open != view.surfaces.is_open("playback.more")
                                {
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
                    tagging: tagging.clone(),
                    file_operations: file_operations.clone(),
                    surfaces: surfaces.clone(),
                    focus_handle: cx.focus_handle(),
                    last_path: None,
                    playhead,
                    volume,
                    speed,
                    playhead_value: None,
                    speed_popup_open: false,
                    tags_open: false,
                    more_open: false,
                    operation_status: None,
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
    tagging: Option<Arc<TaggingService>>,
    file_operations: Option<Arc<FileOperationsService>>,
    surfaces: Arc<photag_ui_bridge::UiSurfaceState>,
    focus_handle: FocusHandle,
    last_path: Option<std::path::PathBuf>,
    playhead: Entity<SliderState>,
    volume: Entity<SliderState>,
    speed: Entity<SliderState>,
    playhead_value: Option<f32>,
    speed_popup_open: bool,
    tags_open: bool,
    more_open: bool,
    operation_status: Option<String>,
    _playhead_subscription: Subscription,
    _volume_subscription: Subscription,
    _speed_subscription: Subscription,
}

impl Render for PlaybackView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.service.snapshot();
        if snapshot.path.is_none() {
            self.last_path = None;
            return div().size_0();
        }
        if self.last_path != snapshot.path {
            self.last_path = snapshot.path.clone();
            let _ = self.surfaces.set_open("playback.tags", false);
            let _ = self.surfaces.set_open("playback.more", false);
            window.focus(&self.focus_handle, cx);
        }
        self.tags_open = self.surfaces.is_open("playback.tags");
        self.more_open = self.surfaces.is_open("playback.more");
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
        let toggle_tags = cx.listener(|view, _: &ClickEvent, _, cx| {
            let _ = view.surfaces.set_open("playback.tags", !view.tags_open);
            let _ = view.surfaces.set_open("playback.more", false);
            cx.notify();
        });
        let toggle_more = cx.listener(|view, _: &ClickEvent, _, cx| {
            let _ = view.surfaces.set_open("playback.more", !view.more_open);
            let _ = view.surfaces.set_open("playback.tags", false);
            cx.notify();
        });
        let frame = match &snapshot.frame_path {
            Some(path) => img(path.clone())
                .max_w_full()
                .max_h_full()
                .w_full()
                .h_full()
                .object_fit(ObjectFit::Contain)
                .into_any_element(),
            None if snapshot.path.is_some() && !snapshot.has_video => div()
                .v_flex()
                .items_center()
                .justify_center()
                .gap_3()
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child(div().text_2xl().text_color(rgb(theme::accent())).child("♫"))
                .child(
                    div()
                        .text_sm()
                        .child(photag_ui_bridge::i18n::tr("Audio playback")),
                )
                .into_any_element(),
            None if snapshot.path.is_some() => div()
                .text_sm()
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child(photag_ui_bridge::i18n::tr("Decoding video frame…"))
                .into_any_element(),
            None => div()
                .text_sm()
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child(photag_ui_bridge::i18n::tr("Open a video to play it."))
                .into_any_element(),
        };
        let title = snapshot
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "No media selected".into());
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
                    row.child(button(format!("{speed:.2}×")).on_click(choose_speed))
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
                        .child(div().text_xs().child(photag_ui_bridge::i18n::tr("Speed")))
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
        let output = if snapshot.audio_available {
            photag_ui_bridge::i18n::tr("Audio enabled")
        } else {
            photag_ui_bridge::i18n::tr("Audio output unavailable")
        };
        let compact_controls = div()
            .h_flex()
            .items_center()
            .gap_2()
            .child(
                button(if snapshot.playing { "Ⅱ" } else { "▶" })
                    .tooltip(photag_ui_bridge::i18n::tr(if snapshot.playing {
                        "Pause"
                    } else {
                        "Play"
                    }))
                    .on_click(toggle),
            )
            .child(
                button("‹")
                    .tooltip(photag_ui_bridge::i18n::tr("Previous frame"))
                    .disabled(!snapshot.has_video)
                    .on_click(previous_frame),
            )
            .child(button("−5s").on_click(back))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child(format_time(snapshot.position_seconds)),
            )
            .child(Slider::new(&self.playhead).flex_1().h_2())
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child(format_time(snapshot.duration_seconds)),
            )
            .child(button("+5s").on_click(forward))
            .child(
                button("›")
                    .tooltip(photag_ui_bridge::i18n::tr("Next frame"))
                    .disabled(!snapshot.has_video)
                    .on_click(next_frame),
            )
            .child(div().text_xs().child(photag_ui_bridge::i18n::tr("Volume")))
            .child(
                div()
                    .w(px(96.))
                    .flex_shrink_0()
                    .child(Slider::new(&self.volume).w_full()),
            )
            .child(button(format!("{:.2}×", snapshot.speed)).on_click(speed_popup))
            .child(button("Tags").icon(IconName::Tag).on_click(toggle_tags))
            .child(
                button("More")
                    .icon(IconName::Ellipsis)
                    .on_click(toggle_more),
            );
        let error = snapshot
            .error
            .map(|error| div().text_xs().text_color(rgb(theme::DANGER)).child(error));
        let tags_panel = self.render_tags(snapshot.media_id, cx);
        let more_panel = self.render_more(snapshot.media_id, cx);
        div()
            .absolute()
            .inset_0()
            .bg(rgb(theme::SURFACE_EXTREME))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, _, cx| {
                match event.keystroke.key.as_str() {
                    "escape" => view.service.close(),
                    "space" => {
                        let _ = view.service.toggle_play_pause();
                    }
                    "left" => {
                        let _ = view
                            .service
                            .seek(view.service.snapshot().position_seconds - 5.);
                    }
                    "right" => {
                        let _ = view
                            .service
                            .seek(view.service.snapshot().position_seconds + 5.);
                    }
                    _ => return,
                }
                cx.notify();
            }))
            .child(
                div()
                    .v_flex()
                    .size_full()
                    .min_h_0()
                    .gap_0()
                    .relative()
                    .border_1()
                    .border_color(rgba(theme::SEPARATOR_LIGHT))
                    .bg(rgb(theme::SURFACE_BASE))
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap_3()
                            .h(px(50.0))
                            .px_3()
                            .border_b_1()
                            .border_color(rgba(theme::SEPARATOR_LIGHT))
                            .child(Icon::new(IconName::Aperture).text_color(rgb(theme::accent())))
                            .child(div().text_sm().font_medium().child("PhoTag"))
                            .child(
                                div()
                                    .w(px(1.0))
                                    .h(px(24.0))
                                    .bg(rgba(theme::SEPARATOR_LIGHT)),
                            )
                            .child(div().flex_1().text_sm().child(title))
                            .child(button("Close").icon(IconName::Close).on_click(close)),
                    )
                    .when(self.tags_open, |panel| {
                        panel.child(
                            div()
                                .absolute()
                                .right(px(16.))
                                .bottom(px(82.))
                                .w(px(300.))
                                .occlude()
                                .child(tags_panel),
                        )
                    })
                    .when(self.more_open, |panel| {
                        panel.child(
                            div()
                                .absolute()
                                .right(px(16.))
                                .bottom(px(82.))
                                .w(px(220.))
                                .occlude()
                                .child(more_panel),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .min_h_0()
                            .w_full()
                            .items_center()
                            .justify_center()
                            .bg(rgba(if snapshot.has_video {
                                theme::base_rgba(theme::SURFACE_BASE)
                            } else {
                                theme::SURFACE_AUDIO << 8 | 0xff
                            }))
                            .child(frame),
                    )
                    .child(
                        div()
                            .v_flex()
                            .gap_2()
                            .px_3()
                            .py_2()
                            .border_t_1()
                            .border_color(rgba(theme::SEPARATOR_LIGHT))
                            .child(compact_controls)
                            .when_some(error, |row, error| row.child(error))
                            .when_some(self.operation_status.clone(), |row, status| {
                                row.child(
                                    div().text_xs().text_color(rgb(theme::DANGER)).child(status),
                                )
                            })
                            .child(speed_controls)
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(theme::TEXT_SECONDARY))
                                    .when(snapshot.playing && !snapshot.audio_available, |row| {
                                        row.child(output)
                                    }),
                            ),
                    ),
            )
    }
}

impl PlaybackView {
    fn render_tags(&self, media_id: Option<i64>, cx: &mut Context<Self>) -> AnyElement {
        let Some(tagging) = self.tagging.as_ref() else {
            return div().size_0().into_any_element();
        };
        let Some(media_id) = media_id else {
            return div()
                .px_3()
                .py_2()
                .text_sm()
                .child(photag_ui_bridge::i18n::tr("Tags require indexed media."))
                .into_any_element();
        };
        let assigned = tagging
            .for_media(media_id)
            .unwrap_or_default()
            .into_iter()
            .map(|tag| tag.id)
            .collect::<std::collections::HashSet<_>>();
        let mut tags = Vec::new();
        if let Ok(tree) = tagging.tag_tree() {
            flatten_tags(&tree, 0, &mut tags);
        }
        let mut panel = div()
            .v_flex()
            .h(px(220.))
            .bg(rgb(theme::SURFACE_MODAL))
            .border_1()
            .border_color(rgb(theme::SEPARATOR))
            .gap_1()
            .px_3()
            .py_2()
            .overflow_y_scrollbar();
        if tags.is_empty() {
            return panel
                .child(photag_ui_bridge::i18n::tr("No tags yet"))
                .into_any_element();
        }
        for (tag, depth) in tags {
            let id = tag.id;
            let is_assigned = assigned.contains(&id);
            let tagging = Arc::clone(tagging);
            panel = panel.child(
                Button::new(format!("playback-tag-{id}"))
                    .ghost()
                    .compact()
                    .toggled(is_assigned)
                    .label(format!("{}{}", "  ".repeat(depth), tag.name))
                    .on_click(cx.listener(move |view, _, _, cx| {
                        let result = if is_assigned {
                            tagging.remove(&[media_id], &[id])
                        } else {
                            tagging.assign(&[media_id], &[id])
                        };
                        view.operation_status = result.err().map(|error| error.to_string());
                        cx.notify();
                    })),
            );
        }
        panel.into_any_element()
    }

    fn render_more(&self, media_id: Option<i64>, cx: &mut Context<Self>) -> AnyElement {
        let (Some(media_id), Some(operations)) = (media_id, self.file_operations.clone()) else {
            return div()
                .px_3()
                .py_2()
                .text_sm()
                .child(photag_ui_bridge::i18n::tr("File actions are unavailable."))
                .into_any_element();
        };
        let reveal = Arc::clone(&operations);
        div()
            .h_flex()
            .gap_2()
            .px_3()
            .py_2()
            .child(
                Button::new("playback-reveal")
                    .label(photag_ui_bridge::i18n::tr("Show in folder"))
                    .on_click(cx.listener(move |_view, _, _, cx| {
                        let reveal = Arc::clone(&reveal);
                        cx.spawn(async move |this, cx| {
                            let result = reveal.reveal_in_file_manager(media_id).await;
                            let _ = this.update(cx, |view, cx| {
                                view.operation_status = result.err().map(|error| error.to_string());
                                cx.notify();
                            });
                        })
                        .detach();
                    })),
            )
            .into_any_element()
    }
}

fn flatten_tags(tags: &[Tag], depth: usize, rows: &mut Vec<(Tag, usize)>) {
    for tag in tags {
        rows.push((tag.clone(), depth));
        flatten_tags(&tag.children, depth.saturating_add(1), rows);
    }
}

fn button(label: impl Into<String>) -> Button {
    let label = label.into();
    Button::new(format!("media-playback-{label}"))
        .ghost()
        .compact()
        .h(px(28.))
        .rounded(px(4.))
        .label(photag_ui_bridge::i18n::tr(&(label).to_string()))
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
