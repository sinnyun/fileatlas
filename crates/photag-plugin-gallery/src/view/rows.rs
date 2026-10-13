//! Virtual-list row geometry: month grouping and rubber-band drag state.

use super::*;

#[derive(Clone)]
pub(super) struct GalleryVisualRow {
    pub media_range: std::ops::Range<usize>,
    pub month_label: Option<String>,
    pub collapsed: bool,
    pub month_count: usize,
}

#[derive(Clone)]
pub(super) struct GalleryRubberband {
    pub start: Point<Pixels>,
    pub end: Point<Pixels>,
    pub initial_selection: Vec<i64>,
    pub initial_anchor: Option<i64>,
    pub add: bool,
    pub clear_on_click: bool,
    pub dragged: bool,
}

pub(super) fn build_gallery_rows(
    skeletons: &[photag_plugin_database::db::queries::MediaSkeleton],
    columns: usize,
    group_by_month: bool,
    collapsed_months: &HashSet<String>,
) -> Vec<GalleryVisualRow> {
    let columns = columns.max(1);
    let mut rows = Vec::new();
    let mut start = 0;
    let mut previous_month: Option<String> = None;
    while start < skeletons.len() {
        let month_at = |index: usize| {
            if skeletons[index].effective_date <= 0 {
                return Some("Unknown date".into());
            }
            chrono::DateTime::<chrono::Utc>::from_timestamp(skeletons[index].effective_date, 0).map(
                |date| {
                    date.with_timezone(&chrono::Local)
                        .format("%B %Y")
                        .to_string()
                },
            )
        };
        let month = group_by_month.then(|| month_at(start)).flatten();
        let month_label = if group_by_month && month.is_some() && month != previous_month {
            month.clone()
        } else {
            None
        };
        let month_count = if month_label.is_some() {
            (start..skeletons.len())
                .take_while(|&index| month_at(index) == month)
                .count()
        } else {
            0
        };
        if let Some(label) = month_label.as_ref()
            && collapsed_months.contains(label)
        {
            let mut end = start + 1;
            while end < skeletons.len() && month_at(end) == month {
                end += 1;
            }
            rows.push(GalleryVisualRow {
                media_range: start..end,
                month_label: Some(label.clone()),
                collapsed: true,
                month_count,
            });
            previous_month = month;
            start = end;
            continue;
        }
        let mut end = (start + columns).min(skeletons.len());
        if group_by_month {
            end = start + 1;
            while end < skeletons.len() && end - start < columns && month_at(end) == month {
                end += 1;
            }
        }
        rows.push(GalleryVisualRow {
            media_range: start..end,
            month_label,
            collapsed: false,
            month_count,
        });
        previous_month = month;
        start = end;
    }
    rows
}
