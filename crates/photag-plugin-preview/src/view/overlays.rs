//! Floating preview chrome: the Info toggle, the Close ✕ and the centered
//! filename with its zoom percentage and inline rename editor.
//!
//! Source: `vendor/photag-source/src/ui/preview.rs:404-454` (info toggle),
//! `:389-402` with the click routing at `:2104,2131-2141` (image close),
//! `:1165-1190` (audio close), `:1458-1475` + `video.rs:726-743` (video close),
//! `:1934-2092` (filename, hover underline, click vs double-click) and
//! `:1949-1999` (rename editor).
use std::sync::Arc;

use gpui_kit::component::button::Button;
use gpui_kit::component::input::Input;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use photag_ui_bridge::i18n::tr;

use super::PreviewView;
use crate::PreviewMediaKind;

/// `preview.rs:407,390`: both corner controls center 25 px from the edges.
const EDGE: f32 = 25.0;
/// `preview.rs:401`: the image close hit box is 30 px.
const HIT: f32 = 30.0;
/// `preview.rs:1168,1461`: audio and video use a 36 px close hit box.
const HIT_LARGE: f32 = 36.0;
/// `preview.rs:431`: filled circle radius 14.
const INFO_RADIUS: f32 = 14.0;
/// `preview.rs:441`: the `"i"` glyph is 18 pt proportional.
const INFO_GLYPH: f32 = 18.0;
/// `preview.rs:2001-2011`: with no tag chips the star row sits at `min.y + 8`
/// and the filename center 22 below it.
const NAME_Y: f32 = 30.0;
/// `preview.rs:1940`: title font 14 pt proportional.
const TITLE_FONT: f32 = 14.0;
/// `preview.rs:1969`: rename field width 360.
const RENAME_WIDTH: f32 = 360.0;
/// `preview.rs:1962`: the rename area anchors 16 below the top edge.
const RENAME_TOP: f32 = 16.0;
/// `preview.rs:2053-2062`: a single click waits out the double-click window.
const DOUBLE_CLICK_DELAY: std::time::Duration = std::time::Duration::from_millis(450);

/// `preview.rs:417-427`: active blue, hover white 230, idle white 180.
const INFO_ACTIVE: u32 = 0x3c82f0ff;
const INFO_HOVER: u32 = 0xffffffe6;
const INFO_IDLE: u32 = 0xffffffb4;
/// `preview.rs:417-427`: glyph white when active, gray(20) otherwise.
const GLYPH_ACTIVE: u32 = 0xffffffff;
const GLYPH_INACTIVE: u32 = 0x141414ff;
/// `preview.rs:436-440`: ring white 220. GPUI border widths are whole pixels,
/// so the source 1.5 stroke draws at 1.
const INFO_RING_COLOR: u32 = 0xffffffdc;
/// `preview.rs:392` / `video.rs:730`: close stroke white 200.
const CLOSE_GLYPH: u32 = 0xffffffc8;
/// `preview.rs:1178-1184`: audio close circle black 160, glyph white 230.
const AUDIO_CLOSE_FILL: u32 = 0x000000a0;
const AUDIO_CLOSE_GLYPH: u32 = 0xffffffe6;
/// `preview.rs:1942-1943`: name white 220, `  —  NN%` suffix white 180.
const NAME_COLOR: u32 = 0xffffffdc;
const SUFFIX_COLOR: u32 = 0xffffffb4;
/// `preview.rs:2046`: hover underline white 180.
const UNDERLINE_COLOR: u32 = 0xffffffb4;

/// `preview.rs:404-454`: circular Info toggle at the top-left corner. Clicks
/// and tooltips need the chrome more than half faded in (`alpha > 0.5`). The
/// source allocates a 30 px hit rect around the 28 px circle; GPUI hover styles
/// only apply to the element that owns them, so the circle is the click target.
pub(super) fn info_toggle(view: &mut PreviewView, cx: &mut Context<PreviewView>) -> AnyElement {
    let active = view.service.snapshot().show_info;
    let (fill, glyph) = if active {
        (INFO_ACTIVE, GLYPH_ACTIVE)
    } else {
        (INFO_IDLE, GLYPH_INACTIVE)
    };
    let hover_fill = if active { INFO_ACTIVE } else { INFO_HOVER };
    div()
        .absolute()
        .left(px(EDGE - INFO_RADIUS))
        .top(px(EDGE - INFO_RADIUS))
        .w(px(INFO_RADIUS * 2.0))
        .h(px(INFO_RADIUS * 2.0))
        .rounded_full()
        .bg(rgba(fill))
        .border_1()
        .border_color(rgba(INFO_RING_COLOR))
        .flex()
        .items_center()
        .justify_center()
        .id("preview-info-toggle")
        .cursor_pointer()
        .hover(move |style| style.bg(rgba(hover_fill)))
        .tooltip(move |window, app| Tooltip::new(tr("Info")).build(window, app))
        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
            if this.service.chrome_alpha() > 0.5 {
                this.service.toggle_info();
                cx.notify();
            }
        }))
        .text_size(px(INFO_GLYPH))
        .text_color(rgba(glyph))
        .child("i")
        .into_any_element()
}

