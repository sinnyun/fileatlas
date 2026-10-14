//! Background polling that keeps the shared Gallery entity in sync with
//! Cordis selection services, the index revision, scan progress and popovers.

use super::*;

pub(super) fn spawn(view: &mut GalleryView, cx: &mut Context<GalleryView>) {
    let scroll_for_monitor = view.scroll.clone();
    cx.spawn(async move |this, cx| {
        loop {
            cx.background_executor()
                .timer(Duration::from_millis(200))
                .await;
            let monitor = scroll_for_monitor.clone();
            let changed = this
                .update(cx, |view, cx| tick(view, &monitor, cx))
                .unwrap_or(false);
            if !changed {
                break;
            }
        }
    })
    .detach();
}

fn tick(view: &mut GalleryView, scroll: &ListState, cx: &mut Context<GalleryView>) -> bool {
    let popover = view.surfaces.value("gallery.popover");
    if view.previous_popover != popover {
        view.previous_popover = popover;
        cx.notify();
    }
    let progress = view
        .service
        .scan_progress()
        .map(|progress| (progress.scanned, progress.total, progress.phase));
    if view.previous_scan_progress != progress {
        view.previous_scan_progress = progress;
        cx.notify();
    }
    if view.service.has_ui_request() {
        cx.notify();
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
    if changed || index_changed {
        view.filter.folder_id = selected;
        view.filter.selected_tag_ids = selected_tags.included;
        view.filter.excluded_tag_ids = selected_tags.excluded;
        view.filter.path_prefix = selected_path;
        view.reload();
        cx.notify();
    }
    if view.tagging.is_some() {
        let current = writeback_failure_signature();
        if current != view.writeback_failure_signature {
            view.writeback_failure_signature = current;
            cx.notify();
        }
    }
    if let Some(group_by_month) = view
        .settings
        .as_ref()
        .and_then(|settings| settings.snapshot().ok())
        .map(|settings| settings.show_date_dividers)
        && group_by_month != view.group_by_month
    {
        view.group_by_month = group_by_month;
        view.row_layout_key = None;
        view.persist_filter_session();
        cx.notify();
    }
    let current_scroll = (-f32::from(scroll.scroll_px_offset_for_scrollbar().y)).max(0.);
    if (current_scroll - view.last_saved_scroll).abs() >= 1.0 {
        view.persist_filter_session();
        view.last_saved_scroll = current_scroll;
    }
    true
}
