use super::super::*;
use fileatlas_ui_bridge::components::WorkbenchPaint as _;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
use gpui_kit::assets::IconName;

pub(super) fn render_rail(view: &SidebarView, cx: &mut Context<SidebarView>) -> Div {
    let items = [
        (NavigationPage::Folders, IconName::Folder, "Folders"),
        (NavigationPage::Tags, IconName::Tag, "Tags"),
        (
            NavigationPage::ProjectTree,
            IconName::Folder,
            "Project tree",
        ),
        (NavigationPage::Extensions, IconName::Puzzle, "Add-ons"),
    ];
    let mut rail = theme::separate_region(
        div()
            .v_flex()
            .w(px(48.))
            .h_full()
            .flex_shrink_0()
            .items_center()
            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
            .py(px(fileatlas_ui_bridge::theme::space_standard()))
            .workbench_corner_right(fileatlas_ui_bridge::components::CornerRole::Large)
            .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Rail),
        theme::RegionEdge::Right,
    );
    for (page, icon, label) in items {
        if page == NavigationPage::ProjectTree && view.project_tree_views.is_empty() {
            continue;
        }
        let selected = view.active_page == page;
        rail = rail.child(
            fileatlas_ui_bridge::components::rail_shortcut(
                format!("navigation-page-{}", page.key()),
                fileatlas_ui_bridge::i18n::tr(label),
                theme::icon(icon),
                selected,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.set_navigation_page(page);
                cx.notify();
            })),
        );
    }
    rail.children(
        view.launcher_shortcuts
            .iter()
            .map(|(_, _, button)| button.clone()),
    )
}

pub(super) fn render_extensions(view: &SidebarView) -> Div {
    let page = div()
        .v_flex()
        .flex_1()
        .min_w_0()
        .h_full()
        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
        .px(px(fileatlas_ui_bridge::theme::space_expanded()))
        .py(px(fileatlas_ui_bridge::theme::space_expanded()))
        .child(
            div()
                .workbench_title()
                .child(fileatlas_ui_bridge::i18n::tr("Add-ons")),
        );
    let extension_views = view
        .extension_views
        .iter()
        .map(|(_, _, extension)| extension.clone())
        .collect::<Vec<_>>();
    if extension_views.is_empty() {
        return page.child(
            div()
                .v_flex()
                .items_center()
                .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                .py(px(fileatlas_ui_bridge::theme::space_expanded()))
                .child(theme::large_icon(IconName::Puzzle))
                .child(
                    div()
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr("No available add-ons.")),
                ),
        );
    }
    page.children(extension_views)
}
