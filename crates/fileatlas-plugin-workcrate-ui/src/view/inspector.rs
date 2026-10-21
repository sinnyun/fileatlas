use super::render::pixel_label;
use super::*;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};

impl WorkCratePanel {
    pub(super) fn edit_row(&self, cx: &mut ViewContext<Self>) -> gpui::Div {
        div()
            .h_flex()
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
            .child(
                Button::new("workcrate-edit-save")
                    .primary()
                    .compact()
                    .label("保存")
                    .disabled(self.busy)
                    .on_click(cx.listener(|view, _, _, cx| view.save_edit(cx))),
            )
            .child(
                Button::new("workcrate-edit-cancel")
                    .ghost()
                    .compact()
                    .label("取消")
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.editing = None;
                        cx.notify();
                    })),
            )
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
        let displayed = if value.is_empty() {
            "点击填写".into()
        } else {
            value.clone()
        };
        if self.editing.as_ref() == Some(&(id.into(), key.into())) {
            return div()
                .v_flex()
                .gap_1()
                .child(label.to_owned())
                .child(self.edit_row(cx));
        }
        let row = div().h_flex().items_center().gap_2().child(
            div()
                .w(px(72.))
                .flex_shrink_0()
                .text_xs()
                .text_color(rgb(theme::text_muted()))
                .child(label.to_owned()),
        );
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
                    .disabled(self.busy)
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
        let mut inspector = div()
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
            .child(section("页面信息"))
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
                    .text_xs()
                    .child(format!("实际像素：{}", pixel_label(board))),
            )
            .child(section("页面内容"));
        if let Some(error) = &card.error {
            inspector = inspector.child(
                div()
                    .text_xs()
                    .text_color(rgb(theme::danger_text()))
                    .child(error.clone()),
            );
        }
        let global = self
            .items
            .iter()
            .find(|p| p.id == board.project_id)
            .map(|p| p.name.clone())
            .unwrap_or_default();
        inspector = inspector.child(
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
        );
        let project_id = board.project_id.clone();
        let reset_board = board.clone();
        inspector = inspector
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_1()
                    .text_xs()
                    .child(if board.info.project_name_override.is_some() {
                        "局部名称"
                    } else {
                        "继承全局"
                    })
                    .child(
                        Button::new("edit-global-project-name")
                            .ghost()
                            .compact()
                            .label("全局设置")
                            .on_click(cx.listener(move |view, _, window, cx| {
                                view.begin_edit(project_id.clone(), "project", &global, window, cx)
                            })),
                    )
                    .child(
                        Button::new("reset-project-name")
                            .ghost()
                            .compact()
                            .label("恢复继承")
                            .disabled(board.info.project_name_override.is_none())
                            .on_click(cx.listener(move |view, _, _, cx| {
                                let mut info = reset_board.info.clone();
                                info.project_name_override = None;
                                let boards = Arc::clone(&view.boards);
                                let board = reset_board.clone();
                                view.perform(
                                    move || {
                                        boards
                                            .update_info(&board.id, &board.title, info)
                                            .map(|_| ())
                                            .map_err(|e| e.to_string())
                                    },
                                    cx,
                                );
                            })),
                    ),
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
            inspector =
                inspector.child(self.field(id, &format!("extra:{key}"), key, value.clone(), cx));
        }
        for field in self
            .preset_fields
            .fields
            .iter()
            .filter(|f| f.key.starts_with("extra:"))
        {
            if !board.info.extra.contains_key(&field.label) {
                inspector =
                    inspector.child(self.field(id, &field.key, &field.label, String::new(), cx));
            }
        }
        let target = id.clone();
        inspector = inspector.child(
            Button::new("page-add-field")
                .ghost()
                .compact()
                .label("＋ 添加字段")
                .on_click(cx.listener(move |view, _, window, cx| {
                    view.begin_edit(target.clone(), "new", "", window, cx)
                })),
        );
        if self.editing.as_ref() == Some(&(id.clone(), "new".into())) {
            inspector = inspector.child(self.edit_row(cx));
        }
        inspector = inspector
            .child(self.text_inspector(card, cx))
            .child(section("页面图片"));
        for (asset, placement) in &board.placements {
            let selection = asset.clone();
            let entry = div()
                .id(format!("inspector-image-{asset}"))
                .v_flex()
                .gap_2()
                .border_b_1()
                .border_color(rgb(theme::separator()))
                .py_3()
                .child(
                    div()
                        .h_flex()
                        .items_start()
                        .gap_2()
                        .child(
                            card.sources
                                .get(asset)
                                .cloned()
                                .map(|image| {
                                    img(image)
                                        .w(px(64.))
                                        .h(px(76.))
                                        .object_fit(ObjectFit::Contain)
                                        .into_any_element()
                                })
                                .unwrap_or_else(|| {
                                    div().w(px(64.)).child("预览不可用").into_any_element()
                                }),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .v_flex()
                                .gap_1()
                                .child(div().truncate().child(file_name(&placement.path)))
                                .child(div().text_xs().child(format!(
                                    "{} × {} px",
                                    placement.pixel_width, placement.pixel_height
                                )))
                                .child(
                                    div().text_xs().truncate().child(
                                        placement
                                            .source_document
                                            .as_ref()
                                            .map(|p| file_name(p))
                                            .unwrap_or_else(|| "未设置扩展源文件".into()),
                                    ),
                                ),
                        ),
                )
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.selected_image = Some(selection.clone());
                    cx.notify();
                }));
            let mut actions = div().h_flex().gap_1();
            for (action, label) in [
                ("refresh", "更新"),
                ("replace", "替换图片"),
                ("link", "扩展链接"),
                ("edit-source", "编辑源文件"),
            ] {
                let asset = asset.clone();
                let action = action.to_owned();
                actions = actions.child(
                    Button::new(format!("image-{action}-{asset}"))
                        .ghost()
                        .compact()
                        .label(label)
                        .disabled(self.busy)
                        .on_click(cx.listener(move |view, _, _, cx| {
                            view.image_action(asset.clone(), &action, cx)
                        })),
                );
            }
            let entry = entry.child(actions);
            inspector = inspector.child(entry);
        }
        inspector.into_any_element()
    }
}

fn section(label: &str) -> gpui::Div {
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
fn file_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}
