//! Native GPUI rendering for the windowed media gallery.

mod behavior;
mod bulk_actions;
mod card;
mod controls;
mod file_actions;
mod filter_actions;
mod keyboard;
mod render;
mod statistics;
mod tagging_picker;

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::{future::Future, path::PathBuf};

use ::gpui::InteractiveElement as _;
use ::gpui::{
    Bounds, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point,
    ScrollStrategy, UniformListScrollHandle, point, px, size, uniform_list,
};
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::date_picker::{DatePickerEvent, DatePickerState, DateTime};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::*;
use photag_plugin_database::db::queries::SortOrder;
use photag_plugin_drag_drop::DragDropService;
use photag_plugin_file_operations::FileOperationsService;
use photag_plugin_history::{HistoryService, UndoEntry};
use photag_plugin_media_playback::MediaPlaybackService;
use photag_plugin_preview::PreviewService;
use photag_plugin_session::{MediaTypeFilter as SavedMediaType, SessionService};
use photag_plugin_settings::SettingsService;
use photag_plugin_tagging::TaggingService;
use photag_ui_bridge::UiViewContribution;
use photag_ui_bridge::theme;

use bulk_actions::{BulkActionKind, BulkActionSource};
use card::{render_media_card, render_media_details};

use crate::{
    GalleryService, GallerySnapshot, IndexRevision, SelectedFolder, SelectedPath, SelectedTag,
    filter::{GalleryFilter, MediaTypeFilter, SearchScope, parse_date_input},
    selection::{GallerySelection, Modifiers as SelectionModifiers},
};

fn writeback_failure_signature() -> Vec<(String, String)> {
    photag_plugin_tagging::writeback::failures()
        .into_iter()
        .map(|failure| (failure.path.display().to_string(), failure.error))
        .collect()
}

#[derive(Clone)]
struct GalleryVisualRow {
    media_range: std::ops::Range<usize>,
    month_label: Option<String>,
    collapsed: bool,
}

#[derive(Clone)]
struct GalleryRubberband {
    start: Point<Pixels>,
    end: Point<Pixels>,
    initial_selection: Vec<i64>,
    initial_anchor: Option<i64>,
    add: bool,
    clear_on_click: bool,
    dragged: bool,
}

fn build_gallery_rows(
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
            chrono::DateTime::<chrono::Utc>::from_timestamp(skeletons[index].effective_date, 0)
                .map(|date| date.format("%B %Y").to_string())
        };
        let month = group_by_month.then(|| month_at(start)).flatten();
        let month_label = if group_by_month && month.is_some() && month != previous_month {
            month.clone()
        } else {
            None
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
        });
        previous_month = month;
        start = end;
    }
    rows
}

pub(super) fn gallery(
    service: Arc<GalleryService>,
    file_operations: Arc<FileOperationsService>,
    drag_drop: Arc<DragDropService>,
    preview: Arc<PreviewService>,
    playback: Arc<MediaPlaybackService>,
    runtime: tokio::runtime::Handle,
    selected_folder: Arc<SelectedFolder>,
    selected_tag: Arc<SelectedTag>,
    selected_path: Arc<SelectedPath>,
    index_revision: Arc<IndexRevision>,
    session: Arc<SessionService>,
    history: Option<Arc<HistoryService>>,
    settings: Option<Arc<SettingsService>>,
    tagging: Option<Arc<TaggingService>>,
) -> UiViewContribution {
    UiViewContribution::new(
        "gallery/media-grid-view",
        "gallery.content",
        0,
        move |window, app| {
            let view_service = Arc::clone(&service);
            let view_file_operations = Arc::clone(&file_operations);
            let view_drag_drop = Arc::clone(&drag_drop);
            let view_preview = Arc::clone(&preview);
            let view_playback = Arc::clone(&playback);
            let view_runtime = runtime.clone();
            let view_selection = Arc::clone(&selected_folder);
            let view_tag_selection = Arc::clone(&selected_tag);
            let view_path_selection = Arc::clone(&selected_path);
            let view_index_revision = Arc::clone(&index_revision);
            let view_session = Arc::clone(&session);
            let view_history = history.clone();
            let view_settings = settings.clone();
            let view_tagging = tagging.clone();
            let saved = view_session.snapshot().unwrap_or_default();
            let search =
                app.new(|cx| InputState::new(window, cx).placeholder("Search your library"));
            let date_from = app.new(|cx| DatePickerState::new(window, cx).date_format("%Y-%m-%d"));
            let date_to = app.new(|cx| DatePickerState::new(window, cx).date_format("%Y-%m-%d"));
            let tag_picker_search =
                app.new(|cx| InputState::new(window, cx).placeholder("Find tags to assign"));
            search.update(app, |input, cx| {
                input.set_value(&saved.search_query, window, cx)
            });
            if let Some(date) = saved
                .date_from
                .as_deref()
                .and_then(parse_date_input)
                .flatten()
            {
                date_from.update(app, |picker, cx| picker.set_date(date, window, cx));
            }
            if let Some(date) = saved
                .date_to
                .as_deref()
                .and_then(parse_date_input)
                .flatten()
            {
                date_to.update(app, |picker, cx| picker.set_date(date, window, cx));
            }
            let rename_input =
                app.new(|cx| InputState::new(window, cx).placeholder("New filename"));
            let bulk_date_from =
                app.new(|cx| DatePickerState::new(window, cx).date_format("%Y-%m-%d"));
            let bulk_date_to =
                app.new(|cx| DatePickerState::new(window, cx).date_format("%Y-%m-%d"));
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
            app.new(|cx| {
                GalleryView::new(
                    view_service,
                    view_file_operations,
                    view_drag_drop,
                    view_preview,
                    view_playback,
                    view_runtime,
                    view_selection,
                    view_tag_selection,
                    view_path_selection,
                    view_index_revision,
                    view_session,
                    view_history,
                    view_settings,
                    view_tagging,
                    tag_picker_search,
                    search,
                    date_from,
                    date_to,
                    bulk_date_from,
                    bulk_date_to,
                    rename_input,
                    bulk_inputs,
                    cx,
                )
            })
            .into()
        },
    )
}

