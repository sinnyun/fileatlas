use super::*;
pub(super) use gpui_kit::assets::IconName;

pub(super) fn rule() -> u32 {
    fileatlas_ui_bridge::theme::separator()
}
pub(super) fn background() -> u32 {
    fileatlas_ui_bridge::theme::surface_base()
}
pub(super) fn inset() -> u32 {
    fileatlas_ui_bridge::theme::surface_faint()
}
pub(super) fn text() -> u32 {
    fileatlas_ui_bridge::theme::text_primary()
}
pub(super) fn muted() -> u32 {
    fileatlas_ui_bridge::theme::text_secondary()
}
pub(super) fn icon(name: IconName, size: f32) -> gpui_kit::component::Icon {
    gpui_kit::component::Icon::new(name).size(px(size))
}
pub(super) fn kind_icon(kind: ContentKind) -> IconName {
    match kind {
        ContentKind::Text => IconName::FileText,
        ContentKind::Link => IconName::Link,
        ContentKind::Code => IconName::Code,
        ContentKind::Html => IconName::Code,
        ContentKind::Image => IconName::FileImage,
    }
}
pub(super) fn section() -> Div {
    fileatlas_ui_bridge::components::section()
}
pub(super) fn action(id: &str, label: &str, primary: bool) -> Button {
    fileatlas_ui_bridge::components::action(id.to_owned(), label.to_owned(), primary)
}
pub(super) fn small_icon(id: String, name: IconName) -> Button {
    fileatlas_ui_bridge::components::icon_button(id, name)
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
