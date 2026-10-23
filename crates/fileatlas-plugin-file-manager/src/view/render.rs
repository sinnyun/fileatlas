use super::*;
use fileatlas_ui_bridge::components::WorkbenchPaint as _;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
use gpui_kit::prelude::FluentBuilder as _;

pub(super) const GRID_GAP: f32 = 16.;
pub(super) const GRID_PADDING: f32 = 16.;

impl Render for FileManagerView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _ = self.surfaces.set_value(
            &format!("file_manager.column.{}.group-frames", self.column_index),
            "{}",
        );
        if !cx.has_active_drag() {
            let _ = self
                .surfaces
                .set_value(&super::group_frame::hover_key(self.column_index), "");
        }
        if let Some(message) = self.quicklook_notice.take() {
            use gpui_kit::component::WindowExt as _;
            window.open_alert_dialog(cx, move |dialog, _, _| {
                dialog
                    .title("QuickLook")
                    .description(message.clone())
                    .ok_text(fileatlas_ui_bridge::i18n::tr("OK"))
            });
        }
        if let Some(request) = self.service.take_ui_request() {
            match request {
                crate::FileManagerUiRequest::InspectorTagName(name) => {
                    self.inspector_tag_name
                        .update(cx, |input, cx| input.set_value(name, window, cx));
                }
                crate::FileManagerUiRequest::HistoryChanged(media_id) => {
                    self.history_generation = self.history_generation.wrapping_add(1);
                    if self.selection.selected_media_ids.first() == Some(&media_id) {
                        self.history_error = None;
                    }
                }
                crate::FileManagerUiRequest::HistorySelect(media_id, version_id) => {
                    if self.selection.selected_media_ids.first() == Some(&media_id) {
                        self.history_selected = Some((media_id, version_id.clone()));
                        let _ = self
                            .surfaces
                            .set_value("file_manager.inspector.history-selected", &version_id);
                    }
                }
                crate::FileManagerUiRequest::LocateFile(id) => self.locate_file(id, cx),
                crate::FileManagerUiRequest::OpenDirectory(path) => self.open_directory(path, cx),
                crate::FileManagerUiRequest::DirectoryUp => self.directory_up(cx),
                crate::FileManagerUiRequest::Refresh => {
                    self.service.invalidate_directory();
                    self.refresh_index();
                }
                crate::FileManagerUiRequest::CancelDirectory => self.service.cancel_directory(),
                crate::FileManagerUiRequest::Thumbnails(value) => {
                    self.service.set_thumbnails_enabled(value);
                    self.refresh_index();
                    self.persist_filter_session();
                }
                crate::FileManagerUiRequest::ShowDescendants(enabled) => {
                    self.set_descendants(enabled, cx)
                }
                crate::FileManagerUiRequest::Layout(list) => self.set_layout(list, cx),
                crate::FileManagerUiRequest::RecordCandidate(id, path) => {
                    self.record_candidates.insert(id, path);
                }
                crate::FileManagerUiRequest::RelinkRecord(id) => self.relink_record(id, cx),
                crate::FileManagerUiRequest::DeleteMissingRecords => {
                    if self.confirm_delete_missing {
                        self.delete_missing_records(cx)
                    } else {
                        self.confirm_delete_missing = true;
                    }
                }
                crate::FileManagerUiRequest::MissingRecords => {
                    self.show_records = true;
                    self.record_read_revision += 1;
                    self.records_page = 0;
                }
                crate::FileManagerUiRequest::Statistics => self.show_statistics = true,
                crate::FileManagerUiRequest::BulkActions => {
                    bulk_actions::reset_inputs(self, window, cx);
                    self.bulk_action = Some(BulkActionKind::Delete);
                    self.bulk_source = BulkActionSource::WholeLibrary;
                    self.confirm_bulk_action = false;
                    self.confirm_bulk_action_ids = None;
                }
                crate::FileManagerUiRequest::ConfigureBulk(action, source) => {
                    bulk_actions::reset_inputs(self, window, cx);
                    self.bulk_action = Some(action);
                    self.bulk_source = source;
                    self.confirm_bulk_action = false;
                    self.confirm_bulk_action_ids = None;
                }
                crate::FileManagerUiRequest::BulkField(index, value) => {
                    if let Some(input) = self.bulk_inputs.get(index) {
                        input.update(cx, |input, cx| input.set_value(&value, window, cx));
                        self.confirm_bulk_action = false;
                        self.confirm_bulk_action_ids = None;
                    }
                }
                crate::FileManagerUiRequest::OpenMediaActions(id) => {
                    self.selected_directory = None;
                    if let Some(media) = self.service.media_by_id(id).ok().flatten() {
                        self.selection.selected_media_ids = vec![id];
                        self.selection.anchor_id = Some(id);
                        self.media_actions_id = Some(id);
                        self.media_actions_item = Some((id, media));
                        self.persist_filter_session();
                    }
                }
                crate::FileManagerUiRequest::CloseMediaActions => {
                    self.media_actions_id = None;
                    self.media_actions_item = None;
                }
                crate::FileManagerUiRequest::CloseDialogs => {
                    self.show_records = false;
                    self.show_statistics = false;
                    self.bulk_action = None;
                    self.show_selection_details = false;
                    self.show_tag_picker = false;
                    self.media_actions_id = None;
                    self.media_actions_item = None;
                    self.confirm_bulk_action = false;
                    self.confirm_bulk_action_ids = None;
                }
                crate::FileManagerUiRequest::Select(id) => {
                    self.selected_directory = None;
                    if self.service.media_by_id(id).ok().flatten().is_some() {
                        self.selection.selected_media_ids = vec![id];
                        self.selection.anchor_id = Some(id);
                        self.persist_filter_session();
                    }
                }
                crate::FileManagerUiRequest::Preview(id) => {
                    self.preview_media(id, cx);
                }
                crate::FileManagerUiRequest::Deselect => {
                    self.inspector_deselected = true;
                    self.selected_directory = None;
                    self.selection = FileManagerSelection::default();
                    self.persist_filter_session();
                }
                crate::FileManagerUiRequest::SelectMany(ids) => {
                    self.selected_directory = None;
                    self.selection.selected_media_ids = ids;
                    self.selection.anchor_id = self.selection.selected_media_ids.first().copied();
                    self.persist_filter_session();
                }
                crate::FileManagerUiRequest::ScrollTo(index) => {
                    if self
                        .snapshot
                        .as_ref()
                        .is_ok_and(|snapshot| index < snapshot.skeletons.len())
                    {
                        self.scroll_to_file_manager_item(index);
                    }
                }
                crate::FileManagerUiRequest::Search(query) => {
                    self.search
                        .update(cx, |input, cx| input.set_value(&query, window, cx));
                    self.filter.search = query;
                    self.reload();
                }
                crate::FileManagerUiRequest::SearchScopesOpen(open) => {
                    self.search_scopes_open = open;
                }
                crate::FileManagerUiRequest::EverythingRecheck => self.recheck_everything(cx),
                crate::FileManagerUiRequest::SearchEntireComputer(enabled) => {
                    self.set_search_entire_computer(enabled, cx)
                }
                crate::FileManagerUiRequest::SearchScope(scope) => {
                    self.toggle_search_scope(scope, cx);
                }
                crate::FileManagerUiRequest::MediaType(value) => self.set_media_type(value, cx),
                crate::FileManagerUiRequest::Sort(value) => self.set_sort(value, cx),
                crate::FileManagerUiRequest::RatingRange(min, max) => {
                    self.filter.rating_min = min.clamp(0, 5);
                    self.filter.rating_max = max.clamp(min, 5);
                    self.reload();
                }
                crate::FileManagerUiRequest::Extension(value, exclude) => {
                    self.set_extension_click_mode(exclude, cx);
                    self.cycle_extension(value, cx);
                }
                crate::FileManagerUiRequest::TagFilter(id, exclude) => {
                    self.toggle_tag_filter(id, exclude, cx)
                }
                crate::FileManagerUiRequest::TagFilterMode(exclude) => {
                    self.tag_filter_exclude_mode = exclude
                }
                crate::FileManagerUiRequest::ExtensionMode(exclude) => {
                    self.set_extension_click_mode(exclude, cx)
                }
                crate::FileManagerUiRequest::DateBound(from, date) => {
                    let picker = if from { &self.date_from } else { &self.date_to };
                    picker.update(cx, |picker, cx| {
                        picker.set_date(
                            gpui_kit::component::calendar::Date::Single(date),
                            window,
                            cx,
                        )
                    });
                    if from {
                        self.filter.date_from = date;
                    } else {
                        self.filter.date_to = date;
                    }
                    self.date_calendar.update(cx, |calendar, cx| {
                        calendar.set_date(
                            gpui_kit::component::calendar::Date::Range(
                                self.filter.date_from,
                                self.filter.date_to,
                            ),
                            window,
                            cx,
                        )
                    });
                    self.reload();
                }
                crate::FileManagerUiRequest::ClearFilters => self.clear_filters(window, cx),
                crate::FileManagerUiRequest::InspectorSection(section) => {
                    self.toggle_inspector_section(section, cx)
                }
                crate::FileManagerUiRequest::TagPicker => {
                    self.show_tag_picker = !self.show_tag_picker
                }
                crate::FileManagerUiRequest::Grouping(mode) => self.set_grouping(mode, cx),
                crate::FileManagerUiRequest::ToggleGroup(label) => {
                    self.toggle_month_collapsed(label, cx)
                }
                crate::FileManagerUiRequest::MoveFilesToSubfolder(ids, path) => {
                    self.move_ids_to_directory(ids, path, cx)
                }
                crate::FileManagerUiRequest::DebugInternalFileDrag(enabled) => {
                    let _ = self.surfaces.set_open(
                        format!(
                            "file_manager.column.{}.internal-file-drag",
                            self.column_index
                        ),
                        enabled,
                    );
                }
                crate::FileManagerUiRequest::MonthGrouping(enabled) => {
                    self.set_month_grouping(enabled, cx)
                }
                crate::FileManagerUiRequest::Hover(id) => {
                    self.hovered_cards.clear();
                    if let Some(id) = id {
                        self.set_card_hover(id, true, cx);
                    }
                }
            }
            cx.notify();
        }
        self.service
            .publish_ui_snapshot(crate::FileManagerUiSnapshot {
                version_states: self.version_state_snapshot(),
                version_states_loading: self.version_states.is_pending(),
                snapshot_revision: self.snapshot_generation,
                records_open: self.show_records,
                records_count: self
                    .records_read
                    .value()
                    .and_then(|r| r.as_ref().ok())
                    .map(Vec::len),
                record_candidates: self
                    .record_candidates
                    .iter()
                    .map(|(id, path)| (*id, path.clone()))
                    .collect(),
                browsing: self.snapshot.as_ref().is_ok_and(|s| s.browser.active),
                search_provider: self.snapshot.as_ref().map_or("idle", |s| s.search_provider),
                everything_state: self
                    .everything_status()
                    .map_or("checking", |status| status.state),
                everything_version: self
                    .everything_status()
                    .and_then(|status| status.version.clone()),
                everything_path: self
                    .everything_status()
                    .and_then(|status| status.path.as_ref().map(|path| path.display().to_string())),
                everything_loading: self
                    .service
                    .everything
                    .as_ref()
                    .is_some_and(|service| service.index_loading()),
                directories: self
                    .snapshot
                    .as_ref()
                    .map(|s| s.browser.directories.clone())
                    .unwrap_or_default(),
                current_directory: self
                    .snapshot
                    .as_ref()
                    .ok()
                    .and_then(|s| s.browser.directory.clone()),
                parent_directory: self
                    .snapshot
                    .as_ref()
                    .ok()
                    .and_then(|s| s.browser.parent.clone()),
                directory_error: self
                    .snapshot
                    .as_ref()
                    .ok()
                    .and_then(|s| s.browser.error.clone()),
                selected_directory: self.selected_directory.clone(),
                filter: self.filter.clone(),
                search_scopes_open: self.search_scopes_open,
                selected_media_ids: self.selection.selected_media_ids.clone(),
                file_copy_buffer_count: self.file_copy_buffer.len(),
                matched_count: self
                    .snapshot
                    .as_ref()
                    .map(|s| s.skeletons.len())
                    .unwrap_or(0),
                group_by_month: self.grouping_mode
                    == fileatlas_plugin_grouping::GroupingMode::Month,
                grouping_mode: self.grouping_mode,
                groups: self
                    .snapshot
                    .as_ref()
                    .map(|s| s.groups.clone())
                    .unwrap_or_default(),
                collapsed_groups: self.collapsed_months.iter().cloned().collect(),
                list_layout: self.list_layout,
                window_offset: self
                    .snapshot
                    .as_ref()
                    .map(|snapshot| snapshot.window_offset)
                    .unwrap_or(0),
                loaded_count: self
                    .snapshot
                    .as_ref()
                    .map(|snapshot| snapshot.media.len())
                    .unwrap_or(0),
                inspector_sections: self.inspector_expanded.iter().copied().collect(),
                inspector_context: self.inspector_context_key(),
                bulk_action: self.bulk_action.map(|action| action.label()),
                bulk_source: self.bulk_source.label(),
                statistics_open: self.show_statistics,
                dialog_reads: self.dialog_read_status(),
                tag_picker_open: self.show_tag_picker,
                media_actions_open: self.media_actions_id.is_some(),
                media_actions_id: self.media_actions_id,
            });
        self.refresh_locale(window, cx);
        // v5 grid: columns come from the FileManager viewport strip (rows own the
        // 16 pt horizontal padding) using the square thumb + 16 pt gap.
        let sidebar_width = self
            .session
            .snapshot()
            .ok()
            .map(|session| session.sidebar_width)
            .filter(|width| {
                width.is_finite()
                    && (theme::SIDEBAR_MIN_WIDTH..=theme::SIDEBAR_MAX_WIDTH).contains(width)
            })
            .unwrap_or(theme::SIDEBAR_WIDTH);
        let previous_viewport_width = f32::from(self.scroll.viewport_bounds().size.width);
        let available_width = if previous_viewport_width > 0.0 {
            previous_viewport_width
        } else {
            f32::from(window.bounds().size.width) - sidebar_width - GRID_PADDING * 2.0
        };
        let cards_width = (available_width - GRID_PADDING * 2.0).max(0.0);
        let columns = if self.list_layout {
            1
        } else {
            ((cards_width + GRID_GAP) / (self.thumb_size + GRID_GAP))
                .floor()
                .max(1.0) as usize
        };
        self.columns = columns;
        let group_by_month = self.grouping_mode != fileatlas_plugin_grouping::GroupingMode::None;
        let row_layout_key = (
            columns,
            group_by_month,
            self.filter.sort,
            self.collapsed_months.clone(),
        );
        if self.row_layout_key.as_ref() != Some(&row_layout_key) {
            self.row_layout = Arc::new(
                self.snapshot
                    .as_ref()
                    .map(|snapshot| {
                        if snapshot.browser.active {
                            if self.grouping_mode == fileatlas_plugin_grouping::GroupingMode::None {
                                super::rows::build_browser_rows(
                                    snapshot.browser.directories.len(),
                                    snapshot.skeletons.len(),
                                    columns,
                                )
                            } else {
                                let mut rows = super::rows::build_browser_rows(
                                    snapshot.browser.directories.len(),
                                    0,
                                    columns,
                                );
                                rows.extend(build_file_manager_rows(
                                    &snapshot.group_labels,
                                    columns,
                                    &self.collapsed_months,
                                ));
                                rows
                            }
                        } else {
                            build_file_manager_rows(
                                &snapshot.group_labels,
                                columns,
                                &self.collapsed_months,
                            )
                        }
                    })
                    .unwrap_or_default(),
            );
            self.row_layout_key = Some(row_layout_key);
            self.scroll.reset(self.row_layout.len());
            if let Some(mut offset) = self.pending_scroll_restore.take() {
                let mut item_ix = 0;
                while item_ix + 1 < self.row_layout.len() {
                    let height = file_manager_row_height(
                        &self.row_layout[item_ix],
                        item_ix,
                        self.thumb_size,
                        self.list_layout,
                    );
                    if offset < height {
                        break;
                    }
                    offset -= height;
                    item_ix += 1;
                }
                self.scroll.scroll_to(ListOffset {
                    item_ix,
                    offset_in_item: px(offset),
                });
            }
        }
        let row_layout = Arc::clone(&self.row_layout);

        let parent_folder_hovered = self.grouping_mode
            == fileatlas_plugin_grouping::GroupingMode::Subfolder
            && self
                .snapshot
                .as_ref()
                .ok()
                .and_then(|snapshot| snapshot.browser.directory.as_deref())
                .is_some_and(|path| {
                    self.surfaces
                        .value(&super::group_frame::hover_key(self.column_index))
                        .as_deref()
                        == Some(path)
                });
        match &self.snapshot {
            Ok(snapshot) => {
                if snapshot.skeletons.is_empty() && snapshot.browser.directories.is_empty() {
                    if self.snapshot_read.is_pending() {
                        return div()
                            .size_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .workbench_assistant()
                            .child(fileatlas_ui_bridge::i18n::tr("Loading…"))
                            .into_any_element();
                    }
                    if snapshot.browser.active {
                        return directory_browser::empty(
                            self,
                            snapshot
                                .browser
                                .error
                                .as_deref()
                                .or_else(|| self.filters_active().then_some("No matching items")),
                            cx,
                        );
                    }
                    return empty_state::render(self.filters_active() || self.scope_active(), cx);
                }
                let list_scroll = self.scroll.clone();
                let list_layout = self.list_layout;
                let card_size = if list_layout { 56. } else { self.thumb_size };
                let list_width = list_layout.then_some(cards_width.max(100.));
                let selected_ids = self.selection.selected_media_ids.clone();
                let loaded_selected_paths = self
                    .snapshot
                    .as_ref()
                    .ok()
                    .map(|snapshot| {
                        selected_ids
                            .iter()
                            .filter_map(|id| {
                                snapshot
                                    .media
                                    .iter()
                                    .find(|media| media.id == *id)
                                    .map(|media| PathBuf::from(media.full_path.as_str()))
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let selected_drag_paths =
                    self.selected_drag_paths(&selected_ids, loaded_selected_paths, cx);
                let row_layout = Arc::clone(&row_layout);
                let mut grid = div()
                    .id("file_manager-grid-root")
                    .on_drag_move::<fileatlas_contracts::FileManagerFileDragPayload>(cx.listener(
                        |this, event, _, cx| super::group_frame::track_drag(this, event, cx),
                    ))
                    .on_drag_move::<fileatlas_contracts::FileStagingDragPayload>(cx.listener(
                        |this, event, _, cx| {
                            super::group_frame::track_staging_drag(this, event, cx)
                        },
                    ))
                    .on_drop::<fileatlas_contracts::FileManagerFileDragPayload>(cx.listener(
                        |this,
                         payload: &fileatlas_contracts::FileManagerFileDragPayload,
                         window,
                         cx| {
                            if payload.allows_drop() {
                                if let Some(destination) = super::group_frame::destination_at(
                                    this,
                                    window.mouse_position(),
                                ) {
                                    this.move_ids_to_directory(
                                        payload.ids.clone(),
                                        destination,
                                        cx,
                                    );
                                }
                            }
                        },
                    ))
                    .on_drop::<fileatlas_contracts::FileStagingDragPayload>(cx.listener(
                        |this,
                         payload: &fileatlas_contracts::FileStagingDragPayload,
                         window,
                         cx| {
                            if let Some(destination) =
                                super::group_frame::destination_at(this, window.mouse_position())
                            {
                                this.move_staged_paths_to_directory(
                                    payload.paths.clone(),
                                    destination,
                                    cx,
                                );
                            }
                        },
                    ))
                    .track_focus(&self.focus_handle)
                    .on_key_down(
                        cx.listener(|this, event, window, cx| this.handle_key(event, window, cx)),
                    )
                    .v_flex()
                    // The shell parent owns the viewport height; growing here avoids
                    // size_full resolving against the window and overlapping the header.
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .pt(px(GRID_PADDING));
                let rubberband = self.rubberband.as_ref().map(|rubberband| {
                    let viewport = self.scroll.viewport_bounds();
                    let band = Bounds::from_corners(
                        point(
                            rubberband.start.x.min(rubberband.end.x),
                            rubberband.start.y.min(rubberband.end.y),
                        ),
                        point(
                            rubberband.start.x.max(rubberband.end.x),
                            rubberband.start.y.max(rubberband.end.y),
                        ),
                    )
                    .intersect(&viewport);
                    let left = f32::from(band.origin.x - viewport.origin.x);
                    let top = f32::from(band.origin.y - viewport.origin.y);
                    (
                        left,
                        top,
                        f32::from(band.size.width),
                        f32::from(band.size.height),
                    )
                });
                grid = grid.child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .w_full()
                        .relative()
                        .when(parent_folder_hovered, |frame| {
                            frame.workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Selection)
                        })
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, event: &MouseDownEvent, _, cx| {
                                this.start_rubberband(event, cx)
                            }),
                        )
                        .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                            this.update_rubberband(event, cx)
                        }))
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(|this, event: &MouseUpEvent, _, cx| {
                                this.finish_rubberband(event, cx)
                            }),
                        )
                        .child(
                            list(
                                list_scroll.clone(),
                                cx.processor(move |this, row_index: usize, window, cx| {
                                    let range = row_index..row_index + 1;
                                    let mut rows = Vec::with_capacity(range.len());
                                    let requested_start = row_layout
                                        .get(range.start)
                                        .map(|row| row.media_range.start)
                                        .unwrap_or(0);
                                    let requested_end = range
                                        .end
                                        .checked_sub(1)
                                        .and_then(|row| row_layout.get(row))
                                        .map(|row| row.media_range.end)
                                        .unwrap_or(requested_start);
                                    let window_range = requested_start.saturating_sub(columns * 2)
                                        ..requested_end.saturating_add(columns * 2);
                                    this.request_visible_window(window_range);
                                    let mut states = Vec::new();
                                    let view_focused = this.focus_handle.is_focused(window);
                                    let loaded: HashSet<_> = this
                                        .snapshot
                                        .as_ref()
                                        .map(|snapshot| {
                                            snapshot.media.iter().map(|media| media.id).collect()
                                        })
                                        .unwrap_or_default();
                                    if let Ok(snapshot) = &this.snapshot {
                                        this.card_motion.retain(|id, _| loaded.contains(id));
                                        this.hovered_cards.retain(|id| loaded.contains(id));
                                        let card_tag_tree = Arc::new(snapshot.tag_tree.clone());
                                        let selected_drag_paths = selected_drag_paths.clone();
                                        for row_index in range {
                                            let Some(layout_row) = row_layout.get(row_index) else {
                                                continue;
                                            };
                                            let first = layout_row.media_range.start;
                                            let end = layout_row.media_range.end;
                                            let month_label = layout_row.month_label.clone();
                                            let has_month_label = month_label.is_some();
                                            let mut cards = Vec::with_capacity(
                                                end - first + layout_row.directory_range.len(),
                                            );
                                            for folder in &snapshot.browser.directories
                                                [layout_row.directory_range.clone()]
                                            {
                                                cards.push(directory_browser::tile(
                                                    folder,
                                                    this.selected_directory.as_deref()
                                                        == Some(folder.path.as_str()),
                                                    card_size,
                                                    list_width,
                                                    cx,
                                                ));
                                            }
                                            if !layout_row.collapsed {
                                                for skeleton in &snapshot.skeletons[first..end] {
                                                    if let Some(media) = snapshot
                                                        .media
                                                        .iter()
                                                        .find(|item| item.id == skeleton.id)
                                                    {
                                                        let selected = this
                                                            .selection
                                                            .selected_media_ids
                                                            .contains(&media.id);
                                                        let rating_target_ids = if selected {
                                                            this.selection
                                                                .selected_media_ids
                                                                .clone()
                                                        } else {
                                                            vec![media.id]
                                                        };
                                                        let drag_paths = if selected
                                                            && !selected_drag_paths.is_empty()
                                                        {
                                                            selected_drag_paths.clone()
                                                        } else {
                                                            vec![PathBuf::from(
                                                                media.full_path.as_str(),
                                                            )]
                                                        };
                                                        let drag_ids = if selected {
                                                            this.selection
                                                                .selected_media_ids
                                                                .clone()
                                                        } else {
                                                            vec![media.id]
                                                        };
                                                        let now = std::time::Instant::now();
                                                        let motion = this
                                                            .card_motion
                                                            .entry(media.id)
                                                            .or_insert_with(|| {
                                                                super::card_motion::CardMotion::new(
                                                                    selected, now,
                                                                )
                                                            });
                                                        let (hover, selection, moving) = motion
                                                            .sample(
                                                                this.hovered_cards
                                                                    .contains(&media.id),
                                                                selected,
                                                                now,
                                                                cx.reduce_motion(),
                                                            );
                                                        if moving && !cx.reduce_motion() {
                                                            window.request_animation_frame();
                                                        }
                                                        states.push(crate::FileManagerCardState {
                                                            media_id: media.id,
                                                            hover,
                                                            selection,
                                                            timestamp_ms:
                                                                std::time::SystemTime::now()
                                                                    .duration_since(
                                                                        std::time::UNIX_EPOCH,
                                                                    )
                                                                    .unwrap_or_default()
                                                                    .as_millis()
                                                                    as u64,
                                                        });
                                                        cards.push(
                                                            render_media_card(
                                                                media,
                                                                hover,
                                                                selection,
                                                                view_focused,
                                                                Arc::clone(&this.drag_drop),
                                                                drag_paths,
                                                                drag_ids,
                                this.grouping_mode != fileatlas_plugin_grouping::GroupingMode::Subfolder
                                    && !this.surfaces.is_open(&format!("file_manager.column.{}.internal-file-drag", this.column_index)),
                                                                Arc::clone(&this.drag_allowed),
                                                                card_size,
                                                                list_width,
                                                                media.micro_thumb.as_deref(),
                                                                this.version_state(
                                                                    media.id,
                                                                    &media.full_path,
                                                                )
                                                                .and_then(|status| {
                                                                    status.state.as_ref()
                                                                }),
                                                                rating_target_ids,
                                                                snapshot
                                                                    .media_tags
                                                                    .get(&media.id)
                                                                    .map(Vec::as_slice)
                                                                    .unwrap_or_default(),
                                                                Arc::clone(&card_tag_tree),
                                                                cx,
                                                            )
                                                            .into_any_element(),
                                                        );
                                                    } else {
                                                        cards.push(
                                                            div()
                                                                .w(px(card_size))
                                                                .h(px(card_size
                                                                    + card::CARD_CAPTION_HEIGHT))
                                                                .workbench_corner(fileatlas_ui_bridge::components::CornerRole::Thumbnail)
                                                                .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Faint)
                                                                .into_any_element(),
                                                        );
                                                    }
                                                }
                                            }
                                            let header = month_label
                                                .clone()
                                                .map(|label| {
                                                    let collapsed = layout_row.collapsed;
                                                    let now = std::time::Instant::now();
                                                    let motion = this
                                                        .month_motion
                                                        .entry(label.clone())
                                                        .or_insert_with(|| {
                                                            super::card_motion::CardMotion::new(
                                                                !collapsed, now,
                                                            )
                                                            .with_selection_seconds(0.15)
                                                        });
                                                    let (_, expanded, moving) = motion.sample(
                                                        false,
                                                        !collapsed,
                                                        now,
                                                        cx.reduce_motion(),
                                                    );
                                                    if moving {
                                                        window.request_animation_frame();
                                                    }
                                                    super::month_header::render(
                                                        label.clone(),
                                                        layout_row.month_count,
                                                        expanded,
                                                        snapshot.browser.directory.as_deref(),
                                                        cx,
                                                    )
                                                    .into_any_element()
                                                })
                                                .unwrap_or_else(|| div().into_any_element());
                                            let month_top_gap = if row_index > 0 && has_month_label
                                            {
                                                theme::space_expanded()
                                            } else {
                                                0.0
                                            };
                                            let header_gap = if has_month_label {
                                                theme::space_standard()
                                            } else {
                                                0.0
                                            };
                                            rows.push(
                                                div()
                                                    .id(format!("file_manager-row-{row_index}"))
                                                    .v_flex()
                                                    .w_full()
                                                    .px(px(GRID_PADDING))
                                                    .h(px(file_manager_row_height(
                                                        layout_row,
                                                        row_index,
                                                        card_size,
                                                        list_layout,
                                                    )))
                                                    .pt(px(month_top_gap))
                                                    .gap_0()
                                                    .child(super::group_frame::render(
                                                        row_index,
                                                        layout_row,
                                                        header,
                                                        div()
                                                            .h_flex()
                                                            .mt(px(header_gap))
                                                            .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
                                                            .children(cards)
                                                            .into_any_element(),
                                                        this,
                                                        cx,
                                                    )),
                                            );
                                        }
                                    }
                                    this.service.publish_card_states(states, &loaded);
                                    rows.into_iter()
                                        .next()
                                        .map(IntoElement::into_any_element)
                                        .unwrap_or_else(|| div().into_any_element())
                                }),
                            )
                            .size_full(),
                        )
                        .vertical_scrollbar(&list_scroll)
                        .children(rubberband.map(|(left, top, width, height)| {
                            fileatlas_ui_bridge::components::selection_region(
                                left, top, width, height,
                            )
                        })),
                );
                grid.into_any_element()
            }
            Err(error) => div()
                .size_full()
                .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::BaseOverlay)
                .p(px(fileatlas_ui_bridge::theme::space_expanded()))
                .workbench_assistant()
                .child(format!("Could not load media: {error}"))
                .into_any_element(),
        }
    }
}

