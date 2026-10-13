//! v5 empty states for the gallery grid (mockup 08): library-empty with an
//! "Add folder" action and filtered no-results with "Clear filters".

use gpui_kit::assets::IconName;
use gpui_kit::component::Icon;

use super::*;
use gpui_kit::component::button::ButtonVariants;

pub(super) fn render(no_results: bool, cx: &mut Context<GalleryView>) -> AnyElement {
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
        Icon::new(IconName::Search)
            .size(px(40.))
            .text_color(rgb(theme::TEXT_MUTED))
            .into_any_element()
    } else {
        div()
            .size(px(72.))
            .rounded(px(theme::ROUND_XL))
            .bg(rgb(theme::SURFACE_FAINT))
            .flex()
            .items_center()
            .justify_center()
            .child(
                Icon::new(IconName::Image)
                    .size(px(40.))
                    .text_color(rgb(theme::TEXT_MUTED)),
            )
            .into_any_element()
    };
    let action = if no_results {
        Button::new("gallery-empty-clear-filters")
            .ghost()
            .label(photag_ui_bridge::i18n::tr("Clear filters"))
            .on_click(cx.listener(|this, _, window, cx| {
                this.clear_filters(window, cx);
            }))
            .into_any_element()
    } else {
        Button::new("gallery-empty-add-folder")
            .primary()
            .label(photag_ui_bridge::i18n::tr("Add folder"))
            .icon(IconName::FolderPlus)
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
                        .mt(px(16.))
                        .font_family(theme::SEMIBOLD_FONT_FAMILY)
                        .text_size(px(theme::FONT_SIZE_HEADING))
                        .text_color(rgb(theme::TEXT_PRIMARY))
                        .child(photag_ui_bridge::i18n::tr(title)),
                )
                .child(
                    div()
                        .mt(px(6.))
                        .text_size(px(theme::FONT_SIZE_SMALL))
                        .text_color(rgb(theme::TEXT_SECONDARY))
                        .child(photag_ui_bridge::i18n::tr(body)),
                )
                .child(div().mt(px(20.)).child(action)),
        )
        .into_any_element()
}
