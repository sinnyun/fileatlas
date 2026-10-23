use super::*;
use fileatlas_ui_bridge::components::WorkbenchForeground as _;
use fileatlas_ui_bridge::components::WorkbenchStyle as _;

impl LauncherPanel {
    fn category_row(
        &self,
        id: Option<i64>,
        name: String,
        icon: IconName,
        count: usize,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .id(format!("launcher-category-{}", id.unwrap_or(0)))
            .workbench_nav(selected)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.selected_category = id;
                this.show_hidden = false;
                cx.notify();
            }))
            .child(div().w(px(20.)).child(theme::icon(icon).size(px(16.))))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .workbench_assistant()
                    .text_ellipsis()
                    .child(name),
            )
            .child(div().workbench_assistant().child(count.to_string()))
            .into_any_element()
    }

    pub(super) fn sidebar(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut categories = div()
            .v_flex()
            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
            .flex_1()
            .min_h_0()
            .overflow_y_scrollbar();
        for category in &self.snapshot.categories {
            let count = self
                .snapshot
                .entries
                .iter()
                .filter(|e| e.category_id == category.id && !e.hidden)
                .count();
            let id = category.id;
            let weak = cx.entity().downgrade();
            let mut row = div()
                .h_flex()
                .items_center()
                .child(div().flex_1().min_w_0().child(self.category_row(
                    Some(id),
                    cards::category_name(&category.name),
                    category_icon(&category.name),
                    count,
                    !self.show_hidden && self.selected_category == Some(id),
                    cx,
                )));
            if category.name != "General" {
                row = row.child(
                    fileatlas_ui_bridge::components::button(format!("launcher-category-menu-{id}"))
                        .ghost()
                        .icon(theme::icon(IconName::Ellipsis))
                        .dropdown_menu(move |menu, _, _| {
                            let rename = weak.clone();
                            let delete = weak.clone();
                            menu.item(PopupMenuItem::new(tr("Rename")).on_click(
                                move |_, window, cx| {
                                    let _ = rename.update(cx, |this, cx| {
                                        if let Some(category) = this
                                            .snapshot
                                            .categories
                                            .iter()
                                            .find(|c| c.id == id)
                                            .cloned()
                                        {
                                            this.start_rename_category(&category, window, cx);
                                        }
                                    });
                                },
                            ))
                            .item(
                                PopupMenuItem::new(tr("Delete category")).on_click(
                                    move |_, _, cx| {
                                        let confirmed = rfd::MessageDialog::new()
                                        .set_title(tr("Delete category"))
                                        .set_description(tr(
                                            "Programs in this category will be moved to Favorites.",
                                        ))
                                        .set_level(rfd::MessageLevel::Warning)
                                        .set_buttons(rfd::MessageButtons::YesNo)
                                        .show()
                                        == rfd::MessageDialogResult::Yes;
                                        if confirmed {
                                            let _ = delete.update(cx, |this, cx| {
                                                let result = this.service.delete_category(id);
                                                if this.selected_category == Some(id) {
                                                    this.selected_category = None;
                                                }
                                                this.report(result, cx);
                                            });
                                        }
                                    },
                                ),
                            )
                        }),
                );
            }
            categories = categories.child(row);
        }
        let visible_count = self.snapshot.entries.iter().filter(|e| !e.hidden).count();
        let hidden_count = self.snapshot.entries.iter().filter(|e| e.hidden).count();
        let mut editor = div()
            .v_flex()
            .gap(px(theme::space_standard()))
            .pt(px(fileatlas_ui_bridge::theme::space_expanded()))
            .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Top)
            .child(
                div()
                    .workbench_assistant()
                    .child(if self.editing_category.is_some() {
                        tr("Rename")
                    } else {
                        tr("Add category")
                    }),
            );
        {
            editor = editor
                .child(fileatlas_ui_bridge::components::text_input(&self.name_input).w_full())
                .child(
                    fileatlas_ui_bridge::components::button("launcher-save-category")
                        .label(tr(if self.editing_category.is_some() {
                            "Save category"
                        } else {
                            "Add category"
                        }))
                        .on_click(cx.listener(|this, _, window, cx| this.save_name(window, cx))),
                );
        }
        if self.editing_category.is_some() {
            editor = editor.child(
                fileatlas_ui_bridge::components::button("launcher-cancel-category")
                    .ghost()
                    .label(tr("Cancel"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.editing_category = None;
                        this.name_input
                            .update(cx, |input, cx| input.set_value("", window, cx));
                        cx.notify();
                    })),
            );
        }
        fileatlas_ui_bridge::components::section()
            .v_flex()
            .w(px(218.))
            .h_full()
            .flex_shrink_0()
            .p(px(fileatlas_ui_bridge::theme::space_expanded()))
            .gap(px(fileatlas_ui_bridge::theme::space_standard()))
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                    .py(px(fileatlas_ui_bridge::theme::space_expanded()))
                    .child(
                        div()
                            .workbench_foreground(
                                fileatlas_ui_bridge::components::ForegroundRole::AccentText,
                            )
                            .child(theme::large_icon(IconName::LayoutGrid)),
                    )
                    .child(fileatlas_ui_bridge::components::heading_block(
                        tr("Launchpad"),
                        tr("Personal program workbench"),
                    )),
            )
            .child(self.category_row(
                None,
                tr("All programs"),
                IconName::LayoutGrid,
                visible_count,
                !self.show_hidden && self.selected_category.is_none(),
                cx,
            ))
            .child(
                div()
                    .id("launcher-category-hidden")
                    .workbench_nav(self.show_hidden)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_hidden = true;
                        this.selected_category = None;
                        cx.notify();
                    }))
                    .child(div().w(px(20.)).child(theme::icon(IconName::EyeOff)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .workbench_assistant()
                            .text_ellipsis()
                            .child(tr("Hidden tools")),
                    )
                    .child(div().workbench_assistant().child(hidden_count.to_string())),
            )
            .child(
                div()
                    .pt(px(fileatlas_ui_bridge::theme::space_expanded()))
                    .pb(px(fileatlas_ui_bridge::theme::space_compact()))
                    .px(px(fileatlas_ui_bridge::theme::space_standard()))
                    .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Top)
                    .workbench_title()
                    .child(tr("Categories")),
            )
            .child(categories)
            .child(editor)
            .into_any_element()
    }
}
