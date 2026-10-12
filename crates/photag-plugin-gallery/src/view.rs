//! Native GPUI rendering for the windowed media gallery.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::*;
use photag_ui_bridge::UiViewContribution;

use crate::{
    GalleryService, GallerySnapshot, IndexRevision, SelectedFolder, SelectedPath, SelectedTag,
    filter::{GalleryFilter, MediaTypeFilter},
    selection::{GalleryKey, GallerySelection, Modifiers as SelectionModifiers, SelectionEffect},
};

pub(super) fn gallery(
    service: Arc<GalleryService>,
    selected_folder: Arc<SelectedFolder>,
    selected_tag: Arc<SelectedTag>,
    selected_path: Arc<SelectedPath>,
    index_revision: Arc<IndexRevision>,
) -> UiViewContribution {
    UiViewContribution::new(
        "gallery/media-grid-view",
        "gallery.content",
        0,
        move |window, app| {
            let view_service = Arc::clone(&service);
            let view_selection = Arc::clone(&selected_folder);
            let view_tag_selection = Arc::clone(&selected_tag);
            let view_path_selection = Arc::clone(&selected_path);
            let view_index_revision = Arc::clone(&index_revision);
            let search =
                app.new(|cx| InputState::new(window, cx).placeholder("Search your library"));
            app.new(|cx| {
                GalleryView::new(
                    view_service,
                    view_selection,
                    view_tag_selection,
                    view_path_selection,
                    view_index_revision,
                    search,
                    cx,
                )
            })
            .into()
        },
    )
}

struct GalleryView {
    service: Arc<GalleryService>,
    selected_folder: Arc<SelectedFolder>,
    selected_tag: Arc<SelectedTag>,
    selected_path: Arc<SelectedPath>,
    index_revision: Arc<IndexRevision>,
    search: Entity<InputState>,
    focus_handle: FocusHandle,
    filter: GalleryFilter,
    selection: GallerySelection,
    snapshot: Result<GallerySnapshot, String>,
    _subscriptions: Vec<Subscription>,
}

