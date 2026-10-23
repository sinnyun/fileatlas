//! v5 empty states for the file_manager grid (mockup 08): library-empty with an
//! "Add folder" action and filtered no-results with "Clear filters".

use super::*;
use fileatlas_ui_bridge::components::WorkbenchForeground as _;
use fileatlas_ui_bridge::components::WorkbenchPaint as _;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::ButtonVariants;

pub(super) fn render(no_results: bool, cx: &mut Context<FileManagerView>) -> AnyElement {
    let (title, body) = if no_results {
        (
            "No matches",
            "Try adjusting or clearing the filters to see media",
        )
    } else {
        (
            "No media added",
            "Add a folder with photos and videos to start browsing",
        )
    };
    let emblem = if no_results {
        theme::empty_state_icon(IconName::Search)
            .workbench_foreground(fileatlas_ui_bridge::components::ForegroundRole::Muted)
            .into_any_element()
    } else {
        div()
            .size(px(72.))
            .workbench_corner(fileatlas_ui_bridge::components::CornerRole::ExtraLarge)
            .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Faint)
            .flex()
            .items_center()
            .justify_center()
            .child(
                theme::empty_state_icon(IconName::Image)
                    .workbench_foreground(fileatlas_ui_bridge::components::ForegroundRole::Muted),
            )
            .into_any_element()
    };
    let action = if no_results {
        fileatlas_ui_bridge::components::button("file_manager-empty-clear-filters")
            .ghost()
            .label(fileatlas_ui_bridge::i18n::tr("Clear filters"))
            .on_click(cx.listener(|this, _, window, cx| {
                this.clear_filters(window, cx);
            }))
            .into_any_element()
    } else {
        fileatlas_ui_bridge::components::button("file_manager-empty-add-folder")
            .primary()
            .label(fileatlas_ui_bridge::i18n::tr("Add folder"))
            .icon(theme::icon(IconName::FolderPlus))
            .on_click(cx.listener(|this, _, _, _| {
                this.dispatch_command("navigation/add-folder");
            }))
            .into_any_element()
    };
    div()
        .size_full()
        .v_flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .v_flex()
                .items_center()
                .child(emblem)
                .child(
                    div()
                        .mt(px(fileatlas_ui_bridge::theme::space_expanded()))
                        .workbench_title()
                        .child(fileatlas_ui_bridge::i18n::tr(title)),
                )
                .child(
                    div()
                        .mt(px(theme::space_standard()))
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr(body)),
                )
                .child(
                    div()
                        .mt(px(fileatlas_ui_bridge::theme::space_expanded()))
                        .child(action),
                ),
        )
        .into_any_element()
}
