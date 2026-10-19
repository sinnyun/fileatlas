use super::*;
use gpui_kit::prelude::FluentBuilder as _;
use photag_ui_bridge::components::WorkbenchStyle as _;

impl ClipboardPanel {
    fn history_row(&self, item: &ClipboardHistoryRecord, cx: &mut Context<Self>) -> AnyElement {
        let id = item.id;
        let selected = self.selected == Some(id);
        let selected_item = item.clone();
        let kind = self.kinds.get(&id).copied().unwrap_or(ContentKind::Text);
        let value = item
            .content
            .lines()
            .take(3)
            .collect::<Vec<_>>()
            .join("\n")
            .chars()
            .take(230)
            .collect::<String>();
        let preview = item.image_path.as_ref().map_or_else(
            || div().child(value).into_any_element(),
            |path| {
                div()
                    .h_flex()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        img(std::path::PathBuf::from(path))
                            .w(px(132.))
                            .h(px(76.))
                            .object_fit(ObjectFit::Contain)
                            .rounded(px(theme::ROUND_SM)),
                    )
                    .child(format!(
                        "剪贴板图片 · {} × {} 像素",
                        item.image_width.unwrap_or_default(),
                        item.image_height.unwrap_or_default()
                    ))
                    .into_any_element()
            },
        );
        let favorite = item.pinned;
        let groups = self.categories.clone();
        let weak = cx.entity().downgrade();
        div()
            .id(format!("clipboard-item-{id}"))
            .v_flex()
            .flex_shrink_0()
            .w_full()
            .min_w_0()
            .gap(px(10.))
            .p(px(14.))
            .workbench_card(selected)
            .when(selected, |row| row.border_l_4())
            .cursor_pointer()
            .on_click(
                cx.listener(move |this, _, window, cx| this.select(&selected_item, window, cx)),
            )
            .child(
                div()
                    .h_flex()
                    .items_start()
                    .gap(px(12.))
                    .child(
                        div()
                            .pt(px(2.))
                            .text_color(rgb(if selected {
                                theme::selection_text()
                            } else {
                                muted()
                            }))
                            .child(icon(kind_icon(kind), 21.)),
                    )
                    .child(preview)
                    .child(
                        small_icon(format!("clipboard-favorite-{id}"), IconName::Star)
                            .text_color(rgb(if selected {
                                theme::selection_text()
                            } else if item.pinned {
                                theme::star_text()
                            } else {
                                muted()
                            }))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                if let Some(item) = this.items.iter().find(|item| item.id == id) {
                                    if item.pinned {
                                        this.request_favorite_removal(id, window, cx);
                                    } else {
                                        let result = this.service.add_favorite(id);
                                        this.finish(result, "已收藏并保存到本地", cx);
                                    }
                                }
                            })),
                    )
                    .child(
                        small_icon(format!("clipboard-menu-{id}"), IconName::Ellipsis)
                            .dropdown_menu(move |mut menu, _, _| {
                                let view = weak.clone();
                                menu = menu.item(PopupMenuItem::new("移至：未分类").on_click(
                                    move |_, _, cx| {
                                        let _ = view.update(cx, |this, cx| {
                                            let result = this.service.assign_category(id, None);
                                            this.finish(result, "已移至未分类", cx);
                                        });
                                    },
                                ));
                                for group in &groups {
                                    let group_id = group.id;
                                    let view = weak.clone();
                                    menu = menu.item(
                                        PopupMenuItem::new(format!("移至：{}", group.name))
                                            .on_click(move |_, _, cx| {
                                                let _ = view.update(cx, |this, cx| {
                                                    let result = this
                                                        .service
                                                        .assign_category(id, Some(group_id));
                                                    this.finish(result, "分组已保存", cx);
                                                });
                                            }),
                                    );
                                }
                                let view = weak.clone();
                                menu.item(
                                    PopupMenuItem::new(if favorite {
                                        "从收藏中删除…"
                                    } else {
                                        "删除记录"
                                    })
                                    .on_click(
                                        move |_, window, cx| {
                                            let _ = view.update(cx, |this, cx| {
                                                if favorite {
                                                    this.request_favorite_removal(id, window, cx);
                                                } else {
                                                    let result = this.service.delete(id);
                                                    this.finish(result, "记录已删除", cx);
                                                }
                                            });
                                        },
                                    ),
                                )
                            }),
                    ),
            )
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap(px(10.))
                    .pl(px(33.))
                    .text_size(px(12.))
                    .text_color(rgb(if selected {
                        theme::selection_text()
                    } else {
                        muted()
                    }))
                    .child(date(item.last_used_at, "%H:%M"))
                    .child(if item.image_path.is_some() {
                        format!(
                            "{} × {} 像素 · {}",
                            item.image_width.unwrap_or_default(),
                            item.image_height.unwrap_or_default(),
                            kind.label()
                        )
                    } else {
                        format!("{} 字 · {}", item.content.chars().count(), kind.label())
                    })
                    .child(div().flex_1())
                    .child(
                        div()
                            .px(px(7.))
                            .py(px(2.))
                            .rounded(px(5.))
                            .border_1()
                            .border_color(rgb(if selected {
                                theme::accent()
                            } else {
                                theme::separator()
                            }))
                            .bg(rgba(theme::fill_subtle()))
                            .max_w(px(110.))
                            .overflow_hidden()
                            .child(self.group_name(item.category_id)),
                    ),
            )
            .into_any_element()
    }
    pub(super) fn history(&self, ids: &[i64], cx: &mut Context<Self>) -> AnyElement {
        let mut rows = div().v_flex().gap(px(8.)).w_full().min_w_0();
        let mut previous = String::new();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        for id in ids {
            let Some(item) = self.items.iter().find(|item| item.id == *id) else {
                continue;
            };
            let day = date(item.last_used_at, "%Y-%m-%d");
            let group = if item.pinned {
                "收藏".to_owned()
            } else if day == today {
                "今天".to_owned()
            } else {
                day
            };
            if group != previous {
                rows = rows.child(
                    div()
                        .h_flex()
                        .items_center()
                        .gap(px(10.))
                        .pt(px(6.))
                        .pb(px(3.))
                        .text_size(px(13.))
                        .font_semibold()
                        .text_color(rgb(muted()))
                        .child(group.clone()),
                );
                previous = group;
            }
            rows = rows.child(self.history_row(item, cx));
        }
        if ids.is_empty() {
            rows = rows.child(
                div()
                    .v_flex()
                    .items_center()
                    .justify_center()
                    .h(px(240.))
                    .gap(px(12.))
                    .text_color(rgb(muted()))
                    .child(icon(IconName::Clipboard, 36.))
                    .child("没有符合条件的记录")
                    .child(
                        div()
                            .text_size(px(12.))
                            .child("复制文本自动收录，也可以新建文本。"),
                    ),
            );
        }
        let weak = cx.entity().downgrade();
        let mut pane = section().v_flex().flex_1().min_w_0().h_full().child(
            div()
                .h_flex()
                .items_center()
                .gap(px(10.))
                .p(px(16.))
                .border_b_1()
                .border_color(rgb(rule()))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h_flex()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            div()
                                .text_size(px(19.))
                                .font_semibold()
                                .child(self.heading()),
                        )
                        .child(
                            div()
                                .text_color(rgb(muted()))
                                .text_size(px(14.))
                                .child(format!("({})", ids.len())),
                        ),
                )
                .child(
                    action("clipboard-new-text", "新建文本", true)
                        .icon(icon(IconName::Plus, 18.))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.selected = None;
                            this.new_group = match this.filter {
                                Filter::Group(id) => Some(id),
                                _ => None,
                            };
                            this.editor
                                .update(cx, |state, cx| state.set_value("", window, cx));
                            this.message = None;
                            cx.notify();
                        })),
                )
                .child(
                    small_icon("clipboard-history-menu".into(), IconName::Ellipsis).dropdown_menu(
                        move |menu, _, _| {
                            let view = weak.clone();
                            menu.item(PopupMenuItem::new("清理临时历史…").on_click(
                                move |_, _, cx| {
                                    let _ = view.update(cx, |this, cx| {
                                        this.confirm_clear = true;
                                        cx.notify();
                                    });
                                },
                            ))
                        },
                    ),
                ),
        );
        if self.confirm_clear {
            pane = pane.child(
                div()
                    .v_flex()
                    .gap(px(8.))
                    .p(px(12.))
                    .bg(rgb(inset()))
                    .text_size(px(13.))
                    .child("确认清理临时历史？收藏内容将保留。")
                    .child(
                        div()
                            .h_flex()
                            .gap(px(8.))
                            .child(
                                action("clipboard-clear-confirm", "确认清理", false).on_click(
                                    cx.listener(|this, _, _, cx| {
                                        let result = this.service.clear_unpinned();
                                        this.confirm_clear = false;
                                        this.finish(result, "临时历史已清理", cx);
                                    }),
                                ),
                            )
                            .child(action("clipboard-clear-cancel", "取消", false).on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.confirm_clear = false;
                                    cx.notify();
                                }),
                            )),
                    ),
            );
        }
        pane.child(
            div()
                .id("clipboard-history-scroll")
                .flex_1()
                .min_h_0()
                .p(px(12.))
                .overflow_y_scrollbar()
                .child(rows),
        )
        .child(
            div()
                .min_h(px(34.))
                .px(px(14.))
                .pb(px(10.))
                .text_size(px(12.))
                .text_color(rgb(muted()))
                .child(
                    self.message
                        .clone()
                        .unwrap_or_else(|| "内容类型自动识别 · 分组选择后即时保存".into()),
                ),
        )
        .into_any_element()
    }
}