impl GalleryView {
    fn new(
        service: Arc<GalleryService>,
        selected_folder: Arc<SelectedFolder>,
        selected_tag: Arc<SelectedTag>,
        selected_path: Arc<SelectedPath>,
        index_revision: Arc<IndexRevision>,
        search: Entity<InputState>,
        cx: &mut Context<Self>,
    ) -> Self {
        let filter = GalleryFilter::default();
        let focus_handle = cx.focus_handle();
        let _subscriptions = vec![cx.subscribe(&search, |this, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                this.filter.search = this.search.read(cx).value().to_string();
                this.reload();
                cx.notify();
            }
        })];
        let selected = selected_folder
            .0
            .lock()
            .map(|selected| *selected)
            .unwrap_or(None);
        let mut filter = filter;
        filter.folder_id = selected;
        filter.selected_tag_ids = selected_tag
            .0
            .lock()
            .map(|selected| selected.iter().copied().collect())
            .unwrap_or_default();
        filter.path_prefix = selected_path
            .0
            .lock()
            .map(|selected| selected.clone())
            .unwrap_or(None);
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
            selected_folder,
            selected_tag,
            selected_path,
            index_revision,
            search,
            focus_handle,
            filter,
            selection: GallerySelection::default(),
            snapshot,
            _subscriptions,
        }
    }

    fn reload(&mut self) {
        self.snapshot = self
            .service
            .load(self.filter.to_query())
            .map_err(|error| error.to_string());
        self.selection = GallerySelection::default();
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
        cx.notify();
    }

    fn handle_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let key = match event.keystroke.key.as_str() {
            "left" => GalleryKey::Left,
            "right" => GalleryKey::Right,
            "up" => GalleryKey::Up,
            "down" => GalleryKey::Down,
            "home" => GalleryKey::Home,
            "end" => GalleryKey::End,
            "escape" => GalleryKey::Escape,
            "a" if event.keystroke.modifiers.control => GalleryKey::SelectAll,
            "0" => GalleryKey::Rating(0),
            "1" => GalleryKey::Rating(1),
            "2" => GalleryKey::Rating(2),
            "3" => GalleryKey::Rating(3),
            "4" => GalleryKey::Rating(4),
            "5" => GalleryKey::Rating(5),
            _ => return,
        };
        let skeletons = self
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.skeletons.as_slice())
            .unwrap_or_default();
        let effect = self.selection.handle_key(
            skeletons,
            6,
            key,
            SelectionModifiers {
                ctrl: event.keystroke.modifiers.control,
                alt: event.keystroke.modifiers.alt,
                shift: event.keystroke.modifiers.shift,
            },
        );
        match effect {
            SelectionEffect::ApplyRating { media_ids, rating } => {
                if let Err(error) = self.service.apply_rating(&media_ids, rating) {
                    self.snapshot = Err(error.to_string());
                } else {
                    self.reload();
                }
            }
            SelectionEffect::None | SelectionEffect::ScrollTo { .. } => {}
            SelectionEffect::ConfirmDelete { .. } | SelectionEffect::OpenPreview { .. } => {}
        }
        cx.notify();
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
                    )
                    .child(Input::new(&self.search).w_full())
                    .child(div().h_flex().gap_1().children([
                        media_type_button(
                            "all",
                            "All",
                            MediaTypeFilter::All,
                            self.filter.media_type,
                            cx,
                        ),
                        media_type_button(
                            "photos",
                            "Photos",
                            MediaTypeFilter::Photos,
                            self.filter.media_type,
                            cx,
                        ),
                        media_type_button(
                            "videos",
                            "Videos",
                            MediaTypeFilter::Videos,
                            self.filter.media_type,
                            cx,
                        ),
                        media_type_button(
                            "audio",
                            "Audio",
                            MediaTypeFilter::Audio,
                            self.filter.media_type,
                            cx,
                        ),
                    ]))
                    .child(div().h_1().w_full().bg(rgb(0x343943)))
                    .child(
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

fn media_type_button(
    id: &str,
    label: &str,
    media_type: MediaTypeFilter,
    active: MediaTypeFilter,
    cx: &mut Context<GalleryView>,
) -> Button {
    let selected_label = if media_type == active {
        format!("• {label}")
    } else {
        label.to_owned()
    };
    Button::new(format!("gallery-media-type-{id}"))
        .label(selected_label)
        .on_click(cx.listener(move |this, _, _, cx| this.set_media_type(media_type, cx)))
}

fn render_media_card(
    media: &photag_plugin_database::db::queries::MediaItem,
    selected: bool,
    cx: &mut Context<GalleryView>,
) -> impl IntoElement + 'static {
    let media_id = media.id;
    let mut card = div()
        .v_flex()
        .w_48()
        .h(px(190.))
        .justify_between()
        .rounded_md()
        .border_1()
        .border_color(if selected {
            rgb(0x61a9ff)
        } else {
            rgb(0x343943)
        })
        .bg(if selected {
            rgb(0x27374b)
        } else {
            rgb(0x20232b)
        })
        .p_2();
    if let Some(path) = media.thumb_path.as_ref() {
        card = card.child(
            img(std::path::PathBuf::from(path))
                .w_full()
                .h(px(125.))
                .object_fit(ObjectFit::Cover)
                .with_fallback(|| {
                    div()
                        .w_full()
                        .h(px(125.))
                        .bg(rgb(0x181a20))
                        .into_any_element()
                }),
        );
    } else {
        card = card.child(
            div()
                .w_full()
                .h(px(125.))
                .rounded_sm()
                .bg(rgb(0x181a20))
                .items_center()
                .justify_center()
                .text_xs()
                .text_color(rgb(0x696f7c))
                .child(media.media_type.to_uppercase()),
        );
    }
    card.id(format!("gallery-media-{media_id}"))
        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
            this.select_media(media_id, event.modifiers(), window, cx)
        }))
        .child(div().text_sm().font_medium().child(media.filename.clone()))
        .child(
            div()
                .text_xs()
                .text_color(rgb(0x9aa0ad))
                .child(format!("{}  ·  ★ {}", media.media_type, media.rating)),
        )
}
