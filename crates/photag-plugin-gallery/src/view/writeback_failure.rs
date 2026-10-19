//! Compact writeback-failure feedback shown beside the Gallery inspector.

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::*;
use photag_plugin_tagging::WritebackFailure;
use photag_ui_bridge::theme;

use super::GalleryView;

pub(super) fn render(
    _view: &mut GalleryView,
    failure: WritebackFailure,
    cx: &mut Context<GalleryView>,
) -> Div {
    let path = failure.path;
    let retry_path = path.clone();
    let filename = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let retry_id = format!("writeback-retry-{}", path.display());
    let dismiss_id = format!("writeback-dismiss-{}", path.display());
    let retry_path_for_click = retry_path.clone();

    div()
        .w_full()
        .v_flex()
        .gap_2()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(rgb(theme::danger()))
        .bg(rgb(theme::surface_modal()))
        .child(
            div()
                .h_flex()
                .items_center()
                .gap_2()
                .child(
                    theme::icon(IconName::CircleX)
                        .size(px(16.))
                        .text_color(rgb(theme::danger_text())),
                )
                .child(
                    div()
                        .text_sm()
                        .font_semibold()
                        .text_color(rgb(theme::danger_text()))
                        .child(photag_ui_bridge::i18n::tr("Metadata writeback failed")),
                ),
        )
        .child(
            div()
                .w_full()
                .overflow_hidden()
                .text_sm()
                .text_ellipsis()
                .text_color(rgb(theme::text_primary()))
                .child(filename),
        )
        .child(
            div()
                .w_full()
                .max_h(px(48.))
                .overflow_hidden()
                .text_xs()
                .text_color(rgb(theme::text_secondary()))
                .child(failure.error),
        )
        .child(
            div()
                .h_flex()
                .justify_end()
                .gap_2()
                .child(
                    Button::new(retry_id)
                        .primary()
                        .compact()
                        .label(photag_ui_bridge::i18n::tr("Retry"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.operation_status = match this
                                .tagging
                                .as_ref()
                                .ok_or_else(|| "Tagging service unavailable".to_owned())
                                .and_then(|tagging| {
                                    tagging
                                        .retry_writeback_failure(&retry_path_for_click)
                                        .map_err(|error| error.to_string())
                                }) {
                                Ok(true) => Some("Metadata writeback retry queued.".into()),
                                Ok(false) => {
                                    Some("This writeback failure is no longer available.".into())
                                }
                                Err(error) => Some(photag_ui_bridge::i18n::tr_args(
                                    "Could not retry metadata writeback: {error}",
                                    &[("error", error)],
                                )),
                            };
                            cx.notify();
                        })),
                )
                .child(
                    Button::new(dismiss_id)
                        .ghost()
                        .compact()
                        .label(photag_ui_bridge::i18n::tr("Dismiss"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(tagging) = &this.tagging {
                                tagging.dismiss_writeback_failure(&path);
                            }
                            cx.notify();
                        })),
                ),
        )
}
