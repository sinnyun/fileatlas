use super::*;

pub(super) fn append(mut panel: Div, view: &SidebarView, cx: &mut Context<SidebarView>) -> Div {
    if view.active_scan_folder_id.is_some() {
        let progress = view.scan_progress.as_ref();
        let scanned = progress.map(|value| value.scanned).unwrap_or(0);
        let total = progress.map(|value| value.total).unwrap_or(0);
        let phase = progress
            .map(|value| value.phase.as_str())
            .unwrap_or("Scanning files…");
        let ratio = scanned as f32 / total.max(1) as f32;
        panel = panel.child(
            div()
                .v_flex()
                .gap_1()
                .p_2()
                .rounded_md()
                .bg(rgb(theme::SURFACE_PANEL))
                .child(
                    div()
                        .h_flex()
                        .items_center()
                        .justify_between()
                        .child(div().text_xs().child(photag_ui_bridge::i18n::tr(phase)))
                        .child(
                            Button::new("navigation-cancel-scan")
                                .ghost()
                                .compact()
                                .label(photag_ui_bridge::i18n::tr("Cancel"))
                                .on_click(
                                    cx.listener(|this, _, _, cx| this.cancel_active_scan(cx)),
                                ),
                        ),
                )
                .child(
                    gpui_kit::component::progress::Progress::new("navigation-scan-progress")
                        .value(ratio.clamp(0.0, 1.0) * 100.0)
                        .w_full(),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(theme::TEXT_SECONDARY))
                        .child(photag_ui_bridge::i18n::tr_args(
                            "{scanned} / {total} files",
                            &[
                                ("scanned", scanned.to_string()),
                                ("total", total.to_string()),
                            ],
                        )),
                ),
        );
    }
    if let Some((target, message)) = view.scan_failure.clone() {
        panel = panel.child(
            div()
                .v_flex()
                .gap_1()
                .p_2()
                .rounded_md()
                .bg(rgb(theme::SURFACE_PANEL))
                .child(
                    div()
                        .h_flex()
                        .items_center()
                        .gap_2()
                        .child(
                            theme::small_icon(gpui_kit::assets::IconName::TriangleAlert)
                                .text_color(rgb(theme::WARNING)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_sm()
                                .text_color(rgb(theme::DANGER))
                                .child(photag_ui_bridge::i18n::tr("Folder scan failed")),
                        )
                        .child(
                            Button::new("navigation-retry-scan")
                                .primary()
                                .compact()
                                .label(photag_ui_bridge::i18n::tr("Retry scan"))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.retry_scan(target.clone(), cx)
                                })),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(theme::TEXT_SECONDARY))
                        .child(message),
                ),
        );
    }
    panel
}
