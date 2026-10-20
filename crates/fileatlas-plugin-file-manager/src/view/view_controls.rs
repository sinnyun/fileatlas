//! Right-aligned view controls in the file_manager filter band.
use super::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::prelude::FluentBuilder as _;

pub(super) fn button(id: &'static str, icon: IconName, label: &str) -> Button {
    fileatlas_ui_bridge::components::icon_button(id, icon)
        .tooltip(fileatlas_ui_bridge::i18n::tr(label))
        .accessibility_label(fileatlas_ui_bridge::i18n::tr(label))
}

pub(super) fn modes(view: &FileManagerView, cx: &mut Context<FileManagerView>) -> impl IntoElement {
    div()
        .id("file_manager-view-modes")
        .h_flex()
        .flex_shrink_0()
        .items_center()
        .gap(px(4.))
        .child(
            button("file_manager-layout-grid", IconName::LayoutGrid, "Grid")
                .toggled(!view.list_layout)
                .on_click(cx.listener(|view, _, _, cx| view.set_layout(false, cx))),
        )
        .child(
            button("file_manager-layout-list", IconName::List, "List")
                .toggled(view.list_layout)
                .on_click(cx.listener(|view, _, _, cx| view.set_layout(true, cx))),
        )
}

pub(super) fn options(
    view: &FileManagerView,
    cx: &mut Context<FileManagerView>,
) -> impl IntoElement {
    div()
        .id("file_manager-view-options")
        .h_flex()
        .flex_shrink_0()
        .items_center()
        .gap(px(4.))
        .child(
            button(
                "file_manager-descendants",
                if view.service.show_descendants() {
                    IconName::ToggleRight
                } else {
                    IconName::ToggleLeft
                },
                "Show subfolders",
            )
            .toggled(view.service.show_descendants())
            .when(view.service.show_descendants(), |button| button.primary())
            .on_click(cx.listener(|view, _, _, cx| {
                view.set_descendants(!view.service.show_descendants(), cx)
            })),
        )
        .child(sliders::thumbnail(view, cx))
}
