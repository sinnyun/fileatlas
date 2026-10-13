//! Native GPUI rendering for the windowed media gallery.

mod bulk_actions;
mod card;
mod controls;
mod file_actions;
mod filter_actions;
mod keyboard;
mod tagging_picker;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::{future::Future, path::PathBuf};

use ::gpui::InteractiveElement as _;
use ::gpui::StatefulInteractiveElement as _;
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
            let bulk_inputs = [
                "From YYYY-MM-DD",
                "To YYYY-MM-DD",
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
    rename_input: Entity<InputState>,
    bulk_inputs: Vec<Entity<InputState>>,
    bulk_action: Option<BulkActionKind>,
    bulk_source: BulkActionSource,
    confirm_bulk_action: bool,
    focus_handle: FocusHandle,
    filter: GalleryFilter,
    selection: GallerySelection,
    scroll: ScrollHandle,
    thumb_size: f32,
    group_by_month: bool,
    drag_allowed: Arc<AtomicBool>,
    video_preview_open: bool,
    show_exif: bool,
    exif_cache: Option<(i64, Result<Vec<(String, String)>, String>)>,
    operation_status: Option<String>,
    confirm_delete: bool,
    snapshot: Result<GallerySnapshot, String>,
    _subscriptions: Vec<Subscription>,
}

