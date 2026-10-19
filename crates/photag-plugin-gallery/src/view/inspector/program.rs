//! Program overview follows the dedicated dark concept: identity, 2×2
//! metrics, data storage paths, status rows, and a separate tip card.
use super::*;

pub(super) fn render(view: &GalleryView) -> AnyElement {
    let snapshot = view.snapshot.as_ref().ok();
    let folders = snapshot.map_or(0, |s| s.browser.directories.len());
    let tags = snapshot.map_or(0, |s| contexts::count_tags(&s.tag_tree));
    let database_path = view
        .cordis
        .try_service::<photag_plugin_database::DatabaseService>()
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
        .px(px(16.))
        .pb(px(20.))
        .child(section_title("Browse overview"))
        .child(
            div()
                .v_flex()
                .w_full()
                .gap(px(10.))
                .child(
                    div()
                        .h_flex()
                        .w_full()
                        .gap(px(10.))
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
                        .gap(px(10.))
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
                .py(px(9.))
                .border_b_1()
                .border_color(rgba(theme::separator_light()))
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(rgb(theme::text_secondary()))
                        .child(photag_ui_bridge::i18n::tr("Browsing state")),
                )
                .child(
                    div()
                        .h_flex()
                        .items_center()
                        .gap(px(7.))
                        .child(div().size(px(7.)).rounded_full().bg(rgb(status_color)))
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(rgb(theme::text_primary()))
                                .child(photag_ui_bridge::i18n::tr(state)),
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
        .child(well().h_flex().items_center().gap(px(12.))
            .child(theme::icon(IconName::Info).size(px(25.)).flex_shrink_0().text_color(rgb(theme::accent_text())))
            .child(div().flex_1().min_w_0().text_size(px(13.)).text_color(rgb(theme::text_secondary()))
                .child(photag_ui_bridge::i18n::tr("Select a folder or tag on the left to view its details, or select a file to inspect it."))))
        .into_any_element()
}

fn storage_row(title: &str, path: &str, icon: IconName, color: u32) -> Div {
    well()
        .v_flex()
        .w_full()
        .gap(px(7.))
        .child(
            div()
                .h_flex()
                .items_center()
                .gap(px(8.))
                .child(theme::icon(icon).size(px(16.)).text_color(rgb(color)))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(theme::text_secondary()))
                        .child(photag_ui_bridge::i18n::tr(title)),
                ),
        )
        .child(
            div()
                .w_full()
                .min_w_0()
                .text_size(px(12.))
                .text_color(rgb(theme::text_primary()))
                .child(path.to_owned()),
        )
}

fn metric(title: &str, value: String, icon: IconName, color: u32) -> Div {
    well()
        .flex_1()
        .min_h(px(86.))
        .p(px(13.))
        .gap(px(10.))
        .child(
            div()
                .h_flex()
                .items_center()
                .gap(px(7.))
                .child(
                    theme::icon(icon)
                        .size(px(17.))
                        .flex_shrink_0()
                        .text_color(rgb(color)),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(theme::text_secondary()))
                        .child(photag_ui_bridge::i18n::tr(title)),
                ),
        )
        .child(
            div()
                .pl(px(24.))
                .text_size(px(25.))
                .font_family(theme::SEMIBOLD_FONT_FAMILY)
                .text_color(rgb(theme::text_primary()))
                .child(value),
        )
}
