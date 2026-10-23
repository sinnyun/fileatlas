//! v5 design 07: native destination selection and the real export state machine.
use crate::{MetadataExportService, MetadataExportStatus};
use fileatlas_ui_bridge::components::WorkbenchForeground as _;
use fileatlas_ui_bridge::components::WorkbenchStyle as _;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
use fileatlas_ui_bridge::i18n::{tr, tr_args};
use fileatlas_ui_bridge::{UiSurfaceState, UiViewContribution, theme};
use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::sync::Arc;
use std::time::Duration;

pub(super) fn export_panel(
    service: Arc<MetadataExportService>,
    surfaces: Arc<UiSurfaceState>,
) -> UiViewContribution {
    UiViewContribution::new(
        "metadata-export/panel-view",
        "workspace.modal",
        15,
        move |_, app| {
            let service = Arc::clone(&service);
            let surfaces = Arc::clone(&surfaces);
            app.new(|cx| {
                fileatlas_ui_bridge::refresh::poll(
                    cx,
                    Duration::from_millis(100),
                    |view: &mut ExportPanel, _| {
                        let mut changed = false;
                        let status = view.service.status().ok();
                        if status != view.last_status {
                            if matches!(
                                status,
                                Some(
                                    MetadataExportStatus::Exporting
                                        | MetadataExportStatus::Succeeded { .. }
                                        | MetadataExportStatus::Failed(_)
                                )
                            ) {
                                let _ = view.surfaces.set_open("metadata-export", true);
                            }
                            view.last_status = status;
                            changed = true;
                        }
                        let visible = view.surfaces.is_open("metadata-export");
                        if visible != view.visible {
                            view.visible = visible;
                            changed = true;
                        }
                        changed.into()
                    },
                );
                ExportPanel {
                    service,
                    surfaces,
                    visible: false,
                    last_status: None,
                    input_error: None,
                }
            })
            .into()
        },
    )
}
struct ExportPanel {
    service: Arc<MetadataExportService>,
    surfaces: Arc<UiSurfaceState>,
    visible: bool,
    last_status: Option<MetadataExportStatus>,
    input_error: Option<String>,
}
impl ExportPanel {
    fn choose_and_export(&mut self, cx: &mut Context<Self>) {
        // Pinned app.rs:1535-1553: JSON-filtered native save dialog followed by
        // the complete metadata query and source-compatible pretty JSON writer.
        let Some(path) = rfd::FileDialog::new()
            .set_title(tr("Export metadata backup"))
            .add_filter("JSON", &["json"])
            .set_file_name("fileatlas_backup.json")
            .save_file()
        else {
            return;
        };
        self.input_error = self
            .service
            .set_destination(path)
            .and_then(|_| self.service.request_export())
            .err()
            .map(|error| error.to_string());
        cx.notify();
    }
}
impl Render for ExportPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible {
            return div().size_0().into_any_element();
        }
        let status = self
            .service
            .status()
            .unwrap_or_else(|error| MetadataExportStatus::Failed(error.to_string()));
        let exporting = matches!(status, MetadataExportStatus::Exporting);
        let (title, detail, icon, color) = match &status {
            MetadataExportStatus::Idle => (
                tr("Ready to export"),
                tr(
                    "Export every indexed media record with its path, rating, and hierarchical tags.",
                ),
                IconName::File,
                theme::accent(),
            ),
            MetadataExportStatus::Exporting => (
                tr("Exporting metadata…"),
                tr("Writing the metadata backup. This panel updates when the export finishes."),
                IconName::Upload,
                theme::accent(),
            ),
            MetadataExportStatus::Succeeded { records, .. } => (
                tr("Export complete"),
                tr_args(
                    "Exported {records} records",
                    &[("records", records.to_string())],
                ),
                IconName::CircleCheck,
                theme::success_text(),
            ),
            MetadataExportStatus::Failed(error) => (
                tr("Export failed"),
                error.clone(),
                IconName::CircleX,
                theme::danger_text(),
            ),
        };
        let mut body = div()
            .v_flex()
            .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
            .p(px(fileatlas_ui_bridge::theme::space_expanded()))
            .child(
                div()
                    .h_flex()
                    .items_start()
                    .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
                    .child(theme::large_icon(icon).workbench_foreground(
                        fileatlas_ui_bridge::components::ForegroundRole::Custom(color),
                    ))
                    .child(
                        fileatlas_ui_bridge::components::heading_block(title, detail)
                            .flex_1()
                            .min_w_0(),
                    ),
            );
        if exporting {
            body = body.child(
                div()
                    .h_flex()
                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                    .child(Spinner::new())
                    .child(div().workbench_assistant().child(tr("Exporting…"))),
            );
        }
        if let MetadataExportStatus::Succeeded { path, .. } = &status {
            body = body.child(fileatlas_ui_bridge::components::card_surface(
                div()
                    .v_flex()
                    .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                    .p(px(fileatlas_ui_bridge::theme::space_expanded()))
                    .child(
                        div().workbench_assistant().child(
                            path.file_name()
                                .map(|name| name.to_string_lossy().into_owned())
                                .unwrap_or_default(),
                        ),
                    )
                    .child(
                        div()
                            .workbench_assistant()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(path.display().to_string()),
                    ),
                false,
            ));
        }
        if let Some(error) = &self.input_error {
            body = body.child(div().workbench_assistant().child(error.clone()));
        }
        fileatlas_ui_bridge::components::modal_overlay(
            div()
                .w(px((f32::from(window.bounds().size.width) - 48.).min(560.)))
                .child(
                    div()
                        .h_flex()
                        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                        .h(px(theme::ui_height(theme::CONTROL_HEIGHT_FOOTER)))
                        .px(px(fileatlas_ui_bridge::theme::space_expanded()))
                        .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Bottom)
                        .child(theme::icon(IconName::Upload).workbench_foreground(
                            fileatlas_ui_bridge::components::ForegroundRole::AccentText,
                        ))
                        .child(
                            div()
                                .flex_1()
                                .workbench_title()
                                .child(tr("Metadata export (JSON)")),
                        )
                        .child(
                            fileatlas_ui_bridge::components::button("export-close-header")
                                .ghost()
                                .icon(theme::icon(IconName::X))
                                .disabled(exporting)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    let _ = this.surfaces.set_open("metadata-export", false);
                                    cx.notify();
                                })),
                        ),
                )
                .child(body)
                .child(
                    div()
                        .h_flex()
                        .justify_end()
                        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                        .px(px(fileatlas_ui_bridge::theme::space_expanded()))
                        .py(px(fileatlas_ui_bridge::theme::space_expanded()))
                        .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Top)
                        .when(!exporting, |row| {
                            row.child(
                                fileatlas_ui_bridge::components::button("export-choose")
                                    .primary()
                                    .label(tr("Choose destination…"))
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.choose_and_export(cx)),
                                    ),
                            )
                        })
                        .child(
                            fileatlas_ui_bridge::components::button("export-close-footer")
                                .label(tr("Close"))
                                .disabled(exporting)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    let _ = this.surfaces.set_open("metadata-export", false);
                                    cx.notify();
                                })),
                        ),
                ),
        )
        .into_any_element()
    }
}