struct GalleryView {
    service: Arc<GalleryService>,
    file_operations: Arc<FileOperationsService>,
    drag_drop: Arc<DragDropService>,
    preview: Arc<PreviewService>,
    playback: Arc<MediaPlaybackService>,
    runtime: tokio::runtime::Handle,
    selected_folder: Arc<SelectedFolder>,
    selected_tag: Arc<SelectedTag>,
    selected_path: Arc<SelectedPath>,
    index_revision: Arc<IndexRevision>,
    session: Arc<SessionService>,
    history: Option<Arc<HistoryService>>,
    settings: Option<Arc<SettingsService>>,
    tagging: Option<Arc<TaggingService>>,
    tag_picker_search: Entity<InputState>,
    tag_picker_selected: std::collections::HashSet<i64>,
    search: Entity<InputState>,
    date_from: Entity<DatePickerState>,
    date_to: Entity<DatePickerState>,
    bulk_date_from: Entity<DatePickerState>,
    bulk_date_to: Entity<DatePickerState>,
    bulk_date_from_value: Option<chrono::NaiveDate>,
    bulk_date_to_value: Option<chrono::NaiveDate>,
    rename_input: Entity<InputState>,
    bulk_inputs: Vec<Entity<InputState>>,
    bulk_action: Option<BulkActionKind>,
    bulk_source: BulkActionSource,
    confirm_bulk_action: bool,
    confirm_bulk_action_ids: Option<Vec<i64>>,
    focus_handle: FocusHandle,
    filter: GalleryFilter,
    selection: GallerySelection,
    rubberband: Option<GalleryRubberband>,
    scroll: UniformListScrollHandle,
    columns: usize,
    row_layout: Arc<Vec<GalleryVisualRow>>,
    row_layout_key: Option<(usize, bool, SortOrder, HashSet<String>)>,
    thumb_size: f32,
    group_by_month: bool,
    show_advanced_filters: bool,
    show_statistics: bool,
    collapsed_months: HashSet<String>,
    extension_exclude_mode: bool,
    drag_allowed: Arc<AtomicBool>,
    video_preview_open: bool,
    show_exif: bool,
    exif_cache: Option<(i64, Result<Vec<(String, String)>, String>)>,
    operation_status: Option<String>,
    writeback_failure_signature: Vec<(String, String)>,
    confirm_delete: bool,
    snapshot: Result<GallerySnapshot, String>,
    _subscriptions: Vec<Subscription>,
}

