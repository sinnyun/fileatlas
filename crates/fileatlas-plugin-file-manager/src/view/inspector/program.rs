//! Program overview follows the dedicated dark concept: identity, 2×2
//! metrics, data storage paths, status rows, and a separate tip card.
use super::*;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;

pub(super) fn render(view: &FileManagerView) -> AnyElement {
    let snapshot = view.snapshot.as_ref().ok();
    let folders = snapshot.map_or(0, |s| s.browser.directories.len());
    let tags = snapshot.map_or(0, |s| contexts::count_tags(&s.tag_tree));
    let database_path = view
        .cordis
        .try_service::<fileatlas_plugin_database::DatabaseService>()
        .ok()
        .map(|database| database.path().display().to_string());
    let lore_path = view.versions.storage_path().display().to_string();
    let busy = view.service.directory_worker_active();
    let error = view
        .snapshot
        .as_ref()
        .err()
        .cloned()
        .or_else(|| snapshot.and_then(|s| s.browser.error.clone()));
    let state = if error.is_some() {
        "Failed"
    } else if busy {
        "Loading…"
    } else {
        "Ready"
    };
    let status_color = if error.is_some() {
        theme::danger()
    } else if busy {
        theme::warning()
    } else {
        theme::success()
    };
    let mut body = div()
        .v_flex()
        .w_full()
        .min_w_0()
        .px(px(fileatlas_ui_bridge::theme::space_expanded()))
        .pb(px(fileatlas_ui_bridge::theme::space_expanded()))
        .child(section_title("Browse overview"))
        .child(
            div()
                .v_flex()
                .w_full()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                .child(
                    div()
                        .h_flex()
                        .w_full()
                        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                        .child(metric(
                            "Current results",
                            snapshot.map_or(0, |s| s.media_count).to_string(),
                            IconName::Image,
                            theme::accent(),
                        ))
                        .child(metric(
                            "Visible folders",
                            folders.to_string(),
                            IconName::Folder,
                            theme::warning_text(),
                        )),
                )
                .child(
                    div()
                        .h_flex()
                        .w_full()
                        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                        .child(metric(
                            "Tags",
                            tags.to_string(),
                            IconName::Tag,
                            theme::category_text(),
                        ))
                        .child(metric(
                            "Version",
                            env!("CARGO_PKG_VERSION").into(),
                            IconName::Info,
                            theme::data_text(),
                        )),
                ),
        )
        .child(inspector_divider())
        .child(section_title("Database storage locations"))
        .child(storage_row(
            "Application database",
            database_path.as_deref().unwrap_or("Unavailable"),
            IconName::Info,
            theme::accent(),
        ))
        .child(storage_row(
            "Lore version storage",
            &lore_path,
            IconName::Folder,
            theme::category_text(),
        ))
        .child(inspector_divider())
        .child(section_title("Program status"))
        .child(
            div()
                .h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .py(px(fileatlas_ui_bridge::theme::space_standard()))
                .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Bottom)
                .child(
                    div()
                        .workbench_body()
                        .child(fileatlas_ui_bridge::i18n::tr("Browsing state")),
                )
                .child(
                    div()
                        .h_flex()
                        .items_center()
                        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                        .child(fileatlas_ui_bridge::components::status_dot(
                            7.,
                            fileatlas_ui_bridge::components::StatusTone::Custom(status_color),
                        ))
                        .child(
                            div()
                                .workbench_body()
                                .child(fileatlas_ui_bridge::i18n::tr(state)),
                        ),
                ),
        )
        .child(detail_row(
            IconName::Folder,
            "Subfolders",
            folders.to_string(),
        ));
    if let Some(error) = error {
        body = body.child(inspector_message(&error));
    }
    body.child(inspector_divider()).child(section_title("Usage tips"))
        .child(well().h_flex().items_center().gap(px(fileatlas_ui_bridge::theme::space_expanded()))
            .child(theme::icon(IconName::Info).size(px(25.)).flex_shrink_0().workbench_foreground(fileatlas_ui_bridge::components::ForegroundRole::AccentText))
            .child(div().flex_1().min_w_0().workbench_assistant()
                .child(fileatlas_ui_bridge::i18n::tr("Select a folder or tag on the left to view its details, or select a file to inspect it."))))
        .into_any_element()
}

fn storage_row(title: &str, path: &str, icon: IconName, color: u32) -> Div {
    well()
        .v_flex()
        .w_full()
        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
        .child(
            div()
                .h_flex()
                .items_center()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                .child(theme::icon(icon).size(px(16.)).workbench_foreground(
                    fileatlas_ui_bridge::components::ForegroundRole::Custom(color),
                ))
                .child(
                    div()
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr(title)),
                ),
        )
        .child(
            div()
                .w_full()
                .min_w_0()
                .workbench_assistant()
                .child(path.to_owned()),
        )
}

fn metric(title: &str, value: String, icon: IconName, color: u32) -> Div {
    well()
        .flex_1()
        .min_h(px(86.))
        .p(px(fileatlas_ui_bridge::theme::space_expanded()))
        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
        .child(
            div()
                .h_flex()
                .items_center()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                .child(
                    theme::icon(icon)
                        .size(px(17.))
                        .flex_shrink_0()
                        .workbench_foreground(
                            fileatlas_ui_bridge::components::ForegroundRole::Custom(color),
                        ),
                )
                .child(
                    div()
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr(title)),
                ),
        )
        .child(
            div()
                .pl(px(fileatlas_ui_bridge::theme::space_expanded()))
                .workbench_body()
                .child(value),
        )
}
