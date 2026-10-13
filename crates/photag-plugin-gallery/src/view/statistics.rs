use super::*;
use photag_plugin_database::db::queries::MediaQuery;

pub(super) fn render(view: &GalleryView, _: &mut Context<GalleryView>) -> impl IntoElement {
    let mut panel = div()
        .v_flex()
        .gap_3()
        .w(px(800.))
        .max_h(px(620.))
        .overflow_y_scrollbar()
        .rounded_md()
        .bg(rgba(theme::modal_rgba(theme::SURFACE_MODAL)))
        .p_4()
        .child(
            div()
                .text_lg()
                .font_semibold()
                .child(photag_ui_bridge::i18n::tr("Library Statistics")),
        );
    match view.service.statistics() {
        Ok(stats) => {
            let audio_count = view
                .service
                .load(MediaQuery {
                    media_type: Some("audio".to_owned()),
                    ..MediaQuery::default()
                })
                .map(|snapshot| {
                    snapshot
                        .skeletons
                        .iter()
                        .filter(|media| media.is_audio)
                        .count() as i64
                });
            let total = audio_count
                .as_ref()
                .ok()
                .map(|audio| stats.total_photos + stats.total_videos + *audio);
            let audio_value = audio_count
                .as_ref()
                .map(|count| count.to_string())
                .unwrap_or_else(|_| "—".to_owned());
            let total_value = total
                .map(|count| count.to_string())
                .unwrap_or_else(|| "—".to_owned());
            panel = panel.child(
                div()
                    .h_flex()
                    .gap_2()
                    .child(metric_card("All files", total_value))
                    .child(metric_card("Photos", stats.total_photos.to_string()))
                    .child(metric_card("Videos", stats.total_videos.to_string()))
                    .child(metric_card("Audio", audio_value)),
            );
            if let Err(error) = &audio_count {
                panel = panel.child(
                    div()
                        .text_xs()
                        .text_color(rgb(theme::DANGER))
                        .child(format!("Audio count unavailable: {error}")),
                );
            }

            let mut rating_panel = stat_section("Rating Distribution");
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
                rating_panel = rating_panel.child(bar_row(
                    photag_ui_bridge::i18n::tr(&label),
                    count,
                    max_rating,
                    color,
                    130.,
                ));
            }
            let mut storage_panel = stat_section("Storage space");
            storage_panel = storage_panel
                .child(
                    div()
                        .text_xl()
                        .font_semibold()
                        .child(format_storage(stats.total_size_bytes)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(theme::TEXT_SECONDARY))
                        .child(photag_ui_bridge::i18n::tr(
                            "Total size across indexed media.",
                        )),
                );
            panel = panel.child(
                div()
                    .h_flex()
                    .items_start()
                    .gap_3()
                    .child(storage_panel.flex_1())
                    .child(rating_panel.flex_1()),
            );

            let mut tags_panel = stat_section("Top 10 Tags");
            if stats.top_tags.is_empty() {
                tags_panel = tags_panel.child(photag_ui_bridge::i18n::tr("No tags assigned yet."));
            } else {
                let max_tags = stats
                    .top_tags
                    .first()
                    .map(|(_, count)| *count)
                    .unwrap_or(1)
                    .max(1);
                for (name, count) in stats.top_tags {
                    tags_panel =
                        tags_panel.child(bar_row(name, count, max_tags, theme::accent(), 130.));
                }
            }
            let mut months_panel = stat_section("Items per month");
            if stats.monthly_counts.is_empty() {
                months_panel = months_panel
                    .child(photag_ui_bridge::i18n::tr("No date information available."));
            } else {
                let max_month = stats
                    .monthly_counts
                    .iter()
                    .map(|(_, count)| *count)
                    .max()
                    .unwrap_or(1)
                    .max(1);
                for (month, count) in stats.monthly_counts {
                    months_panel =
                        months_panel.child(bar_row(month, count, max_month, theme::accent(), 130.));
                }
            }
            panel = panel.child(
                div()
                    .h_flex()
                    .items_start()
                    .gap_3()
                    .child(tags_panel.flex_1())
                    .child(months_panel.flex_1()),
            );
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

fn metric_card(label: &'static str, value: String) -> Div {
    div()
        .v_flex()
        .flex_1()
        .min_w_0()
        .gap_1()
        .p_3()
        .rounded_md()
        .bg(rgba(theme::base_rgba(theme::SURFACE_BASE)))
        .child(
            div()
                .text_xs()
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child(photag_ui_bridge::i18n::tr(label)),
        )
        .child(div().text_xl().font_semibold().child(value))
}

fn stat_section(title: &'static str) -> Div {
    div()
        .v_flex()
        .flex_1()
        .min_w_0()
        .gap_2()
        .p_3()
        .rounded_md()
        .bg(rgba(theme::base_rgba(theme::SURFACE_BASE)))
        .child(
            div()
                .text_sm()
                .font_semibold()
                .child(photag_ui_bridge::i18n::tr(title)),
        )
}

fn format_storage(bytes: u64) -> String {
    const KB: f64 = 1024.;
    const MB: f64 = 1024. * KB;
    const GB: f64 = 1024. * MB;
    if bytes as f64 >= GB {
        format!("{:.1} GB", bytes as f64 / GB)
    } else if bytes as f64 >= MB {
        format!("{:.0} MB", bytes as f64 / MB)
    } else if bytes as f64 >= KB {
        format!("{:.0} KB", bytes as f64 / KB)
    } else {
        format!("{bytes} B")
    }
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
