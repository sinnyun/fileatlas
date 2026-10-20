use super::*;

pub(super) struct Drag {
    start: Point<Pixels>,
    end: Point<Pixels>,
    initial: Vec<u64>,
}

impl DebugView {
    pub(super) fn selection_controls(&self, s: &Arc<DebugService>, cx: &mut Context<Self>) -> Div {
        let ui = s.ui_state();
        div()
            .h_flex()
            .gap_1()
            .items_center()
            .child(div().text_color(rgb(theme::text_muted())).child(
                fileatlas_ui_bridge::i18n::tr_args(
                    "{count} selected",
                    &[("count", ui.selected_ids.len().to_string())],
                ),
            ))
            .child(
                Button::new("debug-copy")
                    .ghost()
                    .compact()
                    .label(tr("Copy logs"))
                    .disabled(ui.selected_ids.is_empty())
                    .on_click(cx.listener(|this, _, _, cx| this.perform_action("copy", cx))),
            )
            .child(
                Button::new("debug-export")
                    .ghost()
                    .compact()
                    .label(tr("Export .txt"))
                    .disabled(ui.selected_ids.is_empty() || self.exporting)
                    .on_click(cx.listener(|this, _, _, cx| this.perform_action("export", cx))),
            )
    }

    pub(super) fn perform_action(&mut self, action: &str, cx: &mut Context<Self>) {
        let Some(service) = self.service.upgrade() else {
            return;
        };
        let text = match service.selected_text() {
            Ok(text) => text,
            Err(error) => {
                service.feedback(error);
                cx.notify();
                return;
            }
        };
        if action == "copy" {
            cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
            let copied = cx
                .read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref()
                == Some(text.as_str());
            service.feedback(if copied {
                fileatlas_ui_bridge::i18n::tr_args(
                    "Copied {count} logs.",
                    &[("count", service.ui_state().selected_ids.len().to_string())],
                )
            } else {
                tr("Could not copy logs.")
            });
        } else if action == "export" && !self.exporting {
            self.exporting = true;
            cx.spawn(async move |this, cx| {
                let file = rfd::AsyncFileDialog::new()
                    .set_file_name("fileatlas-logs.txt")
                    .add_filter("Text", &["txt"])
                    .save_file()
                    .await;
                let message = if let Some(file) = file {
                    let path = file.path();
                    if !is_txt_export_path(path) {
                        tr("Choose a .txt file for the log export.")
                    } else {
                        match file.write(text.as_bytes()).await {
                            Ok(()) => fileatlas_ui_bridge::i18n::tr_args(
                                "Saved logs to {path}",
                                &[("path", path.display().to_string())],
                            ),
                            Err(error) => error.to_string(),
                        }
                    }
                } else {
                    tr("Export cancelled.")
                };
                let _ = this.update(cx, |this, cx| {
                    this.exporting = false;
                    service.feedback(message);
                    cx.notify();
                });
            })
            .detach();
        }
        cx.notify();
    }

    pub(super) fn selectable_list(
        &mut self,
        rows: Div,
        latest_id: u64,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.page_latest = latest_id;
        let bounds = Arc::clone(&self.list_bounds);
        let mut list = div()
            .id("debug-selection-area")
            .relative()
            .flex_1()
            .min_h_0()
            .w_full()
            .overflow_hidden()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.modifiers.control
                    && event.keystroke.key.eq_ignore_ascii_case("c")
                {
                    this.perform_action("copy", cx);
                    cx.stop_propagation();
                } else if event.keystroke.modifiers.control
                    && event.keystroke.key.eq_ignore_ascii_case("a")
                {
                    if let Some(s) = this.service.upgrade() {
                        s.freeze_view(this.page_latest);
                        let _ = s.set_selection(&this.page_ids, None);
                        cx.notify();
                    }
                    cx.stop_propagation();
                } else if event.keystroke.key == "escape" {
                    if let Some(s) = this.service.upgrade() {
                        let _ = s.set_selection(&[], None);
                        this.drag = None;
                        cx.notify();
                    }
                    cx.stop_propagation();
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    window.focus(&this.focus_handle, cx);
                    let Some(s) = this.service.upgrade() else {
                        return;
                    };
                    s.freeze_view(this.page_latest);
                    let ui = s.ui_state();
                    let hit = this
                        .row_bounds
                        .lock()
                        .unwrap()
                        .iter()
                        .find(|(id, b)| this.page_ids.contains(id) && b.contains(&event.position))
                        .map(|(id, _)| *id);
                    if event.modifiers.shift
                        && let (Some(anchor), Some(hit)) = (ui.anchor, hit)
                    {
                        if s.select_range(anchor, hit, event.modifiers.control).is_ok() {
                            cx.notify();
                            return;
                        }
                    }
                    let initial = if event.modifiers.control {
                        ui.selected_ids
                    } else {
                        Vec::new()
                    };
                    let mut selected = initial.clone();
                    if let Some(id) = hit {
                        if event.modifiers.control && selected.contains(&id) {
                            selected.retain(|v| *v != id)
                        } else {
                            selected.push(id)
                        }
                    }
                    let _ = s.set_selection(&selected, hit);
                    this.drag = Some(Drag {
                        start: event.position,
                        end: event.position,
                        initial,
                    });
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                if !event.dragging() {
                    if this.drag.take().is_some() {
                        cx.notify();
                    }
                    return;
                }
                let Some(drag) = this.drag.as_mut() else {
                    return;
                };
                if drag.end == event.position {
                    return;
                }
                drag.end = event.position;
                let band = selection_bounds(drag);
                let viewport = *this.list_bounds.lock().unwrap();
                let mut selected = drag.initial.clone();
                let rows = this.row_bounds.lock().unwrap();
                selected.extend(
                    this.page_ids
                        .iter()
                        .filter(|id| {
                            rows.get(id).is_some_and(|b| {
                                b.intersects(&band) && viewport.is_some_and(|v| b.intersects(&v))
                            })
                        })
                        .copied(),
                );
                if let Some(s) = this.service.upgrade() {
                    let _ = s.set_selection(&selected, None);
                }
                cx.notify();
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.drag.take().is_some() {
                        cx.notify();
                    }
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.drag.take().is_some() {
                        cx.notify();
                    }
                }),
            )
            .child(
                canvas(
                    move |layout, _, _| {
                        *bounds.lock().unwrap() = Some(layout);
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .child(
                div()
                    .id("debug-records")
                    .size_full()
                    .v_flex()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(rows),
            );
        if let (Some(drag), Some(viewport)) = (&self.drag, *self.list_bounds.lock().unwrap()) {
            let band = selection_bounds(drag).intersect(&viewport);
            list = list.child(
                div()
                    .absolute()
                    .left(band.origin.x - viewport.origin.x)
                    .top(band.origin.y - viewport.origin.y)
                    .w(band.size.width)
                    .h(band.size.height)
                    .border_1()
                    .border_color(rgb(theme::accent()))
                    .bg(rgba((theme::accent() << 8) | 0x22)),
            );
        }
        list.into_any_element()
    }
}

fn is_txt_export_path(path: &std::path::Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("txt"))
}

fn selection_bounds(drag: &Drag) -> Bounds<Pixels> {
    Bounds::new(
        point(drag.start.x.min(drag.end.x), drag.start.y.min(drag.end.y)),
        size(
            (drag.start.x - drag.end.x).abs().max(px(1.)),
            (drag.start.y - drag.end.y).abs().max(px(1.)),
        ),
    )
}