/// `preview.rs:389-402`, `:1165-1190` and `video.rs:726-743`: the top-right
/// close. Images and video paint a bare cross, audio a black circle behind it.
/// The source draws the cross as two 2.5 px line segments; GPUI layout has no
/// line primitive, so the closest match is the ✕ glyph.
pub(super) fn close_button(view: &mut PreviewView, cx: &mut Context<PreviewView>) -> AnyElement {
    let audio = view.service.snapshot().media_kind == Some(PreviewMediaKind::Audio);
    let hit = if audio { HIT_LARGE } else { HIT };
    let service = Arc::clone(&view.service);
    div()
        .absolute()
        .right(px(EDGE - hit / 2.0))
        .top(px(EDGE - hit / 2.0))
        .w(px(hit))
        .h(px(hit))
        .flex()
        .items_center()
        .justify_center()
        .id("preview-close")
        .cursor_pointer()
        .tooltip(move |window, app| Tooltip::new(tr("Close")).build(window, app))
        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.rename_target = None;
            service.close();
            cx.notify();
        }))
        .child(
            div()
                .when(audio, |circle| {
                    circle
                        .w(px(32.0))
                        .h(px(32.0))
                        .rounded_full()
                        .bg(rgba(AUDIO_CLOSE_FILL))
                        .flex()
                        .items_center()
                        .justify_center()
                })
                .text_size(px(if audio { INFO_GLYPH } else { 20.0 }))
                .text_color(rgba(if audio {
                    AUDIO_CLOSE_GLYPH
                } else {
                    CLOSE_GLYPH
                }))
                .child("✕"),
        )
        .into_any_element()
}

/// `preview.rs:1934-2092`: centered filename plus zoom percentage. A single
/// click reveals the file in Explorer once the double-click window expires; a
/// double click opens the inline rename editor, the same state F2 enters.
pub(super) fn filename_row(
    filename: String,
    percent: i32,
    cx: &mut Context<PreviewView>,
) -> AnyElement {
    let click = cx.listener(move |this, event: &ClickEvent, window, cx| {
        if this.service.chrome_alpha() <= 0.5 {
            return;
        }
        this.title_click_generation = this.title_click_generation.wrapping_add(1);
        if event.click_count() >= 2 {
            this.begin_rename(window, cx);
            return;
        }
        let generation = this.title_click_generation;
        let weak = cx.entity().downgrade();
        cx.spawn(async move |_, app| {
            app.background_executor().timer(DOUBLE_CLICK_DELAY).await;
            let _ = weak.update(app, move |this: &mut PreviewView, cx| {
                if this.title_click_generation == generation {
                    this.reveal_in_file_manager(cx);
                }
            });
        })
        .detach();
    });
    div()
        .absolute()
        .left_0()
        .right_0()
        .top_0()
        .h(px(NAME_Y * 2.0))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .flex()
                .items_center()
                .id("preview-filename")
                .cursor_pointer()
                .on_click(click)
                .child(
                    div()
                        .text_size(px(TITLE_FONT))
                        .text_color(rgba(NAME_COLOR))
                        .hover(move |style| style.border_b_1().border_color(rgba(UNDERLINE_COLOR)))
                        .child(filename),
                )
                .child(
                    div()
                        .text_size(px(TITLE_FONT))
                        .text_color(rgba(SUFFIX_COLOR))
                        .child(format!("  —  {percent}%")),
                ),
        )
        .into_any_element()
}

/// `preview.rs:2095-2101`: the source's unloaded state. Constant white 150,
/// unlike the faded chrome.
pub(super) fn loading_label() -> AnyElement {
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(18.0))
        .text_color(rgba(0xffffff96))
        .child(tr("Loading..."))
        .into_any_element()
}

/// `preview.rs:1284-1296`: video paints the filename top-center with no
/// interaction and no zoom percentage.
pub(super) fn video_title(filename: String) -> AnyElement {
    div()
        .absolute()
        .left_0()
        .right_0()
        .top_0()
        .h(px(NAME_Y * 2.0))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(TITLE_FONT))
        .text_color(rgba(NAME_COLOR))
        .child(filename)
        .into_any_element()
}

/// `preview.rs:1949-1999`: the inline rename row that replaces the title.
pub(super) fn rename_editor(view: &PreviewView, cx: &mut Context<PreviewView>) -> AnyElement {
    div()
        .absolute()
        .left_0()
        .right_0()
        .top(px(RENAME_TOP))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .occlude()
        .child(Input::new(&view.rename_input).w(px(RENAME_WIDTH)))
        .child(
            Button::new("preview-rename-commit")
                .compact()
                .label("✓")
                .tooltip(tr("Save (Enter)"))
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.commit_rename(cx))),
        )
        .child(
            Button::new("preview-rename-cancel")
                .compact()
                .label("✕")
                .tooltip(tr("Cancel (Esc)"))
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.rename_target = None;
                    cx.notify();
                })),
        )
        .into_any_element()
}