impl GalleryView {
    pub(super) fn set_month_grouping(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.group_by_month = enabled;
        self.row_layout_key = None;
        if let Some(settings) = &self.settings {
            if let Err(error) = settings.update(|settings| settings.show_date_dividers = enabled) {
                self.operation_status =
                    Some(format!("Month grouping preference save failed: {error}"));
            }
        }
        self.persist_filter_session();
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

    fn new(
        service: Arc<GalleryService>,
        file_operations: Arc<FileOperationsService>,
        drag_drop: Arc<DragDropService>,
        preview: Arc<PreviewService>,
        playback: Arc<MediaPlaybackService>,
        runtime: tokio::runtime::Handle,
        selected_folder: Arc<SelectedFolder>,
        selected_tag: Arc<SelectedTag>,
        selected_path: Arc<SelectedPath>,
        index_revision: Arc<IndexRevision>,
        session: Arc<SessionService>,
        history: Option<Arc<HistoryService>>,
        settings: Option<Arc<SettingsService>>,
        tagging: Option<Arc<TaggingService>>,
        tag_picker_search: Entity<InputState>,
        search: Entity<InputState>,
        date_from: Entity<DatePickerState>,
        date_to: Entity<DatePickerState>,
        bulk_date_from: Entity<DatePickerState>,
        bulk_date_to: Entity<DatePickerState>,
        rename_input: Entity<InputState>,
        bulk_inputs: Vec<Entity<InputState>>,
        cx: &mut Context<Self>,
    ) -> Self {
        let saved = session.snapshot().unwrap_or_default();
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
        let mut observed_revision = index_revision.0.load(Ordering::Acquire);
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
        let scroll = UniformListScrollHandle::new();
        let restored_scroll = if saved.gallery_scroll.is_finite() {
            saved.gallery_scroll.max(0.)
        } else {
            0.
        };
        scroll
            .0
            .borrow()
            .base_handle
            .set_offset(point(px(0.), px(-restored_scroll)));
        let scroll_for_monitor = scroll.clone();
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
        let mut last_saved_scroll = restored_scroll;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(200))
                    .await;
                let changed = this
                    .update(cx, |view, cx| {
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
                        let index_changed = revision != observed_revision;
                        observed_revision = revision;
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
                        let current_scroll =
                            (-f32::from(scroll_for_monitor.0.borrow().base_handle.offset().y))
                                .max(0.);
                        if (current_scroll - last_saved_scroll).abs() >= 1.0 {
                            view.persist_filter_session();
                            last_saved_scroll = current_scroll;
                        }
                        true
                    })
                    .unwrap_or(false);
                if !changed {
                    break;
                }
            }
        })
        .detach();
        Self {
            service,
            file_operations,
            drag_drop,
            preview,
            playback,
            runtime,
            selected_folder,
            selected_tag,
            selected_path,
            index_revision,
            session,
            history,
            settings,
            tagging,
            tag_picker_search,
            tag_picker_selected: Default::default(),
            search,
            date_from,
            date_to,
            bulk_date_from,
            bulk_date_to,
            bulk_date_from_value: None,
            bulk_date_to_value: None,
            rename_input,
            bulk_inputs,
            bulk_action: None,
            bulk_source: BulkActionSource::WholeLibrary,
            confirm_bulk_action: false,
            confirm_bulk_action_ids: None,
            focus_handle,
            filter,
            selection,
            rubberband: None,
            scroll,
            columns: 4,
            row_layout: Arc::new(Vec::new()),
            row_layout_key: None,
            thumb_size: saved
                .thumb_size
                .filter(|size| size.is_finite() && (100.0..=300.0).contains(size))
                .unwrap_or(160.),
            group_by_month,
            show_advanced_filters: false,
            show_statistics: false,
            collapsed_months: saved.collapsed_months.clone(),
            extension_exclude_mode: saved.ext_click_mode
                == Some(photag_plugin_session::TagClickMode::Exclude),
            drag_allowed: Arc::new(AtomicBool::new(true)),
            video_preview_open: false,
            show_exif: false,
            exif_cache: None,
            operation_status: None,
            writeback_failure_signature: initial_writeback_failures,
            confirm_delete: false,
            snapshot,
            _subscriptions,
        }
    }

    fn reload(&mut self) {
        self.persist_filter_session();
        self.snapshot = self
            .service
            .load(self.filter.to_query())
            .map_err(|error| error.to_string());
        self.row_layout = Arc::new(Vec::new());
        self.row_layout_key = None;
        self.selection = GallerySelection::default();
        self.scroll
            .0
            .borrow()
            .base_handle
            .set_offset(point(px(0.), px(0.)));
        self.persist_filter_session();
        self.show_exif = false;
        self.exif_cache = None;
        self.confirm_delete = false;
        self.confirm_bulk_action = false;
        self.confirm_bulk_action_ids = None;
    }

    fn persist_filter_session(&self) {
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
        saved.gallery_scroll = (-f32::from(self.scroll.0.borrow().base_handle.offset().y)).max(0.);
        saved.thumb_size = Some(self.thumb_size.clamp(100., 300.));
        saved.group_by_month = Some(self.group_by_month);
        saved.collapsed_months = self.collapsed_months.clone();
        let _ = self.session.replace(saved);
    }
}

pub(super) fn format_bulk_outcome<'a>(
    verb: &str,
    total: usize,
    succeeded: usize,
    failures: impl Iterator<Item = (i64, &'a String)>,
) -> String {
    let failures = failures
        .map(|(media_id, error)| format!("{media_id}: {error}"))
        .collect::<Vec<_>>();
    let mut result = format!("{verb} {succeeded} of {total} selected items.");
    if succeeded < total && failures.is_empty() {
        result.push_str(" Some items were missing or had destination collisions.");
    }
    if !failures.is_empty() {
        result.push_str(" Errors: ");
        result.push_str(&failures.join("; "));
    }
    result
}

impl Drop for GalleryView {
    fn drop(&mut self) {
        self.persist_filter_session();
    }
}
