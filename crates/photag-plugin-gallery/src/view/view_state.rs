//! GalleryView construction, session restore/persist and reload.

use super::*;

pub(super) struct GalleryViewInputs {
    pub tag_picker_search: Entity<InputState>,
    pub search: Entity<InputState>,
    pub date_from: Entity<DatePickerState>,
    pub date_to: Entity<DatePickerState>,
    pub date_calendar: Entity<CalendarState>,
    pub bulk_date_from: Entity<DatePickerState>,
    pub bulk_date_to: Entity<DatePickerState>,
    pub rename_input: Entity<InputState>,
    pub bulk_inputs: Vec<Entity<InputState>>,
}

pub(super) fn build_inputs(
    window: &mut Window,
    app: &mut App,
    session: &Arc<SessionService>,
) -> GalleryViewInputs {
    let saved = session.snapshot().unwrap_or_default();
    let search = app.new(|cx| {
        InputState::new(window, cx).placeholder(photag_ui_bridge::i18n::tr("Search your library"))
    });
    let date_from = app.new(|cx| DatePickerState::new(window, cx).date_format("%Y-%m-%d"));
    let date_to = app.new(|cx| DatePickerState::new(window, cx).date_format("%Y-%m-%d"));
    let saved_date_from = saved
        .date_from
        .as_deref()
        .and_then(parse_date_input)
        .flatten();
    let saved_date_to = saved
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
        input.set_value(&saved.search_query, window, cx)
    });
    if let Some(date) = saved_date_from {
        date_from.update(app, |picker, cx| picker.set_date(date, window, cx));
    }
    if let Some(date) = saved_date_to {
        date_to.update(app, |picker, cx| picker.set_date(date, window, cx));
    }
    let rename_input = app.new(|cx| {
        InputState::new(window, cx).placeholder(photag_ui_bridge::i18n::tr("New filename"))
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
        date_from,
        date_to,
        date_calendar,
        bulk_date_from,
        bulk_date_to,
        rename_input,
        bulk_inputs,
    }
}

