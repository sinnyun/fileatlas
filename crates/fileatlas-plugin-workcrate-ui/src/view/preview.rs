use super::*;

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
                .bg(rgb(theme::surface_faint()))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    card.error
                        .clone()
                        .unwrap_or_else(|| "正在生成官方预览…".into()),
                )
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
        .unwrap_or_else(|| "未记录图片像素".into())
}
