//! Canvas surface: the official bitmap, the selection overlay drawn from official bounds,
//! and the pointer events forwarded to VectorCraft as one gesture sequence.
use super::canvas::*;
use super::*;
use ::gpui::{
    Bounds, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, point, rgba,
    size,
};

impl WorkCratePanel {
    fn canvas_down(&mut self, event: &MouseDownEvent, cx: &mut ViewContext<Self>) {
        let (Some(map), Some(snapshot)) = (self.canvas_window_map(), self.canvas.clone()) else {
            return;
        };
        let pointer = event.position;
        let handle = snapshot
            .selection_bounds
            .iter()
            .flat_map(|bounds| (0..8u8).map(move |index| (index, bounds)))
            .find(|(index, bounds)| {
                let center = map.point_of(
                    handle_point(bounds, *index).0,
                    handle_point(bounds, *index).1,
                );
                f32::from(pointer.x - center.x).abs() <= GRAB
                    && f32::from(pointer.y - center.y).abs() <= GRAB
            })
            .map(|(index, bounds)| (index, bounds.clone()));
        let (kind, origin, down) = match handle {
            Some((index, bounds)) => {
                let point = handle_point(&bounds, index);
                (CanvasDragKind::Handle(index), bounds, point)
            }
            None => (
                CanvasDragKind::Body,
                snapshot.selection_bounds.clone().unwrap_or_default(),
                map.to_document(pointer),
            ),
        };
        let start = map.to_document(pointer);
        self.canvas_drag = Some(CanvasDrag::begin(kind, origin, start, down));
        cx.notify();
    }

    fn canvas_move(&mut self, event: &MouseMoveEvent, cx: &mut ViewContext<Self>) {
        if !event.dragging() {
            return;
        }
        let Some(map) = self.canvas_window_map() else {
            return;
        };
        let Some(drag) = self.canvas_drag.as_mut() else {
            return;
        };
        if drag.drag_to(map.to_document(event.position)) {
            cx.notify();
        }
    }

    fn canvas_up(&mut self, event: &MouseUpEvent, cx: &mut ViewContext<Self>) {
        let (Some(map), Some(drag)) = (self.canvas_window_map(), self.canvas_drag.take()) else {
            return;
        };
        let events = drag.finish(map.to_document(event.position));
        self.canvas_job(
            move |boards| {
                boards
                    .canvas_gesture(serde_json::Value::Array(events))
                    .map_err(|e| e.to_string())
            },
            cx,
        );
    }

