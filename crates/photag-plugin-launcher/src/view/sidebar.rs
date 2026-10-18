use super::*;

impl LauncherPanel {
    fn category_row(
        &self,
        id: Option<i64>,
        name: String,
        icon: &'static str,
        count: usize,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .id(format!("launcher-category-{}", id.unwrap_or(0)))
            .h_flex()
            .items_center()
            .h(px(36.))
            .px(px(10.))
            .gap(px(9.))
            .rounded(px(6.))
            .cursor_pointer()
            .bg(if selected { rgba(0x47bfff24) } else { rgba(0) })
            .text_color(rgb(if selected {
                theme::accent()
            } else {
                theme::TEXT_SECONDARY
            }))
            .hover(|style| style.bg(rgba(0xffffff0c)))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.selected_category = id;
                this.show_hidden = false;
                cx.notify();
            }))
            .child(div().w(px(20.)).text_size(px(17.)).child(icon))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .text_sm()
                    .child(name),
            )
            .child(div().text_xs().child(count.to_string()))
            .into_any_element()
    }

    pub(super) fn sidebar(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut categories = div()
            .v_flex()
            .gap(px(3.))
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
                    Button::new(format!("launcher-category-menu-{id}"))
                        .ghost()
                        .label("⋯")
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
            .gap(px(7.))
            .pt(px(12.))
            .border_t_1()
            .border_color(rgba(theme::SEPARATOR_LIGHT))
            .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(
                if self.editing_category.is_some() {
                    tr("Rename")
                } else {
                    tr("Add category")
                },
            ));
        {
            editor = editor.child(Input::new(&self.name_input).w_full()).child(
                Button::new("launcher-save-category")
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
                Button::new("launcher-cancel-category")
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
        div()
            .v_flex()
            .w(px(218.))
            .h_full()
            .flex_shrink_0()
            .p(px(12.))
            .gap(px(8.))
            .bg(rgb(theme::SURFACE_FAINT))
            .border_r_1()
            .border_color(rgba(theme::SEPARATOR_LIGHT))
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap(px(10.))
                    .py(px(12.))
                    .child(
                        div()
                            .text_size(px(27.))
                            .text_color(rgb(theme::accent()))
                            .child("▦"),
                    )
                    .child(
                        div()
                            .v_flex()
                            .gap(px(4.))
                            .child(
                                div()
                                    .font_semibold()
                                    .text_size(px(17.))
                                    .child(tr("Launchpad")),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(theme::TEXT_MUTED))
                                    .child(tr("Personal program workbench")),
                            ),
                    ),
            )
            .child(self.category_row(
                None,
                tr("All programs"),
                "▦",
                visible_count,
                !self.show_hidden && self.selected_category.is_none(),
                cx,
            ))
            .child(
                div()
                    .id("launcher-category-hidden")
                    .h_flex()
                    .items_center()
                    .h(px(36.))
                    .px(px(10.))
                    .gap(px(9.))
                    .rounded(px(6.))
                    .cursor_pointer()
                    .bg(if self.show_hidden {
                        rgba(0x47bfff24)
                    } else {
                        rgba(0)
                    })
                    .text_color(rgb(if self.show_hidden {
                        theme::accent()
                    } else {
                        theme::TEXT_SECONDARY
                    }))
                    .hover(|s| s.bg(rgba(0xffffff0c)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_hidden = true;
                        this.selected_category = None;
                        cx.notify();
                    }))
                    .child(div().w(px(20.)).child("◉"))
                    .child(div().flex_1().text_sm().child(tr("Hidden tools")))
                    .child(div().text_xs().child(hidden_count.to_string())),
            )
            .child(
                div()
                    .pt(px(14.))
                    .pb(px(5.))
                    .px(px(10.))
                    .border_t_1()
                    .border_color(rgba(theme::SEPARATOR_LIGHT))
                    .text_xs()
                    .text_color(rgb(theme::TEXT_MUTED))
                    .child(tr("Categories")),
            )
            .child(categories)
            .child(editor)
            .into_any_element()
    }
}
