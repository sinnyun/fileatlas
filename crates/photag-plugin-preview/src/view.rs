//! GPUI presentation for the source preview controls.

mod actions;

use std::sync::Arc;

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use photag_plugin_file_operations::FileOperationsService;
use photag_plugin_settings::SettingsService;
use photag_ui_bridge::{UiViewContribution, theme};

use crate::PreviewService;

pub fn preview_view(
    service: Arc<PreviewService>,
    settings: Arc<SettingsService>,
    file_operations: Arc<FileOperationsService>,
    runtime: tokio::runtime::Handle,
) -> UiViewContribution {
    UiViewContribution::new(
        "preview/image-view",
        "workspace.main",
        0,
        move |window, app| {
            let service = Arc::clone(&service);
            let rename_input =
                app.new(|cx| InputState::new(window, cx).placeholder("New filename"));
            app.new(|cx| {
                let view_service = Arc::clone(&service);
                let focus_handle = cx.focus_handle();
                let mut revision = service.snapshot().revision;
                let mut decode_revision = service.decode_revision();
                let mut showing_progressive = service
                    .snapshot()
                    .source
                    .as_deref()
                    .and_then(|source| service.progressive_path(source))
                    .is_some();
                let decode_service = Arc::clone(&service);
                cx.spawn(async move |this, cx| {
                    loop {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(150))
                            .await;
                        let next_decode_revision = decode_service.decode_revision();
                        let snapshot = decode_service.snapshot();
                        let next_showing_progressive = snapshot
                            .source
                            .as_deref()
                            .and_then(|source| decode_service.progressive_path(source))
                            .is_some();
                        let changed = this
                            .update(cx, |view: &mut PreviewView, cx| {
                                let next = view.service.snapshot().revision;
                                if next != revision
                                    || next_decode_revision != decode_revision
                                    || next_showing_progressive != showing_progressive
                                {
                                    revision = next;
                                    decode_revision = next_decode_revision;
                                    showing_progressive = next_showing_progressive;
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
                    settings: Arc::clone(&settings),
                    last_drag_position: None,
                    toolbar_drag_origin: None,
                    toolbar_drag_offset: None,
                    toolbar_error: None,
                    file_operations: Arc::clone(&file_operations),
                    runtime: runtime.clone(),
                    rename_input,
                    rename_target: None,
                    delete_confirmation_open: false,
                    operation_status: None,
                    operation_busy: false,
                    focus_handle,
                }
            })
            .into()
        },
    )
}

struct PreviewView {
    service: Arc<PreviewService>,
    settings: Arc<SettingsService>,
    last_drag_position: Option<(f32, f32)>,
    toolbar_drag_origin: Option<(f32, f32, f32, f32)>,
    toolbar_drag_offset: Option<(f32, f32)>,
    toolbar_error: Option<String>,
    file_operations: Arc<FileOperationsService>,
    runtime: tokio::runtime::Handle,
    rename_input: gpui::Entity<InputState>,
    rename_target: Option<i64>,
    delete_confirmation_open: bool,
    operation_status: Option<String>,
    operation_busy: bool,
    focus_handle: FocusHandle,
}

impl Render for PreviewView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.service.snapshot();
        let display = self.service.display_path();
        let title = snapshot
            .source
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "No image selected".into());
        let service = Arc::clone(&self.service);
        let close = cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.rename_target = None;
            service.close();
            cx.notify();
        });
        let rename = cx.listener(|this, _: &ClickEvent, window, cx| this.begin_rename(window, cx));
        let delete =
            cx.listener(|this, _: &ClickEvent, window, cx| this.request_delete(window, cx));
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
        let zoom_25 = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.set_zoom_percent(25);
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let zoom_50 = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.set_zoom_percent(50);
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let zoom_100 = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.set_zoom_percent(100);
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let zoom_125 = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.set_zoom_percent(125);
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let zoom_150 = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.set_zoom_percent(150);
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let zoom_200 = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.set_zoom_percent(200);
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let zoom_mode = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.toggle_zoom_at_cursor();
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
            .child(button("25%").on_click(zoom_25))
            .child(button("50%").on_click(zoom_50))
            .child(button("100%").on_click(zoom_100))
            .child(button("125%").on_click(zoom_125))
            .child(button("150%").on_click(zoom_150))
            .child(button("200%").on_click(zoom_200))
            .child(
                button(if snapshot.zoom_at_cursor {
                    "Zoom: cursor"
                } else {
                    "Zoom: center"
                })
                .on_click(zoom_mode),
            )
            .child(
                button(if snapshot.show_info {
                    "Hide info"
                } else {
                    "Info"
                })
                .on_click(info),
            )
            .child(button("Rename (F2)").on_click(rename))
            .child(button("Delete").on_click(delete))
            .child(button("Close").on_click(close));
        let bounds = window.bounds().size;
        let viewport_width = (f32::from(bounds.width) - 32.0).max(1.0);
        let viewport_height = (f32::from(bounds.height) - 112.0).max(1.0);
        let has_image = matches!(&display, Ok(Some(_)));
        let (image, fit_scale) = match display {
            Ok(Some(path)) => {
                let dimensions = image::image_dimensions(&path).ok();
                let display_path = if snapshot.fit_to_window {
                    snapshot
                        .source
                        .as_deref()
                        .and_then(|source| self.service.progressive_path(source))
                        .unwrap_or(path)
                } else {
                    path
                };
                let fit_scale = dimensions.map_or(1.0, |(width, height)| {
                    (viewport_width / width as f32)
                        .min(viewport_height / height as f32)
                        .min(1.0)
                });
                let image = match dimensions {
                    Some((width, height)) if !snapshot.fit_to_window => img(display_path)
                        .w(px(width as f32 * snapshot.zoom_scale))
                        .h(px(height as f32 * snapshot.zoom_scale))
                        .object_fit(ObjectFit::Contain)
                        .relative()
                        .left(px(snapshot.pan_x))
                        .top(px(snapshot.pan_y))
                        .into_any_element(),
                    _ => img(display_path)
                        .max_w_full()
                        .max_h_full()
                        .object_fit(ObjectFit::Contain)
                        .relative()
                        .left(px(snapshot.pan_x))
                        .top(px(snapshot.pan_y))
                        .into_any_element(),
                };
                (image, fit_scale)
            }
            Ok(None) => (
                div()
                    .text_sm()
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child("Open an indexed image to preview it.")
                    .into_any_element(),
                1.0,
            ),
            Err(error) => (
                div()
                    .text_sm()
                    .text_color(rgb(theme::DANGER))
                    .child(error.to_string())
                    .into_any_element(),
                1.0,
            ),
        };
        let service = Arc::clone(&self.service);
        let can_zoom = has_image;
        let cursor_center = (
            f32::from(bounds.width) / 2.0,
            f32::from(bounds.height) / 2.0,
        );
        let wheel = cx.listener(move |_, event: &ScrollWheelEvent, _, cx| {
            if !can_zoom {
                return;
            }
            let delta = event.delta.pixel_delta(px(40.0));
            let scroll_y = f32::from(delta.y);
            let cursor_delta = (
                f32::from(event.position.x) - cursor_center.0,
                f32::from(event.position.y) - cursor_center.1,
            );
            service.zoom_wheel(scroll_y, fit_scale, cursor_delta);
            cx.notify();
        });
        let can_pan = has_image;
        let down = cx.listener(move |this, event: &MouseDownEvent, window, cx| {
            window.focus(&this.focus_handle, cx);
            this.last_drag_position =
                can_pan.then_some((f32::from(event.position.x), f32::from(event.position.y)));
        });
        let can_pan = has_image;
        let moved = cx.listener(move |this, event: &MouseMoveEvent, _, cx| {
            if !can_pan || !event.dragging() {
                this.last_drag_position = None;
                return;
            }
            let current = (f32::from(event.position.x), f32::from(event.position.y));
            if let Some(previous) = this.last_drag_position.replace(current) {
                this.service
                    .pan_by(current.0 - previous.0, current.1 - previous.1);
                cx.notify();
            }
        });
        let up = cx.listener(|this, _: &MouseUpEvent, _, _| {
            this.last_drag_position = None;
        });
        let mut stage = div()
            .relative()
            .flex_1()
            .min_h_0()
            .w_full()
            .bg(rgb(theme::SURFACE_BASE))
            .overflow_hidden();
        stage = stage.child(
            div()
                .flex_1()
                .min_h_0()
                .w_full()
                .items_center()
                .justify_center()
                .overflow_hidden()
                .on_scroll_wheel(wheel)
                .on_mouse_down(MouseButton::Left, down)
                .on_mouse_move(moved)
                .on_mouse_up(MouseButton::Left, up)
                .child(image),
        );
        if snapshot.show_info {
            if let Some(path) = snapshot.source.as_deref() {
                stage = stage.child(
                    div()
                        .absolute()
                        .left(px(12.0))
                        .top(px(12.0))
                        .max_h_full()
                        .child(crate::info::info_panel(path)),
                );
            }
        }
        let saved_toolbar_pos = self
            .settings
            .snapshot()
            .map(|settings| settings.toolbar_pos)
            .unwrap_or_default();
        let (toolbar_x, toolbar_y) = self
            .toolbar_drag_offset
            .unwrap_or((saved_toolbar_pos.offset_x, saved_toolbar_pos.offset_y));
        let (toolbar_x, toolbar_y) =
            clamp_toolbar_offset(toolbar_x, toolbar_y, viewport_width, viewport_height);
        let drag_start = cx.listener(move |this, event: &MouseDownEvent, window, cx| {
            window.focus(&this.focus_handle, cx);
            let saved = this
                .settings
                .snapshot()
                .ok()
                .map(|settings| settings.toolbar_pos);
            if saved.is_some_and(|position| !position.locked) {
                this.toolbar_drag_origin = Some((
                    f32::from(event.position.x),
                    f32::from(event.position.y),
                    toolbar_x,
                    toolbar_y,
                ));
                this.toolbar_drag_offset = Some((toolbar_x, toolbar_y));
            }
        });
        let drag_move = cx.listener(move |this, event: &MouseMoveEvent, _, cx| {
            if !event.dragging() {
                return;
            }
            if let Some((start_x, start_y, offset_x, offset_y)) = this.toolbar_drag_origin {
                this.toolbar_drag_offset = Some(clamp_toolbar_offset(
                    offset_x + f32::from(event.position.x) - start_x,
                    offset_y + f32::from(event.position.y) - start_y,
                    viewport_width,
                    viewport_height,
                ));
                cx.notify();
            }
        });
        let drag_end = cx.listener(|this, _: &MouseUpEvent, _, cx| {
            if let Some((offset_x, offset_y)) = this.toolbar_drag_offset.take() {
                this.toolbar_drag_origin = None;
                match this.settings.update(|settings| {
                    settings.toolbar_pos.offset_x = offset_x;
                    settings.toolbar_pos.offset_y = offset_y;
                }) {
                    Ok(_) => this.toolbar_error = None,
                    Err(error) => this.toolbar_error = Some(error.to_string()),
                }
                cx.notify();
            }
        });
        let toolbar = div()
            .absolute()
            .bottom(px(24.0 - toolbar_y))
            .left(px(toolbar_x))
            .w_full()
            .h_flex()
            .items_center()
            .justify_center()
            .gap_2()
            .child(
                div()
                    .id("preview-toolbar-drag-handle")
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .bg(rgb(theme::SURFACE_MODAL))
                    .text_color(rgb(if saved_toolbar_pos.locked {
                        theme::TEXT_SECONDARY
                    } else {
                        theme::TEXT_PRIMARY
                    }))
                    .child(if saved_toolbar_pos.locked {
                        "🔒"
                    } else {
                        "⠿"
                    })
                    .on_mouse_down(MouseButton::Left, drag_start),
            )
            .child(controls);
        stage = stage
            .on_mouse_move(drag_move)
            .on_mouse_up(MouseButton::Left, drag_end)
            .child(toolbar);
        if let Some(error) = &self.toolbar_error {
            stage = stage.child(
                div()
                    .absolute()
                    .bottom(px(4.0))
                    .left(px(8.0))
                    .text_xs()
                    .text_color(rgb(theme::DANGER))
                    .child(format!("Could not save preview toolbar position: {error}")),
            );
        }
        let items = self.service.filmstrip_items();
        let paths: Vec<_> = items.iter().map(|(path, _)| path.clone()).collect();
        let current = snapshot
            .source
            .as_ref()
            .and_then(|source| paths.iter().position(|path| path == source));
        let strip = if paths.len() > 1 {
            let center = current.unwrap_or(0);
            let start = center.saturating_sub(3);
            let end = (start + 7).min(paths.len());
            let items = paths[start..end]
                .iter()
                .enumerate()
                .map(|(offset, path)| {
                    let path = path.clone();
                    let media_id = items[start + offset].1;
                    let service = Arc::clone(&self.service);
                    let selected = start + offset == center;
                    let thumbnail = self.service.filmstrip_thumbnail(&path);
                    div()
                        .id(format!("preview-filmstrip-{offset}"))
                        .w(px(68.0))
                        .h(px(68.0))
                        .p_1()
                        .rounded_md()
                        .bg(rgb(if selected {
                            theme::SURFACE_FAINT
                        } else {
                            theme::SURFACE_MODAL
                        }))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                                window.focus(&this.focus_handle, cx);
                                let _ = service.open_media(path.clone(), media_id);
                                cx.notify();
                            }),
                        )
                        .child(match thumbnail {
                            Some(thumbnail) => img(thumbnail)
                                .w_full()
                                .h_full()
                                .object_fit(ObjectFit::Contain)
                                .into_any_element(),
                            None => div()
                                .size_full()
                                .items_center()
                                .justify_center()
                                .text_xs()
                                .child("…")
                                .into_any_element(),
                        })
                })
                .collect::<Vec<_>>();
            div()
                .h_flex()
                .justify_center()
                .gap_1()
                .children(items)
                .into_any_element()
        } else {
            div().into_any_element()
        };
        let key_service = Arc::clone(&self.service);
        div()
            .v_flex()
            .flex_1()
            .min_h_0()
            .gap_2()
            .p_3()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if this.rename_target.is_some() {
                    match event.keystroke.key.as_str() {
                        "enter" | "Enter" => this.commit_rename(cx),
                        "escape" | "Escape" => {
                            this.rename_target = None;
                            cx.notify();
                        }
                        _ => {}
                    }
                    return;
                }
                if this.delete_confirmation_open {
                    return;
                }
                match event.keystroke.key.as_str() {
                    "left" | "ArrowLeft" => {
                        if !this.operation_busy {
                            key_service.navigate_filmstrip(-1);
                        }
                        cx.notify();
                    }
                    "right" | "ArrowRight" => {
                        if !this.operation_busy {
                            key_service.navigate_filmstrip(1);
                        }
                        cx.notify();
                    }
                    "escape" | "Escape" => {
                        key_service.close();
                        cx.notify();
                    }
                    "i" | "I" => {
                        key_service.toggle_info();
                        cx.notify();
                    }
                    "r" | "R" => {
                        key_service.rotate_clockwise();
                        cx.notify();
                    }
                    "f" | "F" | "0" => {
                        key_service.set_fit();
                        cx.notify();
                    }
                    "+" | "=" => {
                        let zoom = key_service.snapshot().zoom_percent;
                        let next = [25, 50, 100, 125, 150, 200]
                            .into_iter()
                            .find(|preset| *preset > zoom)
                            .unwrap_or(200);
                        key_service.set_zoom_percent(next);
                        cx.notify();
                    }
                    "-" => {
                        let zoom = key_service.snapshot().zoom_percent;
                        let next = [200, 150, 125, 100, 50, 25]
                            .into_iter()
                            .find(|preset| *preset < zoom)
                            .unwrap_or(25);
                        key_service.set_zoom_percent(next);
                        cx.notify();
                    }
                    "f2" | "F2" if !this.operation_busy => this.begin_rename(window, cx),
                    "delete" | "Delete" | "backspace" | "Backspace" => {
                        if !this.operation_busy {
                            this.request_delete(window, cx)
                        }
                    }
                    _ => {}
                }
            }))
            .child(div().text_sm().font_medium().child(title))
            .when_some(self.rename_target, |root, _| {
                root.child(
                    div()
                        .h_flex()
                        .gap_2()
                        .child(Input::new(&self.rename_input).w(px(260.)))
                        .child(
                            Button::new("preview-rename-commit")
                                .label("Rename")
                                .on_click(cx.listener(|this, _, _, cx| this.commit_rename(cx))),
                        )
                        .child(
                            Button::new("preview-rename-cancel")
                                .label("Cancel")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.rename_target = None;
                                    cx.notify();
                                })),
                        ),
                )
            })
            .when_some(self.operation_status.as_ref(), |root, status| {
                root.child(div().text_sm().child(status.clone()))
            })
            .child(stage)
            .child(strip)
    }
}

fn button(label: &'static str) -> Button {
    Button::new(format!("preview-{label}")).label(label)
}

fn clamp_toolbar_offset(x: f32, y: f32, width: f32, height: f32) -> (f32, f32) {
    const TOOLBAR_CONTENT_WIDTH: f32 = 920.0;
    let max_x = ((width - TOOLBAR_CONTENT_WIDTH) / 2.0).max(0.0);
    let min_y = -(height - 72.0).max(0.0);
    (x.clamp(-max_x, max_x), y.clamp(min_y, 16.0))
}
