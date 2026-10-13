use super::*;

pub(super) fn render(view: &GalleryView, _: &mut Context<GalleryView>) -> impl IntoElement {
    let mut panel = div()
        .v_flex()
        .gap_2()
        .max_h(px(420.))
        .overflow_y_scrollbar()
        .rounded_md()
        .bg(rgba(theme::modal_rgba(theme::SURFACE_MODAL)))
        .p_3()
        .child(
            div()
                .text_lg()
                .font_semibold()
                .child(photag_ui_bridge::i18n::tr("Library Statistics")),
        );
    match view.service.statistics() {
        Ok(stats) => {
            panel = panel
                .child(
                    div()
                        .text_sm()
                        .font_semibold()
                        .child(photag_ui_bridge::i18n::tr("Overview")),
                )
                .child(photag_ui_bridge::i18n::tr_args(
                    "Photos: {photos}    Videos: {videos}    Total: {total}",
                    &[
                        ("photos", stats.total_photos.to_string()),
                        ("videos", stats.total_videos.to_string()),
                        (
                            "total",
                            (stats.total_photos + stats.total_videos).to_string(),
                        ),
                    ],
                ))
                .child(photag_ui_bridge::i18n::tr_args(
                    "Total storage: {size} GB",
                    &[(
                        "size",
                        format!(
                            "{:.2}",
                            stats.total_size_bytes as f64 / (1024. * 1024. * 1024.)
                        ),
                    )],
                ))
                .child(
                    div()
                        .text_sm()
                        .font_semibold()
                        .child(photag_ui_bridge::i18n::tr("Rating Distribution")),
                );
            let max_rating = stats
                .rating_counts
                .iter()
                .copied()
                .max()
                .unwrap_or(1)
                .max(1);
            for (rating, count) in stats.rating_counts.iter().copied().enumerate() {
                let label = if rating == 0 {
                    "Unrated".to_owned()
                } else {
                    format!("{rating} star{}", if rating == 1 { "" } else { "s" })
                };
                let color = (((60 + rating as u32 * 35).min(255)) << 16)
                    | (((130 + rating as u32 * 20).min(255)) << 8)
                    | 80;
                panel = panel.child(bar_row(
                    photag_ui_bridge::i18n::tr(&label),
                    count,
                    max_rating,
                    color,
                    200.,
                ));
            }
            panel = panel.child(
                div()
                    .text_sm()
                    .font_semibold()
                    .child(photag_ui_bridge::i18n::tr("Top 10 Tags")),
            );
            if stats.top_tags.is_empty() {
                panel = panel.child(photag_ui_bridge::i18n::tr("No tags assigned yet."));
            } else {
                let max_tags = stats
                    .top_tags
                    .first()
                    .map(|(_, count)| *count)
                    .unwrap_or(1)
                    .max(1);
                for (name, count) in stats.top_tags {
                    panel = panel.child(bar_row(name, count, max_tags, theme::accent(), 180.));
                }
            }
            panel = panel.child(
                div()
                    .text_sm()
                    .font_semibold()
                    .child(photag_ui_bridge::i18n::tr("Photos per Month")),
            );
            if stats.monthly_counts.is_empty() {
                panel = panel.child(photag_ui_bridge::i18n::tr("No date information available."));
            } else {
                let max_month = stats
                    .monthly_counts
                    .iter()
                    .map(|(_, count)| *count)
                    .max()
                    .unwrap_or(1)
                    .max(1);
                for (month, count) in stats.monthly_counts {
                    panel = panel.child(bar_row(month, count, max_month, theme::accent(), 180.));
                }
            }
        }
        Err(error) => {
            panel = panel.child(
                div()
                    .text_color(rgb(theme::DANGER))
                    .child(format!("Error loading statistics: {error}")),
            );
        }
    }
    panel
}

fn bar_row(label: String, count: i64, max: i64, color: u32, width: f32) -> Div {
    let fraction = (count.max(0) as f32 / max.max(1) as f32).clamp(0., 1.);
    div()
        .h_flex()
        .items_center()
        .gap_2()
        .child(div().w(px(112.)).text_xs().child(label))
        .child(
            div()
                .w(px(width))
                .h(px(14.))
                .rounded_sm()
                .bg(rgb(theme::SURFACE_FAINT))
                .child(
                    div()
                        .w(px(width * fraction))
                        .h_full()
                        .rounded_sm()
                        .bg(rgb(color)),
                ),
        )
        .child(div().text_xs().child(count.to_string()))
}
