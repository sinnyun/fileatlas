//! Right-aligned view controls in the file_manager filter band.
use super::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
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
        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
        .child(
            button("file_manager-layout-grid", IconName::LayoutGrid, "Grid")
                .toggled(view.gallery_layout == fileatlas_plugin_workspaces::GalleryLayout::Grid)
                .on_click(cx.listener(|view, _, _, cx| {
                    view.set_layout(fileatlas_plugin_workspaces::GalleryLayout::Grid, cx)
                })),
        )
        .child(
            button(
                "file_manager-layout-waterfall",
                IconName::Image,
                "Waterfall",
            )
            .toggled(view.gallery_layout == fileatlas_plugin_workspaces::GalleryLayout::Waterfall)
            .on_click(cx.listener(|view, _, _, cx| {
                view.set_layout(fileatlas_plugin_workspaces::GalleryLayout::Waterfall, cx)
            })),
        )
        .child(
            button(
                "file_manager-layout-justified",
                IconName::LayoutGrid,
                "Justified rows",
            )
            .toggled(view.gallery_layout == fileatlas_plugin_workspaces::GalleryLayout::Justified)
            .on_click(cx.listener(|view, _, _, cx| {
                view.set_layout(fileatlas_plugin_workspaces::GalleryLayout::Justified, cx)
            })),
        )
        .child(
            button("file_manager-layout-list", IconName::List, "List")
                .toggled(view.gallery_layout == fileatlas_plugin_workspaces::GalleryLayout::List)
                .on_click(cx.listener(|view, _, _, cx| {
                    view.set_layout(fileatlas_plugin_workspaces::GalleryLayout::List, cx)
                })),
        )
        .child(display_options(view, cx))
}

fn display_options(view: &FileManagerView, cx: &mut Context<FileManagerView>) -> impl IntoElement {
    let entity = cx.entity().downgrade();
    let options = view.gallery_options.clone();
    button(
        "file_manager-display-options",
        IconName::Ellipsis,
        "Display options",
    )
    .dropdown_menu(move |mut menu, _, _| {
        for (label, key, checked) in [
            ("Show name", 0, options.show_name),
            ("Show details", 1, options.show_details),
            ("Show extension", 2, options.show_extension),
            ("List: type", 3, options.list_type),
            ("List: dimensions", 4, options.list_dimensions),
            ("List: modified", 5, options.list_modified),
            ("List: size", 6, options.list_size),
            ("List: rating", 7, options.list_rating),
        ] {
            let entity = entity.clone();
            menu = menu.item(
                PopupMenuItem::new(fileatlas_ui_bridge::i18n::tr(label))
                    .checked(checked)
                    .on_click(move |_, _, cx| {
                        let _ = entity.update(cx, |view, cx| {
                            let options = &mut view.gallery_options;
                            match key {
                                0 => options.show_name = !options.show_name,
                                1 => options.show_details = !options.show_details,
                                2 => options.show_extension = !options.show_extension,
                                3 => options.list_type = !options.list_type,
                                4 => options.list_dimensions = !options.list_dimensions,
                                5 => options.list_modified = !options.list_modified,
                                6 => options.list_size = !options.list_size,
                                _ => options.list_rating = !options.list_rating,
                            }
                            view.persist_filter_session();
                            cx.notify();
                        });
                    }),
            );
        }
        menu
    })
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
        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
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
