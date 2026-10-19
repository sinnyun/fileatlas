//! GalleryView construction, session restore/persist and reload.

use super::*;

pub(super) struct GalleryViewInputs {
    pub tag_picker_search: Entity<InputState>,
    pub search: Entity<InputState>,
    pub address: Entity<InputState>,
    pub date_from: Entity<DatePickerState>,
    pub date_to: Entity<DatePickerState>,
    pub date_calendar: Entity<CalendarState>,
    pub bulk_date_from: Entity<DatePickerState>,
    pub bulk_date_to: Entity<DatePickerState>,
    pub rename_input: Entity<InputState>,
    pub inspector_tag_name: Entity<InputState>,
    pub history_title: Entity<InputState>,
    pub history_branch_name: Entity<InputState>,
    pub bulk_inputs: Vec<Entity<InputState>>,
}

pub(super) fn build_inputs(
    window: &mut Window,
    app: &mut App,
    _session: &Arc<SessionService>,
    workspace_column: &photag_plugin_workspaces::ColumnDefinition,
) -> GalleryViewInputs {
    let search = app.new(|cx| {
        InputState::new(window, cx).placeholder(photag_ui_bridge::i18n::tr("Search files"))
    });
    let address = app.new(|cx| InputState::new(window, cx).placeholder("Folder path"));
    let date_from = app.new(|cx| DatePickerState::new(window, cx).date_format("%Y-%m-%d"));
    let date_to = app.new(|cx| DatePickerState::new(window, cx).date_format("%Y-%m-%d"));
    let saved_date_from = workspace_column
        .date_from
        .as_deref()
        .and_then(parse_date_input)
        .flatten();
    let saved_date_to = workspace_column
        .date_to
        .as_deref()
        .and_then(parse_date_input)
        .flatten();
    let date_calendar = app.new(|cx| {
        let mut calendar = CalendarState::new(window, cx);
        calendar.set_date(
            gpui_kit::component::calendar::Date::Range(saved_date_from, saved_date_to),
            window,
            cx,
        );
        calendar
    });
    let tag_picker_search = app.new(|cx| {
        InputState::new(window, cx).placeholder(photag_ui_bridge::i18n::tr("Find tags to assign"))
    });
    search.update(app, |input, cx| {
        input.set_value(&workspace_column.search, window, cx)
    });
    if let Some(path) = workspace_column.address.as_deref() {
        address.update(app, |input, cx| input.set_value(path, window, cx));
    }
    if let Some(date) = saved_date_from {
        date_from.update(app, |picker, cx| picker.set_date(date, window, cx));
    }
    if let Some(date) = saved_date_to {
        date_to.update(app, |picker, cx| picker.set_date(date, window, cx));
    }
    let rename_input = app.new(|cx| {
        InputState::new(window, cx).placeholder(photag_ui_bridge::i18n::tr("New filename"))
    });
    let inspector_tag_name = app.new(|cx| InputState::new(window, cx));
    let history_title = app.new(|cx| {
        InputState::new(window, cx).placeholder(photag_ui_bridge::i18n::tr("Version title"))
    });
    let history_branch_name = app.new(|cx| {
        InputState::new(window, cx).placeholder(photag_ui_bridge::i18n::tr("Branch name"))
    });
    let bulk_date_from = app.new(|cx| DatePickerState::new(window, cx).date_format("%Y-%m-%d"));
    let bulk_date_to = app.new(|cx| DatePickerState::new(window, cx).date_format("%Y-%m-%d"));
    let bulk_inputs = [
        "Older than days",
        "Newer than days",
        "Rating min 0-5",
        "Rating max 0-5",
        "Name contains",
        "Extension",
        "Size min MB",
        "Size max MB",
    ]
    .into_iter()
    .map(|placeholder| app.new(|cx| InputState::new(window, cx).placeholder(placeholder)))
    .collect();
    GalleryViewInputs {
        tag_picker_search,
        search,
        address,
        date_from,
        date_to,
        date_calendar,
        bulk_date_from,
        bulk_date_to,
        rename_input,
        inspector_tag_name,
        history_title,
        history_branch_name,
        bulk_inputs,
    }
}

