use super::*;

impl Render for LauncherPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.renders = self.renders.wrapping_add(1);
        self.request_icons();
        let filtered = self.sorted_entries(&self.search_input.read(cx).value().to_string());
        let count = filtered.len();
        let heading = if self.show_hidden {
            tr("Hidden tools")
        } else {
            self.selected_category
                .and_then(|id| self.snapshot.categories.iter().find(|c| c.id == id))
                .map(|c| cards::category_name(&c.name))
                .unwrap_or_else(|| tr("All programs"))
        };
        let icons = self.icons.value().and_then(|r| r.as_ref().ok());
        let manual_icon_status = self
            .snapshot
            .entries
            .iter()
            .filter(|entry| entry.source == "manual")
            .map(|entry| {
                let status = icons
                    .and_then(|set| set.sources.get(&entry.launch_path))
                    .map(|source| match source {
                        crate::icons::IconSource::Disk => "disk-cache",
                        crate::icons::IconSource::Shell => "shell-extracted",
                        crate::icons::IconSource::Failed => "failed",
                    })
                    .unwrap_or("pending");
                (entry.id.to_string(), status)
            })
            .collect::<std::collections::HashMap<_, _>>();
        let failed_icons = self
            .snapshot
            .entries
            .iter()
            .filter(|entry| {
                icons.is_some_and(|set| {
                    set.sources.get(&entry.launch_path) == Some(&crate::icons::IconSource::Failed)
                })
            })
            .map(|entry| {
                serde_json::json!({
                    "id": entry.id,
                    "display_name": entry.display_name,
                    "launch_path": entry.launch_path,
                })
            })
            .collect::<Vec<_>>();
        let _=self.surfaces.set_value("launcher.ui",&serde_json::json!({
            "selected_entry":self.selected_entry,"draft_category":self.draft_category,"draft_hidden":self.draft_hidden,
            "draft_name":self.entry_name.read(cx).value().to_string(),"sort":self.sort_by.key(),"descending":self.descending,
            "visible_ids":filtered.iter().map(|e|e.id).collect::<Vec<_>>(),"visible_count":count,
            "icons_loaded":icons.map(|i|i.images.values().filter(|v|v.is_some()).count()).unwrap_or(0),
            "icons_failed":icons.map(|i|i.images.values().filter(|v|v.is_none()).count()).unwrap_or(0),
            "icon_cache":{
                "disk_hits":icons.map(|i|i.count(crate::icons::IconSource::Disk)).unwrap_or(0),
                "shell_extractions":icons.map(|i|i.count(crate::icons::IconSource::Shell)).unwrap_or(0),
                "failed":icons.map(|i|i.count(crate::icons::IconSource::Failed)).unwrap_or(0),
                "manual_programs":manual_icon_status,
                "failed_programs":failed_icons,
            },
            "icons_pending":self.icons.is_pending(),"render_count":self.renders,
            "add_program_pending":self.add_program_task.is_some(),
        }).to_string());
        let mut grid = div()
            .h_flex()
            .w_full()
            .flex_shrink_0()
            .flex_wrap()
            .items_start()
            .gap(px(12.));
        for entry in filtered {
            grid = grid.child(self.program_card(entry, cx));
        }
        let sorting = cx.entity().downgrade();
        let mut body = div()
            .v_flex()
            .h_full()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .gap(px(16.))
            .p(px(18.))
            .child(
                div()
                    .h_flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h(px(36.))
                            .h_flex()
                            .items_center()
                            .child(photag_ui_bridge::components::search_input(
                                &self.search_input,
                            )),
                    )
                    .child(
                        Button::new("launcher-add-program")
                            .h(px(34.))
                            .label(tr("Add program"))
                            .icon(theme::icon(IconName::Plus))
                            .disabled(self.add_program_task.is_some())
                            .on_click(
                                cx.listener(|this, _, window, cx| this.add_program(window, cx)),
                            ),
                    )
                    .child(
                        Button::new("launcher-rescan")
                            .h(px(34.))
                            .label(tr(if self.snapshot.scanning {
                                "Scanning…"
                            } else {
                                "Scan programs"
                            }))
                            .disabled(self.snapshot.scanning)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.focus_handle.focus(window, cx);
                                let result = this.service.refresh_discovery();
                                this.report(result, cx);
                            })),
                    ),
            )
            .child(
                div()
                    .h_flex()
                    .flex_shrink_0()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .v_flex()
                            .gap(px(4.))
                            .min_w_0()
                            .child(div().text_size(px(18.)).font_semibold().child(heading))
                            .child(div().text_xs().text_color(rgb(theme::text_muted())).child(
                                photag_ui_bridge::i18n::tr_args(
                                    "Showing {count} programs",
                                    &[("count", count.to_string())],
                                ),
                            )),
                    )
                    .child(
                        div()
                            .h_flex()
                            .gap(px(4.))
                            .child(
                                Button::new("launcher-sort")
                                    .label(format!("{} ▾", self.sort_by.label()))
                                    .dropdown_menu(move |mut menu, _, _| {
                                        for sort in [SortBy::Name, SortBy::Installed, SortBy::Size]
                                        {
                                            let view = sorting.clone();
                                            menu = menu.item(
                                                PopupMenuItem::new(sort.label()).on_click(
                                                    move |_, window, cx| {
                                                        let _ = view.update(cx, |this, cx| {
                                                            this.focus_handle.focus(window, cx);
                                                            this.sort_by = sort;
                                                            this.descending = sort != SortBy::Name;
                                                            cx.notify();
                                                        });
                                                    },
                                                ),
                                            );
                                        }
                                        menu
                                    }),
                            )
                            .child(
                                Button::new("launcher-sort-direction")
                                    .ghost()
                                    .label(if self.descending { "↓" } else { "↑" })
                                    .tooltip(tr(if self.descending {
                                        "Descending"
                                    } else {
                                        "Ascending"
                                    }))
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.focus_handle.focus(window, cx);
                                        this.descending = !this.descending;
                                        cx.notify();
                                    })),
                            ),
                    ),
            );
        if let Some(message) = &self.message {
            body = body.child(
                div()
                    .text_sm()
                    .text_color(rgb(theme::danger_text()))
                    .child(message.clone()),
            );
        }
        let contents = if count == 0 {
            div()
                .v_flex()
                .items_center()
                .justify_center()
                .h(px(200.))
                .gap(px(12.))
                .text_color(rgb(theme::text_muted()))
                .child(theme::empty_state_icon(IconName::LayoutGrid))
                .child(tr(if self.snapshot.scanning {
                    "Discovering system programs…"
                } else {
                    "No matching programs"
                }))
                .child(
                    div()
                        .text_xs()
                        .text_center()
                        .child(tr("Try another category, search term, or add a program.")),
                )
                .into_any_element()
        } else {
            grid.into_any_element()
        };
        body = body.child(
            div()
                .v_flex()
                .flex_1()
                .min_h_0()
                .overflow_y_scrollbar()
                .child(contents),
        );
        div()
            .track_focus(&self.focus_handle)
            .v_flex()
            .size_full()
            .min_h_0()
            .text_size(px(theme::FONT_SIZE_BODY))
            .bg(rgb(theme::surface_base()))
            .child(
                div()
                    .h_flex()
                    .items_stretch()
                    .flex_1()
                    .min_h_0()
                    .child(self.sidebar(cx))
                    .child(body)
                    .child(self.inspector(cx)),
            )
            .child(
                div()
                    .h_flex()
                    .flex_shrink_0()
                    .h(px(34.))
                    .items_center()
                    .justify_between()
                    .px(px(14.))
                    .border_t_1()
                    .border_color(rgba(theme::separator_light()))
                    .text_xs()
                    .text_color(rgb(theme::text_muted()))
                    .child(photag_ui_bridge::i18n::tr_args(
                        "{programs} programs · {categories} categories",
                        &[
                            ("programs", self.snapshot.entries.len().to_string()),
                            ("categories", self.snapshot.categories.len().to_string()),
                        ],
                    ))
                    .child(tr(
                        "Click to select · Double-click to launch · Right-click to manage",
                    ))
                    .child(
                        div()
                            .text_color(rgb(if self.snapshot.scanning || self.icons.is_pending() {
                                theme::accent_text()
                            } else {
                                theme::success_text()
                            }))
                            .child(tr(if self.snapshot.scanning {
                                "◌ Scanning"
                            } else if self.icons.is_pending() {
                                "Loading icons…"
                            } else {
                                "● Scan complete"
                            })),
                    ),
            )
    }
}
