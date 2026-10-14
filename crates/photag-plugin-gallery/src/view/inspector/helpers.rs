//! Inspector sections and real media metadata formatting.
use super::*;

pub(super) fn basic_info_rows(
    media: &photag_plugin_database::db::queries::MediaItem,
) -> AnyElement {
    let extension = std::path::Path::new(&media.filename)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_uppercase())
        .unwrap_or_else(|| "—".to_string());
    let mut rows = div()
        .v_flex()
        .w_full()
        .pb(px(6.))
        .child(info_row("File type", extension))
        .child(info_row("File size", card::format_bytes(media.file_size)));
    if let (Some(width), Some(height)) = (media.width, media.height) {
        rows = rows.child(info_row("Image dimensions", format!("{width} × {height}")));
    }
    rows.child(info_row(
        "Date taken",
        media
            .date_taken
            .map(card::format_timestamp)
            .unwrap_or_else(|| "—".to_string()),
    ))
    .into_any_element()
}

pub(super) fn info_row(label: &str, value: String) -> Div {
    div()
        .h_flex()
        .w_full()
        .gap(px(8.))
        .py(px(3.))
        .child(
            div()
                .w(px(84.))
                .flex_shrink_0()
                .text_size(px(theme::FONT_SIZE_SMALL))
                .text_color(rgb(theme::TEXT_MUTED))
                .child(photag_ui_bridge::i18n::tr(label)),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(theme::FONT_SIZE_SMALL))
                .text_color(rgb(theme::TEXT_PRIMARY))
                .child(value),
        )
}

pub(super) fn section_header(
    key: &'static str,
    title: &str,
    expanded: bool,
    cx: &mut Context<GalleryView>,
) -> Stateful<Div> {
    div()
        .id(format!("gallery-inspector-section-{key}"))
        .h_flex()
        .w_full()
        .items_center()
        .gap(px(6.))
        .h(px(34.))
        .cursor_pointer()
        .child(
            theme::small_icon(if expanded {
                IconName::ChevronDown
            } else {
                IconName::ChevronRight
            })
            .text_color(rgb(theme::TEXT_MUTED)),
        )
        .child(
            div()
                .text_size(px(theme::FONT_SIZE_BODY))
                .text_color(rgb(theme::TEXT_PRIMARY))
                .child(photag_ui_bridge::i18n::tr(title)),
        )
        .on_click(cx.listener(move |this, _, _, cx| this.toggle_inspector_section(key, cx)))
}

pub(super) fn inspector_empty() -> AnyElement {
    div()
        .size_full()
        .v_flex()
        .items_center()
        .justify_center()
        .gap(px(8.))
        .child(
            theme::icon(IconName::Info)
                .size(px(24.))
                .text_color(rgb(theme::TEXT_MUTED)),
        )
        .child(
            div()
                .text_size(px(theme::FONT_SIZE_SMALL))
                .text_color(rgb(theme::TEXT_MUTED))
                .child(photag_ui_bridge::i18n::tr("No file selected")),
        )
        .into_any_element()
}

pub(super) fn inspector_message(message: &str) -> AnyElement {
    div()
        .w_full()
        .py(px(8.))
        .text_size(px(theme::FONT_SIZE_SMALL))
        .text_color(rgb(theme::TEXT_SECONDARY))
        .child(photag_ui_bridge::i18n::tr(message))
        .into_any_element()
}

pub(super) fn inspector_divider() -> Div {
    div()
        .h(px(1.))
        .w_full()
        .my(px(4.))
        .bg(rgb(theme::SEPARATOR))
}

pub(super) fn parse_tag_color(value: Option<&str>) -> u32 {
    let [red, green, blue] = value
        .and_then(theme::parse_hex_color)
        .unwrap_or([120, 120, 130]);
    (u32::from(red) << 16) | (u32::from(green) << 8) | u32::from(blue)
}
