//! Canvas surface: the official bitmap, the selection overlay drawn from official bounds,
//! and the pointer events forwarded to VectorCraft as one gesture sequence.
use super::canvas::*;
use super::*;
use ::gpui::{Bounds, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, point, size};
use fileatlas_ui_bridge::components::WorkbenchPaint as _;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;

impl WorkCratePanel {
    fn canvas_down(&mut self, event: &MouseDownEvent, cx: &mut ViewContext<Self>) {
        if self.busy || self.canvas_busy {
            return;
        }
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
        let scale = drag.is_handle().then(|| drag.predicted());
        let events = drag.finish(map.to_document(event.position));
        match scale {
            Some(rect) => {
                let Some(snapshot) = self.canvas.clone() else {
                    return;
                };
                if snapshot.selection.is_empty() {
                    return;
                }
                let ids = snapshot.selection.clone();
                self.canvas_job(
                    move |boards| {
                        boards
                            .canvas_set_bounds(&ids, &rect)
                            .map_err(|e| e.to_string())
                    },
                    cx,
                );
            }
            None => self.canvas_job(
                move |boards| {
                    boards
                        .canvas_gesture(serde_json::Value::Array(events))
                        .map_err(|e| e.to_string())
                },
                cx,
            ),
        }
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
            .p(px(fileatlas_ui_bridge::theme::space_expanded()))
            .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Faint)
            // Explorer and other programs drop ExternalPaths; FileAtlas views drag typed payloads.
            .drag_over::<::gpui::ExternalPaths>(|style, _, _, _| {
                fileatlas_ui_bridge::components::drop_target_style(style)
            })
            .drag_over::<fileatlas_contracts::FileManagerFileDragPayload>(|style, _, _, _| {
                fileatlas_ui_bridge::components::drop_target_style(style)
            })
            .drag_over::<fileatlas_contracts::FileStagingDragPayload>(|style, _, _, _| {
                fileatlas_ui_bridge::components::drop_target_style(style)
            })
            .on_drop::<::gpui::ExternalPaths>(cx.listener(
                |view, paths: &::gpui::ExternalPaths, _, cx| {
                    view.canvas_drop_files(
                        paths
                            .paths()
                            .iter()
                            .map(|p| p.to_string_lossy().into_owned())
                            .collect(),
                        cx,
                    );
                },
            ))
            .on_drop::<fileatlas_contracts::FileManagerFileDragPayload>(cx.listener(
                |view, payload: &fileatlas_contracts::FileManagerFileDragPayload, _, cx| {
                    if payload.allows_drop() {
                        view.canvas_drop_files(payload.paths.clone(), cx);
                    }
                },
            ))
            .on_drop::<fileatlas_contracts::FileStagingDragPayload>(cx.listener(
                |view, payload: &fileatlas_contracts::FileStagingDragPayload, _, cx| {
                    view.canvas_drop_files(payload.paths.clone(), cx);
                },
            ))
            .child(super::tracking::measure(
                "canvas-viewport",
                self.surfaces.clone(),
            ));
        let Some(snapshot) = self.canvas.clone() else {
            return area
                .child(div().workbench_assistant().child(if self.canvas_busy {
                    "正在连接官方画布…"
                } else {
                    "官方画布未就绪"
                }))
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
        let mut artboard = fileatlas_ui_bridge::components::artboard(
            div()
                .id("workcrate-canvas-artboard")
                .flex_shrink_0()
                .w(width)
                .h(height)
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
                )),
        );
        if let (Some(map), Some(bounds)) = (local, frame) {
            let rect = map.rect_of(bounds);
            artboard = artboard.child(fileatlas_ui_bridge::components::selection_frame(
                f32::from(rect.origin.x),
                f32::from(rect.origin.y),
                f32::from(rect.size.width),
                f32::from(rect.size.height),
            ));
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
                        .px(px(fileatlas_ui_bridge::theme::space_standard()))
                        .py(px(fileatlas_ui_bridge::theme::space_compact()))
                        .workbench_card(false)
                        .workbench_assistant()
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
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(|view, event: &MouseUpEvent, _, cx| view.canvas_up(event, cx)),
                ),
        )
        .into_any_element()
    }

    pub(super) fn canvas_toolbar(&self, cx: &mut ViewContext<Self>) -> gpui::Div {
        use gpui_kit::assets::IconName;
        let divider = || {
            fileatlas_ui_bridge::components::separator_line(
                1.,
                18.,
                fileatlas_ui_bridge::components::DividerTone::Standard,
            )
        };
        div()
            .h_flex()
            .flex_wrap()
            .items_center()
            .gap(px(fileatlas_ui_bridge::theme::space_standard()))
            .px(px(fileatlas_ui_bridge::theme::space_expanded()))
            .py(px(fileatlas_ui_bridge::theme::space_standard()))
            .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Bottom)
            .child(self.screen_action(
                "workcrate-canvas-add-image",
                Some(IconName::ImagePlus),
                "添加图片",
                false,
                |v, cx| v.canvas_pick_and_place(false, cx),
                cx,
            ))
            .child(self.screen_action(
                "workcrate-canvas-create-text",
                Some(IconName::Type),
                "新建文字",
                false,
                |v, cx| v.canvas_create_text(cx),
                cx,
            ))
            .child(divider())
            .child(self.icon_action(
                "workcrate-canvas-zoom-out",
                IconName::ZoomOut,
                "缩小画布",
                |v, cx| v.set_canvas_zoom(1. / 1.25, cx),
                cx,
            ))
            .child(
                div()
                    .workbench_assistant()
                    .child(format!("{:.0}%", self.canvas_zoom * 100.)),
            )
            .child(self.icon_action(
                "workcrate-canvas-zoom-in",
                IconName::ZoomIn,
                "放大画布",
                |v, cx| v.set_canvas_zoom(1.25, cx),
                cx,
            ))
            .child(divider())
            .child(self.icon_action(
                "workcrate-canvas-undo",
                IconName::Undo2,
                "撤销上一步",
                |v, cx| v.canvas_undo_redo(false, cx),
                cx,
            ))
            .child(self.icon_action(
                "workcrate-canvas-redo",
                IconName::Redo2,
                "重做",
                |v, cx| v.canvas_undo_redo(true, cx),
                cx,
            ))
            .child(divider())
            .child(self.screen_action(
                "place-design-file",
                Some(IconName::FilePlus),
                "放置设计文件",
                false,
                |v, cx| v.place_file(cx),
                cx,
            ))
            .child(self.screen_action(
                "export-current-image",
                Some(IconName::FileImage),
                "导出本页图片",
                false,
                |v, cx| {
                    v.selected_pages = v.selected_board.iter().cloned().collect();
                    v.export_catalog(true, cx);
                },
                cx,
            ))
            .child(div().flex_1())
            .child(
                div()
                    .workbench_assistant()
                    .child(canvas_status(self.canvas.as_ref())),
            )
            .child(self.screen_action(
                "workcrate-canvas-handoff",
                Some(IconName::ExternalLink),
                "在 VectorCraft 中编辑",
                true,
                |v, cx| v.canvas_handoff_editor(cx),
                cx,
            ))
    }
}

const CANVAS_BORDER: f32 = 2.;
