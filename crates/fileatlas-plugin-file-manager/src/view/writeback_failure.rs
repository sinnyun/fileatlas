//! Compact writeback-failure feedback shown beside the FileManager inspector.

use fileatlas_plugin_tagging::WritebackFailure;
use fileatlas_ui_bridge::components::WorkbenchForeground as _;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
use fileatlas_ui_bridge::theme;
use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::*;

use super::FileManagerView;

pub(super) fn render(
    _view: &mut FileManagerView,
    failure: WritebackFailure,
    cx: &mut Context<FileManagerView>,
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

    fileatlas_ui_bridge::components::card_surface(
        div()
            .w_full()
            .v_flex()
            .gap(px(fileatlas_ui_bridge::theme::space_standard()))
            .p(px(fileatlas_ui_bridge::theme::space_expanded()))
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                    .child(
                        theme::icon(IconName::CircleX)
                            .size(px(16.))
                            .workbench_foreground(
                                fileatlas_ui_bridge::components::ForegroundRole::Danger,
                            ),
                    )
                    .child(
                        div()
                            .workbench_assistant()
                            .child(fileatlas_ui_bridge::i18n::tr("Metadata writeback failed")),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .overflow_hidden()
                    .text_ellipsis()
                    .workbench_body()
                    .child(filename),
            )
            .child(
                div()
                    .w_full()
                    .max_h(px(48.))
                    .overflow_hidden()
                    .workbench_assistant()
                    .child(failure.error),
            )
            .child(
                div()
                    .h_flex()
                    .justify_end()
                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                    .child(
                        fileatlas_ui_bridge::components::button(retry_id)
                            .primary()
                            .label(fileatlas_ui_bridge::i18n::tr("Retry"))
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
                                    Ok(false) => Some(
                                        "This writeback failure is no longer available.".into(),
                                    ),
                                    Err(error) => Some(fileatlas_ui_bridge::i18n::tr_args(
                                        "Could not retry metadata writeback: {error}",
                                        &[("error", error)],
                                    )),
                                };
                                cx.notify();
                            })),
                    )
                    .child(
                        fileatlas_ui_bridge::components::button(dismiss_id)
                            .ghost()
                            .label(fileatlas_ui_bridge::i18n::tr("Dismiss"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(tagging) = &this.tagging {
                                    tagging.dismiss_writeback_failure(&path);
                                }
                                cx.notify();
                            })),
                    ),
            ),
        false,
    )
}
