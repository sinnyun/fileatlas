use super::*;
use fileatlas_ui_bridge::components::WorkbenchPaint as _;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
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
            .h(px(theme::ui_height(theme::CONTROL_HEIGHT_STANDARD)))
            .gap(px(theme::space_standard()))
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
                fileatlas_ui_bridge::components::button("workcrate-edit-save")
                    .primary()
                    .label(if saving { "保存中" } else { "保存" })
                    .disabled(self.busy || saving)
                    .on_click(cx.listener(|view, _, _, cx| view.save_edit(cx))),
                &self.surfaces,
            ))
            .child(super::tracking::control(
                "field-edit-cancel",
                fileatlas_ui_bridge::components::button("workcrate-edit-cancel")
                    .ghost()
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
        let row = div()
            .h_flex()
            .items_center()
            .h(px(theme::ui_height(theme::CONTROL_HEIGHT_STANDARD)))
            .gap(px(fileatlas_ui_bridge::theme::space_standard()))
            .child(
                div()
                    .w(px(72.))
                    .flex_shrink_0()
                    .workbench_assistant()
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
                    fileatlas_ui_bridge::components::button("workcrate-edit-save")
                        .primary()
                        .label(if saving { "保存中" } else { "保存" })
                        .disabled(self.busy || saving)
                        .on_click(cx.listener(|view, _, _, cx| view.save_edit(cx))),
                    &self.surfaces,
                ))
                .child(super::tracking::control(
                    "field-edit-cancel",
                    fileatlas_ui_bridge::components::button("workcrate-edit-cancel")
                        .ghost()
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
        let project_id = self.selected_project.clone();
        let selected = value.clone();
        let mut row = row.child(
            fileatlas_ui_bridge::components::button(format!("field-{id}-{key}"))
                .outline()
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
                fileatlas_ui_bridge::components::button(format!("preset-{id}-{key}"))
                    .outline()
                    .label("⌄")
                    .disabled(self.busy || saving)
                    .dropdown_menu(move |mut menu, _, _| {
                        let options = projects
                            .effective_field_presets(project_id.as_deref().unwrap_or(""))
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
        let mut content_body = div()
            .v_flex()
            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
            .children(
                card.error
                    .as_ref()
                    .map(|error| div().workbench_assistant().child(error.clone())),
            )
            .child(self.field(
                id,
                "local",
                "物料类别",
                board.info.material_category.clone(),
                cx,
            ))
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
                    fileatlas_ui_bridge::components::button("page-add-field")
                        .ghost()
                        .label("＋ 新增物料参数行")
                        .on_click(cx.listener(move |view, _, window, cx| {
                            view.begin_edit(target.clone(), "new", "", window, cx)
                        })),
                    &self.surfaces,
                )
                .w_full()
                .workbench_corner(fileatlas_ui_bridge::components::CornerRole::Medium)
                .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Faint),
            )
            .when(adding, |el| {
                el.child(
                    div()
                        .workbench_assistant()
                        .child("输入参数名称，保存后即可填写内容"),
                )
                .child(self.edit_row(cx))
            });

        div()
            .id("workcrate-inspector")
            .relative()
            .child(super::tracking::measure("inspector", self.surfaces.clone()))
            .v_flex()
            .flex_shrink_0()
            .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
            .p(px(fileatlas_ui_bridge::theme::space_expanded()))
            .child(section_card("页面内容", "同步物料栏", content_body))
            .into_any_element()
    }
}

pub(super) fn section_card(label: &str, caption: &str, body: gpui::Div) -> gpui::Div {
    div()
        .v_flex()
        .flex_shrink_0()
        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
        .p(px(fileatlas_ui_bridge::theme::space_standard()))
        .workbench_section()
        .child(
            div()
                .h_flex()
                .items_center()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                .child(fileatlas_ui_bridge::components::status_dot(
                    6.,
                    fileatlas_ui_bridge::components::StatusTone::Accent,
                ))
                .child(div().workbench_body().child(label.to_owned()))
                .child(div().flex_1())
                .child(div().workbench_assistant().child(caption.to_owned())),
        )
        .child(body)
}
