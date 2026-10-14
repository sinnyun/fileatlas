//! Reusable GPUI floating frames. Content and close behavior remain plugin-owned.
use std::sync::Arc;

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::*;

use crate::{UiSurfaceState, theme};

pub const HEADER_HEIGHT: f32 = 36.;

#[derive(Clone, Debug)]
pub struct PanelOptions {
    pub id: String,
    pub title: String,
    pub default_size: (f32, f32),
    pub min_size: (f32, f32),
}

impl PanelOptions {
    pub fn new(id: impl Into<String>, title: impl Into<String>, size: (f32, f32)) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            default_size: size,
            min_size: (360., 240.),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl PanelBounds {
    fn clamp(self, viewport: (f32, f32), minimum: (f32, f32)) -> Self {
        let width = self.width.clamp(minimum.0.min(viewport.0), viewport.0);
        let height = self.height.clamp(minimum.1.min(viewport.1), viewport.1);
        Self {
            width,
            height,
            x: self.x.clamp(0., viewport.0 - width),
            y: self.y.clamp(0., viewport.1 - height),
        }
    }
}

/// Actual content coordinates in logical client pixels, including frame/title offset.
pub fn content_bounds(surfaces: &UiSurfaceState, id: &str, window: &Window) -> PanelBounds {
    let values = surfaces.value(&format!("panel.{id}.content_bounds"));
    let parsed = values.as_deref().and_then(|value| {
        let v = value
            .split(',')
            .map(str::parse::<f32>)
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        (v.len() == 4).then(|| PanelBounds {
            x: v[0],
            y: v[1],
            width: v[2],
            height: v[3],
        })
    });
    parsed.unwrap_or(PanelBounds {
        x: 0.,
        y: 0.,
        width: window.bounds().size.width.into(),
        height: window.bounds().size.height.into(),
    })
}

type Visibility = Arc<dyn Fn() -> bool + Send + Sync>;
type Close = Arc<dyn Fn() + Send + Sync>;

struct FloatingPanel {
    content: AnyView,
    focus: FocusHandle,
    was_visible: bool,
    options: PanelOptions,
    surfaces: Arc<UiSurfaceState>,
    visible: Visibility,
    close: Close,
    bounds: Option<PanelBounds>,
    restore: Option<PanelBounds>,
    drag: Option<(Point<Pixels>, PanelBounds, i8, i8)>,
}

/// Mount within a lifecycle-owned `workspace.modal` view contribution.
/// Every mount has independent geometry; use distinct IDs for simultaneous instances.
pub fn mount(
    content: AnyView,
    options: PanelOptions,
    surfaces: Arc<UiSurfaceState>,
    visible: impl Fn() -> bool + Send + Sync + 'static,
    close: impl Fn() + Send + Sync + 'static,
    app: &mut App,
) -> AnyView {
    app.new(|cx| {
        cx.spawn(async move |this, cx| {
            let mut previous = false;
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(100))
                    .await;
                if this
                    .update(cx, |panel: &mut FloatingPanel, cx| {
                        let active = (panel.visible)();
                        if active != previous {
                            previous = active;
                            panel.drag = None;
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        FloatingPanel {
            content,
            focus: cx.focus_handle(),
            was_visible: false,
            options,
            surfaces,
            visible: Arc::new(visible),
            close: Arc::new(close),
            bounds: None,
            restore: None,
            drag: None,
        }
    })
    .into()
}

impl Drop for FloatingPanel {
    fn drop(&mut self) {
        let _ = self.surfaces.remove_panel(&self.options.id);
    }
}

impl Render for FloatingPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let visible = (self.visible)();
        let _ = self
            .surfaces
            .set_open(format!("panel.{}", self.options.id), visible);
        if !visible {
            self.was_visible = false;
            return div().size_0().into_any_element();
        }
        if !self.was_visible {
            self.was_visible = true;
            let _ = self.surfaces.set_value("panel.active", &self.options.id);
            window.focus(&self.focus, cx);
        }
        let viewport = (
            f32::from(window.bounds().size.width).max(1.),
            f32::from(window.bounds().size.height).max(1.),
        );
        let size = self.options.default_size;
        let bounds = self
            .bounds
            .unwrap_or(PanelBounds {
                x: (viewport.0 - size.0).max(0.) / 2.,
                y: (viewport.1 - size.1).max(0.) / 2.,
                width: size.0,
                height: size.1,
            })
            .clamp(viewport, self.options.min_size);
        self.bounds = Some(bounds);
        let _ = self.surfaces.set_value(
            &format!("panel.{}.bounds", self.options.id),
            &format!(
                "{},{},{},{}",
                bounds.x, bounds.y, bounds.width, bounds.height
            ),
        );
        let measure = |key: String, surfaces: Arc<UiSurfaceState>, entity: EntityId| {
            canvas(
                move |b, window, cx| {
                    let value = format!(
                        "{},{},{},{}",
                        f32::from(b.origin.x),
                        f32::from(b.origin.y),
                        f32::from(b.size.width),
                        f32::from(b.size.height)
                    );
                    if surfaces.value(&key).as_deref() != Some(&value) {
                        let _ = surfaces.set_value(&key, &value);
                        cx.notify(entity);
                        window.request_animation_frame();
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .size_full()
        };
        let frame_measure = measure(
            format!("panel.{}.client_bounds", self.options.id),
            self.surfaces.clone(),
            cx.entity_id(),
        );
        let body_measure = measure(
            format!("panel.{}.content_bounds", self.options.id),
            self.surfaces.clone(),
            cx.entity_id(),
        );
        let close = self.close.clone();
        let mut frame = div()
            .id(format!("floating-panel-{}", self.options.id))
            .absolute()
            .left(px(bounds.x))
            .top(px(bounds.y))
            .w(px(bounds.width))
            .h(px(bounds.height))
            .v_flex()
            .rounded(px(6.))
            .bg(rgb(theme::SURFACE_MODAL))
            .border_1()
            .border_color(rgba(theme::SEPARATOR_LIGHT))
            .occlude()
            .child(frame_measure)
            .capture_any_mouse_down(cx.listener(|panel, _, _, _| {
                let _ = panel.surfaces.set_value("panel.active", &panel.options.id);
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|panel, _, _, cx| {
                    let _ = panel.surfaces.set_value("panel.active", &panel.options.id);
                    cx.stop_propagation();
                }),
            )
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .h_flex()
                    .h(px(HEADER_HEIGHT))
                    .flex_shrink_0()
                    .items_center()
                    .px_2()
                    .gap_2()
                    .border_b_1()
                    .border_color(rgba(theme::SEPARATOR_LIGHT))
                    .child(
                        div()
                            .id(format!("panel-title-{}", self.options.id))
                            .flex_1()
                            .h_full()
                            .flex()
                            .items_center()
                            .cursor(CursorStyle::OpenHand)
                            .child(crate::i18n::tr(&self.options.title))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |panel, event: &MouseDownEvent, _, cx| {
                                    panel.drag = Some((event.position, bounds, 0, 0));
                                    let _ =
                                        panel.surfaces.set_value("panel.active", &panel.options.id);
                                    cx.stop_propagation();
                                    cx.notify();
                                }),
                            ),
                    )
                    .child(
                        Button::new(format!("panel-maximize-{}", self.options.id))
                            .ghost()
                            .compact()
                            .icon(theme::icon(if self.restore.is_some() {
                                IconName::Minimize
                            } else {
                                IconName::Maximize
                            }))
                            .on_click(cx.listener(move |panel, _, _, cx| {
                                panel.bounds = Some(if let Some(restore) = panel.restore.take() {
                                    restore
                                } else {
                                    panel.restore = Some(bounds);
                                    PanelBounds {
                                        x: 0.,
                                        y: 0.,
                                        width: viewport.0,
                                        height: viewport.1,
                                    }
                                });
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new(format!("panel-close-{}", self.options.id))
                            .ghost()
                            .compact()
                            .icon(theme::icon(gpui_kit::assets::IconName::X))
                            .on_click(cx.listener(move |_, _, _, cx| {
                                close();
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_hidden()
                    .child(body_measure)
                    .child(self.content.clone()),
            );
        for (x, y) in [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ] {
            let mut handle = div()
                .id(format!("panel-resize-{}-{x}-{y}", self.options.id))
                .absolute();
            handle = if x == 0 {
                handle.left(px(8.)).right(px(8.)).h(px(5.))
            } else if y == 0 {
                handle.top(px(8.)).bottom(px(8.)).w(px(5.))
            } else {
                handle.w(px(8.)).h(px(8.))
            };
            if x < 0 {
                handle = handle.left_0();
            } else if x > 0 {
                handle = handle.right_0();
            }
            if y < 0 {
                handle = handle.top_0();
            } else if y > 0 {
                handle = handle.bottom_0();
            }
            handle = handle
                .cursor(if x == 0 {
                    CursorStyle::ResizeUpDown
                } else if y == 0 {
                    CursorStyle::ResizeLeftRight
                } else if x == y {
                    CursorStyle::ResizeUpLeftDownRight
                } else {
                    CursorStyle::ResizeUpRightDownLeft
                })
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |panel, event: &MouseDownEvent, _, cx| {
                        panel.restore = None;
                        panel.drag = Some((event.position, bounds, x, y));
                        let _ = panel.surfaces.set_value("panel.active", &panel.options.id);
                        cx.stop_propagation();
                        cx.notify();
                    }),
                );
            frame = frame.child(handle);
        }
        let mut root = div()
            .absolute()
            .inset_0()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|panel, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key.eq_ignore_ascii_case("escape")
                    && !panel.surfaces.is_capturing_key("settings")
                    && !panel.surfaces.take_suppressed_escape("settings")
                {
                    (panel.close)();
                    cx.notify();
                }
                cx.stop_propagation();
            }))
            .child(frame);
        if self.drag.is_some() {
            // Capture within this GPUI window for the entire drag, even past the frame edge.
            let capture = div()
                .absolute()
                .inset_0()
                .occlude()
                .on_mouse_move(cx.listener(move |panel, event: &MouseMoveEvent, _, cx| {
                    if !event.dragging() {
                        panel.drag = None;
                        cx.notify();
                        return;
                    }
                    if let Some((start, origin, x, y)) = panel.drag {
                        let dx = f32::from(event.position.x - start.x);
                        let dy = f32::from(event.position.y - start.y);
                        let mut next = origin;
                        if x == 0 && y == 0 {
                            next.x += dx;
                            next.y += dy;
                        } else {
                            if x < 0 {
                                let delta = dx.clamp(
                                    -origin.x,
                                    origin.width - panel.options.min_size.0.min(viewport.0),
                                );
                                next.x += delta;
                                next.width -= delta;
                            }
                            if x > 0 {
                                next.width = (origin.width + dx).min(viewport.0 - origin.x);
                            }
                            if y < 0 {
                                let delta = dy.clamp(
                                    -origin.y,
                                    origin.height - panel.options.min_size.1.min(viewport.1),
                                );
                                next.y += delta;
                                next.height -= delta;
                            }
                            if y > 0 {
                                next.height = (origin.height + dy).min(viewport.1 - origin.y);
                            }
                        }
                        panel.bounds = Some(next.clamp(viewport, panel.options.min_size));
                        cx.notify();
                    }
                }))
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|panel, _, _, cx| {
                        panel.drag = None;
                        cx.notify();
                    }),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(|panel, _, _, cx| {
                        panel.drag = None;
                        cx.notify();
                    }),
                );
            root = root.child(capture);
        }
        root.into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[::core::prelude::v1::test]
    fn frame_remains_reachable_in_small_windows() {
        let b = PanelBounds {
            x: 900.,
            y: -50.,
            width: 900.,
            height: 680.,
        }
        .clamp((640., 480.), (680., 520.));
        assert_eq!(
            b,
            PanelBounds {
                x: 0.,
                y: 0.,
                width: 640.,
                height: 480.
            }
        );
    }
}
