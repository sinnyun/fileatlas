use super::*;
use fileatlas_ui_bridge::components::WorkbenchForeground as _;
use fileatlas_ui_bridge::components::WorkbenchPaint as _;

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
                .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                .p(px(fileatlas_ui_bridge::theme::space_standard()))
                .workbench_corner(fileatlas_ui_bridge::components::CornerRole::Medium)
                .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Panel)
                .child(
                    div()
                        .h_flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .workbench_assistant()
                                .child(fileatlas_ui_bridge::i18n::tr(phase)),
                        )
                        .child(
                            fileatlas_ui_bridge::components::button("navigation-cancel-scan")
                                .ghost()
                                .label(fileatlas_ui_bridge::i18n::tr("Cancel"))
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
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr_args(
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
                .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                .p(px(fileatlas_ui_bridge::theme::space_standard()))
                .workbench_corner(fileatlas_ui_bridge::components::CornerRole::Medium)
                .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Panel)
                .child(
                    div()
                        .h_flex()
                        .items_center()
                        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                        .child(
                            theme::small_icon(gpui_kit::assets::IconName::TriangleAlert)
                                .workbench_foreground(
                                    fileatlas_ui_bridge::components::ForegroundRole::Warning,
                                ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .workbench_assistant()
                                .child(fileatlas_ui_bridge::i18n::tr("Folder scan failed")),
                        )
                        .child(
                            fileatlas_ui_bridge::components::button("navigation-retry-scan")
                                .primary()
                                .label(fileatlas_ui_bridge::i18n::tr("Retry scan"))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.retry_scan(target.clone(), cx)
                                })),
                        ),
                )
                .child(div().workbench_assistant().child(message)),
        );
    }
    panel
}