impl GalleryView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        service: Arc<GalleryService>,
        file_operations: Arc<FileOperationsService>,
        drag_drop: Arc<DragDropService>,
        preview: Arc<PreviewService>,
        playback: Arc<MediaPlaybackService>,
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
        inputs: GalleryViewInputs,
        cx: &mut Context<Self>,
    ) -> Self {
        let GalleryViewInputs {
            tag_picker_search,
            search,
            date_from,
            date_to,
            date_calendar,
            bulk_date_from,
            bulk_date_to,
            rename_input,
            bulk_inputs,
        } = inputs;
        let saved = session.snapshot().unwrap_or_default();
        if surfaces.value("gallery.inspector.tab").is_none() {
            let _ = surfaces.set_value("gallery.inspector.tab", "information");
        }
        let initial_inspector_tab = surfaces.value("gallery.inspector.tab");
        let mut filter = GalleryFilter::default();
        if let Some(settings) = &settings
            && let Ok(settings_snapshot) = settings.snapshot()
        {
            filter.search_scopes = settings_snapshot.search_scopes.into();
        }
        filter.search = saved.search_query.clone();
        filter.sort = saved.sort_order.unwrap_or(SortOrder::NewestFirst);
        filter.media_type = match saved.media_type_filter.unwrap_or(SavedMediaType::All) {
            SavedMediaType::All => MediaTypeFilter::All,
            SavedMediaType::Photos => MediaTypeFilter::Photos,
            SavedMediaType::Videos => MediaTypeFilter::Videos,
            SavedMediaType::Audio => MediaTypeFilter::Audio,
        };
        filter.extensions = saved.selected_extensions.clone();
        filter.excluded_extensions = saved.excluded_extensions.clone();
        filter.date_from = saved
            .date_from
            .as_deref()
            .and_then(parse_date_input)
            .flatten();
        filter.date_to = saved
            .date_to
            .as_deref()
            .and_then(parse_date_input)
            .flatten();
        filter.rating_min = saved.rating_min.clamp(0, 5);
        filter.rating_max = saved.rating_max.unwrap_or(5).clamp(filter.rating_min, 5);
        filter.selected_tag_ids = saved.selected_tag_ids.clone();
        filter.excluded_tag_ids = saved.excluded_tag_ids.clone();
        if let Ok(mut selected_tags) = selected_tag.0.lock() {
            if selected_tags.included.is_empty() && selected_tags.excluded.is_empty() {
                selected_tags.included = saved.selected_tag_ids.clone();
                selected_tags.excluded = saved.excluded_tag_ids.clone();
            }
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
            .unwrap_or(None)
            .or(saved.selected_folder_id);
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
        let snapshot = service
            .load(filter.to_query())
            .map_err(|error| error.to_string());
        let mut selection = GallerySelection::default();
        if let Ok(snapshot) = &snapshot {
            let mut restored_ids = std::collections::HashSet::new();
            selection.selected_media_ids = saved
                .selected_media_ids
                .iter()
                .copied()
                .filter(|id| {
                    restored_ids.insert(*id)
                        && snapshot.skeletons.iter().any(|media| media.id == *id)
                })
                .collect();
            selection.anchor_id = selection.selected_media_ids.first().copied();
        }
        let scroll = ListState::new(0, ListAlignment::Top, px(240.));
        let restored_scroll = if saved.gallery_scroll.is_finite() {
            saved.gallery_scroll.max(0.)
        } else {
            0.
        };
        let settings_group_by_month = settings
            .as_ref()
            .and_then(|settings| settings.snapshot().ok())
            .map(|settings| settings.show_date_dividers);
        let group_by_month =
            settings_group_by_month.unwrap_or_else(|| saved.group_by_month.unwrap_or(true));
        let initial_writeback_failures = tagging
            .as_ref()
            .map(|_| writeback_failure_signature())
            .unwrap_or_default();
        Self {
            month_motion: std::collections::HashMap::new(),
            hovered_cards: HashSet::new(),
            card_motion: std::collections::HashMap::new(),
            locale_revision: u64::MAX,
            service,
            file_operations,
            drag_drop,
            preview,
            playback,
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
            date_from,
            date_to,
            date_calendar,
            bulk_date_from,
            bulk_date_to,
            bulk_date_from_value: None,
            bulk_date_to_value: None,
            rename_input,
            bulk_inputs,
            bulk_action: None,
            bulk_source: BulkActionSource::WholeLibrary,
            bulk_media_type: MediaTypeFilter::All,
            bulk_tag_ids: Vec::new(),
            bulk_extension_exclude: false,
            confirm_bulk_action: false,
            confirm_bulk_action_ids: None,
            focus_handle,
            filter,
            selection,
            rubberband: None,
            scroll,
            pending_scroll_restore: Some(restored_scroll),
            columns: 4,
            row_layout: Arc::new(Vec::new()),
            row_layout_key: None,
            thumb_size: saved
                .thumb_size
                .filter(|size| {
                    size.is_finite()
                        && (theme::THUMB_SIZE_MIN..=theme::THUMB_SIZE_MAX).contains(size)
                })
                .unwrap_or(theme::THUMB_SIZE_DEFAULT),
            group_by_month,
            show_selection_details: false,
            show_tag_picker: false,
            media_actions_id: None,
            slider_drag: None,
            show_statistics: false,
            collapsed_months: saved.collapsed_months.clone(),
            inspector_expanded: ["basic", "tags"].into_iter().collect(),
            tag_filter_exclude_mode: false,
            extension_exclude_mode: saved.ext_click_mode
                == Some(photag_plugin_session::TagClickMode::Exclude),
            drag_allowed: Arc::new(AtomicBool::new(true)),
            video_preview_open: false,
            show_exif: false,
            exif_cache: None,
            operation_status: None,
            writeback_failure_signature: initial_writeback_failures,
            previous_inspector_tab: initial_inspector_tab,
            confirm_delete: false,
            snapshot,
            observed_index_revision,
            last_saved_scroll: restored_scroll,
            previous_scan_progress: None,
            previous_popover: surfaces.value("gallery.popover"),
            _subscriptions,
        }
    }

    pub(super) fn reload(&mut self) {
        self.persist_filter_session();
        self.snapshot = self
            .service
            .load(self.filter.to_query())
            .map_err(|error| error.to_string());
        self.row_layout = Arc::new(Vec::new());
        self.row_layout_key = None;
        self.selection = GallerySelection::default();
        self.scroll.reset(0);
        self.pending_scroll_restore = None;
        self.persist_filter_session();
        self.show_exif = false;
        self.exif_cache = None;
        self.confirm_delete = false;
        self.confirm_bulk_action = false;
        self.confirm_bulk_action_ids = None;
    }

    pub(super) fn persist_filter_session(&self) {
        let Ok(mut saved) = self.session.snapshot() else {
            return;
        };
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
        saved.gallery_scroll = (-f32::from(self.scroll.scroll_px_offset_for_scrollbar().y)).max(0.);
        saved.thumb_size = Some(
            self.thumb_size
                .clamp(theme::THUMB_SIZE_MIN, theme::THUMB_SIZE_MAX),
        );
        saved.group_by_month = Some(self.group_by_month);
        saved.collapsed_months = self.collapsed_months.clone();
        let _ = self.session.replace(saved);
    }
}
