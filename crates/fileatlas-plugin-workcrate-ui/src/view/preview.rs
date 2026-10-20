use super::*;

pub(super) fn page_preview(card: &BoardCard, width: f32) -> gpui::AnyElement {
    let height = width
        * (card.board.source.settings.page_height / card.board.source.settings.page_width) as f32;
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
                .bg(rgb(theme::surface_faint()))
                .flex()
                .items_center()
                .justify_center()
                .child("页面预览")
                .into_any_element()
        })
}
pub(super) fn physical_label(board: &WorkCrateBoard) -> String {
    if !board.info.size_label.is_empty() {
        return board.info.size_label.clone();
    }
    board
        .info
        .physical_size_cm
        .map(|[w, h]| format!("{w} × {h} cm"))
        .unwrap_or_else(|| "未填写尺寸标记".into())
}
pub(super) fn pixel_label(board: &WorkCrateBoard) -> String {
    board
        .info
        .pixel_size
        .map(|[w, h]| format!("{w} × {h} px"))
        .unwrap_or_else(|| "等待首张图片".into())
}
pub(super) fn figcraft_fit_scale(w: f64, h: f64, pw: f64, ph: f64) -> f64 {
    (((w - 128.).max(50.) / pw.max(1e-6)).min((h - 128.).max(50.) / ph.max(1e-6))).clamp(0.02, 1.0)
}