    pub(super) fn canvas_area(&self, cx: &mut ViewContext<Self>) -> gpui::AnyElement {
        let viewport = self.canvas_viewport();
        let area = div()
            .id("workcrate-canvas-viewport")
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_scroll()
            .flex()
            .items_center()
            .justify_center()
            .p_3()
            .bg(rgb(theme::surface_faint()))
            .child(super::tracking::measure(
                "canvas-viewport",
                self.surfaces.clone(),
            ));
        let Some(snapshot) = self.canvas.clone() else {
            return area
                .child(div().text_sm().text_color(rgb(theme::text_muted())).child(
                    if self.canvas_busy {
                        "正在连接官方画布…"
                    } else {
                        "官方画布未就绪"
                    },
                ))
                .into_any_element();
        };
        let inner = viewport.size.width - px(24. + CANVAS_BORDER);
        let inner_h = viewport.size.height - px(24. + CANVAS_BORDER);
        let fit = (f32::from(inner) / snapshot.artboard.width as f32)
            .min(f32::from(inner_h) / snapshot.artboard.height as f32);
        let scale = fit * self.canvas_zoom;
        let width = px(snapshot.artboard.width as f32 * scale);
        let height = px(snapshot.artboard.height as f32 * scale);
        // Overlay children are positioned inside the artboard, so their map starts at (0,0);
        // pointer events arrive in window pixels and use the measured map instead.
        let local = CanvasMap::new(
            Bounds {
                origin: point(px(0.), px(0.)),
                size: size(width, height),
            },
            snapshot.artboard.clone(),
        );
        let drag = self.canvas_drag.as_ref();
        let shown = match (drag, local.as_ref(), &snapshot.selection_bounds) {
            (Some(drag), Some(_), Some(bounds)) if bounds.width > 0. || bounds.height > 0. => {
                Some(predict(drag, drag.current))
            }
            _ => None,
        };
        let frame = shown.as_ref().or(snapshot.selection_bounds.as_ref());
        let mut artboard = div()
            .id("workcrate-canvas-artboard")
            .relative()
            .flex_shrink_0()
            .w(width)
            .h(height)
            .border_1()
            .border_color(rgb(theme::separator()))
            .bg(rgb(theme::surface_card()))
            .child(
                self.canvas_image
                    .clone()
                    .map(|image| {
                        img(image)
                            .size_full()
                            .object_fit(ObjectFit::Fill)
                            .into_any_element()
                    })
                    .unwrap_or_else(|| {
                        div()
                            .size_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child("正在读取官方预览…")
                            .into_any_element()
                    }),
            )
            .child(super::tracking::measure(
                "canvas-artboard",
                self.surfaces.clone(),
            ));
        if let (Some(map), Some(bounds)) = (local, frame) {
            let rect = map.rect_of(bounds);
            artboard = artboard
                .child(selection_box(rect))
                .children((0..8u8).map(move |index| {
                    let point =
                        map.point_of(handle_point(bounds, index).0, handle_point(bounds, index).1);
                    handle_box(point)
                }));
            if let Some(drag) = drag {
                let label = match drag.kind {
                    CanvasDragKind::Body => format!(
                        "移动 {:+.0}, {:+.0}",
                        drag.current.0 - drag.start.0,
                        drag.current.1 - drag.start.1
                    ),
                    CanvasDragKind::Handle(_) => {
                        format!("缩放 {:.0} × {:.0}", bounds.width, bounds.height)
                    }
                };
                artboard = artboard.child(
                    div()
                        .absolute()
                        .left(px(f32::from(rect.origin.x + rect.size.width) + 8.))
                        .top(px(f32::from(rect.origin.y)))
                        .px_2()
                        .py_1()
                        .rounded(px(4.))
                        .bg(rgb(theme::surface_elevated()))
                        .border_1()
                        .border_color(rgb(theme::separator()))
                        .text_xs()
                        .child(label),
                );
            }
        }
        area.child(
            artboard
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|view, event: &MouseDownEvent, _, cx| view.canvas_down(event, cx)),
                )
                .on_mouse_move(
                    cx.listener(|view, event: &MouseMoveEvent, _, cx| view.canvas_move(event, cx)),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|view, event: &MouseUpEvent, _, cx| view.canvas_up(event, cx)),
                ),
        )
        .into_any_element()
    }

    fn canvas_mode_button(
        &self,
        label: &str,
        editing: bool,
        cx: &mut ViewContext<Self>,
    ) -> gpui::Div {
        let id = if editing {
            "workcrate-canvas-edit"
        } else {
            "workcrate-canvas-preview"
        };
        let button = Button::new(id.to_owned()).compact().label(label.to_owned());
        super::tracking::control(
            id,
            if self.canvas_mode == editing {
                button.primary()
            } else {
                button.outline()
            }
            .disabled(self.busy)
            .on_click(cx.listener(move |view, _, _, cx| view.toggle_canvas_mode(editing, cx))),
            &self.surfaces,
        )
    }

    pub(super) fn canvas_toolbar(&self, cx: &mut ViewContext<Self>) -> gpui::Div {
        let row = div()
            .h_flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(rgb(theme::separator()))
            .child(self.canvas_mode_button("预览", false, cx))
            .child(self.canvas_mode_button("编辑", true, cx));
        if !self.canvas_mode {
            return row;
        }
        row.child(div().w(px(1.)).h(px(18.)).bg(rgb(theme::separator())))
            .child(self.action(
                "workcrate-canvas-zoom-out",
                "－",
                |v, cx| {
                    v.set_canvas_zoom(1. / 1.25, cx);
                },
                cx,
            ))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(theme::text_muted()))
                    .child(format!("{:.0}%", self.canvas_zoom * 100.)),
            )
            .child(self.action(
                "workcrate-canvas-zoom-in",
                "＋",
                |v, cx| {
                    v.set_canvas_zoom(1.25, cx);
                },
                cx,
            ))
            .child(div().w(px(1.)).h(px(18.)).bg(rgb(theme::separator())))
            .child(self.action(
                "workcrate-canvas-undo",
                "撤销",
                |v, cx| {
                    v.canvas_undo_redo(false, cx);
                },
                cx,
            ))
            .child(self.action(
                "workcrate-canvas-redo",
                "重做",
                |v, cx| {
                    v.canvas_undo_redo(true, cx);
                },
                cx,
            ))
            .child(div().flex_1())
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(theme::text_muted()))
                    .child(canvas_status(self.canvas.as_ref())),
            )
            .child(self.action(
                "workcrate-canvas-handoff",
                "在 VectorCraft 中编辑",
                |v, cx| v.canvas_handoff_editor(cx),
                cx,
            ))
    }
}

const CANVAS_BORDER: f32 = 2.;

fn selection_box(rect: Bounds<Pixels>) -> gpui::Div {
    div()
        .absolute()
        .left(px(f32::from(rect.origin.x)))
        .top(px(f32::from(rect.origin.y)))
        .w(px(f32::from(rect.size.width)))
        .h(px(f32::from(rect.size.height)))
        .border_1()
        .border_color(rgb(theme::accent()))
        .bg(rgba(theme::fill_transparent_overlay() | 24))
}

fn handle_box(center: Point<Pixels>) -> gpui::Div {
    div()
        .absolute()
        .left(px(f32::from(center.x) - HANDLE / 2.))
        .top(px(f32::from(center.y) - HANDLE / 2.))
        .w(px(HANDLE))
        .h(px(HANDLE))
        .border_1()
        .border_color(rgb(theme::accent()))
        .bg(rgb(theme::surface_elevated()))
}
