use super::*;
use gpui_kit::component::input::{InputEvent, InputState};
use serde_json::{Value, json};

pub(super) struct TextOptionsEditor {
    pub(super) board: String,
    pub(super) id: String,
    pub(super) range: Option<[usize; 2]>,
    pub(super) scope: String,
    pub(super) key: String,
    pub(super) snapshot: Value,
    pub(super) value: gpui::Entity<InputState>,
    pub(super) search: gpui::Entity<InputState>,
    _subscription: gpui::Subscription,
    pub(super) busy: bool,
}

impl TextOptionsEditor {
    pub(super) fn current(&self) -> Value {
        if self.scope == "openType" {
            return json!(
                self.snapshot["openType"]["features"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .chain(
                        self.snapshot["openType"]["stylisticSets"]
                            .as_array()
                            .into_iter()
                            .flatten()
                    )
                    .find(|f| f["tag"] == self.key)
                    .is_some_and(|f| f["applied"] == "all")
            );
        }
        self.snapshot[&self.scope]
            .get(&self.key)
            .cloned()
            .unwrap_or(Value::Null)
    }
    pub(super) fn keys(&self, cx: &ViewContext<WorkCratePanel>) -> Vec<String> {
        let query = self.search.read(cx).value().to_lowercase();
        let keys: Vec<String> = if self.scope == "openType" {
            self.snapshot["openType"]["features"]
                .as_array()
                .into_iter()
                .flatten()
                .chain(
                    self.snapshot["openType"]["stylisticSets"]
                        .as_array()
                        .into_iter()
                        .flatten(),
                )
                .filter_map(|f| f["tag"].as_str().map(str::to_owned))
                .collect()
        } else {
            self.snapshot["defaults"][&self.scope]
                .as_object()
                .map(|m| m.keys().cloned().collect())
                .unwrap_or_default()
        };
        keys.into_iter()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .filter(|k: &String| {
                format!("{} {k}", super::text_option_labels::label(k))
                    .to_lowercase()
                    .contains(&query)
            })
            .collect()
    }
}

impl WorkCratePanel {
    pub(super) fn options_diagnostics(&self) -> Value {
        self.text_options.as_ref().map(|e|json!({"board":e.board,"id":e.id,"scope":e.scope,"key":e.key,"range":e.range,"busy":e.busy,"snapshot":e.snapshot})).unwrap_or(Value::Null)
    }
    pub(super) fn open_text_options(&mut self, window: &mut Window, cx: &mut ViewContext<Self>) {
        let Some(e) = &self.text_editor else { return };
        let range = e.body.read(cx).selected_range();
        let range = (range.start != range.end).then_some([range.start, range.end]);
        let board = e.board.clone();
        let id = e.id.clone();
        let family = self
            .boards_view
            .iter()
            .find(|c| c.board.id == board)
            .map(|c| {
                c.board.text_appearance(&id, 0)["chars"]["fontFamily"]
                    .as_str()
                    .unwrap_or("Microsoft YaHei")
                    .to_owned()
            })
            .unwrap_or_default();
        let value = cx.new(|cx| {
            let mut i = InputState::new(window, cx);
            i.set_value(family, window, cx);
            i
        });
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("搜索全部原生设置"));
        let subscription = cx.subscribe(&search, |_, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        self.text_options = Some(TextOptionsEditor {
            board: board.clone(),
            id: id.clone(),
            range,
            scope: "chars".into(),
            key: "fontFamily".into(),
            snapshot: Value::Null,
            value,
            search,
            _subscription: subscription,
            busy: true,
        });
        self.load_text_options(board, id, range, cx);
        cx.notify();
    }
    fn load_text_options(
        &mut self,
        board: String,
        id: String,
        range: Option<[usize; 2]>,
        cx: &mut ViewContext<Self>,
    ) {
        let boards = self.boards.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn({
                    let board = board.clone();
                    let id = id.clone();
                    async move {
                        boards
                            .text_options(&board, &id, range)
                            .map_err(|e| e.to_string())
                    }
                })
                .await;
            let _ = this.update(cx, |view, cx| {
                let Some(e) = view
                    .text_options
                    .as_mut()
                    .filter(|e| e.board == board && e.id == id && e.range == range)
                else {
                    return;
                };
                e.busy = false;
                match result {
                    Ok(v) => e.snapshot = v,
                    Err(e) => view.message = Some(e),
                };
                cx.notify();
            });
        })
        .detach();
    }
    pub(super) fn select_text_option(
        &mut self,
        scope: String,
        key: String,
        window: &mut Window,
        cx: &mut ViewContext<Self>,
    ) {
        let Some(e) = &mut self.text_options else {
            return;
        };
        if e.scope != scope {
            e.search.update(cx, |s, cx| s.set_value("", window, cx));
        }
        e.scope = scope;
        let key = if e.scope == "openType" {
            let keys = e.keys(cx);
            if keys.contains(&key) {
                key
            } else {
                keys.into_iter().next().unwrap_or_default()
            }
        } else {
            key
        };
        e.key = key;
        let v = e.current();
        let display = if let Some(s) = v.as_str() {
            s.to_owned()
        } else {
            v.to_string()
        };
        e.value.update(cx, |i, cx| i.set_value(display, window, cx));
        cx.notify();
    }
    pub(super) fn apply_current_text_option(&mut self, cx: &mut ViewContext<Self>) {
        let Some(e) = &self.text_options else { return };
        let raw = e.value.read(cx).value().to_string();
        let current = e.current();
        let parsed = if current.is_string() {
            Ok(Value::String(raw))
        } else {
            serde_json::from_str(&raw)
        };
        match parsed {
            Ok(v) => self.apply_text_option(e.scope.clone(), json!({e.key.clone():v}), true, cx),
            Err(err) => {
                self.message = Some(format!("选项值格式不正确：{err}"));
                cx.notify();
            }
        }
    }
    pub(super) fn apply_native_text_option(
        &mut self,
        scope: String,
        attrs: Value,
        cx: &mut ViewContext<Self>,
    ) {
        self.apply_text_option(scope, attrs, false, cx);
    }
    fn apply_text_option(
        &mut self,
        scope: String,
        attrs: Value,
        saved_range: bool,
        cx: &mut ViewContext<Self>,
    ) {
        if self.text_write_active {
            return;
        }
        let Some(e) = &self.text_editor else { return };
        let board = e.board.clone();
        let id = e.id.clone();
        let selected = e.body.read(cx).selected_range();
        let range = self
            .text_options
            .as_ref()
            .filter(|o| saved_range && o.board == board && o.id == id)
            .map(|o| o.range)
            .unwrap_or_else(|| {
                (selected.start != selected.end).then_some([selected.start, selected.end])
            });
        if let Some(o) = &mut self.text_options {
            if o.busy {
                return;
            }
            o.busy = true;
        }
        self.text_format_busy = true;
        let service = self.boards.clone();
        let scale = self.fit_scale * self.device_scale;
        cx.spawn(async move |this, cx| {
            let target = id.clone();
            let result = cx
                .background_executor()
                .spawn(async move {
                    let board = service
                        .set_text_options(&board, &target, range, &scope, attrs)
                        .map_err(|e| e.to_string())?;
                    let options = service
                        .text_options(&board.id, &target, range)
                        .map_err(|e| e.to_string())?;
                    let r = service
                        .render_text(&board.id, &target, scale)
                        .map_err(|e| e.to_string())?;
                    let pixels = image::RgbaImage::from_raw(r.width, r.height, r.to_straight())
                        .ok_or("文本像素无效")?;
                    Ok::<_, String>((board, options, render_image(pixels)))
                })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.text_format_busy = false;
                if let Some(o) = &mut view.text_options {
                    o.busy = false;
                }
                match result {
                    Ok((board, options, pixels)) => {
                        if let Some(card) =
                            view.boards_view.iter_mut().find(|c| c.board.id == board.id)
                        {
                            card.board = board.clone();
                        }
                        if view.selected_board.as_ref() == Some(&board.id) {
                            view.text_images.insert(id.clone(), pixels);
                            view.text_inline = false;
                        }
                        if let Some(o) = view
                            .text_options
                            .as_mut()
                            .filter(|o| o.board == board.id && o.id == id)
                        {
                            o.snapshot = options;
                        }
                        view.refresh_page_preview(board.id, cx);
                    }
                    Err(e) => view.message = Some(e),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}
