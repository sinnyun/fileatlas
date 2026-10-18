//! Right-aligned view controls in the gallery filter band.
use super::*;
use gpui_kit::assets::IconName;

pub(super) fn button(id: &'static str, icon: IconName, label: &str) -> Button {
    photag_ui_bridge::components::icon_button(id, icon)
        .tooltip(photag_ui_bridge::i18n::tr(label))
        .accessibility_label(photag_ui_bridge::i18n::tr(label))
}

pub(super) fn render(view: &GalleryView, cx: &mut Context<GalleryView>) -> impl IntoElement {
    div()
        .id("gallery-view-controls")
        .h_flex()
        .flex_shrink_0()
        .items_center()
        .gap(px(4.))
        .child(
            button("gallery-layout-grid", IconName::LayoutGrid, "Grid")
                .toggled(!view.list_layout)
                .on_click(cx.listener(|view, _, _, cx| view.set_layout(false, cx))),
        )
        .child(
            button("gallery-layout-list", IconName::List, "List")
                .toggled(view.list_layout)
                .on_click(cx.listener(|view, _, _, cx| view.set_layout(true, cx))),
        )
        .child(
            button(
                "gallery-descendants",
                IconName::FolderTree,
                "Show subfolders",
            )
            .toggled(view.service.show_descendants())
            .on_click(cx.listener(|view, _, _, cx| {
                view.set_descendants(!view.service.show_descendants(), cx)
            })),
        )
        .child(sliders::thumbnail(view, cx))
}
