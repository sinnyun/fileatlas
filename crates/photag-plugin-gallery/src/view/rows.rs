//! Virtual-list row geometry: month grouping and rubber-band drag state.

use super::*;

#[derive(Clone)]
pub(super) struct GalleryVisualRow {
    pub media_range: std::ops::Range<usize>,
    pub directory_range: std::ops::Range<usize>,
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
    labels: &[Option<String>],
    columns: usize,
    collapsed: &HashSet<String>,
) -> Vec<GalleryVisualRow> {
    photag_plugin_grouping::GroupingService::rows(labels, columns, collapsed)
        .into_iter()
        .map(|row| GalleryVisualRow {
            media_range: row.media_range,
            directory_range: 0..0,
            month_label: row.month_label,
            collapsed: row.collapsed,
            month_count: row.month_count,
        })
        .collect()
}

pub(super) fn build_browser_rows(
    folders: usize,
    files: usize,
    columns: usize,
) -> Vec<GalleryVisualRow> {
    (0..folders + files)
        .step_by(columns.max(1))
        .map(|start| {
            let end = (start + columns.max(1)).min(folders + files);
            GalleryVisualRow {
                directory_range: start.min(folders)..end.min(folders),
                media_range: start.saturating_sub(folders)..end.saturating_sub(folders),
                month_label: None,
                collapsed: false,
                month_count: 0,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::build_browser_rows;
    #[test]
    fn folder_and_file_ranges_share_a_row_without_shifting_file_indices() {
        let rows = build_browser_rows(3, 5, 4);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].directory_range, 0..3);
        assert_eq!(rows[0].media_range, 0..1);
        assert_eq!(rows[1].directory_range, 3..3);
        assert_eq!(rows[1].media_range, 1..5);
        assert_eq!(build_browser_rows(2, 0, 1).len(), 2);
        assert!(build_browser_rows(0, 0, 4).is_empty());
    }
}
