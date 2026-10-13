//! Image-stage right-click actions ported from PhoTag's preview menu.

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::*;
use photag_ui_bridge::theme;

use super::PreviewView;

pub(super) fn render(view: &PreviewView, cx: &mut Context<PreviewView>) -> Div {
    let mut menu = div()
        .h_flex()
        .items_center()
        .gap_1()
        .rounded_md()
        .bg(rgba(theme::modal_rgba(theme::SURFACE_MODAL)))
        .p_2()
        .child(
            div()
                .text_sm()
                .font_semibold()
                .child(photag_ui_bridge::i18n::tr("Preview actions")),
        )
        .child(
            Button::new("preview-context-open")
                .label(photag_ui_bridge::i18n::tr("Open externally"))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.context_menu_open = false;
                    this.open_in_default_app(cx);
                })),
        )
        .child(
            Button::new("preview-context-reveal")
                .label(photag_ui_bridge::i18n::tr("Show in folder"))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.context_menu_open = false;
                    this.reveal_in_file_manager(cx);
                })),
        );
    if let Some(media_id) = view.service.snapshot().media_id {
        menu = menu.child(
            Button::new("preview-context-ai-tags")
                .label(photag_ui_bridge::i18n::tr("AI Suggest Tags"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.service.request_ai_suggestions(media_id);
                    this.context_menu_open = false;
                    this.operation_status = Some("AI tag suggestions requested.".into());
                    cx.notify();
                })),
        );
        for rating in 0..=5 {
            menu = menu.child(
                Button::new(format!("preview-context-rating-{rating}"))
                    .label(photag_ui_bridge::i18n::tr(
                        &(format!("{rating}★")).to_string(),
                    ))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.context_menu_open = false;
                        this.apply_rating_shortcut(rating, cx);
                    })),
            );
        }
        menu = menu.child(
            Button::new("preview-context-delete")
                .label(photag_ui_bridge::i18n::tr("Delete…"))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.context_menu_open = false;
                    this.request_delete(window, cx);
                })),
        );
    }
    menu.child(
        Button::new("preview-context-close")
            .label(photag_ui_bridge::i18n::tr(&("Close menu").to_string()))
            .on_click(cx.listener(|this, _, _, cx| {
                this.context_menu_open = false;
                cx.notify();
            })),
    )
}