impl FileManagerView {
    fn selected_drag_paths(
        &mut self,
        selected_ids: &[i64],
        loaded_paths: Vec<PathBuf>,
        _: &mut Context<Self>,
    ) -> Vec<PathBuf> {
        if selected_ids.is_empty() {
            return Vec::new();
        }
        if loaded_paths.len() == selected_ids.len() {
            return loaded_paths;
        }
        if let Some((cached_ids, paths)) = &self.selected_drag_paths_cache
            && cached_ids == selected_ids
        {
            return paths.clone();
        }

        let service = self.service.clone();
        let ids = selected_ids.to_vec();
        self.selected_drag_read.request(ids.clone(), move || {
            service.media_paths_by_ids(&ids).map_err(|e| e.to_string())
        });
        loaded_paths
    }
}

pub(super) fn file_manager_row_height(
    row: &FileManagerVisualRow,
    index: usize,
    thumb: f32,
    list: bool,
) -> f32 {
    let header = if row.month_label.is_some() {
        theme::GROUP_HEADER_HEIGHT + theme::space_standard()
    } else {
        0.
    };
    let month_gap = if index > 0 && row.month_label.is_some() {
        theme::space_expanded()
    } else {
        0.
    };
    let cards = if row.collapsed {
        0.
    } else {
        if list {
            56.
        } else {
            thumb + card::CARD_CAPTION_HEIGHT
        }
    };
    header + month_gap + cards + 16.
}
