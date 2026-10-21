use super::*;
use gpui_kit::component::Sizable as _;

#[derive(Clone)]
pub(super) struct TextGesture {
    pub id: String,
    pub start: gpui::Point<gpui::Pixels>,
    pub before: [f64; 4],
    pub frame: [f64; 4],
    pub corner: Option<usize>,
}

impl WorkCratePanel {
    pub(super) fn text_layer(
        &self,
        board: &WorkCrateBoard,
        id: &str,
        fit: f64,
        header: f64,
        overlay: bool,
        cx: &mut ViewContext<Self>,
    ) -> gpui::AnyElement {
        let Ok(before) = board.text_frame(id) else {
            return div().into_any_element();
        };
        let f = self
            .text_gesture
            .as_ref()
            .filter(|g| g.id == id)
            .map(|g| g.frame)
            .unwrap_or(before);
        let selected = self.selected_text.as_deref() == Some(id);
        let mut frame = div()
            .id(format!("canvas-text-{id}-{overlay}"))
            .absolute()
            .left(px((f[0] * fit) as f32))
            .top(px(((f[1] - header) * fit) as f32))
            .w(px(((f[2] - f[0]) * fit) as f32))
            .h(px(((f[3] - f[1]) * fit) as f32));
        let inline =
            selected && self.text_inline && self.text_editor.as_ref().is_some_and(|e| e.id == id);
        if !overlay && inline {
            return div().into_any_element();
        }
        if !overlay {
            if let Some(image) = self.text_images.get(id).cloned().filter(|_| !inline) {
                frame = frame.child(
                    div().size_full().overflow_hidden().child(
                        img(image)
                            .w(px(((before[2] - before[0]) * fit) as f32))
                            .h(px(((before[3] - before[1]) * fit) as f32)),
                    ),
                );
            }
            let id = id.to_owned();
            frame = frame.cursor(gpui::CursorStyle::OpenHand).on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |view, event: &gpui::MouseDownEvent, window, cx| {
                    if event.click_count >= 2 {
                        view.selected_text = Some(id.clone());
                        view.selected_image = None;
                        view.text_gesture = None;
                        view.sync_text_editor(window, cx);
                        view.text_inline = true;
                        if let Some(e) = &view.text_editor {
                            e.body.update(cx, |s, cx| s.focus(window, cx));
                        }
                        cx.notify();
                        cx.stop_propagation();
                        return;
                    }
                    view.begin_text_gesture(id.clone(), None, event.position, cx);
                    cx.stop_propagation();
                }),
            );
        } else if selected {
            if inline {
                if let Some(e) = &self.text_editor {
                    let size = e
                        .size
                        .read(cx)
                        .value()
                        .parse::<f32>()
                        .ok()
                        .filter(|v| v.is_finite() && (0.1..=1296.).contains(v))
                        .unwrap_or(board.texts[id].size_pt as f32)
                        * fit as f32;
                    let color = e
                        .color
                        .read(cx)
                        .value()
                        .unwrap_or(rgb(theme::text_primary()).into());
                    let appearance =
                        board.text_appearance(id, e.body.read(cx).selected_range().start);
                    let family = appearance["chars"]["fontFamily"]
                        .as_str()
                        .unwrap_or("Microsoft YaHei")
                        .to_owned();
                    let font_style = appearance["chars"]["fontStyle"]
                        .as_str()
                        .unwrap_or("Regular")
                        .to_lowercase();
                    let leading = appearance["chars"]["leading"]["value"]
                        .as_f64()
                        .filter(|v| *v > 0.)
                        .map(|v| v as f32 * fit as f32)
                        .unwrap_or(
                            size * appearance["para"]["autoLeading"].as_f64().unwrap_or(1.2) as f32,
                        );
                    let padding = gpui_kit::component::Size::XSmall.input_px();
                    frame = frame.child(
                        super::tracking::control(
                            "canvas-text-input",
                            div()
                                .relative()
                                .size_full()
                                .overflow_hidden()
                                .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| {
                                    cx.stop_propagation()
                                })
                                .child(
                                    fileatlas_ui_bridge::components::textarea(&e.body)
                                        .readonly(self.text_format_busy)
                                        .bg(gpui::transparent_black())
                                        .with_size(gpui_kit::component::Size::XSmall)
                                        .appearance(false)
                                        .bordered(false)
                                        .absolute()
                                        .left(-padding)
                                        .top_0()
                                        .h(px(((f[3] - f[1]) * fit) as f32))
                                        .w(px(((f[2] - f[0]) * fit) as f32) + padding * 2.)
                                        .p_0()
                                        .text_size(px(size))
                                        .line_height(px(leading))
                                        .font_family(family)
                                        .when(font_style.contains("bold"), |t| t.font_bold())
                                        .when(
                                            font_style.contains("italic")
                                                || font_style.contains("oblique"),
                                            |t| t.italic(),
                                        )
                                        .when(appearance["chars"]["underline"] == true, |t| {
                                            t.underline()
                                        })
                                        .when(appearance["chars"]["strikethrough"] == true, |t| {
                                            t.line_through()
                                        })
                                        .when(appearance["para"]["align"] == "center", |t| {
                                            t.text_center()
                                        })
                                        .when(appearance["para"]["align"] == "right", |t| {
                                            t.text_right()
                                        })
                                        .text_color(color),
                                ),
                            &self.surfaces,
                        )
                        .size_full(),
                    );
                }
                let id = id.to_owned();
                frame = frame.child(
                    div()
                        .id("text-move-strip")
                        .absolute()
                        .top(px(-20.))
                        .left_0()
                        .h(px(20.))
                        .w_full()
                        .text_xs()
                        .bg(rgb(theme::surface_base()))
                        .text_color(rgb(theme::text_primary()))
                        .cursor(gpui::CursorStyle::OpenHand)
                        .child("拖动移动文本")
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |view, event: &gpui::MouseDownEvent, _, cx| {
                                view.begin_text_gesture(id.clone(), None, event.position, cx);
                                cx.stop_propagation();
                            }),
                        ),
                );
            }
            frame = frame.border_1().border_color(rgb(theme::accent()));
            for corner in 0..4 {
                let id = id.to_owned();
                let right = corner % 2 == 1;
                let bottom = corner >= 2;
                frame = frame.child(
                    div()
                        .id(format!("text-handle-{id}-{corner}"))
                        .absolute()
                        .left(px(if right {
                            ((f[2] - f[0]) * fit) as f32 - 5.
                        } else {
                            -5.
                        }))
                        .top(px(if bottom {
                            ((f[3] - f[1]) * fit) as f32 - 5.
                        } else {
                            -5.
                        }))
                        .size(px(10.))
                        .border_2()
                        .border_color(rgb(theme::accent()))
                        .bg(rgb(theme::surface_base()))
                        .cursor(if right == bottom {
                            gpui::CursorStyle::ResizeUpLeftDownRight
                        } else {
                            gpui::CursorStyle::ResizeUpRightDownLeft
                        })
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |view, event: &gpui::MouseDownEvent, _, cx| {
                                view.begin_text_gesture(
                                    id.clone(),
                                    Some(corner),
                                    event.position,
                                    cx,
                                );
                                cx.stop_propagation();
                            }),
                        ),
                );
            }
        }
        frame.into_any_element()
    }

    fn begin_text_gesture(
        &mut self,
        id: String,
        corner: Option<usize>,
        position: gpui::Point<gpui::Pixels>,
        cx: &mut ViewContext<Self>,
    ) {
        if self.busy || self.edit_saving {
            return;
        }
        let Some(before) = self
            .boards_view
            .iter()
            .find(|c| Some(&c.board.id) == self.selected_board.as_ref())
            .and_then(|c| c.board.text_frame(&id).ok())
        else {
            return;
        };
        self.selected_text = Some(id.clone());
        self.selected_image = None;
        self.gesture = None;
        self.crop_mode = false;
        self.text_gesture = Some(TextGesture {
            id,
            start: position,
            before,
            frame: before,
            corner,
        });
        cx.notify();
    }

    pub(super) fn move_text_gesture(
        &mut self,
        position: gpui::Point<gpui::Pixels>,
        cx: &mut ViewContext<Self>,
    ) {
        let Some(g) = &mut self.text_gesture else {
            return;
        };
        let page = self
            .boards_view
            .iter()
            .find(|c| Some(&c.board.id) == self.selected_board.as_ref())
            .map(|c| {
                (
                    c.board.source.settings.page_width,
                    c.board.source.settings.page_height,
                )
            })
            .unwrap_or((1190.55, 841.89));
        let f = g.before;
        let mut dx = f64::from(position.x - g.start.x) / self.fit_scale;
        let mut dy = f64::from(position.y - g.start.y) / self.fit_scale;
        let mut next = f;
        if let Some(corner) = g.corner {
            if corner % 2 == 0 {
                next[0] = (f[0] + dx).clamp(0., f[2] - 8.);
            } else {
                next[2] = (f[2] + dx).clamp(f[0] + 8., page.0.max(f[0] + 8.));
            }
            if corner < 2 {
                next[1] = (f[1] + dy).clamp(page.1 * 0.06, f[3] - 8.);
            } else {
                next[3] = (f[3] + dy).clamp(f[1] + 8., page.1.max(f[1] + 8.));
            }
        } else {
            dx = dx.clamp(-f[0], (page.0 - f[2]).max(-f[0]));
            dy = dy.clamp(
                page.1 * 0.06 - f[1],
                (page.1 - f[3]).max(page.1 * 0.06 - f[1]),
            );
            next = [f[0] + dx, f[1] + dy, f[2] + dx, f[3] + dy];
        }
        if next != g.frame {
            g.frame = next;
            cx.notify();
        }
    }

    pub(super) fn end_text_gesture(&mut self, window: &mut Window, cx: &mut ViewContext<Self>) {
        if self.edit_saving {
            return;
        }
        let Some(g) = self.text_gesture.clone() else {
            return;
        };
        if g.frame == g.before {
            self.text_gesture = None;
            cx.notify();
            return;
        }
        self.sync_text_editor(window, cx);
        if let Some(e) = &mut self.text_editor {
            e.frame = g.frame;
            e.width.update(cx, |s, cx| {
                s.set_value(format!("{:.1}", g.frame[2] - g.frame[0]), window, cx)
            });
            e.height.update(cx, |s, cx| {
                s.set_value(format!("{:.1}", g.frame[3] - g.frame[1]), window, cx)
            });
            self.edit_saving = true;
            self.queue_text_update(cx);
        }
    }
}
