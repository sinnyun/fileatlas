use super::*;
use fileatlas_ui_bridge::components::WorkbenchPaint as _;

pub(super) fn page_preview(card: &BoardCard, width: f32) -> gpui::AnyElement {
    let height = width * (card.board.height_pt / card.board.width_pt) as f32;
    card.image
        .clone()
        .map(|image| {
            img(image)
                .w(px(width))
                .h(px(height))
                .object_fit(ObjectFit::Contain)
                .into_any_element()
        })
        .unwrap_or_else(|| {
            div()
                .w(px(width))
                .h(px(height))
                .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Faint)
                .flex()
                .items_center()
                .justify_center()
                .child(if card.preview_checked {
                    "暂无本地预览，请同步工程"
                } else {
                    "正在读取本地预览…"
                })
                .into_any_element()
        })
}
