use super::super::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::Icon;

pub(super) fn render_rail(view: &SidebarView, cx: &mut Context<SidebarView>) -> Div {
    let items = [
        (NavigationPage::Folders, IconName::Folder, "Folders"),
        (NavigationPage::Tags, IconName::Tag, "Tags"),
        (NavigationPage::Extensions, IconName::Puzzle, "Add-ons"),
    ];
    let mut rail = div()
        .v_flex()
        .w(px(48.))
        .h_full()
        .flex_shrink_0()
        .items_center()
        .gap(px(5.))
        .py(px(6.))
        .border_r_1()
        .border_color(rgb(theme::SEPARATOR));
    for (page, icon, label) in items {
        let selected = view.active_page == page;
        rail = rail.child(
            div()
                .id(format!("navigation-page-{}", page.key()))
                .v_flex()
                .w(px(42.))
                .h(px(54.))
                .items_center()
                .justify_center()
                .gap(px(3.))
                .rounded(px(6.))
                .text_xs()
                .text_color(rgb(if selected {
                    theme::accent_hover()
                } else {
                    theme::TEXT_SECONDARY
                }))
                .when(selected, |item| item.bg(rgba((theme::accent() << 8) | 55)))
                .hover(|style| style.bg(rgba(0xffffff10)))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.set_navigation_page(page);
                    cx.notify();
                }))
                .child(Icon::new(icon).size(px(17.)))
                .child(photag_ui_bridge::i18n::tr(label)),
        );
    }
    rail
}

pub(super) fn render_extensions(view: &SidebarView) -> Div {
    let page = div()
        .v_flex()
        .flex_1()
        .min_w_0()
        .h_full()
        .gap(px(10.))
        .px(px(14.))
        .py(px(12.))
        .child(
            div()
                .font_semibold()
                .text_size(px(15.))
                .child(photag_ui_bridge::i18n::tr("Add-ons")),
        );
    if view.extension_views.is_empty() {
        return page.child(
            div()
                .v_flex()
                .items_center()
                .gap(px(6.))
                .py(px(26.))
                .text_color(rgb(theme::TEXT_MUTED))
                .child(Icon::new(IconName::Puzzle).size(px(22.)))
                .child(photag_ui_bridge::i18n::tr("No available add-ons.")),
        );
    }
    page.children(view.extension_views.iter().cloned())
}
