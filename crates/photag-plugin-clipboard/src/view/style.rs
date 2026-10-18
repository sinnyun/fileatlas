use super::*;
pub(super) use gpui_kit::assets::IconName;

pub(super) use photag_ui_bridge::theme::{
    SEPARATOR as RULE, SURFACE_BASE as BACKGROUND, SURFACE_FAINT as INSET, TEXT_PRIMARY as TEXT,
    TEXT_SECONDARY as MUTED,
};
pub(super) fn icon(name: IconName, size: f32) -> gpui_kit::component::Icon {
    gpui_kit::component::Icon::new(name).size(px(size))
}
pub(super) fn kind_icon(kind: ContentKind) -> IconName {
    match kind {
        ContentKind::Text => IconName::FileText,
        ContentKind::Link => IconName::Link,
        ContentKind::Code => IconName::Code,
    }
}
pub(super) fn section() -> Div {
    photag_ui_bridge::components::section()
}
pub(super) fn action(id: &str, label: &str, primary: bool) -> Button {
    photag_ui_bridge::components::action(id.to_owned(), label.to_owned(), primary)
}
pub(super) fn small_icon(id: String, name: IconName) -> Button {
    photag_ui_bridge::components::icon_button(id, name)
}
pub(super) fn date(timestamp: i64, format: &str) -> String {
    chrono::DateTime::from_timestamp(timestamp, 0)
        .map(|time| {
            time.with_timezone(&chrono::Local)
                .format(format)
                .to_string()
        })
        .unwrap_or_else(|| "—".into())
}
