//! Background polling that keeps the shared Gallery entity in sync with
//! Cordis selection services, the index revision, scan progress and popovers.

use super::*;

pub(super) fn spawn(view: &mut GalleryView, cx: &mut Context<GalleryView>) {
    let scroll_for_monitor = view.scroll.clone();
    photag_ui_bridge::refresh::poll(cx, Duration::from_millis(200), move |view, cx| {
        tick(view, &scroll_for_monitor, cx).into()
    });
}

fn tick(view: &mut GalleryView, scroll: &ListState, cx: &mut Context<GalleryView>) -> bool {
    let mut repaint = false;
    let popover = view.surfaces.value("gallery.popover");
    if view.previous_popover.changed(popover) {
        repaint = true;
    }
    let inspector_tab = view.surfaces.value("gallery.inspector.tab");
    if view.previous_inspector_tab.changed(inspector_tab) {
        repaint = true;
    }
    let progress = view
        .service
        .scan_progress()
        .map(|progress| (progress.scanned, progress.total, progress.phase));
    if view.previous_scan_progress.changed(progress) {
        repaint = true;
    }
    if view.service.has_ui_request() {
        repaint = true;
    }
    let selected = view
        .selected_folder
        .0
        .lock()
        .map(|selected| *selected)
        .unwrap_or(None);
    let selected_tags = view
        .selected_tag
        .0
        .lock()
        .map(|selected| selected.clone())
        .unwrap_or_default();
    let selected_path = view
        .selected_path
        .0
        .lock()
        .map(|selected| selected.clone())
        .unwrap_or(None);
    let changed = view.filter.folder_id != selected
        || view.filter.selected_tag_ids != selected_tags.included
        || view.filter.excluded_tag_ids != selected_tags.excluded
        || view.filter.path_prefix != selected_path;
    let revision = view.index_revision.0.load(Ordering::Acquire);
    let index_changed = revision != view.observed_index_revision;
    view.observed_index_revision = revision;
    if changed {
        view.filter.folder_id = selected;
        view.filter.selected_tag_ids = selected_tags.included;
        view.filter.excluded_tag_ids = selected_tags.excluded;
        view.filter.path_prefix = selected_path;
        view.reload();
        repaint = true;
    } else if index_changed {
        view.refresh_index();
        repaint = true;
    }
    if view.tagging.is_some() {
        let current = writeback_failure_signature();
        if current != view.writeback_failure_signature {
            view.writeback_failure_signature = current;
            repaint = true;
        }
    }
    if let Some(group_by_month) = view
        .settings
        .as_ref()
        .and_then(|settings| settings.snapshot().ok())
        .map(|settings| settings.show_date_dividers)
        && matches!(
            view.grouping_mode,
            photag_plugin_grouping::GroupingMode::Month
                | photag_plugin_grouping::GroupingMode::None
        )
        && group_by_month != (view.grouping_mode == photag_plugin_grouping::GroupingMode::Month)
    {
        view.set_month_grouping(group_by_month, cx);
        repaint = true;
    }
    let current_scroll = (-f32::from(scroll.scroll_px_offset_for_scrollbar().y)).max(0.);
    repaint |= view.poll_dialog_reads(cx);
    if (current_scroll - view.last_saved_scroll).abs() >= 1.0 {
        view.persist_filter_session();
        view.last_saved_scroll = current_scroll;
    }
    repaint
}
