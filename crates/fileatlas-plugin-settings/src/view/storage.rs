use super::*;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
use gpui_kit::prelude::FluentBuilder;

impl SettingsView {
    fn request_storage_summary(&mut self) {
        let revision = self.storage_summary_revision;
        let catalog = self.storage_catalog();
        self.storage_summary.request(revision, move || {
            Ok(plugins::measure_storage_catalog(catalog))
        });
    }

    pub(super) fn poll_storage_summary(&mut self) -> bool {
        let active = self.surfaces.is_open("settings")
            && self.surfaces.value("settings.tab").as_deref() == Some("storage");
        if !active {
            self.storage_summary_active = false;
            return false;
        }
        let signature = self.storage_plugin_signature();
        let refresh_signal = self.surfaces.value("settings.storage.refresh");
        let refresh_requested = refresh_signal != self.storage_summary_refresh_signal;
        self.storage_summary_refresh_signal = refresh_signal;
        let mut changed = false;
        if !self.storage_summary_active
            || self.storage_summary_catalog_signature != signature
            || refresh_requested
        {
            self.storage_summary_active = true;
            self.storage_summary_catalog_signature = signature;
            self.storage_summary_revision = self.storage_summary_revision.wrapping_add(1);
            self.request_storage_summary();
            changed = true;
        }
        self.storage_summary.poll() | changed
    }

    fn refresh_storage_summary(&mut self, cx: &mut Context<Self>) {
        self.storage_summary_revision = self.storage_summary_revision.wrapping_add(1);
        self.storage_summary_active = true;
        self.request_storage_summary();
        cx.notify();
    }

    pub(super) fn render_storage_page(&self, cx: &mut Context<Self>) -> Div {
        let key = self.storage_summary_revision;
        let rows = if self.storage_summary.key() == Some(&key) {
            self.storage_summary
                .value()
                .and_then(|result| result.as_ref().ok())
        } else {
            None
        };
        let mut groups: Vec<(String, Vec<&plugins::StorageSummaryRow>)> = Vec::new();
        if let Some(rows) = rows {
            for row in rows {
                if let Some((_, entries)) = groups
                    .iter_mut()
                    .find(|(plugin_id, _)| plugin_id == &row.plugin_id)
                {
                    entries.push(row);
                } else {
                    groups.push((row.plugin_id.clone(), vec![row]));
                }
            }
        }
        let plugin_titles = rows.map(|rows| {
            rows.iter()
                .map(|row| (row.plugin_id.as_str(), row.plugin_title.as_str()))
                .collect::<std::collections::HashMap<_, _>>()
        });
        let groups = groups.into_iter().map(|(plugin_id, entries)| {
            let title = plugin_titles
                .as_ref()
                .and_then(|titles| titles.get(plugin_id.as_str()))
                .copied()
                .unwrap_or(plugin_id.as_str());
            let items = entries.into_iter().enumerate().map(|(index, row)| {
                let address = row.path.as_ref().map(|path| {
                    let full_path = path.display().to_string();
                    let clipboard_path = full_path.clone();
                    div()
                        .h_flex()
                        .items_center()
                        .min_w_0()
                        .gap(px(theme::space_compact()))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .workbench_assistant()
                                .child(plugins::compact_storage_path(path)),
                        )
                        .child(
                            fileatlas_ui_bridge::components::icon_button(
                                format!("settings-storage-copy-{}-{index}", row.plugin_id),
                                gpui_kit::assets::IconName::Copy,
                            )
                            .tooltip(full_path)
                            .accessibility_label(fileatlas_ui_bridge::i18n::tr("Copy full path"))
                            .on_click(cx.listener(
                                move |_, _, _, cx| {
                                    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                                        clipboard_path.clone(),
                                    ));
                                },
                            )),
                        )
                });
                let usage = match row.usage.bytes {
                    Some(bytes) if row.usage.complete => plugins::format_storage_size(bytes),
                    Some(bytes) if bytes > 0 => fileatlas_ui_bridge::i18n::tr_args(
                        "At least {size} (some contents could not be read)",
                        &[("size", plugins::format_storage_size(bytes))],
                    ),
                    Some(_) => fileatlas_ui_bridge::i18n::tr("Size could not be fully read"),
                    None => fileatlas_ui_bridge::i18n::tr("No registered path"),
                };
                let used = fileatlas_ui_bridge::i18n::tr_args("Used {size}", &[("size", usage)]);
                design::row()
                    .child(design::label(row.label, row.note))
                    .child(
                        div()
                            .v_flex()
                            .items_end()
                            .min_w_0()
                            .child(address.unwrap_or_else(|| {
                                div()
                                    .workbench_assistant()
                                    .child(fileatlas_ui_bridge::i18n::tr(
                                        "This plugin has no registered path",
                                    ))
                            }))
                            .child(div().workbench_assistant().child(used)),
                    )
            });
            div()
                .v_flex()
                .gap(px(theme::space_compact()))
                .child(
                    div()
                        .workbench_title()
                        .child(fileatlas_ui_bridge::i18n::tr(title)),
                )
                .children(items)
        });
        let loading = self.storage_summary.is_pending()
            || (self.storage_summary.key() != Some(&key) && self.storage_summary.value().is_none());
        let error = self
            .storage_summary
            .value()
            .and_then(|result| result.as_ref().err());
        div()
            .v_flex()
            .gap(px(theme::space_standard()))
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .justify_between()
                    .child(self.section("Plugin storage locations").flex_1())
                    .child(
                        fileatlas_ui_bridge::components::button("settings-storage-summary-refresh")
                            .outline()
                            .icon(theme::small_icon(gpui_kit::assets::IconName::RefreshCw))
                            .label(fileatlas_ui_bridge::i18n::tr("Refresh"))
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.refresh_storage_summary(cx);
                            })),
                    ),
            )
            .when_some(error, |this, error| {
                this.child(
                    div()
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr_args(
                            "Storage information could not be read: {error}",
                            &[("error", error.to_string())],
                        )),
                )
            })
            .when(loading, |this| {
                this.child(
                    div()
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr(
                            "Collecting plugin storage sizes…",
                        )),
                )
            })
            .when(rows.is_some_and(|rows| rows.is_empty()), |this| {
                this.child(
                    div()
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr(
                            "No plugin storage locations are registered.",
                        )),
                )
            })
            .children(groups)
            .child(
                div()
                    .workbench_assistant()
                    .child(fileatlas_ui_bridge::i18n::tr(
                        "Sizes are per path; overlapping directories should not be added together.",
                    )),
            )
    }
}
