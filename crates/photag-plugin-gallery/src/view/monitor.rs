//! Background polling that keeps the shared Gallery entity in sync with
//! Cordis selection services, the index revision, scan progress and popovers.

use super::*;

pub(super) fn spawn(view: &mut GalleryView, cx: &mut Context<GalleryView>) {
    let wake = view.service.wake.clone();
    view._ready_task = Some(photag_ui_bridge::refresh::listen(cx, wake, tick));
    photag_ui_bridge::refresh::poll(cx, Duration::from_millis(200), move |view, cx| {
        tick(view, cx).into()
    });
}

fn tick(view: &mut GalleryView, cx: &mut Context<GalleryView>) -> bool {
    let mut repaint = false;
    repaint |= view.poll_everything_installation();
    let search_revision = view
        .service
        .everything
        .as_ref()
        .map_or(0, |service| service.revision());
    if search_revision != view.observed_everything_revision {
        view.observed_everything_revision = search_revision;
        if !view.filter.search.trim().is_empty() {
            view.reload();
            repaint = true;
        }
    }
    let navigation_revision = view.service.navigation_revision.load(Ordering::Acquire);
    if navigation_revision != view.observed_navigation_revision {
        view.observed_navigation_revision = navigation_revision;
        view.inspector_deselected = false;
        view.selected_directory = None;
        view.selection = GallerySelection::default();
        repaint = true;
    }
    if view.service.poll_directory() {
        view.snapshot_directory_dirty = true;
    }
    if view.snapshot_read.poll() {
        if let Some(result) = view.snapshot_read.value() {
            view.snapshot = result
                .as_ref()
                .map(|snapshot| snapshot.clone())
                .map_err(|e| e.clone());
            view.visible_demand = None;
            view.row_layout_key = None;
            // Keep old geometry until render replaces it; another ready chunk
            // may need the logical scroll anchor before that paint.
            // A restore triggers a directory refresh. Its initial empty/partial
            // snapshot must not deselect the file and replace History with the
            // folder inspector before the complete file list arrives.
            if let Ok(snapshot) = &view.snapshot
                && !snapshot.browser.loading
                && snapshot.browser.error.is_none()
            {
                let valid = snapshot
                    .skeletons
                    .iter()
                    .map(|m| m.id)
                    .collect::<HashSet<_>>();
                view.selection
                    .selected_media_ids
                    .retain(|id| valid.contains(id));
                view.selection.anchor_id = view.selection.anchor_id.filter(|id| valid.contains(id));
            }
        }
        repaint = true;
    }
    // Publish a finished partial result before scheduling the next directory
    // revision. Replacing its key on every chunk can starve the first display.
    let status = view.service.directory_status();
    let publish_chunk = status.0 != view.directory_snapshot_scope
        || !status.2
        || view.directory_snapshot_started.elapsed() >= Duration::from_millis(50);
    if view.snapshot_directory_dirty && !view.snapshot_read.is_pending() && publish_chunk {
        view.snapshot_directory_dirty = false;
        view.directory_snapshot_scope = status.0;
        view.directory_snapshot_started = std::time::Instant::now();
        view.refresh_index();
        repaint = true;
    }
    if view.viewport_read.poll() {
        if view.viewport_read.key().is_some_and(|key| {
            key.0 == view.service.directory_status().0 && key.1 == view.snapshot_generation
        }) {
            if let Some(Ok(window)) = view.viewport_read.value() {
                if let Ok(snapshot) = &mut view.snapshot {
                    snapshot.window_offset = window.offset;
                    snapshot.media = window.media.clone();
                    snapshot.media_tags = window.media_tags.clone();
                }
            }
            repaint = true;
        }
    }
    if view.history_read.poll() {
        repaint = true;
    }
    repaint |= view.version_states.poll();
    if view.history_branches.poll() {
        repaint = true;
    }
    view.flush_visible_window();
    let restore_scope_ready = !view.restore_in_directory
        || view
            .snapshot
            .as_ref()
            .ok()
            .and_then(|s| s.browser.directory.as_ref())
            .is_some_and(|directory| {
                view.restored_file_paths.iter().all(|path| {
                    std::path::Path::new(path).parent() == Some(std::path::Path::new(directory))
                })
            });
    if !view.restored_file_paths.is_empty()
        && !view.service.directory_status().2
        && !view.snapshot_read.is_pending()
        && restore_scope_ready
    {
        let paths = view.restored_file_paths.clone();
        let service = view.service.clone();
        let key = (view.service.directory_status().0, paths.clone());
        view.selection_restore
            .request(key, move || service.ids_for_paths(&paths));
        if view.selection_restore.poll() {
            if let Some(Ok(ids)) = view.selection_restore.value() {
                if let Ok(snapshot) = &view.snapshot {
                    view.selection.selected_media_ids = ids
                        .iter()
                        .copied()
                        .filter(|id| snapshot.skeletons.iter().any(|s| s.id == *id))
                        .collect();
                    view.selection.anchor_id = view.selection.selected_media_ids.first().copied();
                }
            }
            view.restored_file_paths.clear();
            view.persist_filter_session();
            repaint = true;
        }
    }
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
        if view.service.browser_enabled() {
            view.service.reconcile_directory_event();
        }
        view.refresh_index();
        repaint = true;
    }
    if view.tagging.is_some() {
        let current = view
            .tagging
            .as_deref()
            .map(writeback_failure_signature)
            .unwrap_or_default();
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
    let current_scroll = view
        .pending_scroll_restore
        .unwrap_or_else(|| view.scroll_offset());
    repaint |= view.poll_dialog_reads(cx);
    repaint |= view.poll_records();
    if (current_scroll - view.last_saved_scroll).abs() >= 1.0 {
        view.persist_filter_session();
        view.last_saved_scroll = current_scroll;
    }
    repaint
}
