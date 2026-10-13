//! Preview overlays that remain part of the redesigned view: loading status
//! and the inline rename editor.

use gpui_kit::component::button::Button;
use gpui_kit::component::input::Input;
use gpui_kit::*;
use photag_ui_bridge::i18n::tr;

use super::PreviewView;

/// `preview.rs:1969`: rename field width 360.
const RENAME_WIDTH: f32 = 360.0;
/// `preview.rs:1962`: the rename area anchors 16 below the top edge.
const RENAME_TOP: f32 = 16.0;

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
