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
    GalleryService, GallerySnapshot, IndexRevision, SelectedFolder, SelectedTag,
    filter::GalleryFilter,
};

pub(super) fn gallery(
    service: Arc<GalleryService>,
    selected_folder: Arc<SelectedFolder>,
    selected_tag: Arc<SelectedTag>,
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
            let view_index_revision = Arc::clone(&index_revision);
            let search =
                app.new(|cx| InputState::new(window, cx).placeholder("Search your library"));
            app.new(|cx| {
                GalleryView::new(
                    view_service,
                    view_selection,
                    view_tag_selection,
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
    index_revision: Arc<IndexRevision>,
    search: Entity<InputState>,
    filter: GalleryFilter,
    snapshot: Result<GallerySnapshot, String>,
    _subscriptions: Vec<Subscription>,
}

impl GalleryView {
    fn new(
        service: Arc<GalleryService>,
        selected_folder: Arc<SelectedFolder>,
        selected_tag: Arc<SelectedTag>,
        index_revision: Arc<IndexRevision>,
        search: Entity<InputState>,
        cx: &mut Context<Self>,
    ) -> Self {
        let filter = GalleryFilter::default();
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
                        let changed = view.filter.folder_id != selected
                            || view.filter.selected_tag_ids != selected_tag;
                        let revision = view.index_revision.0.load(Ordering::Acquire);
                        let index_changed = revision != observed_revision;
                        observed_revision = revision;
                        if changed || index_changed {
                            view.filter.folder_id = selected;
                            view.filter.selected_tag_ids = selected_tag;
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
            index_revision,
            search,
            filter,
            snapshot,
            _subscriptions,
        }
    }

    fn reload(&mut self) {
        self.snapshot = self
            .service
            .load(self.filter.to_query())
            .map_err(|error| error.to_string());
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
                let mut grid = div()
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
                    .child(div().h_1().w_full().bg(rgb(0x343943)))
                    .child(
                        div()
                            .h_flex()
                            .flex_wrap()
                            .flex_1()
                            .content_start()
                            .gap_3()
                            .overflow_y_scrollbar()
                            .children(snapshot.media.iter().map(render_media_card)),
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
                grid
            }
            Err(error) => div()
                .size_full()
                .bg(rgb(0x181a20))
                .p_4()
                .text_color(rgb(0xef7777))
                .child(format!("Could not load media: {error}")),
        }
    }
}

fn render_media_card(media: &photag_plugin_database::db::queries::MediaItem) -> Div {
    let mut card = div()
        .v_flex()
        .w_48()
        .h(px(190.))
        .justify_between()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x343943))
        .bg(rgb(0x20232b))
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
    card.child(div().text_sm().font_medium().child(media.filename.clone()))
        .child(
            div()
                .text_xs()
                .text_color(rgb(0x9aa0ad))
                .child(format!("{}  ·  ★ {}", media.media_type, media.rating)),
        )
}
