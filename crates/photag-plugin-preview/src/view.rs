//! GPUI presentation for the source preview controls.

use std::sync::Arc;

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use photag_ui_bridge::UiViewContribution;

use crate::PreviewService;

pub fn preview_view(service: Arc<PreviewService>) -> UiViewContribution {
    UiViewContribution::new("preview/image-view", "workspace.main", 0, move |_, app| {
        let service = Arc::clone(&service);
        app.new(|cx| {
            let view_service = Arc::clone(&service);
            let mut revision = service.snapshot().revision;
            cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(150))
                        .await;
                    let changed = this
                        .update(cx, |view: &mut PreviewView, cx| {
                            let next = view.service.snapshot().revision;
                            if next != revision {
                                revision = next;
                                cx.notify();
                            }
                        })
                        .is_ok();
                    if !changed {
                        break;
                    }
                }
            })
            .detach();
            PreviewView {
                service: view_service,
            }
        })
        .into()
    })
}

struct PreviewView {
    service: Arc<PreviewService>,
}

impl Render for PreviewView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.service.snapshot();
        let display = self.service.display_path();
        let title = snapshot
            .source
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "No image selected".into());
        let service = Arc::clone(&self.service);
        let close = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.close();
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let rotate_left = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.rotate_counter_clockwise();
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let rotate_right = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.rotate_clockwise();
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let flip_h = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.flip_horizontal();
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let flip_v = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.flip_vertical();
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let fit = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.set_fit();
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let info = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.toggle_info();
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let zoom_100 = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.set_zoom_percent(100);
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let zoom_200 = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.set_zoom_percent(200);
            cx.notify();
        });
        let controls = div()
            .h_flex()
            .items_center()
            .gap_2()
            .child(button("Rotate left").on_click(rotate_left))
            .child(button("Rotate right").on_click(rotate_right))
            .child(button("Flip H").on_click(flip_h))
            .child(button("Flip V").on_click(flip_v))
            .child(button("Fit").on_click(fit))
            .child(button("100%").on_click(zoom_100))
            .child(button("200%").on_click(zoom_200))
            .child(button("Info").on_click(info))
            .child(button("Close").on_click(close));
        let image = match display {
            Ok(Some(path)) => img(path)
                .max_w_full()
                .max_h_full()
                .object_fit(ObjectFit::Contain)
                .into_any_element(),
            Ok(None) => div()
                .text_sm()
                .text_color(rgb(0x9aa0ad))
                .child("Open an indexed image to preview it.")
                .into_any_element(),
            Err(error) => div()
                .text_sm()
                .text_color(rgb(0xf08080))
                .child(error.to_string())
                .into_any_element(),
        };
        let info_text = if snapshot.show_info {
            snapshot
                .source
                .as_ref()
                .map(|path| {
                    format!(
                        "{}  ·  {}°  ·  {}%",
                        path.display(),
                        snapshot.quarter_turns * 90,
                        if snapshot.fit_to_window {
                            0
                        } else {
                            snapshot.zoom_percent
                        }
                    )
                })
                .unwrap_or_default()
        } else {
            String::new()
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
                    .child(image),
            )
            .when(!info_text.is_empty(), |element| {
                element.child(div().text_xs().text_color(rgb(0x9aa0ad)).child(info_text))
            })
    }
}

fn button(label: &'static str) -> Button {
    Button::new(label).label(label)
}
