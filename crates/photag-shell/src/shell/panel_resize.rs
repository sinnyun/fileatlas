//! Visible splitters for the two v2 paged side panels.
use super::*;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum PanelSide {
    Sidebar,
    Inspector,
}

pub(crate) const INSPECTOR_MIN: f32 = 240.;
pub(crate) const INSPECTOR_MAX: f32 = 480.;

impl PhoTagShell {
    pub(super) fn panel_splitter(&self, side: PanelSide, cx: &Context<Self>) -> impl IntoElement {
        let active = self.resizing_panel == Some(side);
        div()
            .id(match side {
                PanelSide::Sidebar => "sidebar-splitter",
                PanelSide::Inspector => "inspector-splitter",
            })
            .absolute()
            .top_0()
            .h_full()
            .w(px(8.))
            .when(side == PanelSide::Sidebar, |line| {
                line.left(px(self.sidebar_width - 4.))
            })
            .when(side == PanelSide::Inspector, |line| {
                line.right(px(self.inspector_width - 4.))
            })
            .cursor_col_resize()
            .flex()
            .items_center()
            .justify_center()
            .bg(if active {
                rgba((theme::accent() << 8) | 42)
            } else {
                rgba(0)
            })
            .hover(|style| style.bg(rgba((theme::accent() << 8) | 42)))
            .child(div().w(px(2.)).h(px(24.)).rounded_full().bg(rgb(if active {
                theme::accent()
            } else {
                theme::TEXT_MUTED
            })))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    this.resizing_panel = Some(side);
                    this.drag_origin_x = f32::from(event.position.x);
                    this.drag_origin_width = if side == PanelSide::Sidebar {
                        this.sidebar_width
                    } else {
                        this.inspector_width
                    };
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
    }

    fn set_panel_width(&mut self, side: PanelSide, requested: f32) {
        if !requested.is_finite() {
            return;
        }
        match side {
            PanelSide::Sidebar => {
                let other = if self.inspector_open {
                    self.inspector_width
                } else {
                    0.
                };
                let max = (self.viewport_width - other - 320.)
                    .clamp(theme::SIDEBAR_MIN_WIDTH, theme::SIDEBAR_MAX_WIDTH);
                self.sidebar_width = requested.clamp(theme::SIDEBAR_MIN_WIDTH, max);
            }
            PanelSide::Inspector => {
                let other = if self.sidebar_open {
                    self.sidebar_width
                } else {
                    0.
                };
                let max = (self.viewport_width - other - 320.).clamp(INSPECTOR_MIN, INSPECTOR_MAX);
                self.inspector_width = requested.clamp(INSPECTOR_MIN, max);
            }
        }
        self.publish_panel_widths();
    }

    pub(super) fn publish_panel_widths(&self) {
        let _ = self
            .surfaces
            .set_value("shell.sidebar-width", &self.sidebar_width.to_string());
        let _ = self
            .surfaces
            .set_value("shell.inspector-width", &self.inspector_width.to_string());
    }

    pub(super) fn move_panel_drag(
        &mut self,
        event: &MouseMoveEvent,
        _window: &Window,
        cx: &mut Context<Self>,
    ) {
        let Some(side) = self.resizing_panel else {
            return;
        };
        if !event.dragging() {
            self.finish_panel_drag(cx);
            return;
        }
        let delta = f32::from(event.position.x) - self.drag_origin_x;
        let width = self.drag_origin_width
            + if side == PanelSide::Sidebar {
                delta
            } else {
                -delta
            };
        self.set_panel_width(side, width);
        cx.notify();
    }

    pub(super) fn finish_panel_drag(&mut self, cx: &mut Context<Self>) {
        if self.resizing_panel.take().is_some() {
            self.save_panel_widths();
            cx.notify();
        }
    }

    fn save_panel_widths(&self) {
        if let Ok(mut saved) = self.session.snapshot() {
            saved.sidebar_width = self.sidebar_width;
            saved.inspector_width = self.inspector_width;
            if let Err(error) = self.session.replace(saved) {
                tracing::error!("panel widths session save failed: {error}");
            }
        }
    }

    /// MCP uses the same width setter and persistence path as a splitter drag.
    pub(crate) fn poll_panel_requests(&mut self, cx: &mut Context<Self>) {
        if self.capture.pointer_pending() || self.capture.key_pending() {
            cx.notify();
        }
        for (side, key) in [
            (PanelSide::Sidebar, "shell.sidebar-width.request"),
            (PanelSide::Inspector, "shell.inspector-width.request"),
        ] {
            if let Some(value) = self.surfaces.value(key).filter(|value| !value.is_empty()) {
                let _ = self.surfaces.set_value(key, "");
                if let Ok(width) = value.parse::<f32>() {
                    self.set_panel_width(side, width);
                    self.save_panel_widths();
                    cx.notify();
                }
            }
        }
        self.publish_panel_widths();
    }

    pub(super) fn dispatch_mcp_pointer(&self, window: &Window, cx: &mut Context<Self>) {
        if self.capture.get() == self.native_window_handle {
            dispatch_mcp_input(&self.capture, window, cx);
        }
    }
}

pub(super) fn dispatch_mcp_input(
    capture: &photag_plugin_mcp::CaptureWindowHandle,
    window: &Window,
    cx: &mut App,
) {
    if let Some(key) = capture.take_key() {
        window.defer(cx, move |window, cx| {
            if let Some(text) = key.strip_prefix("text:") {
                for character in text.chars() {
                    let value = character.to_string();
                    window.dispatch_keystroke(
                        gpui_kit::Keystroke {
                            key: if character == ' ' {
                                "space".into()
                            } else {
                                value.clone()
                            },
                            key_char: Some(value),
                            ..Default::default()
                        },
                        cx,
                    );
                }
            } else if let Ok(keystroke) = gpui_kit::Keystroke::parse(&key) {
                window.dispatch_event(
                    PlatformInput::KeyDown(gpui_kit::KeyDownEvent {
                        keystroke,
                        is_held: false,
                        prefer_character_input: false,
                    }),
                    cx,
                );
            }
        });
    }
    if let Some(input) = capture.take_pointer() {
        window.defer(cx, move |window, cx| {
            let position = point(px(input.x), px(input.y));
            let event = match input.kind {
                "scroll" => PlatformInput::ScrollWheel(ScrollWheelEvent {
                    position,
                    delta: ScrollDelta::Pixels(point(px(0.), px(input.scroll_y))),
                    ..Default::default()
                }),
                "down" | "right_down" | "double_down" => PlatformInput::MouseDown(MouseDownEvent {
                    button: if input.kind == "right_down" {
                        MouseButton::Right
                    } else {
                        MouseButton::Left
                    },
                    position,
                    click_count: if input.kind.starts_with("double_") {
                        2
                    } else {
                        1
                    },
                    ..Default::default()
                }),
                "drag" | "move" => PlatformInput::MouseMove(MouseMoveEvent {
                    position,
                    pressed_button: if input.kind == "drag" {
                        Some(MouseButton::Left)
                    } else {
                        None
                    },
                    ..Default::default()
                }),
                _ => PlatformInput::MouseUp(MouseUpEvent {
                    button: if input.kind == "right_up" {
                        MouseButton::Right
                    } else {
                        MouseButton::Left
                    },
                    position,
                    click_count: if input.kind.starts_with("double_") {
                        2
                    } else {
                        1
                    },
                    ..Default::default()
                }),
            };
            window.dispatch_event(event, cx);
        });
    }
}
