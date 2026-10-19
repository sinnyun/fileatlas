use super::*;
use gpui_kit::prelude::FluentBuilder as _;
use photag_ui_bridge::components::WorkbenchStyle as _;

impl ClipboardPanel {
    fn nav(
        &self,
        id: &str,
        label: &str,
        image: IconName,
        count: usize,
        filter: Filter,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let selected = self.filter == filter;
        div()
            .id(id.to_owned())
            .workbench_nav(selected)
            .w_full()
            .on_click(cx.listener(move |this, _, window, cx| this.set_filter(filter, window, cx)))
            .child(icon(image, 19.))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .text_size(px(14.))
                    .child(label.to_owned()),
            )
            .child(div().text_size(px(12.)).child(count.to_string()))
            .into_any_element()
    }
    fn start_group_edit(&mut self, id: Option<i64>, window: &mut Window, cx: &mut Context<Self>) {
        self.category_form = true;
        self.rename_group = id;
        self.confirm_group_delete = false;
        let value = id.map(|id| self.group_name(Some(id))).unwrap_or_default();
        self.category_name.update(cx, |state, cx| {
            state.set_value(value, window, cx);
            state.focus(window, cx);
        });
        cx.notify();
    }
    fn group_form(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .v_flex()
            .gap(px(8.))
            .p(px(10.))
            .rounded(px(7.))
            .bg(rgb(inset()))
            .border_1()
            .border_color(rgb(rule()))
            .child(div().text_size(px(12.)).text_color(rgb(muted())).child(
                if self.rename_group.is_some() {
                    "重命名分组"
                } else {
                    "新建分组"
                },
            ))
            .child(
                photag_ui_bridge::components::text_input(&self.category_name)
                    .h(px(34.))
                    .w_full()
                    .appearance(false)
                    .bordered(false),
            )
            .child(
                div()
                    .h_flex()
                    .gap(px(6.))
                    .child(
                        action("clipboard-group-submit", "保存", true)
                            .flex_1()
                            .on_click(cx.listener(|this, _, _, cx| {
                                let name = this.category_name.read(cx).value().trim().to_owned();
                                let result = match this.rename_group {
                                    Some(id) => this.service.rename_category(id, &name),
                                    None => this.service.add_category(&name),
                                };
                                if result.is_ok() {
                                    this.category_form = false;
                                    this.rename_group = None;
                                }
                                this.finish(result, "分组已保存", cx);
                            })),
                    )
                    .child(
                        action("clipboard-group-cancel", "取消", false)
                            .flex_1()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.category_form = false;
                                this.rename_group = None;
                                cx.notify();
                            })),
                    ),
            )
            .into_any_element()
    }
    pub(super) fn sidebar(&self, cx: &mut Context<Self>) -> AnyElement {
        let pinned = self.items.iter().filter(|item| item.pinned).count();
        let unclassified = self
            .items
            .iter()
            .filter(|item| item.category_id.is_none())
            .count();
        let mut navigation = div()
            .v_flex()
            .gap(px(3.))
            .child(self.nav(
                "clipboard-filter-all",
                "全部历史",
                IconName::Clipboard,
                self.items.len(),
                Filter::All,
                cx,
            ))
            .child(self.nav(
                "clipboard-filter-favorites",
                "收藏",
                IconName::Star,
                pinned,
                Filter::Favorites,
                cx,
            ))
            .child(self.nav(
                "clipboard-filter-unclassified",
                "未分类",
                IconName::Inbox,
                unclassified,
                Filter::Unclassified,
                cx,
            ))
            .child(
                div()
                    .mt(px(12.))
                    .pt(px(14.))
                    .mb(px(6.))
                    .border_t_1()
                    .border_color(rgb(rule()))
                    .text_color(rgb(muted()))
                    .text_size(px(12.))
                    .font_semibold()
                    .child("内容类型 · 自动识别"),
            );
        for kind in [
            ContentKind::Text,
            ContentKind::Link,
            ContentKind::Code,
            ContentKind::Html,
            ContentKind::Image,
        ] {
            let count = self.kinds.values().filter(|value| **value == kind).count();
            navigation = navigation.child(self.nav(
                &format!("clipboard-filter-{}", kind.key()),
                kind.label(),
                kind_icon(kind),
                count,
                Filter::Kind(kind),
                cx,
            ));
        }
        navigation = navigation.child(
            div()
                .h_flex()
                .items_center()
                .justify_between()
                .mt(px(12.))
                .pt(px(10.))
                .border_t_1()
                .border_color(rgb(rule()))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(muted()))
                        .font_semibold()
                        .child("我的分组"),
                )
                .child(
                    small_icon("clipboard-group-new".into(), IconName::Plus).on_click(
                        cx.listener(|this, _, window, cx| this.start_group_edit(None, window, cx)),
                    ),
                ),
        );
        for group in &self.categories {
            let count = self
                .items
                .iter()
                .filter(|item| item.category_id == Some(group.id))
                .count();
            navigation = navigation.child(self.nav(
                &format!("clipboard-filter-group-{}", group.id),
                &group.name,
                IconName::Folder,
                count,
                Filter::Group(group.id),
                cx,
            ));
        }
        if self.categories.is_empty() && !self.category_form {
            navigation = navigation.child(
                div()
                    .px(px(12.))
                    .py(px(10.))
                    .text_size(px(12.))
                    .text_color(rgb(muted()))
                    .child("点击 ＋ 创建自己的分组"),
            );
        }
        let show_group_controls = self.category_form || matches!(self.filter, Filter::Group(_));
        let mut group_controls = div().v_flex().flex_shrink_0();
        if self.category_form {
            group_controls = group_controls.child(self.group_form(cx));
        } else if let Filter::Group(id) = self.filter {
            group_controls = group_controls.child(
                div()
                    .h_flex()
                    .gap(px(6.))
                    .mt(px(8.))
                    .child(
                        action("clipboard-group-rename", "重命名", false)
                            .flex_1()
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.start_group_edit(Some(id), window, cx)
                            })),
                    )
                    .child(
                        action("clipboard-group-delete", "删除分组", false)
                            .flex_1()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.confirm_group_delete = true;
                                cx.notify();
                            })),
                    ),
            );
            if self.confirm_group_delete {
                group_controls = group_controls.child(
                    div()
                        .v_flex()
                        .gap(px(8.))
                        .p(px(10.))
                        .text_size(px(12.))
                        .text_color(rgb(muted()))
                        .child("仅删除分组，记录将回到未分类。")
                        .child(
                            action("clipboard-group-delete-confirm", "确认删除分组", false)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    let result = this.service.delete_category(id);
                                    this.confirm_group_delete = false;
                                    this.finish(result, "分组已删除，记录已保留", cx);
                                })),
                        ),
                );
            }
        }
        let paused = self.capture_paused;
        section()
            .v_flex()
            .w(px(220.))
            .flex_shrink_0()
            .h_full()
            .gap(px(18.))
            .p(px(14.))
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap(px(12.))
                    .pt(px(6.))
                    .child(
                        div()
                            .text_color(rgb(theme::accent_text()))
                            .child(icon(IconName::Clipboard, 28.)),
                    )
                    .child(div().text_size(px(23.)).font_semibold().child("剪贴板")),
            )
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap(px(8.))
                    .h(px(40.))
                    .child(photag_ui_bridge::components::search_input(&self.search)),
            )
            .child(
                div()
                    .id("clipboard-nav-scroll")
                    .v_flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(navigation),
            )
            .when(show_group_controls, |sidebar| sidebar.child(group_controls))
            .child(
                div()
                    .v_flex()
                    .gap(px(10.))
                    .pt(px(14.))
                    .border_t_1()
                    .border_color(rgb(rule()))
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap(px(8.))
                            .child(div().size(px(8.)).rounded_full().bg(rgb(if paused {
                                theme::warning()
                            } else {
                                theme::success()
                            })))
                            .child(div().flex_1().text_size(px(13.)).child(if paused {
                                "已暂停捕获"
                            } else {
                                "正在捕获剪贴板"
                            }))
                            .child(
                                small_icon(
                                    "clipboard-pause-capture".into(),
                                    if paused {
                                        IconName::Play
                                    } else {
                                        IconName::Pause
                                    },
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.service.set_paused(!this.service.is_paused());
                                        this.sync();
                                        cx.notify();
                                    },
                                )),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(muted()))
                            .child("Ctrl+Shift+V 唤醒工作台"),
                    ),
            )
            .into_any_element()
    }
}
