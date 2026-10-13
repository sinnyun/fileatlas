//! Bottom filmstrip plate ported from the pinned source.
//!
//! Source: `vendor/photag-source/src/ui/preview.rs:797-936` (layout, plate and
//! card painting) and `:1695-1708` (activation gate plus the toolbar lift).
use std::sync::Arc;

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use photag_ui_bridge::theme;

use super::PreviewView;
/// `preview.rs:802`: plate height, shared with the toolbar lift so the two
/// stacked bands cannot drift apart.
use super::toolbar::FILMSTRIP_HEIGHT as FILMSTRIP_H;
use crate::PreviewMediaKind;
use crate::classify_media_path;

/// `preview.rs:826`: plate inset from the screen bottom.
const STRIP_PAD: f32 = 8.0;
/// `preview.rs:827-828`: 64 px thumbs with a 4 px gap.
const THUMB_SIZE: f32 = 64.0;
const THUMB_GAP: f32 = 4.0;
/// `preview.rs:890`: card corner radius.
const CARD_ROUND: f32 = 3.0;
/// `preview.rs:853`: plate fill RGBA(20, 20, 25, 210).
const PLATE: u32 = 0x141419d2;
/// `preview.rs:891`: card fill RGBA(40, 40, 50, 255/2).
const CARD: u32 = 0x2828327f;

/// `preview.rs:814-936`: the full-width strip centered on the current item.
/// Returns `None` under the source gate (`preview.rs:1696`): the toggle off, no
/// displayable image, or a fully faded chrome.
pub(super) fn plate(
    view: &mut PreviewView,
    width: f32,
    cx: &mut Context<PreviewView>,
) -> Option<AnyElement> {
    let snapshot = view.service.snapshot();
    let is_image = snapshot.media_kind == Some(PreviewMediaKind::Image);
    let has_display = snapshot.source.is_some() && view.service.display_path().ok()?.is_some();
    if !view.filmstrip_visible() || !is_image || !has_display || view.service.chrome_alpha() <= 0.01
    {
        return None;
    }
    let items = view.service.filmstrip_items();
    let total = items.len();
    if total == 0 {
        return None;
    }
    let cur = snapshot
        .source
        .as_ref()
        .and_then(|source| items.iter().position(|(path, _)| path == source))
        .unwrap_or(0);
    // `preview.rs:833-840`: spread the row across the inner width, centered on
    // the current thumb, with 2 items of headroom on each side.
    let item_w = THUMB_SIZE + THUMB_GAP;
    let inner_w = width - STRIP_PAD * 2.0;
    let visible = ((inner_w / item_w).floor() as usize).max(1) + 2;
    let start = cur.saturating_sub(visible / 2);
    let end = (start + visible + 1).min(total);
    // `preview.rs:861-863`: place thumbs so the current one lands at center.
    let start_x = width * 0.5 - THUMB_SIZE * 0.5 - (cur - start) as f32 * item_w;

    let rows = (start..end)
        .map(|index| {
            let (path, media_id) = items[index].clone();
            let thumbnail = view.service.filmstrip_thumbnail(&path);
            let service = Arc::clone(&view.service);
            let target = path.clone();
            div()
                .id(format!("preview-filmstrip-{index}"))
                .relative()
                .w(px(THUMB_SIZE))
                .h(px(THUMB_SIZE))
                .rounded(px(CARD_ROUND))
                .bg(rgba(CARD))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    window.focus(&this.focus_handle, cx);
                    let _ = service.open_media(target.clone(), media_id);
                    cx.notify();
                }))
                .child(match thumbnail {
                    Some(thumbnail) => img(thumbnail)
                        .w_full()
                        .h_full()
                        .object_fit(ObjectFit::Contain)
                        .into_any_element(),
                    // The source leaves an unloaded card empty; the migrated
                    // decoder has no poster path for some video/audio items, so
                    // a quiet glyph keeps those selectable. See PORTS.md.
                    None => div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(theme::FONT_SIZE_TINY))
                        .text_color(rgba(0xffffff99))
                        .child(match classify_media_path(&path) {
                            PreviewMediaKind::Audio => "♫",
                            PreviewMediaKind::Video => "▶",
                            _ => "…",
                        })
                        .into_any_element(),
                })
                .when(index == cur, |thumb| {
                    thumb.child(
                        div()
                            .absolute()
                            .inset_0()
                            .rounded(px(CARD_ROUND))
                            .border_2()
                            .border_color(rgba(theme::accent())),
                    )
                })
                .into_any_element()
        })
        .collect::<Vec<_>>();

    Some(
        div()
            .absolute()
            .left_0()
            .right_0()
            .bottom(px(STRIP_PAD))
            .h(px(FILMSTRIP_H))
            .bg(rgba(PLATE))
            .child(
                div()
                    .absolute()
                    .left(px(start_x))
                    .top(px((FILMSTRIP_H - THUMB_SIZE) / 2.0))
                    .flex()
                    .gap(px(THUMB_GAP))
                    .children(rows),
            )
            .opacity(view.service.chrome_alpha())
            .into_any_element(),
    )
}
