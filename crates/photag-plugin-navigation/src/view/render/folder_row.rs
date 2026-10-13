//! Source sidebar.rs:65-110,164-178 and theme.rs:535-547.
use super::super::*;

pub(super) fn folder_row(
    id: String,
    label: String,
    count: i64,
    selected: bool,
    icon: char,
) -> Stateful<Div> {
    let mut row = div()
        .id(id)
        .relative()
        .h_flex()
        .w_full()
        .h(px(26.))
        .items_center()
        .rounded(px(4.))
        .pl(px(4.))
        .pr(px(6.))
        .gap(px(6.))
        .hover(|style| style.bg(rgba(0xffffff10)))
        .child(
            div()
                .font_family(theme::ICON_FONT_FAMILY)
                .text_size(px(12.))
                .text_color(rgb(if selected {
                    theme::accent_hover()
                } else {
                    theme::TEXT_SECONDARY
                }))
                .child(icon.to_string()),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(13.))
                .text_ellipsis()
                .child(label),
        );
    if count > 0 {
        row = row.child(
            div()
                .text_size(px(10.5))
                .text_color(rgb(theme::TEXT_MUTED))
                .child(count.to_string()),
        );
    }
    if selected {
        row = row
            .bg(rgba(theme::accent_alpha(theme::accent(), 70. / 255.)))
            .child(
                div()
                    .absolute()
                    .left(px(2.))
                    .top(px(26. * 0.225))
                    .w(px(3.))
                    .h(px(26. * 0.55))
                    .rounded(px(1.5))
                    .bg(rgb(theme::accent())),
            );
    }
    row
}
