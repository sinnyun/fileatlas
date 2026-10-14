//! Group section band; user-requested raised background distinguishes it from media rows.
use super::*;

pub(super) fn render(
    label: String,
    count: usize,
    expanded: f32,
    cx: &mut Context<GalleryView>,
) -> Stateful<Div> {
    let display = photag_ui_bridge::i18n::tr(&label);
    div()
        .id(format!("gallery-month-{label}"))
        .relative()
        .w_full()
        .h(px(40.))
        .bg(rgb(0x343e49))
        .border_1()
        .border_color(rgba(0xffffff24))
        .rounded(px(theme::ROUND_SM))
        .h_flex()
        .items_center()
        .pl(px(28.))
        .pr(px(12.))
        .pt(px(4.))
        .gap(px(8.))
        .child(
            div()
                .min_w_0()
                .overflow_hidden()
                .text_ellipsis()
                .font_family(theme::SEMIBOLD_FONT_FAMILY)
                .text_size(px(theme::FONT_SIZE_HEADING))
                .text_color(rgb(theme::TEXT_PRIMARY))
                .child(display),
        )
        .child(
            div()
                .text_size(px(theme::FONT_SIZE_SMALL))
                .pt(px(2.))
                .text_color(rgb(theme::TEXT_MUTED))
                .child("·"),
        )
        .child(
            div()
                .text_size(px(theme::FONT_SIZE_SMALL))
                .pt(px(2.))
                .text_color(rgb(theme::TEXT_MUTED))
                .child(photag_ui_bridge::i18n::tr_args(
                    "{count} items",
                    &[("count", count.to_string())],
                )),
        )
        .child(
            div()
                .absolute()
                .left(px(8.))
                .top_0()
                .h_full()
                .flex()
                .items_center()
                .child(theme::small_icon(if expanded >= 0.5 {
                    gpui_kit::assets::IconName::ChevronDown
                } else {
                    gpui_kit::assets::IconName::ChevronRight
                })),
        )
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(cx.listener(move |this, _, _, cx| {
            cx.stop_propagation();
            this.toggle_month_collapsed(label.clone(), cx);
        }))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0x404d5c)))
}
