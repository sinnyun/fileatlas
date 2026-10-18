use super::*;

fn information_row(label: &str, value: String) -> AnyElement {
    div()
        .v_flex()
        .gap(px(4.))
        .child(
            div()
                .text_xs()
                .text_color(rgb(theme::TEXT_MUTED))
                .child(tr(label)),
        )
        .child(div().text_sm().child(value))
        .into_any_element()
}

fn size_text(size: Option<i64>) -> String {
    match size {
        Some(size) if size >= 1024 * 1024 * 1024 => {
            format!("{:.2} GB", size as f64 / (1024. * 1024. * 1024.))
        }
        Some(size) if size >= 1024 * 1024 => format!("{:.1} MB", size as f64 / (1024. * 1024.)),
        Some(size) => format!("{:.1} KB", size as f64 / 1024.),
        None => tr("Unknown"),
    }
}

impl LauncherPanel {
    pub(super) fn inspector(&self, cx: &mut Context<Self>) -> AnyElement {
        let selected = self
            .selected_entry
            .and_then(|id| self.snapshot.entries.iter().find(|e| e.id == id))
            .cloned();
        let mut panel = div()
            .v_flex()
            .w(px(250.))
            .h_full()
            .flex_shrink_0()
            .p(px(14.))
            .gap(px(14.))
            .border_l_1()
            .border_color(rgba(theme::SEPARATOR_LIGHT))
            .bg(rgb(theme::SURFACE_FAINT))
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .font_semibold()
                            .text_size(px(15.))
                            .child(tr("Program information")),
                    )
                    .child(
                        Button::new("launcher-inspector-close")
                            .ghost()
                            .label("×")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.selected_entry = None;
                                cx.notify();
                            })),
                    ),
            );
        let Some(entry) = selected else {
            return panel
                .child(
                    div()
                        .v_flex()
                        .flex_1()
                        .items_center()
                        .justify_center()
                        .gap(px(14.))
                        .text_color(rgb(theme::TEXT_MUTED))
                        .child(div().text_size(px(32.)).child("▦"))
                        .child(div().text_sm().child(tr("Select a program")))
                        .child(
                            div()
                                .text_xs()
                                .text_center()
                                .child(tr("Select an icon to view and edit its information.")),
                        ),
                )
                .into_any_element();
        };
        let id = entry.id;
        let pin_scope = entry.pin_scope;
        let weak = cx.entity().downgrade();
        let pin_view = cx.entity().downgrade();
        let categories = self.snapshot.categories.clone();
        let current_category = categories
            .iter()
            .find(|c| c.id == self.draft_category)
            .map(|c| cards::category_name(&c.name))
            .unwrap_or_default();
        let dirty = self.entry_name.read(cx).value().trim() != entry.display_name
            || self.draft_category != entry.category_id
            || self.draft_hidden != entry.hidden;
        let form = div()
            .v_flex()
            .gap(px(14.))
            .flex_shrink_0()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .py(px(8.))
                    .child(self.icon(&entry, 72.)),
            )
            .child(
                div()
                    .v_flex()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(theme::TEXT_MUTED))
                            .child(tr("Display name")),
                    )
                    .child(Input::new(&self.entry_name).w_full()),
            )
            .child(
                div()
                    .v_flex()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(theme::TEXT_MUTED))
                            .child(tr("Categories")),
                    )
                    .child(
                        Button::new("launcher-inspector-category")
                            .label(format!("{current_category}  ▾"))
                            .dropdown_menu(move |mut menu, _, _| {
                                for category in &categories {
                                    let view = weak.clone();
                                    let category_id = category.id;
                                    menu = menu.item(
                                        PopupMenuItem::new(cards::category_name(&category.name))
                                            .on_click(move |_, _, cx| {
                                                let _ = view.update(cx, |this, cx| {
                                                    this.draft_category = category_id;
                                                    cx.notify();
                                                });
                                            }),
                                    );
                                }
                                menu
                            }),
                    ),
            )
            .child(
                Button::new("launcher-inspector-hidden")
                    .label(tr(if self.draft_hidden {
                        "☑ Hidden program"
                    } else {
                        "☐ Hide this program"
                    }))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.draft_hidden = !this.draft_hidden;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("launcher-inspector-pin")
                    .label(format!(
                        "{}: {}  ▾",
                        tr("Pin to top"),
                        tr(match pin_scope {
                            crate::PinScope::None => "Not pinned",
                            crate::PinScope::Category => "Category top",
                            crate::PinScope::AllPrograms => "All programs top",
                        })
                    ))
                    .dropdown_menu(move |mut menu, _, _| {
                        for (scope, label) in [
                            (crate::PinScope::Category, "Pin to category top"),
                            (crate::PinScope::AllPrograms, "Pin to all programs top"),
                            (crate::PinScope::None, "Unpin"),
                        ] {
                            let view = pin_view.clone();
                            let marker = if pin_scope == scope { "✓ " } else { "" };
                            menu = menu.item(
                                PopupMenuItem::new(format!("{marker}{}", tr(label))).on_click(
                                    move |_, _, cx| {
                                        let _ = view.update(cx, |this, cx| {
                                            this.report(this.service.set_pin_scope(id, scope), cx);
                                        });
                                    },
                                ),
                            );
                        }
                        menu
                    }),
            )
            .child(
                div()
                    .pt(px(12.))
                    .border_t_1()
                    .border_color(rgba(theme::SEPARATOR_LIGHT))
                    .font_semibold()
                    .child(tr("Installation information")),
            )
            .child(information_row(
                "Installation date",
                entry.installed_on.clone().unwrap_or_else(|| tr("Unknown")),
            ))
            .child(information_row(
                "Registered size",
                size_text(entry.size_bytes),
            ))
            .child(information_row(
                "Source",
                tr(match entry.source.as_str() {
                    "installed" => "Installed program",
                    "manual" => "Manually added",
                    _ => "Start menu shortcut",
                }),
            ))
            .child(information_row(
                "Path",
                entry
                    .launch_path
                    .to_string_lossy()
                    .replace('\\', "\\\u{200b}"),
            ))
            .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(tr(
                "Installation information comes from Windows. Missing values are unknown.",
            )));
        panel = panel
            .child(
                div()
                    .v_flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(form),
            )
            .child(
                div()
                    .v_flex()
                    .gap(px(7.))
                    .flex_shrink_0()
                    .child(
                        Button::new("launcher-inspector-save")
                            .primary()
                            .label(tr("Save changes"))
                            .disabled(!dirty)
                            .on_click(cx.listener(|this, _, _, cx| this.save_entry(cx))),
                    )
                    .child(
                        Button::new("launcher-inspector-launch")
                            .label(tr("Launch"))
                            .disabled(!entry.available)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let result = this.service.launch(id);
                                this.report(result, cx);
                            })),
                    ),
            );
        panel.into_any_element()
    }
}