impl GalleryView {
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
        rename_input: Entity<InputState>,
        bulk_inputs: Vec<Entity<InputState>>,
        cx: &mut Context<Self>,
    ) -> Self {
        let saved = session.snapshot().unwrap_or_default();
        let mut filter = GalleryFilter::default();
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
            if let Some(tag_id) = *selected_tag {
                filter.selected_tag_ids = vec![tag_id];
            }
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
        let scroll = ScrollHandle::new();
        let restored_scroll = if saved.gallery_scroll.is_finite() {
            saved.gallery_scroll.max(0.)
        } else {
            0.
        };
        scroll.set_offset(point(px(0.), px(-restored_scroll)));
        let scroll_for_monitor = scroll.clone();
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
                        let selected_tag = view
                            .selected_tag
                            .0
                            .lock()
                            .map(|selected| selected.iter().copied().collect::<Vec<_>>())
                            .unwrap_or_default();
                        let selected_path = view
                            .selected_path
                            .0
                            .lock()
                            .map(|selected| selected.clone())
                            .unwrap_or(None);
                        let changed = view.filter.folder_id != selected
                            || view.filter.selected_tag_ids != selected_tag
                            || view.filter.path_prefix != selected_path;
                        let revision = view.index_revision.0.load(Ordering::Acquire);
                        let index_changed = revision != observed_revision;
                        observed_revision = revision;
                        if changed || index_changed {
                            view.filter.folder_id = selected;
                            view.filter.selected_tag_ids = selected_tag;
                            view.filter.path_prefix = selected_path;
                            view.reload();
                            cx.notify();
                        }
                        let current_scroll = (-f32::from(scroll_for_monitor.offset().y)).max(0.);
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
            rename_input,
            bulk_inputs,
            bulk_action: None,
            bulk_source: BulkActionSource::WholeLibrary,
            confirm_bulk_action: false,
            focus_handle,
            filter,
            selection,
            scroll,
            thumb_size: saved
                .thumb_size
                .filter(|size| size.is_finite() && (100.0..=300.0).contains(size))
                .unwrap_or(160.),
            group_by_month: saved.group_by_month.unwrap_or(true),
            drag_allowed: Arc::new(AtomicBool::new(true)),
            video_preview_open: false,
            show_exif: false,
            exif_cache: None,
            operation_status: None,
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
        self.selection = GallerySelection::default();
        self.scroll.set_offset(point(px(0.), px(0.)));
        self.persist_filter_session();
        self.show_exif = false;
        self.exif_cache = None;
        self.confirm_delete = false;
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
        saved.gallery_scroll = (-f32::from(self.scroll.offset().y)).max(0.);
        saved.thumb_size = Some(self.thumb_size.clamp(100., 300.));
        saved.group_by_month = Some(self.group_by_month);
        let _ = self.session.replace(saved);
    }

    fn select_media(
        &mut self,
        media_id: i64,
        modifiers: Modifiers,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        let Ok(snapshot) = &self.snapshot else {
            return;
        };
        self.selection.select_click(
            &snapshot.skeletons,
            media_id,
            SelectionModifiers {
                ctrl: modifiers.control,
                alt: modifiers.alt,
                shift: modifiers.shift,
            },
        );
        self.show_exif = false;
        self.exif_cache = None;
        self.confirm_delete = false;
        self.persist_filter_session();
        cx.notify();
    }

    fn toggle_exif(&mut self, cx: &mut Context<Self>) {
        self.show_exif = !self.show_exif;
        if self.show_exif {
            let selected_path = self
                .selection
                .selected_media_ids
                .first()
                .and_then(|&media_id| {
                    self.snapshot
                        .as_ref()
                        .ok()?
                        .media
                        .iter()
                        .find(|media| media.id == media_id)
                        .map(|media| (media_id, media.full_path.clone()))
                });
            if let Some((media_id, path)) = selected_path {
                self.exif_cache = Some((
                    media_id,
                    card::read_exif_fields(&path).map_err(str::to_string),
                ));
            }
        }
        cx.notify();
    }

    fn start_operation<F>(&mut self, operation: F, refresh: bool, cx: &mut Context<Self>)
    where
        F: Future<Output = String> + Send + 'static,
    {
        self.operation_status = Some("Working…".to_string());
        self.confirm_delete = false;
        let operation = self.runtime.spawn(operation);
        cx.spawn(async move |this, cx| {
            let status = operation
                .await
                .unwrap_or_else(|error| format!("Operation task failed: {error}"));
            let _ = this.update(cx, |this, cx| {
                if refresh {
                    this.reload();
                }
                this.operation_status = Some(status);
                cx.notify();
            });
        })
        .detach();
    }

    fn open_selected(&mut self, reveal: bool, cx: &mut Context<Self>) {
        let Some(&media_id) = self.selection.selected_media_ids.first() else {
            return;
        };
        let operations = Arc::clone(&self.file_operations);
        self.start_operation(
            async move {
                let result = if reveal {
                    operations.reveal_in_file_manager(media_id).await
                } else {
                    operations.open_external(media_id).await
                };
                result.map_or_else(
                    |error| format!("File action failed: {error}"),
                    |_| {
                        if reveal {
                            "Opened containing folder.".to_string()
                        } else {
                            "Opened with the system default application.".to_string()
                        }
                    },
                )
            },
            false,
            cx,
        );
    }

    fn preview_selected(&mut self, cx: &mut Context<Self>) {
        let Some(&media_id) = self.selection.selected_media_ids.first() else {
            return;
        };
        self.preview_media(media_id, cx);
    }

    fn preview_media(&mut self, media_id: i64, cx: &mut Context<Self>) {
        match self.service.media_by_id(media_id) {
            Ok(Some(media)) if media.media_type == "image" => {
                let filmstrip = self
                    .snapshot
                    .as_ref()
                    .map(|snapshot| {
                        let image_ids = snapshot
                            .skeletons
                            .iter()
                            .filter(|item| !item.is_video && !item.is_audio)
                            .map(|item| item.id)
                            .collect::<Vec<_>>();
                        self.service
                            .media_paths_by_ids(&image_ids)
                            .unwrap_or_default()
                    })
                    .unwrap_or_default();
                self.preview.set_filmstrip(filmstrip);
                match self.preview.open(media.full_path) {
                    Ok(()) => self.operation_status = Some("Image preview opened.".into()),
                    Err(error) => self.operation_status = Some(error.to_string()),
                }
            }
            Ok(Some(media)) if media.media_type == "video" => {
                match self.playback.open(media.full_path) {
                    Ok(()) => {
                        self.video_preview_open = true;
                        self.operation_status = Some("Video playback opened.".into());
                    }
                    Err(error) => self.operation_status = Some(error.to_string()),
                }
            }
            Ok(Some(_)) => {
                self.operation_status = Some("Preview supports images and video.".into())
            }
            Ok(None) => self.operation_status = Some("Selected media is no longer indexed.".into()),
            Err(error) => self.operation_status = Some(error.to_string()),
        }
        cx.notify();
    }

    fn close_video_preview(&mut self, cx: &mut Context<Self>) {
        self.playback.close();
        self.video_preview_open = false;
        self.operation_status = Some("Returned to the gallery.".into());
        cx.notify();
    }

    fn apply_rating(&mut self, rating: u8, cx: &mut Context<Self>) {
        let media_ids = self.selection.selected_media_ids.clone();
        self.apply_rating_to(&media_ids, rating);
        cx.notify();
    }

    pub(super) fn apply_rating_to(&mut self, media_ids: &[i64], rating: u8) {
        if media_ids.is_empty() {
            return;
        }
        let old_rating = media_ids
            .first()
            .and_then(|id| self.service.media_by_id(*id).ok().flatten())
            .map(|media| media.rating);
        match self.service.apply_rating(media_ids, rating) {
            Ok(()) => {
                let new_rating = i32::from(rating.min(5));
                if let (Some(history), Some(old_rating)) = (&self.history, old_rating) {
                    if old_rating != new_rating {
                        history.push(UndoEntry::RatingChanged {
                            media_ids: media_ids.to_vec(),
                            old_rating,
                            new_rating,
                        });
                    }
                }
                self.reload();
            }
            Err(error) => self.snapshot = Err(error.to_string()),
        }
    }

    fn set_media_type(&mut self, media_type: MediaTypeFilter, cx: &mut Context<Self>) {
        if self.filter.media_type != media_type {
            self.filter.media_type = media_type;
            self.reload();
            cx.notify();
        }
    }

    fn toggle_search_scope(&mut self, scope: SearchScope, cx: &mut Context<Self>) {
        self.filter.toggle_search_scope(scope);
        self.reload();
        cx.notify();
    }

    fn set_sort(&mut self, sort: SortOrder, cx: &mut Context<Self>) {
        if self.filter.sort != sort {
            self.filter.sort = sort;
            self.reload();
            cx.notify();
        }
    }

    fn set_rating_bound(&mut self, minimum: bool, value: i32, cx: &mut Context<Self>) {
        let old_range = (self.filter.rating_min, self.filter.rating_max);
        self.filter.set_rating_bound(minimum, value);
        if old_range == (self.filter.rating_min, self.filter.rating_max) {
            return;
        }
        self.reload();
        cx.notify();
    }

    fn cycle_extension(&mut self, extension: String, cx: &mut Context<Self>) {
        self.filter.cycle_extension(&extension);
        self.reload();
        cx.notify();
    }

    fn clear_extensions(&mut self, cx: &mut Context<Self>) {
        self.filter.clear_extensions();
        self.reload();
        cx.notify();
    }

    fn load_more(&mut self) {
        let offset = match &self.snapshot {
            Ok(snapshot) => snapshot.media.len(),
            Err(_) => return,
        };
        match self.service.load_page(self.filter.to_query(), offset) {
            Ok(page) => {
                if let Ok(snapshot) = &mut self.snapshot {
                    snapshot.media.extend(page.media);
                    snapshot.media_tags.extend(page.media_tags);
                }
            }
            Err(error) => self.snapshot = Err(error.to_string()),
        }
    }

    pub(super) fn scroll_to_gallery_item(&self, index: usize) {
        let headers_before_target = if self.group_by_month
            && matches!(
                self.filter.sort,
                SortOrder::NewestFirst | SortOrder::OldestFirst
            ) {
            self.snapshot
                .as_ref()
                .map(|snapshot| {
                    let mut previous = None;
                    snapshot
                        .media
                        .iter()
                        .take(index)
                        .filter_map(|media| {
                            snapshot
                                .skeletons
                                .iter()
                                .find(|item| item.id == media.id)
                                .and_then(|item| {
                                    chrono::DateTime::<chrono::Utc>::from_timestamp(
                                        item.effective_date,
                                        0,
                                    )
                                })
                                .map(|date| date.format("%B %Y").to_string())
                        })
                        .filter(|month| {
                            let changed = previous.as_ref() != Some(month);
                            previous = Some(month.clone());
                            changed
                        })
                        .count()
                })
                .unwrap_or(0)
        } else {
            0
        };
        self.scroll.scroll_to_item(index + headers_before_target);
    }
}

