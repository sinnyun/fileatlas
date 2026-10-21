use super::*;
use gpui_kit::component::{
    color_picker::{ColorPickerEvent, ColorPickerState, ColorSelect},
    input::{InputEvent, InputState, TextareaState},
};

pub(super) struct TextEditor {
    pub id: String,
    pub(super) board: String,
    pub(super) source_revision: String,
    pub(super) frame: [f64; 4],
    pub(super) body: gpui::Entity<TextareaState>,
    pub(super) size: gpui::Entity<InputState>,
    pub(super) width: gpui::Entity<InputState>,
    pub(super) height: gpui::Entity<InputState>,
    pub(super) color: gpui::Entity<ColorPickerState>,
    pub(super) selection: std::ops::Range<usize>,
    _subscriptions: Vec<gpui::Subscription>,
}

fn input(
    value: String,
    window: &mut Window,
    cx: &mut ViewContext<WorkCratePanel>,
) -> gpui::Entity<InputState> {
    cx.new(|cx| {
        let mut s = InputState::new(window, cx);
        s.set_value(value, window, cx);
        s
    })
}

impl WorkCratePanel {
    pub(super) fn text_draft(&self, cx: &ViewContext<Self>) -> Option<serde_json::Value> {
        let e = self.text_editor.as_ref()?;
        Some(
            serde_json::json!({"id":e.id,"body":e.body.read(cx).value().to_string(),"selection":[e.body.read(cx).selected_range().start,e.body.read(cx).selected_range().end],"size":e.size.read(cx).value().to_string(),"color":e.color.read(cx).value().map(|v|format!("#{:06x}",theme::packed(v)))}),
        )
    }
    pub(super) fn sync_text_editor(&mut self, window: &mut Window, cx: &mut ViewContext<Self>) {
        let selected = self
            .boards_view
            .iter()
            .find(|c| Some(&c.board.id) == self.selected_board.as_ref())
            .and_then(|c| {
                self.selected_text.as_ref().and_then(|id| {
                    Some((
                        c.board.id.clone(),
                        id.clone(),
                        c.board.texts.get(id)?.clone(),
                        c.board.text_frame(id).ok()?,
                    ))
                })
            });
        let Some((board, id, text, frame)) = selected else {
            self.text_editor = None;
            self.text_inline = false;
            return;
        };
        let source_revision = serde_json::to_string(&text).unwrap_or_default();
        if let Some(editor) = &mut self.text_editor {
            if editor.id == id && editor.board == board && editor.source_revision == source_revision
            {
                if editor.frame != frame && !self.text_write_active {
                    editor.frame = frame;
                    editor.width.update(cx, |i, cx| {
                        i.set_value(format!("{:.1}", frame[2] - frame[0]), window, cx)
                    });
                    editor.height.update(cx, |i, cx| {
                        i.set_value(format!("{:.1}", frame[3] - frame[1]), window, cx)
                    });
                }
                return;
            }
        }
        let body = cx.new(|cx| {
            let mut s = TextareaState::new(window, cx);
            s.set_value(text.text, window, cx);
            s
        });
        if self.text_inline {
            body.update(cx, |s, cx| s.focus(window, cx));
        }
        let size = input(text.size_pt.to_string(), window, cx);
        let width = input(format!("{:.1}", frame[2] - frame[0]), window, cx);
        let height = input(format!("{:.1}", frame[3] - frame[1]), window, cx);
        let rgb_value = theme::parse_hex_color(&text.color)
            .map(|[r, g, b]| u32::from_be_bytes([0, r, g, b]))
            .unwrap_or_default();
        let color = cx.new(|cx| ColorPickerState::new(window, cx).default_value(rgb(rgb_value)));
        let subscription = cx.subscribe(&color, |view: &mut Self, _, event, cx| {
            if matches!(event, ColorPickerEvent::Change(_)) {
                view.queue_text_update(cx);
            }
        });
        let mut subscriptions = vec![subscription];
        subscriptions.push(cx.subscribe(&body, |view, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                view.queue_text_update(cx);
            }
        }));
        for field in [&size, &width, &height] {
            subscriptions.push(cx.subscribe(field, |view, _, event, cx| {
                if matches!(event, InputEvent::Change) {
                    view.queue_text_update(cx);
                }
            }));
        }
        subscriptions.push(cx.observe(&body, |view, _, cx| {
            if let Some(e) = &mut view.text_editor {
                let range = e.body.read(cx).selected_range();
                if e.selection != range {
                    e.selection = range;
                    cx.notify();
                }
            }
        }));
        self.text_editor = Some(TextEditor {
            id,
            board,
            source_revision,
            frame,
            body,
            size,
            width,
            height,
            color,
            selection: 0..0,
            _subscriptions: subscriptions,
        });
    }

    pub(super) fn text_inspector(&self, card: &BoardCard, cx: &mut ViewContext<Self>) -> gpui::Div {
        let mut section = div()
            .v_flex()
            .gap_2()
            .pt_2()
            .child(div().font_semibold().child("页面文本"));
        for (id, text) in &card.board.texts {
            let selected = self.selected_text.as_ref() == Some(id);
            let id = id.clone();
            let control_id = format!("select-text-{id}");
            section = section.child(super::tracking::control(
                &control_id,
                Button::new(format!("select-text-{id}"))
                    .ghost()
                    .compact()
                    .when(selected, |b| b.outline())
                    .label(
                        text.text
                            .lines()
                            .next()
                            .filter(|t| !t.is_empty())
                            .unwrap_or("空文本")
                            .to_owned(),
                    )
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.selected_text = Some(id.clone());
                        view.selected_image = None;
                        view.text_editor = None;
                        view.text_inline = false;
                        cx.notify();
                    })),
                &self.surfaces,
            ));
        }
        let Some(editor) = &self.text_editor else {
            return section.child(
                div()
                    .text_xs()
                    .text_color(rgb(theme::text_muted()))
                    .child("点击添加文本，或选择画布中的文本框"),
            );
        };
        macro_rules! tracked {
            ($id:expr,$element:expr) => {
                super::tracking::control($id, $element, &self.surfaces)
            };
        }
        section = section
            .child(div().text_xs().child(format!(
                        "字体：{}",
                        card.board.text_appearance(
                            &editor.id,
                            editor.body.read(cx).selected_range().start
                        )["chars"]["fontFamily"]
                            .as_str()
                            .unwrap_or("Microsoft YaHei")
                    )))
            .when(!self.text_inline, |s| {
                s.child(tracked!(
                    "text-body",
                    fileatlas_ui_bridge::components::textarea(&editor.body)
                        .readonly(self.text_format_busy)
                        .h(px(100.))
                        .w_full()
                ))
            })
            .child(div().text_xs().child(if self.text_write_active {
                "正在保存文本…"
            } else if self.text_inline {
                "画布中输入 · 拖动上沿移动 · 四角调整框"
            } else {
                "双击画布文本直接输入 · 修改自动保存"
            }))
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_2()
                    .child("字号")
                    .child(tracked!(
                        "text-size",
                        fileatlas_ui_bridge::components::text_input(&editor.size).w(px(70.))
                    ))
                    .child("pt"),
            )
            .child(
                div().h_flex().items_center().gap_2().child("颜色").child(
                    super::tracking::control(
                        "text-color",
                        ColorSelect::new(&editor.color)
                            .accessibility_label("文字颜色")
                            .w(px(150.)),
                        &self.surfaces,
                    ),
                ),
            )
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_2()
                    .child("宽")
                    .child(tracked!(
                        "text-width",
                        fileatlas_ui_bridge::components::text_input(&editor.width).w(px(65.))
                    ))
                    .child("高")
                    .child(tracked!(
                        "text-height",
                        fileatlas_ui_bridge::components::text_input(&editor.height).w(px(65.))
                    ))
                    .child("pt"),
            )
            .child(
                div()
                    .h_flex()
                    .gap_2()
                    .child(tracked!(
                        "text-save",
                        Button::new("text-save")
                            .primary()
                            .compact()
                            .label("保存文本")
                            .disabled(self.busy)
                            .on_click(cx.listener(|view, _, _, cx| view.save_text(cx)))
                    ))
                    .child(super::tracking::control(
                        "text-delete",
                        Button::new("text-delete")
                            .ghost()
                            .compact()
                            .label("删除文本")
                            .disabled(self.busy || self.text_write_active)
                            .on_click(cx.listener(|view, _, _, cx| view.delete_selected_text(cx))),
                        &self.surfaces,
                    )),
            );
        section.child(self.text_options_panel(cx))
    }

    pub(super) fn add_canvas_text(&mut self, cx: &mut ViewContext<Self>) {
        let Some(id) = self.selected_board.clone() else {
            return;
        };
        if self.busy {
            return;
        }
        self.busy = true;
        let boards = self.boards.clone();
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { boards.add_text(&id).map_err(|e| e.to_string()) })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                match result {
                    Ok(board) => {
                        view.selected_text = board
                            .texts
                            .keys()
                            .max_by_key(|id| id.parse::<u64>().unwrap_or_default())
                            .cloned();
                        view.selected_image = None;
                        view.text_editor = None;
                        view.text_inline = true;
                        if let Some(card) =
                            view.boards_view.iter_mut().find(|c| c.board.id == board.id)
                        {
                            card.board = board.clone();
                        }
                        view.refresh_page_preview(board.id, cx);
                    }
                    Err(e) => view.message = Some(e),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn save_text(&mut self, cx: &mut ViewContext<Self>) {
        if self.live_text_draft(cx).is_none() {
            self.message = Some("字号需为 0.1～1296 pt，文本框宽高不能小于 4 pt".into());
            cx.notify();
            return;
        }
        self.queue_text_update(cx);
    }

    fn delete_selected_text(&mut self, cx: &mut ViewContext<Self>) {
        let (Some(board), Some(id)) = (self.selected_board.clone(), self.selected_text.take())
        else {
            return;
        };
        let boards = self.boards.clone();
        self.text_editor = None;
        self.perform(
            move || {
                boards
                    .delete_text(&board, &id)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            },
            cx,
        );
    }
}
