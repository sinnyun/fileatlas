use super::*;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use serde_json::{Value, json};
impl WorkCratePanel {
    pub(super) fn text_options_panel(&self, cx: &mut ViewContext<Self>) -> gpui::Div {
        let mut section = div().v_flex().gap_2();
        let Some(text) = &self.text_editor else {
            return section;
        };
        let appearance = self
            .boards_view
            .iter()
            .find(|c| c.board.id == text.board)
            .map(|c| {
                c.board
                    .text_appearance(&text.id, text.body.read(cx).selected_range().start)
            })
            .unwrap_or(Value::Null);
        let chars = &appearance["chars"];
        let mut quick = div().h_flex().flex_wrap().gap_1();
        let font_style = chars["fontStyle"]
            .as_str()
            .unwrap_or("Regular")
            .to_lowercase();
        for (id, label, scope, key, on, value) in [
            (
                "bold",
                "粗体",
                "action",
                "command",
                font_style.contains("bold"),
                json!("type.bold"),
            ),
            (
                "italic",
                "斜体",
                "action",
                "command",
                font_style.contains("italic") || font_style.contains("oblique"),
                json!("type.italic"),
            ),
            (
                "underline",
                "下划线",
                "chars",
                "underline",
                chars["underline"] == true,
                json!(chars["underline"] != true),
            ),
            (
                "strike",
                "删除线",
                "chars",
                "strikethrough",
                chars["strikethrough"] == true,
                json!(chars["strikethrough"] != true),
            ),
        ] {
            quick = quick.child(super::tracking::control(
                &format!("text-{id}"),
                Button::new(format!("text-{id}"))
                    .compact()
                    .ghost()
                    .when(on, |b| b.outline())
                    .label(label)
                    .disabled(self.text_write_active || self.text_format_busy)
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.apply_native_text_option(scope.into(), json!({key:value}), cx)
                    })),
                &self.surfaces,
            ));
        }
        let mut align = div().h_flex().flex_wrap().gap_1();
        for (value, label) in [
            ("left", "左对齐"),
            ("center", "居中"),
            ("right", "右对齐"),
            ("fullyJustified", "两端对齐"),
        ] {
            align = align.child(super::tracking::control(
                &format!("text-align-{value}"),
                Button::new(format!("text-align-{value}"))
                    .compact()
                    .ghost()
                    .label(label)
                    .disabled(self.text_write_active || self.text_format_busy)
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.apply_native_text_option("para".into(), json!({"align":value}), cx)
                    })),
                &self.surfaces,
            ));
        }
        section = section
            .child(quick)
            .child(align)
            .child(super::tracking::control(
                "text-more",
                Button::new("text-more")
                    .outline()
                    .compact()
                    .label("更多文本设置…")
                    .disabled(self.text_write_active || self.text_format_busy)
                    .on_click(
                        cx.listener(|view, _, window, cx| view.open_text_options(window, cx)),
                    ),
                &self.surfaces,
            ));
        let Some(e) = self
            .text_options
            .as_ref()
            .filter(|e| e.id == text.id && e.board == text.board)
        else {
            return section;
        };
        let mut groups = div().h_flex().flex_wrap().gap_1();
        for (scope, label, first) in [
            ("chars", "字符", "fontFamily"),
            ("para", "段落", "align"),
            ("frame", "文本框", "columns"),
            ("openType", "OpenType", "liga"),
        ] {
            groups = groups.child(super::tracking::control(
                &format!("text-option-scope-{scope}"),
                Button::new(format!("text-group-{scope}"))
                    .ghost()
                    .compact()
                    .when(e.scope == scope, |b| b.outline())
                    .label(label)
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.select_text_option(scope.into(), first.into(), window, cx)
                    })),
                &self.surfaces,
            ));
        }
        section = section.child(groups).child(div().text_xs().child(
            if let Some(r) = e.range.filter(|_| e.scope != "frame") {
                let body = text.body.read(cx).value().to_string();
                format!(
                    "应用于已选文字（{} 个字符）",
                    body.get(r[0]..r[1]).unwrap_or_default().chars().count()
                )
            } else {
                "应用于整个文本框".into()
            },
        ));
        if e.busy {
            return section.child("正在读取/保存原生文本设置…");
        }
        section = section.child(super::tracking::control(
            "text-option-search",
            fileatlas_ui_bridge::components::text_input(&e.search).w_full(),
            &self.surfaces,
        ));
        let keys = e.keys(cx);
        if e.scope == "openType" && keys.is_empty() {
            return section.child("当前字体没有可用的 OpenType 特性");
        }
        let weak = cx.entity().downgrade();
        let scope = e.scope.clone();
        section = section.child(super::tracking::control(
            "text-option-key",
            Button::new("text-option-key")
                .outline()
                .compact()
                .label(format!(
                    "{} · {}",
                    super::text_option_labels::label(&e.key),
                    e.key
                ))
                .dropdown_menu(move |mut menu, _, _| {
                    for key in &keys {
                        let key = key.clone();
                        let weak = weak.clone();
                        let scope = scope.clone();
                        menu = menu.item(
                            PopupMenuItem::new(format!(
                                "{} · {key}",
                                super::text_option_labels::label(&key)
                            ))
                            .on_click(move |_, window, cx| {
                                let _ = weak.update(cx, |view, cx| {
                                    view.select_text_option(scope.clone(), key.clone(), window, cx)
                                });
                            }),
                        );
                    }
                    menu
                }),
            &self.surfaces,
        ));
        let current = e.current();
        section = section.child(div().text_xs().child(format!(
            "当前值：{}",
            if let Some(s) = current.as_str() {
                s.to_owned()
            } else {
                current.to_string()
            }
        )));
        let choices = super::text_option_labels::choices(&e.key, &current);
        let mut edit = div().h_flex().gap_1().child(
            super::tracking::control(
                "text-option-value",
                fileatlas_ui_bridge::components::text_input(&e.value).w_full(),
                &self.surfaces,
            )
            .flex_1()
            .min_w_0(),
        );
        if !choices.is_empty() {
            let weak = cx.entity().downgrade();
            edit = edit.child(
                Button::new("text-option-choices")
                    .outline()
                    .compact()
                    .label("⌄")
                    .dropdown_menu(move |mut menu, _, _| {
                        for v in &choices {
                            let v = v.clone();
                            let weak = weak.clone();
                            menu = menu.item(PopupMenuItem::new(v.clone()).on_click(
                                move |_, window, cx| {
                                    let _ = weak.update(cx, |view, cx| {
                                        if let Some(e) = &view.text_options {
                                            e.value.update(cx, |i, cx| {
                                                i.set_value(v.clone(), window, cx)
                                            });
                                        }
                                        cx.notify();
                                    });
                                },
                            ));
                        }
                        menu
                    }),
            );
        }
        section
            .child(edit)
            .child(super::tracking::control(
                "text-option-apply",
                Button::new("text-option-apply")
                    .primary()
                    .compact()
                    .label("应用设置")
                    .disabled(self.text_write_active || self.text_format_busy)
                    .on_click(cx.listener(|view, _, _, cx| view.apply_current_text_option(cx))),
                &self.surfaces,
            ))
            .child(
                div().text_xs().text_color(rgb(theme::text_muted())).child(
                    "数值单位沿用原生 pt / em；复合值可填写 JSON。更多设置使用原生排版预览。",
                ),
            )
    }
}