impl Render for GalleryView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tag_picker = if self.tagging.is_some() && !self.selection.selected_media_ids.is_empty()
        {
            Some(tagging_picker::render(self, cx))
        } else {
            None
        };
        match &self.snapshot {
            Ok(snapshot) => {
                let mut media_cards: Vec<AnyElement> = Vec::with_capacity(snapshot.media.len());
                let selected_drag_paths = if self.selection.selected_media_ids.is_empty() {
                    Vec::new()
                } else {
                    self.service
                        .media_paths_by_ids(&self.selection.selected_media_ids)
                        .unwrap_or_default()
                };
                let mut previous_month = None;
                for media in &snapshot.media {
                    let month = snapshot
                        .skeletons
                        .iter()
                        .find(|item| item.id == media.id)
                        .and_then(|item| {
                            chrono::DateTime::<chrono::Utc>::from_timestamp(item.effective_date, 0)
                        })
                        .map(|date| date.format("%B %Y").to_string());
                    let date_sort = matches!(
                        self.filter.sort,
                        SortOrder::NewestFirst | SortOrder::OldestFirst
                    );
                    if self.group_by_month && date_sort && month != previous_month {
                        if let Some(label) = &month {
                            media_cards.push(
                                div()
                                    .w_full()
                                    .pt_2()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(rgb(theme::TEXT_SECONDARY))
                                    .child(label.clone())
                                    .into_any_element(),
                            );
                        }
                    }
                    previous_month = month;
                    let selected = self.selection.selected_media_ids.contains(&media.id);
                    let drag_paths = if selected && !selected_drag_paths.is_empty() {
                        selected_drag_paths.clone()
                    } else {
                        vec![PathBuf::from(media.full_path.as_str())]
                    };
                    media_cards.push(
                        render_media_card(
                            media,
                            selected,
                            Arc::clone(&self.drag_drop),
                            drag_paths,
                            Arc::clone(&self.drag_allowed),
                            self.thumb_size,
                            cx,
                        )
                        .into_any_element(),
                    );
                }
                let mut grid = div()
                    .id("gallery-grid-root")
                    .track_focus(&self.focus_handle)
                    .on_key_down(
                        cx.listener(|this, event, window, cx| this.handle_key(event, window, cx)),
                    )
                    .v_flex()
                    .size_full()
                    .gap_3()
                    .bg(rgb(theme::SURFACE_PANEL))
                    .p_4()
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .justify_between()
                            .child(div().text_lg().font_semibold().child(
                                if !self.filter.selected_tag_ids.is_empty() {
                                    "Tagged Media"
                                } else if self.filter.folder_id.is_some() {
                                    "Selected Folder"
                                } else {
                                    "All Media"
                                },
                            ))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(theme::TEXT_SECONDARY))
                                    .child(format!("{} items", snapshot.media_count)),
                            ),
                    );
                let mut gallery_options =
                    div()
                        .h_flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(theme::TEXT_SECONDARY))
                                .child("Thumbnail size"),
                        )
                        .child(Button::new("gallery-thumb-smaller").label("−").on_click(
                            cx.listener(|this, _, _, cx| {
                                this.thumb_size = (this.thumb_size - 20.).max(100.);
                                cx.notify();
                            }),
                        ))
                        .child(
                            div()
                                .text_xs()
                                .child(format!("{} px", self.thumb_size as u32)),
                        )
                        .child(Button::new("gallery-thumb-larger").label("+").on_click(
                            cx.listener(|this, _, _, cx| {
                                this.thumb_size = (this.thumb_size + 20.).min(300.);
                                cx.notify();
                            }),
                        ));
                if matches!(
                    self.filter.sort,
                    SortOrder::NewestFirst | SortOrder::OldestFirst
                ) {
                    gallery_options = gallery_options.child(
                        Button::new("gallery-month-groups")
                            .label(if self.group_by_month {
                                "Months: on"
                            } else {
                                "Months: off"
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.group_by_month = !this.group_by_month;
                                cx.notify();
                            })),
                    );
                }
                grid = grid.child(gallery_options);
                if self.video_preview_open {
                    grid = grid.child(
                        Button::new("gallery-return-from-video")
                            .label("Return to gallery")
                            .on_click(cx.listener(|this, _, _, cx| this.close_video_preview(cx))),
                    );
                }
                grid = grid.child(Input::new(&self.search).w_full());
                grid = grid.child(controls::search_scope_row(&self.filter, cx));
                grid = grid.child(controls::clear_filters_button(cx));
                grid = grid.child(controls::active_filter_row(
                    &self.filter,
                    &snapshot.tag_tree,
                    cx,
                ));
                grid = grid.child(controls::media_type_row(&self.filter, cx));
                grid = grid.child(controls::sort_row(&self.filter, cx));
                grid = grid.child(controls::rating_row(&self.filter, cx));
                grid = grid.child(controls::date_row(&self.date_from, &self.date_to));
                grid = grid.child(controls::extension_row(
                    &self.filter,
                    &snapshot.available_extensions,
                    cx,
                ));
                if let Some(tag_picker) = tag_picker {
                    grid = grid.child(tag_picker);
                }
                grid = grid.child(bulk_actions::render(self, cx));
                if let Some(status) = &self.operation_status {
                    grid = grid.child(
                        div()
                            .text_sm()
                            .text_color(rgb(theme::TEXT_SECONDARY))
                            .child(status.clone()),
                    );
                }
                grid = grid.child(div().h_1().w_full().bg(rgb(theme::SURFACE_FAINT)));
                grid = grid.child(
                    div()
                        .id("gallery-media-scroll")
                        .h_flex()
                        .flex_wrap()
                        .flex_1()
                        .content_start()
                        .gap_3()
                        .overflow_y_scroll()
                        .track_scroll(&self.scroll)
                        .overflow_y_scroll()
                        .vertical_scrollbar(&self.scroll)
                        .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                            let delta = f32::from(event.delta.pixel_delta(px(40.)).y);
                            let current = (-f32::from(this.scroll.offset().y)).max(0.);
                            let count = this
                                .snapshot
                                .as_ref()
                                .map(|snapshot| snapshot.media.len())
                                .unwrap_or(0);
                            let columns = 4usize;
                            let loaded_height =
                                (count.div_ceil(columns) as f32) * (this.thumb_size + 76.);
                            if delta < 0. && current >= loaded_height - (this.thumb_size + 76.) * 2.
                            {
                                this.load_more();
                                cx.notify();
                            }
                        }))
                        .children(media_cards),
                );
                if snapshot.media.is_empty() {
                    grid = grid.child(
                        div()
                            .flex_1()
                            .w_full()
                            .items_center()
                            .justify_center()
                            .text_sm()
                            .text_color(rgb(theme::TEXT_SECONDARY))
                            .child("No indexed media in this view."),
                    );
                }
                if !self.selection.selected_media_ids.is_empty() {
                    grid = grid.child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap_1()
                            .child(div().text_sm().child(format!(
                                "{} selected · Rate",
                                self.selection.selected_media_ids.len()
                            )))
                            .children((0..=5).map(|rating| {
                                Button::new(format!("gallery-rating-{rating}"))
                                    .label(format!("{rating}★"))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.apply_rating(rating, cx)
                                    }))
                            }))
                            .child(
                                Button::new("gallery-clear-selection")
                                    .label("Clear")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.selection = GallerySelection::default();
                                        this.persist_filter_session();
                                        cx.notify();
                                    })),
                            ),
                    );
                    let mut actions = div()
                        .v_flex()
                        .gap_2()
                        .rounded_md()
                        .bg(rgb(theme::SURFACE_PANEL))
                        .p_3()
                        .child(div().text_sm().font_semibold().child("File actions"));
                    if self.selection.selected_media_ids.len() == 1 {
                        actions = actions.child(
                            div()
                                .h_flex()
                                .gap_2()
                                .child(
                                    Button::new("gallery-preview-selected")
                                        .label("Preview")
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.preview_selected(cx)),
                                        ),
                                )
                                .child(Button::new("gallery-open-selected").label("Open").on_click(
                                    cx.listener(|this, _, _, cx| this.open_selected(false, cx)),
                                ))
                                .child(
                                    Button::new("gallery-reveal-selected")
                                        .label("Reveal")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.open_selected(true, cx)
                                        })),
                                ),
                        );
                        actions = actions.child(
                            div()
                                .h_flex()
                                .gap_2()
                                .child(Input::new(&self.rename_input).w(px(240.)))
                                .child(
                                    Button::new("gallery-rename-selected")
                                        .label("Rename")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.rename_selected_with_repair(cx)
                                        })),
                                ),
                        );
                    }
                    actions = actions.child(
                        div()
                            .h_flex()
                            .gap_2()
                            .child(
                                Button::new("gallery-copy-selected")
                                    .label("Copy…")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.copy_selected_with_dialog(cx)
                                    })),
                            )
                            .child(
                                Button::new("gallery-move-selected")
                                    .label("Move…")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.move_selected_with_dialog(cx)
                                    })),
                            )
                            .child(
                                Button::new("gallery-delete-selected")
                                    .label("Delete selected")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.request_delete_confirmation(window, cx)
                                    })),
                            ),
                    );
                    grid = grid.child(actions);
                }
                if self.selection.selected_media_ids.len() == 1 {
                    let selected_id = self.selection.selected_media_ids[0];
                    if let Some(media) = snapshot.media.iter().find(|media| media.id == selected_id)
                    {
                        let tags = snapshot
                            .media_tags
                            .get(&selected_id)
                            .map(Vec::as_slice)
                            .unwrap_or_default();
                        let open_count = self.service.open_count(selected_id).unwrap_or(0);
                        let exif_fields = self
                            .exif_cache
                            .as_ref()
                            .filter(|(media_id, _)| *media_id == selected_id)
                            .map(|(_, fields)| fields);
                        grid = grid.child(render_media_details(
                            media,
                            tags,
                            open_count,
                            self.show_exif,
                            exif_fields,
                            cx,
                        ));
                    }
                }
                if (snapshot.media.len() as i64) < snapshot.media_count {
                    grid = grid.child(
                        Button::new("gallery-load-more")
                            .label(format!(
                                "Load more ({} of {})",
                                snapshot.media.len(),
                                snapshot.media_count
                            ))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.load_more();
                                cx.notify();
                            })),
                    );
                }
                grid.into_any_element()
            }
            Err(error) => div()
                .size_full()
                .bg(rgb(theme::SURFACE_PANEL))
                .p_4()
                .text_color(rgb(theme::DANGER))
                .child(format!("Could not load media: {error}"))
                .into_any_element(),
        }
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
