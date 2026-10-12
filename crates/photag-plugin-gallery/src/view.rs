//! Native GPUI rendering for the windowed media gallery.

mod bulk_actions;
mod card;
mod controls;
mod filter_actions;
mod keyboard;

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::{future::Future, path::PathBuf};

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::*;
use photag_plugin_database::db::queries::SortOrder;
use photag_plugin_file_operations::FileOperationsService;
use photag_plugin_preview::PreviewService;
use photag_plugin_session::{MediaTypeFilter as SavedMediaType, SessionService};
use photag_ui_bridge::UiViewContribution;

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
    preview: Arc<PreviewService>,
    runtime: tokio::runtime::Handle,
    selected_folder: Arc<SelectedFolder>,
    selected_tag: Arc<SelectedTag>,
    selected_path: Arc<SelectedPath>,
    index_revision: Arc<IndexRevision>,
    session: Arc<SessionService>,
) -> UiViewContribution {
    UiViewContribution::new(
        "gallery/media-grid-view",
        "gallery.content",
        0,
        move |window, app| {
            let view_service = Arc::clone(&service);
            let view_file_operations = Arc::clone(&file_operations);
            let view_preview = Arc::clone(&preview);
            let view_runtime = runtime.clone();
            let view_selection = Arc::clone(&selected_folder);
            let view_tag_selection = Arc::clone(&selected_tag);
            let view_path_selection = Arc::clone(&selected_path);
            let view_index_revision = Arc::clone(&index_revision);
            let view_session = Arc::clone(&session);
            let saved = view_session.snapshot().unwrap_or_default();
            let search =
                app.new(|cx| InputState::new(window, cx).placeholder("Search your library"));
            let date_from =
                app.new(|cx| InputState::new(window, cx).placeholder("From YYYY-MM-DD"));
            let date_to = app.new(|cx| InputState::new(window, cx).placeholder("To YYYY-MM-DD"));
            search.update(app, |input, cx| {
                input.set_value(&saved.search_query, window, cx)
            });
            date_from.update(app, |input, cx| {
                input.set_value(saved.date_from.as_deref().unwrap_or_default(), window, cx)
            });
            date_to.update(app, |input, cx| {
                input.set_value(saved.date_to.as_deref().unwrap_or_default(), window, cx)
            });
            let rename_input =
                app.new(|cx| InputState::new(window, cx).placeholder("New filename"));
            let destination_input =
                app.new(|cx| InputState::new(window, cx).placeholder("Destination folder path"));
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
                    view_preview,
                    view_runtime,
                    view_selection,
                    view_tag_selection,
                    view_path_selection,
                    view_index_revision,
                    view_session,
                    search,
                    date_from,
                    date_to,
                    rename_input,
                    destination_input,
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
    preview: Arc<PreviewService>,
    runtime: tokio::runtime::Handle,
    selected_folder: Arc<SelectedFolder>,
    selected_tag: Arc<SelectedTag>,
    selected_path: Arc<SelectedPath>,
    index_revision: Arc<IndexRevision>,
    session: Arc<SessionService>,
    search: Entity<InputState>,
    date_from: Entity<InputState>,
    date_to: Entity<InputState>,
    rename_input: Entity<InputState>,
    destination_input: Entity<InputState>,
    bulk_inputs: Vec<Entity<InputState>>,
    bulk_action: Option<BulkActionKind>,
    bulk_source: BulkActionSource,
    confirm_bulk_action: bool,
    focus_handle: FocusHandle,
    filter: GalleryFilter,
    selection: GallerySelection,
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
        preview: Arc<PreviewService>,
        runtime: tokio::runtime::Handle,
        selected_folder: Arc<SelectedFolder>,
        selected_tag: Arc<SelectedTag>,
        selected_path: Arc<SelectedPath>,
        index_revision: Arc<IndexRevision>,
        session: Arc<SessionService>,
        search: Entity<InputState>,
        date_from: Entity<InputState>,
        date_to: Entity<InputState>,
        rename_input: Entity<InputState>,
        destination_input: Entity<InputState>,
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
            cx.subscribe(&search, |this, _, event, cx| {
                if matches!(event, InputEvent::Change) {
                    this.filter.search = this.search.read(cx).value().to_string();
                    this.reload();
                    cx.notify();
                }
            }),
            cx.subscribe(&date_from, |this, _, event, cx| {
                if matches!(event, InputEvent::Change) {
                    let input = this.date_from.read(cx).value().to_string();
                    if let Some(date) = parse_date_input(&input) {
                        this.filter.date_from = date;
                        this.reload();
                        cx.notify();
                    }
                }
            }),
            cx.subscribe(&date_to, |this, _, event, cx| {
                if matches!(event, InputEvent::Change) {
                    let input = this.date_to.read(cx).value().to_string();
                    if let Some(date) = parse_date_input(&input) {
                        this.filter.date_to = date;
                        this.reload();
                        cx.notify();
                    }
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
        _subscriptions.push(cx.subscribe(&destination_input, |_, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
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
            preview,
            runtime,
            selected_folder,
            selected_tag,
            selected_path,
            index_revision,
            session,
            search,
            date_from,
            date_to,
            rename_input,
            destination_input,
            bulk_inputs,
            bulk_action: None,
            bulk_source: BulkActionSource::WholeLibrary,
            confirm_bulk_action: false,
            focus_handle,
            filter,
            selection: GallerySelection::default(),
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
        match self.service.media_by_id(media_id) {
            Ok(Some(media)) if media.media_type == "image" => {
                match self.preview.open(media.full_path) {
                    Ok(()) => self.operation_status = Some("Image preview opened.".into()),
                    Err(error) => self.operation_status = Some(error.to_string()),
                }
            }
            Ok(Some(_)) => {
                self.operation_status = Some("Preview currently supports images only.".into())
            }
            Ok(None) => self.operation_status = Some("Selected media is no longer indexed.".into()),
            Err(error) => self.operation_status = Some(error.to_string()),
        }
        cx.notify();
    }

    fn rename_selected(&mut self, cx: &mut Context<Self>) {
        if self.selection.selected_media_ids.len() != 1 {
            return;
        }
        let media_id = self.selection.selected_media_ids[0];
        let new_filename = self.rename_input.read(cx).value().to_string();
        let operations = Arc::clone(&self.file_operations);
        let gallery = Arc::clone(&self.service);
        self.start_operation(
            async move {
                match operations.rename(media_id, new_filename.clone()).await {
                    Err(error) => format!("Rename failed: {error}"),
                    Ok(()) => match gallery.media_by_id(media_id) {
                        Ok(Some(media)) if media.filename == new_filename => {
                            format!("Renamed to {}.", media.filename)
                        }
                        Ok(_) => {
                            "Rename was not applied; check the filename and collisions.".into()
                        }
                        Err(error) => format!("Rename completed, but refresh failed: {error}"),
                    },
                }
            },
            true,
            cx,
        );
    }

    fn copy_selected(&mut self, cx: &mut Context<Self>) {
        let ids = self.selection.selected_media_ids.clone();
        let destination = self.destination_input.read(cx).value().to_string();
        if ids.is_empty() || destination.trim().is_empty() {
            self.operation_status = Some("Enter a destination folder path first.".into());
            cx.notify();
            return;
        }
        let operations = Arc::clone(&self.file_operations);
        self.start_operation(
            async move {
                match operations
                    .bulk_copy_to_folder(ids.clone(), PathBuf::from(destination))
                    .await
                {
                    Err(error) => format!("Copy failed: {error}"),
                    Ok(outcomes) => format_bulk_outcome(
                        "Copied",
                        ids.len(),
                        outcomes.iter().filter(|outcome| outcome.copied).count(),
                        outcomes.iter().filter_map(|outcome| {
                            outcome
                                .error
                                .as_ref()
                                .map(|error| (outcome.media_id, error))
                        }),
                    ),
                }
            },
            false,
            cx,
        );
    }

    fn move_selected(&mut self, cx: &mut Context<Self>) {
        let ids = self.selection.selected_media_ids.clone();
        let destination = self.destination_input.read(cx).value().to_string();
        if ids.is_empty() || destination.trim().is_empty() {
            self.operation_status = Some("Enter a destination folder path first.".into());
            cx.notify();
            return;
        }
        let operations = Arc::clone(&self.file_operations);
        self.start_operation(
            async move {
                match operations
                    .bulk_move_into_folder(ids.clone(), PathBuf::from(destination))
                    .await
                {
                    Err(error) => format!("Move failed: {error}"),
                    Ok(outcomes) => format_bulk_outcome(
                        "Moved",
                        ids.len(),
                        outcomes.iter().filter(|outcome| outcome.moved).count(),
                        outcomes.iter().filter_map(|outcome| {
                            outcome
                                .error
                                .as_ref()
                                .map(|error| (outcome.media_id, error))
                        }),
                    ),
                }
            },
            true,
            cx,
        );
    }

    fn delete_selected(&mut self, cx: &mut Context<Self>) {
        if !self.confirm_delete {
            self.confirm_delete = true;
            self.operation_status =
                Some("Click Delete again to send selected files to Recycle Bin.".into());
            cx.notify();
            return;
        }
        let ids = self.selection.selected_media_ids.clone();
        if ids.is_empty() {
            return;
        }
        let operations = Arc::clone(&self.file_operations);
        self.start_operation(
            async move {
                match operations.bulk_delete_to_recycle_bin(ids.clone()).await {
                    Err(error) => format!("Delete failed: {error}"),
                    Ok(outcomes) => {
                        let deleted = outcomes
                            .iter()
                            .filter(|outcome| outcome.removed_from_database)
                            .count();
                        let failures = outcomes.iter().filter_map(|outcome| {
                            outcome
                                .error
                                .as_ref()
                                .map(|error| (outcome.media_id, error))
                        });
                        format_bulk_outcome("Removed", ids.len(), deleted, failures)
                    }
                }
            },
            true,
            cx,
        );
    }

    fn apply_rating(&mut self, rating: u8, cx: &mut Context<Self>) {
        match self
            .service
            .apply_rating(&self.selection.selected_media_ids, rating)
        {
            Ok(()) => {
                self.reload();
                cx.notify();
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
}

fn format_bulk_outcome<'a>(
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

impl Render for GalleryView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match &self.snapshot {
            Ok(snapshot) => {
                let mut media_cards = Vec::with_capacity(snapshot.media.len());
                for media in &snapshot.media {
                    let selected = self.selection.selected_media_ids.contains(&media.id);
                    media_cards.push(render_media_card(media, selected, cx));
                }
                let mut grid = div()
                    .id("gallery-grid-root")
                    .track_focus(&self.focus_handle)
                    .on_key_down(cx.listener(|this, event, _, cx| this.handle_key(event, cx)))
                    .v_flex()
                    .size_full()
                    .gap_3()
                    .bg(rgb(0x181a20))
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
                                    .text_color(rgb(0x9aa0ad))
                                    .child(format!("{} items", snapshot.media_count)),
                            ),
                    );
                grid = grid.child(Input::new(&self.search).w_full());
                grid = grid.child(controls::search_scope_row(&self.filter, cx));
                grid = grid.child(controls::clear_filters_button(cx));
                grid = grid.child(controls::active_filter_row(&self.filter, cx));
                grid = grid.child(controls::media_type_row(&self.filter, cx));
                grid = grid.child(controls::sort_row(&self.filter, cx));
                grid = grid.child(controls::rating_row(&self.filter, cx));
                grid = grid.child(controls::date_row(&self.date_from, &self.date_to));
                grid = grid.child(controls::extension_row(
                    &self.filter,
                    &snapshot.available_extensions,
                    cx,
                ));
                grid = grid.child(bulk_actions::render(self, cx));
                grid = grid.child(div().h_1().w_full().bg(rgb(0x343943)));
                grid = grid.child(
                    div()
                        .h_flex()
                        .flex_wrap()
                        .flex_1()
                        .content_start()
                        .gap_3()
                        .overflow_y_scrollbar()
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
                            .text_color(rgb(0x9aa0ad))
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
                                        cx.notify();
                                    })),
                            ),
                    );
                    let mut actions = div()
                        .v_flex()
                        .gap_2()
                        .rounded_md()
                        .bg(rgb(0x20232b))
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
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.rename_selected(cx)),
                                        ),
                                ),
                        );
                    }
                    actions = actions.child(
                        div()
                            .h_flex()
                            .gap_2()
                            .child(Input::new(&self.destination_input).w(px(320.)))
                            .child(
                                Button::new("gallery-copy-selected")
                                    .label("Copy to folder")
                                    .on_click(cx.listener(|this, _, _, cx| this.copy_selected(cx))),
                            )
                            .child(
                                Button::new("gallery-move-selected")
                                    .label("Move to folder")
                                    .on_click(cx.listener(|this, _, _, cx| this.move_selected(cx))),
                            )
                            .child(
                                Button::new("gallery-delete-selected")
                                    .label(if self.confirm_delete {
                                        "Confirm Recycle Bin delete"
                                    } else {
                                        "Delete selected"
                                    })
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.delete_selected(cx)),
                                    ),
                            ),
                    );
                    if let Some(status) = &self.operation_status {
                        actions = actions.child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x9aa0ad))
                                .child(status.clone()),
                        );
                    }
                    actions = actions.child(div().text_xs().text_color(rgb(0x747b89)).child(
                        "Enter a destination folder path; native folder dialogs are still pending.",
                    ));
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
                .bg(rgb(0x181a20))
                .p_4()
                .text_color(rgb(0xef7777))
                .child(format!("Could not load media: {error}"))
                .into_any_element(),
        }
    }
}