fn parse_workspace_sort(value: &str) -> SortOrder {
    match value.to_ascii_lowercase().replace(['_', ' '], "").as_str() {
        "oldest" | "oldestfirst" => SortOrder::OldestFirst,
        "biggest" | "biggestfirst" | "largestfirst" => SortOrder::BiggestFirst,
        "smallest" | "smallestfirst" => SortOrder::SmallestFirst,
        "nameasc" | "nameascending" | "az" => SortOrder::NameAsc,
        "namedesc" | "namedescending" | "za" => SortOrder::NameDesc,
        "highestrated" | "ratingdescending" => SortOrder::HighestRated,
        "lowestrated" | "ratingascending" => SortOrder::LowestRated,
        _ => SortOrder::NewestFirst,
    }
}

impl GalleryView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        service: Arc<GalleryService>,
        file_operations: Arc<FileOperationsService>,
        drag_drop: Arc<DragDropService>,
        quicklook: Arc<QuickLookService>,
        versions: Arc<photag_plugin_versions::VersionsService>,
        runtime: tokio::runtime::Handle,
        cordis: CordisContext,
        selected_folder: Arc<SelectedFolder>,
        selected_tag: Arc<SelectedTag>,
        selected_path: Arc<SelectedPath>,
        index_revision: Arc<IndexRevision>,
        session: Arc<SessionService>,
        history: Option<Arc<HistoryService>>,
        settings: Option<Arc<SettingsService>>,
        tagging: Option<Arc<TaggingService>>,
        surfaces: Arc<photag_ui_bridge::UiSurfaceState>,
        ui_views: Arc<photag_ui_bridge::UiViewCatalog>,
        column_index: usize,
        workspace_column: photag_plugin_workspaces::ColumnDefinition,
        inputs: GalleryViewInputs,
        cx: &mut Context<Self>,
    ) -> Self {
        let GalleryViewInputs {
            tag_picker_search,
            search,
            address,
            date_from,
            date_to,
            date_calendar,
            bulk_date_from,
            bulk_date_to,
            rename_input,
            inspector_tag_name,
            history_title,
            history_branch_name,
            bulk_inputs,
        } = inputs;
        let saved = session.snapshot().unwrap_or_default();
        let (address_history, address_history_index) =
            if workspace_column.address_history.is_empty() {
                let history = workspace_column.address.iter().cloned().collect::<Vec<_>>();
                (history, 0)
            } else {
                let index = workspace_column
                    .address_history_index
                    .min(workspace_column.address_history.len() - 1);
                (workspace_column.address_history.clone(), index)
            };
        if surfaces.value("gallery.inspector.tab").is_none() {
            let _ = surfaces.set_value("gallery.inspector.tab", "information");
        }
        let initial_inspector_tab = surfaces.value("gallery.inspector.tab");
        let mut filter = GalleryFilter::default();
        if let Some(settings) = &settings
            && let Ok(settings_snapshot) = settings.snapshot()
        {
            filter.search_scopes = settings_snapshot.search_scopes.into();
            filter.search_entire_computer = settings_snapshot.search_entire_computer;
        }
        filter.search = workspace_column.search.clone();
        filter.sort = parse_workspace_sort(&workspace_column.sort_order);
        filter.media_type = match workspace_column.media_type.as_str() {
            "photos" | "images" => MediaTypeFilter::Photos,
            "videos" | "video" => MediaTypeFilter::Videos,
            "audio" => MediaTypeFilter::Audio,
            _ => MediaTypeFilter::All,
        };
        filter.search_scopes = crate::filter::SearchScopes {
            filenames: workspace_column.search_in_name,
            paths: workspace_column.search_in_path,
            tags: workspace_column.search_in_tags,
            time: workspace_column.search_in_time,
        };
        filter.folder_id = workspace_column.folder_id;
        filter.path_prefix = workspace_column.address.clone();
        filter.extensions = workspace_column.included_extensions.clone();
        filter.excluded_extensions = workspace_column.excluded_extensions.clone();
        filter.date_from = workspace_column
            .date_from
            .as_deref()
            .and_then(parse_date_input)
            .flatten();
        filter.date_to = workspace_column
            .date_to
            .as_deref()
            .and_then(parse_date_input)
            .flatten();
        filter.rating_min = i32::from(workspace_column.rating_min.min(5));
        filter.rating_max = i32::from(
            workspace_column
                .rating_max
                .clamp(workspace_column.rating_min.min(5), 5),
        );
        filter.selected_tag_ids = workspace_column.included_tag_ids.clone();
        filter.excluded_tag_ids = workspace_column.excluded_tag_ids.clone();
        if let Ok(mut selected_path_state) = selected_path.0.lock() {
            *selected_path_state = workspace_column.address.clone();
        }
        if let Ok(mut selected_folder_state) = selected_folder.0.lock() {
            *selected_folder_state = workspace_column.folder_id;
        }
        if let Ok(mut selected_tags) = selected_tag.0.lock() {
            selected_tags.included = workspace_column.included_tag_ids.clone();
            selected_tags.excluded = workspace_column.excluded_tag_ids.clone();
        }
        let focus_handle = cx.focus_handle();
        let mut _subscriptions = vec![
            cx.subscribe(&tag_picker_search, |_, _, event, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }),
            cx.subscribe(&search, |this, _, event, cx| {
                if matches!(event, InputEvent::Change) {
                    this.filter.search = this.search.read(cx).value().to_string();
                    this.reload();
                    cx.notify();
                }
            }),
            cx.subscribe(&date_from, |this, _, event, cx| {
                if let DatePickerEvent::Change(DateTime::Single(date)) = event {
                    this.filter.date_from = date.map(|value| value.date());
                    this.reload();
                    cx.notify();
                }
            }),
            cx.subscribe(&date_to, |this, _, event, cx| {
                if let DatePickerEvent::Change(DateTime::Single(date)) = event {
                    this.filter.date_to = date.map(|value| value.date());
                    this.reload();
                    cx.notify();
                }
            }),
            cx.subscribe(&bulk_date_from, |this, _, event, cx| {
                if let DatePickerEvent::Change(DateTime::Single(date)) = event {
                    this.bulk_date_from_value = date.map(|value| value.date());
                    this.confirm_bulk_action = false;
                    this.confirm_bulk_action_ids = None;
                    cx.notify();
                }
            }),
            cx.subscribe(&bulk_date_to, |this, _, event, cx| {
                if let DatePickerEvent::Change(DateTime::Single(date)) = event {
                    this.bulk_date_to_value = date.map(|value| value.date());
                    this.confirm_bulk_action = false;
                    this.confirm_bulk_action_ids = None;
                    cx.notify();
                }
            }),
        ];
        _subscriptions.extend(bulk_inputs.iter().map(|input| {
            cx.subscribe(input, |_, _, event, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            })
        }));
        let selected = selected_folder
            .0
            .lock()
            .map(|selected| *selected)
            .unwrap_or(workspace_column.folder_id);
        filter.folder_id = selected;
        if let Ok(selected_tag) = selected_tag.0.lock() {
            filter.selected_tag_ids = selected_tag.included.clone();
            filter.excluded_tag_ids = selected_tag.excluded.clone();
        }
        filter.path_prefix = selected_path
            .0
            .lock()
            .map(|selected| selected.clone())
            .unwrap_or(None)
            .or(saved.selected_path.clone());
        let observed_index_revision = index_revision.0.load(Ordering::Acquire);
        let settings_group_by_month = settings
            .as_ref()
            .and_then(|settings| settings.snapshot().ok())
            .map(|settings| settings.show_date_dividers);
        let grouping_mode = photag_plugin_grouping::GroupingMode::parse(&workspace_column.grouping)
            .or_else(|| {
                saved
                    .grouping_mode
                    .as_deref()
                    .and_then(photag_plugin_grouping::GroupingMode::parse)
            })
            .unwrap_or_else(|| {
                if settings_group_by_month.unwrap_or_else(|| saved.group_by_month.unwrap_or(true)) {
                    photag_plugin_grouping::GroupingMode::Month
                } else {
                    photag_plugin_grouping::GroupingMode::None
                }
            });
        service.set_show_descendants(workspace_column.show_subfolders);
        service.set_thumbnails_enabled(workspace_column.thumbnails_enabled);
        // Initial search can prepare/load Everything. Keep startup responsive
        // and run the first real query through the same ReadModel as reloads.
        let snapshot = Ok(GallerySnapshot {
            search_provider: "loading",
            ..Default::default()
        });
        let selection = GallerySelection::default();
        let scroll = ListState::new(0, ListAlignment::Top, px(240.));
        let restored_scroll = if workspace_column.scroll_offset.is_finite() {
            workspace_column.scroll_offset.max(0.)
        } else {
            0.
        };
        let restored_file_paths = if workspace_column.selected_file_paths.is_empty()
            && workspace_column.address.is_none()
            && column_index == 0
        {
            saved.selected_file_paths.clone()
        } else {
            workspace_column.selected_file_paths.clone()
        };
        let initial_writeback_failures = tagging
            .as_ref()
            .map(|tagging| writeback_failure_signature(tagging))
            .unwrap_or_default();
        let observed_navigation_revision = service.navigation_revision.load(Ordering::Acquire);
        let mut view = Self {
            _ready_task: None,
            month_motion: std::collections::HashMap::new(),
            hovered_cards: HashSet::new(),
            card_motion: std::collections::HashMap::new(),
            locale_revision: u64::MAX,
            service,
            file_operations,
            drag_drop,
            quicklook,
            versions,
            runtime,
            cordis,
            selected_folder,
            selected_tag,
            selected_path,
            index_revision,
            session,
            history,
            settings,
            tagging,
            surfaces: Arc::clone(&surfaces),
            ui_views,
            tag_picker_search,
            tag_picker_selected: Default::default(),
            search,
            address,
            address_editing: false,
            column_index,
            _workspace_task: None,
            workspace_id: String::new(),
            pending_workspace_search: None,
            pending_workspace_address: None,
            pending_workspace_dates: None,
            address_history,
            address_history_index,
            date_from,
            date_to,
            date_calendar,
            bulk_date_from,
            bulk_date_to,
            bulk_date_from_value: None,
            bulk_date_to_value: None,
            rename_input,
            inspector_tag_name,
            history_title,
            history_branch_name,
            inspector_tag_loaded: None,
            inspector_tag_draft: None,
            history_read: Default::default(),
            version_states: Default::default(),
            history_branches: Default::default(),
            history_generation: 0,
            history_selected: None,
            history_create_open: false,
            history_busy: false,
            history_error: None,
            bulk_inputs,
            bulk_action: None,
            bulk_source: BulkActionSource::WholeLibrary,
            statistics_read: Default::default(),
            bulk_read: Default::default(),
            bulk_media_type: MediaTypeFilter::All,
            bulk_tag_ids: Vec::new(),
            bulk_extension_exclude: false,
            confirm_bulk_action: false,
            confirm_bulk_action_ids: None,
            focus_handle,
            filter,
            selection,
            selected_directory: None,
            inspector_deselected: false,
            observed_navigation_revision,
            rubberband: None,
            scroll,
            pending_scroll_restore: Some(restored_scroll),
            columns: 4,
            row_layout: Arc::new(Vec::new()),
            row_layout_key: None,
            thumb_size: Some(workspace_column.thumbnail_size)
                .filter(|size| {
                    size.is_finite()
                        && (theme::THUMB_SIZE_MIN..=theme::THUMB_SIZE_MAX).contains(size)
                })
                .unwrap_or(theme::THUMB_SIZE_DEFAULT),
            grouping_mode,
            before_descendants_grouping: saved
                .before_descendants_grouping
                .as_deref()
                .and_then(photag_plugin_grouping::GroupingMode::parse),
            restored_file_paths,
            restore_in_directory: workspace_column.address.is_some(),
            selection_restore: Default::default(),
            list_layout: workspace_column.list_layout,
            show_selection_details: false,
            show_tag_picker: false,
            search_scopes_open: false,
            inspector_tag_picker_open: false,
            media_actions_id: None,
            media_actions_item: None,
            slider_drag: None,
            show_statistics: false,
            show_records: false,
            records_page: 0,
            record_read_revision: 0,
            location_picker_pending: false,
            confirm_delete_missing: false,
            record_candidates: Default::default(),
            records_read: Default::default(),
            collapsed_months: photag_plugin_grouping::restore_collapsed(
                &saved.collapsed_months,
                grouping_mode,
            ),
            inspector_expanded: ["basic", "tags"].into_iter().collect(),
            tag_filter_exclude_mode: false,
            extension_exclude_mode: saved.ext_click_mode
                == Some(photag_plugin_session::TagClickMode::Exclude),
            drag_allowed: Arc::new(AtomicBool::new(true)),
            quicklook_notice: None,
            quicklook_busy: false,
            show_exif: false,
            exif_cache: None,
            exif_read: Default::default(),
            operation_status: None,
            writeback_failure_signature: initial_writeback_failures,
            previous_inspector_tab: photag_ui_bridge::refresh::ChangeGate::new(
                initial_inspector_tab,
            ),
            confirm_delete: false,
            snapshot,
            snapshot_read: Default::default(),
            snapshot_generation: 0,
            snapshot_directory_dirty: false,
            directory_snapshot_scope: 0,
            directory_snapshot_started: std::time::Instant::now(),
            visible_demand: None,
            viewport_read: Default::default(),
            inspector_fallback: None,
            inspector_read: Default::default(),
            selection_details_cache: None,
            selection_details_read: Default::default(),
            selected_drag_paths_cache: None,
            selected_drag_read: Default::default(),
            everything_installation: Default::default(),
            everything_check_revision: 0,
            observed_everything_revision: 0,
            observed_index_revision,
            last_saved_scroll: restored_scroll,
            previous_scan_progress: Default::default(),
            previous_popover: photag_ui_bridge::refresh::ChangeGate::new(
                surfaces.value("gallery.popover"),
            ),
            _subscriptions,
        };
        view.reload_state(false);
        view.publish_workspace_snapshot();
        view
    }

    pub(super) fn refresh_index(&mut self) {
        let selected = self.selection.selected_media_ids.clone();
        let anchor = self.selection.anchor_id;
        let scroll = self
            .pending_scroll_restore
            .unwrap_or_else(|| self.scroll_offset());
        self.reload_state(false);
        // The retained snapshot can itself be a partial directory result.
        // Validate selection when monitor receives a complete replacement, not
        // against the old rows while scheduling that replacement.
        self.selection.selected_media_ids = selected;
        self.selection.anchor_id = anchor;
        self.pending_scroll_restore = Some(scroll);
        self.persist_filter_session();
    }

    // ListState's scrollbar estimate omits unmeasured rows after reset. A
    // second streamed result can arrive before paint, so use our known row
    // geometry and logical anchor instead of that temporary estimate.
    pub(super) fn scroll_offset(&self) -> f32 {
        let top = self.scroll.logical_scroll_top();
        self.row_layout
            .iter()
            .take(top.item_ix)
            .enumerate()
            .map(|(i, row)| {
                super::render::gallery_row_height(row, i, self.thumb_size, self.list_layout)
            })
            .sum::<f32>()
            + f32::from(top.offset_in_item)
    }

    pub(super) fn watch_workspace(
        &mut self,
        workspaces: Arc<photag_plugin_workspaces::WorkspaceService>,
        cx: &mut Context<Self>,
    ) {
        self.workspace_id = workspaces
            .active()
            .map(|workspace| workspace.id)
            .unwrap_or_default();
        let wake = workspaces.wake();
        self._workspace_task = Some(photag_ui_bridge::refresh::listen(
            cx,
            wake,
            move |view: &mut GalleryView, _cx| {
                let Ok(workspace) = workspaces.active() else {
                    return false;
                };
                if workspace.id == view.workspace_id {
                    return false;
                }
                view.workspace_id = workspace.id;
                let column = workspace
                    .columns
                    .get(view.column_index)
                    .cloned()
                    .unwrap_or_default();
                view.apply_workspace_column(column);
                view.reload();
                true
            },
        ));
    }

    fn apply_workspace_column(&mut self, column: photag_plugin_workspaces::ColumnDefinition) {
        self.filter.search = column.search.clone();
        self.pending_workspace_search = Some(column.search);
        self.filter.search_scopes = crate::filter::SearchScopes {
            filenames: column.search_in_name,
            paths: column.search_in_path,
            tags: column.search_in_tags,
            time: column.search_in_time,
        };
        self.filter.path_prefix = column.address.clone();
        self.filter.folder_id = column.folder_id;
        self.pending_workspace_address = Some(column.address.unwrap_or_default());
        self.address_history = column.address_history;
        if self.address_history.is_empty() {
            if let Some(path) = self.filter.path_prefix.clone() {
                self.address_history.push(path);
            }
        }
        self.address_history_index = column
            .address_history_index
            .min(self.address_history.len().saturating_sub(1));
        self.restored_file_paths = column.selected_file_paths;
        self.pending_scroll_restore = Some(column.scroll_offset.max(0.0));
        self.restore_in_directory = self.filter.path_prefix.is_some();
        self.filter.media_type = match column.media_type.as_str() {
            "photos" | "images" => MediaTypeFilter::Photos,
            "videos" | "video" => MediaTypeFilter::Videos,
            "audio" => MediaTypeFilter::Audio,
            _ => MediaTypeFilter::All,
        };
        self.filter.sort = parse_workspace_sort(&column.sort_order);
        self.filter.selected_tag_ids = column.included_tag_ids.clone();
        self.filter.excluded_tag_ids = column.excluded_tag_ids.clone();
        self.filter.extensions = column.included_extensions;
        self.filter.excluded_extensions = column.excluded_extensions;
        self.filter.date_from = column
            .date_from
            .as_deref()
            .and_then(parse_date_input)
            .flatten();
        self.filter.date_to = column
            .date_to
            .as_deref()
            .and_then(parse_date_input)
            .flatten();
        self.pending_workspace_dates = Some((self.filter.date_from, self.filter.date_to));
        self.filter.rating_min = i32::from(column.rating_min.min(5));
        self.filter.rating_max = i32::from(column.rating_max.clamp(column.rating_min.min(5), 5));
        if let Ok(mut selected_path) = self.selected_path.0.lock() {
            *selected_path = self.filter.path_prefix.clone();
        }
        if let Ok(mut selected_folder) = self.selected_folder.0.lock() {
            *selected_folder = self.filter.folder_id;
        }
        if let Ok(mut selected_tag) = self.selected_tag.0.lock() {
            selected_tag.included = self.filter.selected_tag_ids.clone();
            selected_tag.excluded = self.filter.excluded_tag_ids.clone();
        }
        self.service.set_show_descendants(column.show_subfolders);
        self.service
            .set_thumbnails_enabled(column.thumbnails_enabled);
        self.list_layout = column.list_layout;
        self.thumb_size = if column.thumbnail_size.is_finite() {
            column
                .thumbnail_size
                .clamp(theme::THUMB_SIZE_MIN, theme::THUMB_SIZE_MAX)
        } else {
            theme::THUMB_SIZE_DEFAULT
        };
        if let Some(grouping) = photag_plugin_grouping::GroupingMode::parse(&column.grouping) {
            self.grouping_mode = grouping;
        }
    }

    pub(super) fn apply_pending_workspace_inputs(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(search) = self.pending_workspace_search.take() {
            self.search
                .update(cx, |input, cx| input.set_value(&search, window, cx));
        }
        if let Some(address) = self.pending_workspace_address.take() {
            self.address
                .update(cx, |input, cx| input.set_value(&address, window, cx));
        }
        if let Some((date_from, date_to)) = self.pending_workspace_dates.take() {
            self.date_from.update(cx, |picker, cx| {
                picker.set_date(
                    gpui_kit::component::calendar::Date::Single(date_from),
                    window,
                    cx,
                )
            });
            self.date_to.update(cx, |picker, cx| {
                picker.set_date(
                    gpui_kit::component::calendar::Date::Single(date_to),
                    window,
                    cx,
                )
            });
        }
    }

    #[tracing::instrument(skip_all)]
    pub(super) fn reload(&mut self) {
        self.reload_state(true);
    }

    fn reload_state(&mut self, persist: bool) {
        self.snapshot_generation += 1;
        self.visible_demand = None;
        self.service.viewport_request.fetch_add(1, Ordering::AcqRel);
        let service = Arc::clone(&self.service);
        let query = self.filter.to_query();
        let mode = self.grouping_mode;
        self.snapshot_read.set_wake(self.service.wake.clone());
        let offset = if persist {
            0
        } else {
            self.snapshot.as_ref().map(|s| s.window_offset).unwrap_or(0)
        };
        self.snapshot_read
            .request(self.snapshot_generation, move || {
                service
                    .load_grouped_window(query, mode, offset)
                    .map_err(|e| e.to_string())
            });
        // Directory chunks and watcher changes update the existing view. Keep
        // rendered rows, rich media and selection until their replacement arrives.
        if !persist {
            return;
        }
        self.inspector_fallback = None;
        self.inspector_read = Default::default();
        self.selection_details_cache = None;
        self.selection_details_read = Default::default();
        self.selected_drag_paths_cache = None;
        self.selected_drag_read = Default::default();
        self.row_layout = Arc::new(Vec::new());
        self.row_layout_key = None;
        self.selection = GallerySelection::default();
        self.selected_directory = None;
        self.scroll.reset(0);
        self.pending_scroll_restore = None;
        if persist {
            self.persist_filter_session();
        }
        self.show_exif = false;
        self.exif_cache = None;
        self.confirm_delete = false;
        self.confirm_bulk_action = false;
        self.confirm_bulk_action_ids = None;
    }

    pub(super) fn persist_filter_session(&self) {
        self.publish_workspace_snapshot();
        // The legacy SessionService is a single-view record. Column-specific
        // state is persisted by ProgramWorkspace, never by competing writers.
        if self.column_index != 0 {
            return;
        }
        let Ok(mut saved) = self.session.snapshot() else {
            return;
        };
        saved.before_descendants_grouping =
            self.before_descendants_grouping.map(|m| m.key().to_owned());
        saved.show_descendants = self.service.show_descendants();
        saved.thumbnails_disabled = !self.service.thumbnails_enabled();
        saved.selected_file_paths = self
            .service
            .media_paths_by_ids(&self.selection.selected_media_ids)
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        saved.search_query = self.filter.search.clone();
        saved.selected_folder_id = self.filter.folder_id;
        saved.selected_path = self.filter.path_prefix.clone();
        saved.sort_order = Some(self.filter.sort);
        saved.media_type_filter = Some(match self.filter.media_type {
            MediaTypeFilter::All => SavedMediaType::All,
            MediaTypeFilter::Photos => SavedMediaType::Photos,
            MediaTypeFilter::Videos => SavedMediaType::Videos,
            MediaTypeFilter::Audio => SavedMediaType::Audio,
        });
        saved.selected_extensions = self.filter.extensions.clone();
        saved.excluded_extensions = self.filter.excluded_extensions.clone();
        saved.date_from = self
            .filter
            .date_from
            .map(|date| date.format("%Y-%m-%d").to_string());
        saved.date_to = self
            .filter
            .date_to
            .map(|date| date.format("%Y-%m-%d").to_string());
        saved.rating_min = self.filter.rating_min;
        saved.rating_max = Some(self.filter.rating_max);
        saved.selected_tag_ids = self.filter.selected_tag_ids.clone();
        saved.excluded_tag_ids = self.filter.excluded_tag_ids.clone();
        saved.selected_media_ids = self.selection.selected_media_ids.clone();
        saved.gallery_scroll = self
            .pending_scroll_restore
            .unwrap_or_else(|| self.scroll_offset());
        saved.thumb_size = Some(
            self.thumb_size
                .clamp(theme::THUMB_SIZE_MIN, theme::THUMB_SIZE_MAX),
        );
        saved.group_by_month =
            Some(self.grouping_mode == photag_plugin_grouping::GroupingMode::Month);
        saved.grouping_mode = Some(self.grouping_mode.key().into());
        saved.gallery_list_layout = self.list_layout;
        saved.collapsed_months = self.collapsed_months.clone();
        let _ = self.session.replace(saved);
    }

    fn publish_workspace_snapshot(&self) {
        let Ok(workspaces) = self
            .cordis
            .try_service::<photag_plugin_workspaces::WorkspaceService>()
        else {
            return;
        };
        let Ok(mut workspace) = workspaces.active() else {
            return;
        };
        if let Some(previous) = self
            .surfaces
            .value(photag_plugin_workspaces::CURRENT_SNAPSHOT_SURFACE_KEY)
            .and_then(|json| {
                serde_json::from_str::<photag_plugin_workspaces::ProgramWorkspace>(&json).ok()
            })
            .filter(|previous| previous.id == workspace.id)
        {
            workspace = previous;
        }
        let runtime_layout = self
            .cordis
            .try_service::<photag_plugin_columns::ColumnLayoutService>()
            .ok()
            .and_then(|service| service.snapshot().ok())
            .map(|state| state.layout)
            .unwrap_or(workspace.layout);
        workspace.layout = runtime_layout;
        workspace
            .columns
            .resize(runtime_layout.column_count(), Default::default());
        if self.column_index >= runtime_layout.column_count() {
            return;
        }
        let column = &mut workspace.columns[self.column_index];
        column.address = self.filter.path_prefix.clone();
        column.folder_id = self.filter.folder_id;
        column.address_history = self.address_history.clone();
        column.address_history_index = self.address_history_index;
        column.search = self.filter.search.clone();
        column.search_in_name = self.filter.search_scopes.filenames;
        column.search_in_path = self.filter.search_scopes.paths;
        column.search_in_tags = self.filter.search_scopes.tags;
        column.search_in_time = self.filter.search_scopes.time;
        column.media_type = match self.filter.media_type {
            MediaTypeFilter::All => "all",
            MediaTypeFilter::Photos => "photos",
            MediaTypeFilter::Videos => "videos",
            MediaTypeFilter::Audio => "audio",
        }
        .into();
        column.included_tag_ids = self.filter.selected_tag_ids.clone();
        column.excluded_tag_ids = self.filter.excluded_tag_ids.clone();
        column.included_extensions = self.filter.extensions.clone();
        column.excluded_extensions = self.filter.excluded_extensions.clone();
        column.date_from = self
            .filter
            .date_from
            .map(|date| date.format("%Y-%m-%d").to_string());
        column.date_to = self
            .filter
            .date_to
            .map(|date| date.format("%Y-%m-%d").to_string());
        column.rating_min = self.filter.rating_min.clamp(0, 5) as u8;
        column.rating_max = self.filter.rating_max.clamp(0, 5) as u8;
        column.sort_order = format!("{:?}", self.filter.sort).to_ascii_lowercase();
        column.grouping = self.grouping_mode.key().into();
        column.list_layout = self.list_layout;
        column.show_subfolders = self.service.show_descendants();
        column.thumbnails_enabled = self.service.thumbnails_enabled();
        column.thumbnail_size = self.thumb_size;
        column.selected_file_paths = self
            .service
            .media_paths_by_ids(&self.selection.selected_media_ids)
            .unwrap_or_default()
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect();
        column.scroll_offset = self
            .pending_scroll_restore
            .unwrap_or_else(|| self.scroll_offset())
            .max(0.0);
        if let Ok(json) = serde_json::to_string(&workspace) {
            let _ = self.surfaces.set_value(
                photag_plugin_workspaces::CURRENT_SNAPSHOT_SURFACE_KEY,
                &json,
            );
        }
    }
}
