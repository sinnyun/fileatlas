//! Virtual-list row geometry: month grouping and rubber-band drag state.

use super::*;

#[derive(Clone)]
pub(super) struct FileManagerVisualRow {
    pub media_range: std::ops::Range<usize>,
    pub directory_range: std::ops::Range<usize>,
    pub month_label: Option<String>,
    pub collapsed: bool,
    pub month_count: usize,
    /// Group identity is retained on continuation rows, unlike the title.
    pub group_label: Option<String>,
    pub group_end: bool,
}

#[derive(Clone)]
pub(super) struct FileManagerRubberband {
    pub start: Point<Pixels>,
    pub end: Point<Pixels>,
    pub initial_selection: Vec<i64>,
    pub initial_anchor: Option<i64>,
    pub add: bool,
    pub clear_on_click: bool,
    pub dragged: bool,
}

pub(super) fn build_file_manager_rows(
    labels: &[Option<String>],
    columns: usize,
    collapsed: &HashSet<String>,
) -> Vec<FileManagerVisualRow> {
    fileatlas_plugin_grouping::GroupingService::rows(labels, columns, collapsed)
        .into_iter()
        .map(|row| {
            let group_label = labels.get(row.media_range.start).cloned().flatten();
            let group_end = labels.get(row.media_range.end) != Some(&group_label);
            FileManagerVisualRow {
                media_range: row.media_range,
                directory_range: 0..0,
                month_label: row.month_label,
                collapsed: row.collapsed,
                month_count: row.month_count,
                group_label,
                group_end,
            }
        })
        .collect()
}

pub(super) fn build_browser_rows(
    folders: usize,
    files: usize,
    columns: usize,
) -> Vec<FileManagerVisualRow> {
    (0..folders + files)
        .step_by(columns.max(1))
        .map(|start| {
            let end = (start + columns.max(1)).min(folders + files);
            FileManagerVisualRow {
                directory_range: start.min(folders)..end.min(folders),
                media_range: start.saturating_sub(folders)..end.saturating_sub(folders),
                month_label: None,
                collapsed: false,
                month_count: 0,
                group_label: None,
                group_end: false,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{build_browser_rows, build_file_manager_rows};
    use std::collections::HashSet;

    #[test]
    fn continuation_and_collapsed_rows_keep_their_group_frame_identity() {
        let labels =
            ["alpha", "alpha", "alpha", "beta", "beta"].map(|label| Some(label.to_string()));
        let rows = build_file_manager_rows(&labels, 2, &HashSet::new());
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[1].group_label.as_deref(), Some("alpha"));
        assert_eq!(rows[1].month_label, None);
        assert!(!rows[0].group_end);
        assert!(rows[1].group_end);
        assert!(rows[2].group_end);
        let rows = build_file_manager_rows(&labels, 2, &HashSet::from(["alpha".into()]));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].group_label.as_deref(), Some("alpha"));
        assert_eq!(rows[0].media_range, 0..3);
        assert!(rows[0].collapsed && rows[0].group_end);
        assert_eq!(rows[1].group_label.as_deref(), Some("beta"));
    }
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
