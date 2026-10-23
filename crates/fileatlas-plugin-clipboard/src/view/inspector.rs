use super::*;
use fileatlas_ui_bridge::components::WorkbenchForeground as _;

impl ClipboardPanel {
    pub(super) fn inspector(&self, cx: &mut Context<Self>) -> AnyElement {
        let selected = self
            .selected
            .and_then(|id| self.items.iter().find(|item| item.id == id));
        let id = selected.map(|item| item.id);
        let group_id = selected
            .map(|item| item.category_id)
            .unwrap_or(self.new_group);
        let value = self.editor.read(cx).value();
        let selected_image = selected.and_then(|item| item.image_path.as_deref());
        let content_length = format!("{} 字", value.chars().count());
        let dirty = selected.is_none_or(|item| value.as_ref() != item.content);
        let mut heading =
            div()
                .h_flex()
                .items_center()
                .justify_between()
                .child(div().workbench_title().child(if selected.is_some() {
                    "编辑内容"
                } else {
                    "新建文本"
                }));
        if let Some(id) = id.filter(|_| selected.is_some_and(|item| !item.pinned)) {
            heading = heading.child(
                small_icon("clipboard-delete-selected".into(), IconName::Trash).on_click(
                    cx.listener(move |this, _, window, cx| {
                        let result = this.service.delete(id);
                        if result.is_ok() {
                            this.editor
                                .update(cx, |state, cx| state.set_value("", window, cx));
                        }
                        this.finish(result, "记录已删除", cx);
                    }),
                ),
            );
        }
        let groups = self.categories.clone();
        let weak = cx.entity().downgrade();
        let category = action(
            "clipboard-category-choice",
            &self.group_name(group_id),
            false,
        )
        .w_full()
        .flex_1()
        .icon(icon(IconName::Folder, 17.))
        .dropdown_caret(true)
        .dropdown_menu(move |mut menu, _, _| {
            let view = weak.clone();
            menu = menu.item(PopupMenuItem::new("未分类").on_click(move |_, _, cx| {
                let _ = view.update(cx, |this, cx| this.assign_group(None, cx));
            }));
            for group in &groups {
                let group_id = group.id;
                let view = weak.clone();
                menu = menu.item(PopupMenuItem::new(group.name.clone()).on_click(
                    move |_, _, cx| {
                        let _ = view.update(cx, |this, cx| this.assign_group(Some(group_id), cx));
                    },
                ));
            }
            menu
        });
        let mut pane = section()
            .v_flex()
            .w(px(324.))
            .flex_shrink_0()
            .h_full()
            .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
            .p(px(fileatlas_ui_bridge::theme::space_expanded()))
            .child(heading)
            .child(if let Some(path) = selected_image {
                div()
                    .id("clipboard-selected-image-preview")
                    .v_flex()
                    .flex_1()
                    .min_h(px(100.))
                    .min_w_0()
                    .items_center()
                    .justify_center()
                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                    .child(
                        img(std::path::PathBuf::from(path))
                            .w_full()
                            .h(px(300.))
                            .object_fit(ObjectFit::Contain),
                    )
                    .child(format!(
                        "图片 · {} × {} 像素",
                        selected
                            .and_then(|item| item.image_width)
                            .unwrap_or_default(),
                        selected
                            .and_then(|item| item.image_height)
                            .unwrap_or_default()
                    ))
                    .into_any_element()
            } else {
                div()
                    .v_flex()
                    .flex_1()
                    .min_h(px(100.))
                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                    .child(
                        fileatlas_ui_bridge::components::textarea(&self.editor)
                            .h_full()
                            .w_full(),
                    )
                    .into_any_element()
            })
            .child(
                div()
                    .h_flex()
                    .justify_between()
                    .workbench_assistant()
                    .child(if selected_image.is_some() {
                        "图片内容".to_owned()
                    } else {
                        content_length
                    })
                    .child(if selected_image.is_some() {
                        "原图已保存在 FileAtlas 本地"
                    } else if dirty {
                        "内容未保存"
                    } else {
                        "内容已保存"
                    }),
            )
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
                    .child(div().w(px(38.)).workbench_body().child("分组"))
                    .child(div().flex_1().min_w_0().child(category)),
            )
            .child(div().workbench_assistant().child(if id.is_some() {
                "分组变更即时保存，独立于正文编辑"
            } else {
                "新建记录时一并保存分组"
            }));
        if let Some(item) = selected {
            let item_id = item.id;
            pane = pane
                .child(
                    div()
                        .h_flex()
                        .items_center()
                        .justify_between()
                        .child("收藏")
                        .child(
                            action(
                                "clipboard-favorite-action",
                                if item.pinned {
                                    "从收藏中删除…"
                                } else {
                                    "收藏到本地"
                                },
                                false,
                            )
                            .icon(icon(IconName::Star, 17.))
                            .workbench_foreground(if item.pinned {
                                fileatlas_ui_bridge::components::ForegroundRole::Star
                            } else {
                                fileatlas_ui_bridge::components::ForegroundRole::Muted
                            })
                            .on_click(cx.listener(
                                move |this, _, window, cx| {
                                    if let Some(item) =
                                        this.items.iter().find(|item| item.id == item_id)
                                    {
                                        if item.pinned {
                                            this.request_favorite_removal(item_id, window, cx);
                                        } else {
                                            let result = this.service.add_favorite(item_id);
                                            this.finish(result, "已收藏并保存到本地", cx);
                                        }
                                    }
                                },
                            )),
                        ),
                )
                .child(
                    div()
                        .v_flex()
                        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                        .workbench_assistant()
                        .child(
                            div().h_flex().justify_between().child("内容类型").child(
                                self.kinds
                                    .get(&item.id)
                                    .copied()
                                    .unwrap_or(ContentKind::Text)
                                    .label(),
                            ),
                        )
                        .child(
                            div()
                                .h_flex()
                                .justify_between()
                                .child("首次收录")
                                .child(date(item.created_at, "%Y-%m-%d %H:%M")),
                        ),
                );
        }
        if let Some(id) = self.confirm_favorite_delete {
            pane = pane.child(
                div()
                    .v_flex()
                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                    .workbench_assistant()
                    .child("将永久删除这条本地收藏内容，无法恢复。")
                    .child(
                        div()
                            .h_flex()
                            .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                            .child(
                                action("clipboard-favorite-delete-confirm", "从收藏中删除", false)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        let result = this.service.remove_favorite(id);
                                        if result.is_ok() {
                                            this.confirm_favorite_delete = None;
                                            this.editor.update(cx, |state, cx| {
                                                state.set_value("", window, cx)
                                            });
                                        }
                                        this.finish(result, "本地收藏内容已删除", cx);
                                    })),
                            )
                            .child(
                                action("clipboard-favorite-delete-cancel", "取消", false).on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.confirm_favorite_delete = None;
                                        cx.notify();
                                    }),
                                ),
                            ),
                    ),
            );
        }
        if selected_image.is_none() {
            pane = pane.child(
                action(
                    "clipboard-save-edit",
                    if id.is_some() {
                        "保存内容"
                    } else {
                        "添加到历史"
                    },
                    true,
                )
                .icon(icon(IconName::Save, 18.))
                .on_click(cx.listener(|this, _, _, cx| {
                    let content = this.editor.read(cx).value().to_owned();
                    if let Some(item) = this
                        .selected
                        .and_then(|id| this.items.iter().find(|item| item.id == id))
                    {
                        let result = this.service.save_item(
                            item.id,
                            &content,
                            item.category_id,
                            item.pinned,
                        );
                        this.finish(result, "内容已保存，类型已重新识别", cx);
                    } else {
                        match this.service.add_text(&content, this.new_group, false) {
                            Ok(id) => {
                                this.selected = Some(id);
                                this.sync();
                                this.message = Some("已添加到历史".into());
                            }
                            Err(error) => this.message = Some(error),
                        }
                        cx.notify();
                    }
                })),
            );
        }
        if let Some(id) = id {
            pane = pane.child(
                action(
                    "clipboard-use-selected",
                    if selected_image.is_some() {
                        "复制图片到系统剪贴板"
                    } else {
                        "复制到剪贴板"
                    },
                    false,
                )
                .icon(icon(IconName::Copy, 18.))
                .on_click(cx.listener(move |this, _, _, cx| {
                    let result = this.service.use_item(id);
                    this.finish(result, "已复制到系统剪贴板", cx);
                })),
            );
        }
        pane.into_any_element()
    }
    fn assign_group(&mut self, group: Option<i64>, cx: &mut Context<Self>) {
        if let Some(id) = self.selected {
            let result = self.service.assign_category(id, group);
            self.finish(result, "分组已即时保存", cx);
        } else {
            self.new_group = group;
            cx.notify();
        }
    }
}
