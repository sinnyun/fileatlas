//! Native GPUI rendering for the windowed media gallery.

use std::sync::Arc;

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::*;
use photag_ui_bridge::UiViewContribution;

use crate::{GalleryService, GallerySnapshot, filter::GalleryFilter};

pub(super) fn gallery(service: Arc<GalleryService>) -> UiViewContribution {
    UiViewContribution::new(
        "gallery/media-grid-view",
        "gallery.content",
        0,
        move |window, app| {
            let view_service = Arc::clone(&service);
            let search =
                app.new(|cx| InputState::new(window, cx).placeholder("Search your library"));
            app.new(|cx| GalleryView::new(view_service, search, cx))
                .into()
        },
    )
}

struct GalleryView {
    service: Arc<GalleryService>,
    search: Entity<InputState>,
    filter: GalleryFilter,
    snapshot: Result<GallerySnapshot, String>,
    _subscriptions: Vec<Subscription>,
}

impl GalleryView {
    fn new(
        service: Arc<GalleryService>,
        search: Entity<InputState>,
        cx: &mut Context<Self>,
    ) -> Self {
        let filter = GalleryFilter::default();
        let snapshot = service
            .load(filter.to_query())
            .map_err(|error| error.to_string());
        let _subscriptions = vec![cx.subscribe(&search, |this, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                this.filter.search = this.search.read(cx).value().to_string();
                this.reload();
                cx.notify();
            }
        })];
        Self {
            service,
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
}

impl Render for GalleryView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
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
                            .child(div().text_lg().font_semibold().child("All Media"))
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
    div()
        .v_flex()
        .w_48()
        .h(px(160.))
        .justify_between()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x343943))
        .bg(rgb(0x20232b))
        .p_3()
        .child(div().text_sm().font_medium().child(media.filename.clone()))
        .child(
            div()
                .text_xs()
                .text_color(rgb(0x9aa0ad))
                .child(format!("{}  ·  ★ {}", media.media_type, media.rating)),
        )
}
