//! FileManager filter scopes and shared surface commands.
use super::*;

impl FileManagerView {
    pub(super) fn stage_media_ids(&self, ids: Vec<i64>) {
        let service = Arc::clone(&self.service);
        let cordis = self.cordis.clone();
        self.runtime.spawn(async move {
            let paths =
                match tokio::task::spawn_blocking(move || service.media_paths_by_ids(&ids)).await {
                    Ok(Ok(paths)) => paths,
                    Ok(Err(error)) => {
                        tracing::warn!(%error, "could not resolve selected files for staging");
                        return;
                    }
                    Err(error) => {
                        tracing::warn!(%error, "selected path lookup worker failed");
                        return;
                    }
                };
            let paths = paths
                .into_iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect();
            if let Err(error) = cordis
                .emit::<fileatlas_contracts::FileStagingAddRequested>(Routing::Unscoped, paths)
                .await
            {
                tracing::debug!(%error, "file staging feature is unavailable");
            }
        });
    }

    /// Dispatch a plugin command through Cordis, matching the shell's
    /// UiCommandInvoked contract so feature plugins stay decoupled.
    pub(super) fn dispatch_command(&self, command_id: &str) {
        let cordis = self.cordis.clone();
        let command_id = command_id.to_owned();
        self.runtime.spawn(async move {
            if let Err(error) = cordis
                .emit::<UiCommandInvoked>(Routing::Unscoped, command_id)
                .await
            {
                tracing::error!("command dispatch failed: {error}");
            }
        });
    }

    /// True when any filter-level control narrows the result set. Folder,
    /// path and tag scopes come from navigation and are not included.
    pub(super) fn filters_active(&self) -> bool {
        !self.filter.search.is_empty()
            || self.filter.media_type != MediaTypeFilter::All
            || self.filter.date_from.is_some()
            || self.filter.date_to.is_some()
            || self.filter.rating_min != 0
            || self.filter.rating_max != 5
            || !self.filter.extensions.is_empty()
            || !self.filter.excluded_extensions.is_empty()
    }

    /// True when navigation scopes the view to a folder, path or tag.
    pub(super) fn scope_active(&self) -> bool {
        self.filter.folder_id.is_some()
            || self.filter.path_prefix.is_some()
            || !self.filter.selected_tag_ids.is_empty()
            || !self.filter.excluded_tag_ids.is_empty()
    }

    /// Replace the shared tag include/exclude scope. Emitted through Cordis so
    /// the same observer that serves the sidebar keeps folder/path clearing
    /// and the poll-loop sync consistent.
    pub(super) fn emit_tag_filter_selection(&self, included: Vec<i64>, excluded: Vec<i64>) {
        let cordis = self.cordis.clone();
        let selection = fileatlas_contracts::TagFilterSelection { included, excluded };
        self.runtime.spawn(async move {
            if let Err(error) = cordis
                .emit::<fileatlas_contracts::TagFilterSelectionChanged>(
                    Routing::Unscoped,
                    selection,
                )
                .await
            {
                tracing::error!("tag filter dispatch failed: {error}");
            }
        });
    }

    /// Toggle one tag in the shared include/exclude tag scope.
    pub(super) fn toggle_tag_filter(&mut self, tag_id: i64, exclude: bool, cx: &mut Context<Self>) {
        let (mut included, mut excluded) = (
            self.filter.selected_tag_ids.clone(),
            self.filter.excluded_tag_ids.clone(),
        );
        let (target, other) = if exclude {
            (&mut excluded, &mut included)
        } else {
            (&mut included, &mut excluded)
        };
        other.retain(|id| *id != tag_id);
        if target.contains(&tag_id) {
            target.retain(|id| *id != tag_id);
        } else {
            target.push(tag_id);
        }
        self.emit_tag_filter_selection(included, excluded);
        cx.notify();
    }

    pub(super) fn set_month_grouping(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.set_grouping(
            if enabled {
                fileatlas_plugin_grouping::GroupingMode::Month
            } else {
                fileatlas_plugin_grouping::GroupingMode::None
            },
            cx,
        );
    }

    #[tracing::instrument(skip_all, fields(mode=?mode))]
    pub(super) fn set_grouping(
        &mut self,
        mode: fileatlas_plugin_grouping::GroupingMode,
        cx: &mut Context<Self>,
    ) {
        self.grouping_mode = mode;
        self.collapsed_months.clear();
        self.row_layout_key = None;
        if let Some(settings) = &self.settings {
            if let Err(error) = settings.update(|s| {
                s.show_date_dividers = mode == fileatlas_plugin_grouping::GroupingMode::Month
            }) {
                self.operation_status = Some(format!("Grouping preference save failed: {error}"));
            }
        }
        self.refresh_index();
        self.pending_scroll_restore = None;
        cx.notify();
    }

    pub(super) fn toggle_month_collapsed(&mut self, month: String, cx: &mut Context<Self>) {
        if !self.collapsed_months.insert(month.clone()) {
            self.collapsed_months.remove(&month);
        }
        self.row_layout_key = None;
        self.persist_filter_session();
        cx.notify();
    }
}
