//! GPUI media tile rendering and click selection.

use gpui_kit::base::StyledExt as _;
use gpui_kit::*;

use super::GalleryView;

pub(super) fn render_media_card(
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
