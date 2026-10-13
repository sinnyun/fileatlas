//! v5 month group header: chevron + title 15 semibold + muted 12 count.
use super::*;

pub(super) fn render(
    label: String,
    count: usize,
    expanded: f32,
    cx: &mut Context<GalleryView>,
) -> Stateful<Div> {
    let display = if label == "Unknown date" {
        photag_ui_bridge::i18n::tr(&label)
    } else {
        label
            .rsplit_once(' ')
            .map(|(month, year)| {
                photag_ui_bridge::i18n::tr_args(
                    "{month} {year}",
                    &[
                        ("month", photag_ui_bridge::i18n::tr(month)),
                        ("year", year.into()),
                    ],
                )
            })
            .unwrap_or_else(|| label.clone())
    };
    div()
        .id(format!("gallery-month-{label}"))
        .relative()
        .w_full()
        .h(px(40.))
        .h_flex()
        .items_center()
        .pl(px(28.))
        .pt(px(4.))
        .gap(px(8.))
        .child(
            div()
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
            canvas(
                |bounds, _, _| bounds,
                move |bounds, _, window, _| {
                    let center = bounds.origin + point(px(12.), px(22.));
                    let angle = expanded * std::f32::consts::FRAC_PI_2;
                    let (sin, cos) = angle.sin_cos();
                    let mut path = PathBuilder::stroke(px(1.6));
                    for (i, (x, y)) in [(-4.6 * 0.45, -4.6), (4.6 * 0.55, 0.), (-4.6 * 0.45, 4.6)]
                        .into_iter()
                        .enumerate()
                    {
                        let point = center + point(px(x * cos - y * sin), px(x * sin + y * cos));
                        if i == 0 {
                            path.move_to(point);
                        } else {
                            path.line_to(point);
                        }
                    }
                    if let Ok(path) = path.build() {
                        window.paint_path(path, rgb(theme::TEXT_SECONDARY));
                    }
                },
            )
            .absolute()
            .inset_0(),
        )
        .on_click(cx.listener(move |this, _, _, cx| {
            this.toggle_month_collapsed(label.clone(), cx);
        }))
        .cursor_pointer()
        .hover(|style| style.bg(rgba(0xffffff06)))
}
