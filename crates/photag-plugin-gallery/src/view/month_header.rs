//! Group section band; user-requested raised background distinguishes it from media rows.
use super::*;

pub(super) fn render(
    label: String,
    count: usize,
    expanded: f32,
    scope: Option<&str>,
    cx: &mut Context<GalleryView>,
) -> Stateful<Div> {
    let display = scope
        .and_then(|scope| std::path::Path::new(&label).strip_prefix(scope).ok())
        .map(|relative| {
            if relative.as_os_str().is_empty() {
                photag_ui_bridge::i18n::tr("Current folder")
            } else {
                relative.to_string_lossy().into_owned()
            }
        })
        .unwrap_or_else(|| photag_ui_bridge::i18n::tr(&label));
    theme::separate_region(
        div()
            .relative()
            .w_full()
            .h(px(theme::GROUP_HEADER_HEIGHT))
            .bg(rgb(theme::surface_group_header()))
            .rounded(px(theme::ROUND_SM))
            .h_flex()
            .items_center()
            .pl(px(28.))
            .pr(px(12.))
            .pt(px(4.))
            .gap(px(8.)),
        theme::RegionEdge::All,
    )
    .id(format!("gallery-month-{label}"))
    .child(
        div()
            .min_w_0()
            .overflow_hidden()
            .text_ellipsis()
            .font_family(theme::SEMIBOLD_FONT_FAMILY)
            .text_size(px(theme::FONT_SIZE_HEADING))
            .text_color(rgb(theme::text_primary()))
            .child(display),
    )
    .child(
        div()
            .text_size(px(theme::FONT_SIZE_SMALL))
            .pt(px(2.))
            .text_color(rgb(theme::text_muted()))
            .child("·"),
    )
    .child(
        div()
            .text_size(px(theme::FONT_SIZE_SMALL))
            .pt(px(2.))
            .text_color(rgb(theme::text_muted()))
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
    .hover(|style| style.bg(rgb(theme::surface_group_header_hover())))
}
