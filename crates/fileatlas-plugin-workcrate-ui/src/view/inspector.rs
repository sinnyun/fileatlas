use super::preview::pixel_label;
use super::*;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};

impl WorkCratePanel {
    pub(super) fn edit_row(&self, cx: &mut ViewContext<Self>) -> gpui::Div {
        let saving = self
            .editing
            .as_ref()
            .is_some_and(|field| self.field_saves.contains(field));
        div()
            .h_flex()
            .items_center()
            .h(px(32.))
            .gap_1()
            .child(
                super::tracking::control(
                    "field-edit-input",
                    fileatlas_ui_bridge::components::text_input(&self.edit_input).w_full(),
                    &self.surfaces,
                )
                .flex_1()
                .min_w_0(),
            )
            .child(super::tracking::control(
                "field-edit-save",
                Button::new("workcrate-edit-save")
                    .primary()
                    .compact()
                    .label(if saving { "保存中" } else { "保存" })
                    .disabled(self.busy || saving)
                    .on_click(cx.listener(|view, _, _, cx| view.save_edit(cx))),
                &self.surfaces,
            ))
            .child(super::tracking::control(
                "field-edit-cancel",
                Button::new("workcrate-edit-cancel")
                    .ghost()
                    .compact()
                    .label("取消")
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.editing = None;
                        cx.notify();
                    })),
                &self.surfaces,
            ))
    }

    fn field(
        &self,
        id: &str,
        key: &str,
        label: &str,
        value: String,
        cx: &mut ViewContext<Self>,
    ) -> gpui::Div {
        let target = id.to_owned();
        let field_key = key.to_owned();
        let saving = self.field_saves.contains(&(id.to_owned(), key.to_owned()));
        let editing = self.editing.as_ref() == Some(&(id.into(), key.into()));
        let displayed = if saving {
            "保存中…".into()
        } else if value.is_empty() {
            "点击填写".into()
        } else {
            value.clone()
        };
        // The row keeps one height whether it shows a value or an editor, so committing or
        // cancelling a field never reflows the panel around it.
        let row = div().h_flex().items_center().h(px(32.)).gap_2().child(
            div()
                .w(px(72.))
                .flex_shrink_0()
                .text_size(px(12.))
                .text_color(rgb(theme::text_secondary()))
                .child(label.to_owned()),
        );
        if editing {
            return row
                .child(
                    super::tracking::control(
                        "field-edit-input",
                        fileatlas_ui_bridge::components::text_input(&self.edit_input).w_full(),
                        &self.surfaces,
                    )
                    .flex_1()
                    .min_w_0(),
                )
                .child(super::tracking::control(
                    "field-edit-save",
                    Button::new("workcrate-edit-save")
                        .primary()
                        .compact()
                        .label(if saving { "保存中" } else { "保存" })
                        .disabled(self.busy || saving)
                        .on_click(cx.listener(|view, _, _, cx| view.save_edit(cx))),
                    &self.surfaces,
                ))
                .child(super::tracking::control(
                    "field-edit-cancel",
                    Button::new("workcrate-edit-cancel")
                        .ghost()
                        .compact()
                        .label("取消")
                        .on_click(cx.listener(|view, _, _, cx| {
                            view.editing = None;
                            cx.notify();
                        })),
                    &self.surfaces,
                ))
                .relative()
                .child(super::tracking::measure(
                    &format!("field-{key}"),
                    self.surfaces.clone(),
                ));
        }
        // Resolve choices when the menu opens. Settings may have changed while this
        // editor stayed mounted, so the popup must not keep a render-time snapshot.
        let projects = Arc::clone(&self.projects);
        let selected = value.clone();
        let mut row = row.child(
            Button::new(format!("field-{id}-{key}"))
                .outline()
                .compact()
                .flex_1()
                .min_w_0()
                .label(displayed)
                .disabled(saving)
                .on_click(cx.listener(move |view, _, window, cx| {
                    view.begin_edit(target.clone(), &field_key, &value, window, cx)
                })),
        );
        if matches!(key, "local" | "material" | "size" | "quantity") || key.starts_with("extra:") {
            let target = id.to_owned();
            let key = key.to_owned();
            let weak = cx.entity().downgrade();
            row = row.child(super::tracking::control(
                &format!("preset-{key}"),
                Button::new(format!("preset-{id}-{key}"))
                    .outline()
                    .compact()
                    .w(px(32.))
                    .label("⌄")
                    .disabled(self.busy || saving)
                    .dropdown_menu(move |mut menu, _, _| {
                        let options = projects
                            .field_presets()
                            .ok()
                            .and_then(|presets| {
                                presets.fields.into_iter().find(|field| field.key == key)
                            })
                            .map(|field| field.values)
                            .unwrap_or_default();
                        if options.is_empty() {
                            menu = menu.item(
                                PopupMenuItem::new("暂无预设，请在插件设置中添加").disabled(true),
                            );
                        }
                        for value in &options {
                            let next = value.clone();
                            let weak = weak.clone();
                            let target = target.clone();
                            let key = key.clone();
                            menu = menu.item(
                                PopupMenuItem::new(value.clone())
                                    .checked(value == &selected)
                                    .on_click(move |_, _, cx| {
                                        let _ = weak.update(cx, |view, cx| {
                                            view.set_field_value(
                                                target.clone(),
                                                key.clone(),
                                                next.clone(),
                                                cx,
                                            )
                                        });
                                    }),
                            );
                        }
                        menu
                    }),
                &self.surfaces,
            ));
        }
        row.relative().child(super::tracking::measure(
            &format!("field-{key}"),
            self.surfaces.clone(),
        ))
    }

    pub(super) fn inspector(
        &self,
        card: &BoardCard,
        cx: &mut ViewContext<Self>,
    ) -> gpui::AnyElement {
        let board = &card.board;
        let id = &board.id;
        let global = self
            .items
            .iter()
            .find(|p| p.id == board.project_id)
            .map(|p| p.name.clone())
            .unwrap_or_default();
        let mut info_body = div()
            .v_flex()
            .gap_1()
            .child(self.field(id, "title", "页面名称", board.title.clone(), cx))
            .child(self.field(
                id,
                "physical",
                "尺寸标记",
                super::preview::physical_label(board),
                cx,
            ))
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_2()
                    .px(px(8.))
                    .h(px(26.))
                    .rounded(px(6.))
                    .bg(rgb(theme::surface_faint()))
                    .text_size(px(11.))
                    .text_color(rgb(theme::text_muted()))
                    .child(format!(
                        "画板 {:.1} × {:.1} pt",
                        board.width_pt, board.height_pt
                    ))
                    .child(div().flex_1())
                    .child(pixel_label(board)),
            );
        if let Some(error) = &card.error {
            info_body = info_body.child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(theme::danger_text()))
                    .child(error.clone()),
            );
        }

        let project_id = board.project_id.clone();
        let global_name = global.clone();
        let reset_board = board.clone();
        let inherited = board.info.project_name_override.is_none();
        let mut content_body = div()
            .v_flex()
            .gap_1()
            .child(
                self.field(
                    id,
                    "local",
                    "项目名称",
                    board
                        .info
                        .project_name_override
                        .clone()
                        .unwrap_or(global.clone()),
                    cx,
                ),
            )
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_1()
                    .h(px(24.))
                    .text_size(px(11.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(rgb(theme::text_muted()))
                            .child(if inherited {
                                "继承项目名称"
                            } else {
                                "本页使用局部名称"
                            }),
                    )
                    .child(
                        Button::new("edit-global-project-name")
                            .ghost()
                            .compact()
                            .label("全局设置")
                            .on_click(cx.listener(move |view, _, window, cx| {
                                view.begin_edit(
                                    project_id.clone(),
                                    "project",
                                    &global_name,
                                    window,
                                    cx,
                                )
                            })),
                    )
                    .child(super::tracking::control(
                        "reset-project-name",
                        Button::new("reset-project-name")
                            .ghost()
                            .compact()
                            .label("恢复继承")
                            .disabled(board.info.project_name_override.is_none())
                            .on_click(cx.listener(move |view, _, _, cx| {
                                view.save_field(
                                    reset_board.id.clone(),
                                    "local".into(),
                                    String::new(),
                                    super::management::field_label("local").to_owned(),
                                    cx,
                                );
                            })),
                        &self.surfaces,
                    )),
            )
            .when(
                self.editing
                    .as_ref()
                    .is_some_and(|(target, key)| target == &board.project_id && key == "project"),
                |el| el.child(self.edit_row(cx)),
            )
            .child(self.field(id, "material", "物料名称", board.info.material.clone(), cx))
            .child(self.field(id, "size", "尺寸", board.info.size.clone(), cx))
            .child(self.field(id, "quantity", "数量", board.info.quantity.clone(), cx));
        for (key, value) in &board.info.extra {
            content_body =
                content_body.child(self.field(id, &format!("extra:{key}"), key, value.clone(), cx));
        }
        for field in self
            .preset_fields
            .fields
            .iter()
            .filter(|f| f.key.starts_with("extra:"))
        {
            if !board.info.extra.contains_key(&field.label) {
                content_body =
                    content_body.child(self.field(id, &field.key, &field.label, String::new(), cx));
            }
        }
        let target = id.clone();
        let adding = self.editing.as_ref() == Some(&(id.clone(), "new".into()));
        content_body = content_body
            .child(
                super::tracking::control(
                    "page-add-field",
                    Button::new("page-add-field")
                        .ghost()
                        .compact()
                        .w_full()
                        .label("＋ 新增物料参数行")
                        .on_click(cx.listener(move |view, _, window, cx| {
                            view.begin_edit(target.clone(), "new", "", window, cx)
                        })),
                    &self.surfaces,
                )
                .w_full()
                .rounded(px(6.))
                .bg(rgb(theme::surface_faint())),
            )
            .when(adding, |el| {
                el.child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(theme::text_muted()))
                        .child("输入参数名称，保存后即可填写内容"),
                )
                .child(self.edit_row(cx))
            });

        div()
            .id("workcrate-inspector")
            .relative()
            .child(super::tracking::measure("inspector", self.surfaces.clone()))
            .h_full()
            .w(px(310.))
            .flex_shrink_0()
            .min_h_0()
            .v_flex()
            .gap_3()
            .p_3()
            .border_l_1()
            .border_color(rgb(theme::separator()))
            .overflow_y_scroll()
            .child(section_card("页面信息", "画板规格", info_body))
            .child(section_card("页面内容", "同步物料栏", content_body))
            .into_any_element()
    }
}

fn section_card(label: &str, caption: &str, body: gpui::Div) -> gpui::Div {
    div()
        .v_flex()
        .flex_shrink_0()
        .gap_2()
        .p(px(10.))
        .rounded(px(10.))
        .border_1()
        .border_color(rgb(theme::separator()))
        .bg(rgb(theme::surface_card()))
        .child(
            div()
                .h_flex()
                .items_center()
                .gap_2()
                .child(div().size(px(6.)).rounded_full().bg(rgb(theme::accent())))
                .child(
                    div()
                        .text_size(px(13.))
                        .font_semibold()
                        .text_color(rgb(theme::text_primary()))
                        .child(label.to_owned()),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(theme::text_muted()))
                        .child(caption.to_owned()),
                ),
        )
        .child(body)
}
pub(super) fn section(label: &str) -> gpui::Div {
    div()
        .h_flex()
        .items_center()
        .gap_2()
        .pt_2()
        .child(
            div()
                .text_size(px(13.))
                .font_semibold()
                .child(label.to_owned()),
        )
        .child(div().flex_1().h(px(1.)).bg(rgb(theme::separator())))
}
