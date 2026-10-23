//! Inspector sections and real media metadata formatting.
use super::*;
use fileatlas_ui_bridge::components::WorkbenchPaint as _;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;

pub(super) fn basic_info_rows(media: &fileatlas_plugin_database::MediaItem) -> AnyElement {
    let extension = std::path::Path::new(&media.filename)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_uppercase())
        .unwrap_or_else(|| "—".to_string());
    let mut rows = div()
        .v_flex()
        .w_full()
        .min_w_0()
        .pb(px(fileatlas_ui_bridge::theme::space_standard()))
        .child(info_row("File name", media.filename.clone()))
        .child(info_row("File type", extension))
        .child(info_row("File size", card::format_bytes(media.file_size)))
        .child(info_row(
            "Modified",
            card::format_timestamp(media.modified_at),
        ));
    if let (Some(width), Some(height)) = (media.width, media.height) {
        rows = rows.child(info_row("Image dimensions", format!("{width} × {height}")));
    }
    if let Some(timestamp) = media.date_taken {
        rows = rows.child(info_row("Date taken", card::format_timestamp(timestamp)));
    }
    rows.into_any_element()
}

pub(super) fn info_row(label: &str, value: String) -> Div {
    div()
        .h_flex()
        .w_full()
        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
        .py(px(fileatlas_ui_bridge::theme::space_standard()))
        .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Bottom)
        .child(
            div()
                .w(px(84.))
                .flex_shrink_0()
                .workbench_assistant()
                .child(fileatlas_ui_bridge::i18n::tr(label)),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_right()
                .workbench_body()
                .child(value),
        )
}

pub(super) fn section_header(
    key: &'static str,
    title: &str,
    expanded: bool,
    cx: &mut Context<FileManagerView>,
) -> Stateful<Div> {
    div()
        .id(format!("file_manager-inspector-section-{key}"))
        .h_flex()
        .w_full()
        .items_center()
        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
        .h(px(theme::ui_height(theme::CONTROL_HEIGHT_TOOLBAR)))
        .cursor_pointer()
        .child(
            theme::small_icon(if expanded {
                IconName::ChevronDown
            } else {
                IconName::ChevronRight
            })
            .workbench_foreground(fileatlas_ui_bridge::components::ForegroundRole::Muted),
        )
        .child(
            div()
                .workbench_title()
                .child(fileatlas_ui_bridge::i18n::tr(title)),
        )
        .on_click(cx.listener(move |this, _, _, cx| this.toggle_inspector_section(key, cx)))
}

pub(super) fn inspector_empty() -> AnyElement {
    div()
        .size_full()
        .v_flex()
        .items_center()
        .justify_center()
        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
        .child(
            theme::icon(IconName::Info)
                .size(px(24.))
                .workbench_foreground(fileatlas_ui_bridge::components::ForegroundRole::Muted),
        )
        .child(
            div()
                .workbench_assistant()
                .child(fileatlas_ui_bridge::i18n::tr("No file selected")),
        )
        .into_any_element()
}

pub(super) fn inspector_message(message: &str) -> AnyElement {
    div()
        .w_full()
        .py(px(fileatlas_ui_bridge::theme::space_standard()))
        .workbench_assistant()
        .child(fileatlas_ui_bridge::i18n::tr(message))
        .into_any_element()
}

pub(super) fn inspector_divider() -> Div {
    div()
        .h(px(1.))
        .w_full()
        .flex_shrink_0()
        .my(px(fileatlas_ui_bridge::theme::space_standard()))
        .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Transparent(
            theme::separator_light(),
        ))
}

pub(super) fn section_title(title: &str) -> Div {
    fileatlas_ui_bridge::components::section_heading(fileatlas_ui_bridge::i18n::tr(title))
        .h_flex()
        .w_full()
        .items_center()
        .justify_between()
        .py(px(theme::space_standard()))
}

pub(super) fn well() -> Div {
    fileatlas_ui_bridge::components::section()
        .v_flex()
        .w_full()
        .min_w_0()
        .p(px(theme::space_expanded()))
        .gap(px(theme::space_standard()))
}

pub(super) fn detail_row(icon: IconName, label: &str, value: String) -> Div {
    div()
        .h_flex()
        .w_full()
        .min_w_0()
        .items_center()
        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
        .py(px(fileatlas_ui_bridge::theme::space_standard()))
        .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Bottom)
        .child(
            theme::icon(icon)
                .size(px(16.))
                .flex_shrink_0()
                .workbench_foreground(fileatlas_ui_bridge::components::ForegroundRole::Secondary),
        )
        .child(
            div()
                .workbench_body()
                .child(fileatlas_ui_bridge::i18n::tr(label)),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_right()
                .workbench_body()
                .child(value),
        )
}

pub(super) fn parse_tag_color(value: Option<&str>) -> u32 {
    let [red, green, blue] = value
        .and_then(theme::parse_hex_color)
        .unwrap_or([120, 120, 130]);
    (u32::from(red) << 16) | (u32::from(green) << 8) | u32::from(blue)
}

/// Windows namespace prefixes are useful for IO but add noise to visible paths.
pub(super) use fileatlas_ui_bridge::display_path;

/// The reader supplies friendly fields followed by raw EXIF fields. Collapse
/// exact aliases for display while preserving distinct values and unknown tags.
pub(super) fn display_exif_fields(fields: &[(String, String)]) -> Vec<(String, String)> {
    let mut seen = HashSet::new();
    fields
        .iter()
        .filter_map(|(label, value)| {
            let label = match label.as_str() {
                "Make" => "Camera make",
                "Model" => "Camera model",
                "LensModel" => "Lens",
                "FocalLength" => "Focal length",
                "FNumber" => "Aperture",
                "Shutter" | "ExposureTime" => "Shutter speed",
                "PhotographicSensitivity" => "ISO",
                "DateTime" => "Metadata date",
                "DateTimeOriginal" => "Date taken",
                label => label,
            }
            .to_string();
            let value = value.trim_matches('"').to_string();
            seen.insert((label.clone(), value.clone()))
                .then_some((label, value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn exif_display_removes_alias_duplicates_without_losing_distinct_metadata() {
        let input = [
            ("Camera model", "\"Canon EOS\""),
            ("Model", "\"Canon EOS\""),
            ("Model", "Other"),
            ("Custom", "\"Canon EOS\""),
            ("Shutter", "1/200 s"),
            ("ExposureTime", "1/200 s"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect::<Vec<_>>();
        let rows = super::display_exif_fields(&input);
        assert_eq!(rows.len(), 4);
        assert!(rows.contains(&("Camera model".into(), "Other".into())));
        assert!(rows.contains(&("Custom".into(), "Canon EOS".into())));
        assert!(rows.contains(&("Shutter speed".into(), "1/200 s".into())));
    }
}
