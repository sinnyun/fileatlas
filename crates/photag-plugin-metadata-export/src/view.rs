//! GPUI panel for selecting and running PhoTag's metadata backup export.

use std::sync::Arc;
use std::time::Duration;

use cordis::{Context as CordisContext, Routing};
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::*;
use photag_ui_bridge::{UiCommandInvoked, UiViewContribution};

use crate::{EXPORT_COMMAND_ID, MetadataExportService, MetadataExportStatus};

pub(super) fn export_panel(
    service: Arc<MetadataExportService>,
    cordis: CordisContext,
) -> UiViewContribution {
    UiViewContribution::new(
        "metadata-export/panel-view",
        "workspace.metadata-export",
        0,
        move |window, app| {
            let path = app.new(|cx| {
                InputState::new(window, cx).placeholder("C:\\Backups\\photag_backup.json")
            });
            app.new(|_| ExportPanel::new(Arc::clone(&service), cordis.clone(), path))
                .into()
        },
    )
}

struct ExportPanel {
    service: Arc<MetadataExportService>,
    cordis: CordisContext,
    path: Entity<InputState>,
    input_error: Option<String>,
}

impl ExportPanel {
    fn new(
        service: Arc<MetadataExportService>,
        cordis: CordisContext,
        path: Entity<InputState>,
    ) -> Self {
        Self {
            service,
            cordis,
            path,
            input_error: None,
        }
    }

    fn choose_destination(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Match the pinned PhoTag action: use rfd's native Save File dialog,
        // with its OS-provided overwrite confirmation, instead of asking the
        // user to type an absolute path into the panel.
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Export metadata backup")
            .add_filter("JSON", &["json"])
            .set_file_name("photag_backup.json")
            .save_file()
        {
            self.path.update(cx, |state, cx| {
                state.set_value(&path.to_string_lossy(), window, cx)
            });
            self.input_error = None;
            cx.notify();
        }
    }

    fn export(&mut self, cx: &mut Context<Self>) {
        let path = self.path.read(cx).value().trim().to_owned();
        if path.is_empty() {
            self.input_error = Some("Enter a destination JSON file path.".into());
            cx.notify();
            return;
        }
        if let Err(error) = self.service.set_destination(path) {
            self.input_error = Some(error.to_string());
            cx.notify();
            return;
        }
        self.input_error = None;
        let cordis = self.cordis.clone();
        cx.spawn(async move |this, cx| {
            if let Err(error) = cordis
                .emit::<UiCommandInvoked>(Routing::Unscoped, EXPORT_COMMAND_ID.to_owned())
                .await
            {
                this.update(cx, |view, cx| {
                    view.input_error = Some(format!("Could not start export: {error}"));
                    cx.notify();
                })
                .ok();
                return;
            }
            loop {
                let exporting = this
                    .update(cx, |view, cx| {
                        cx.notify();
                        matches!(view.service.status(), Ok(MetadataExportStatus::Exporting))
                    })
                    .unwrap_or(false);
                if !exporting {
                    break;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(180))
                    .await;
            }
        })
        .detach();
    }
}

impl Render for ExportPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status = match self.service.status() {
            Ok(MetadataExportStatus::Idle) => {
                "Choose a JSON destination and export all indexed media metadata.".into()
            }
            Ok(MetadataExportStatus::Exporting) => "Exporting metadata…".into(),
            Ok(MetadataExportStatus::Succeeded { path, records }) => {
                format!("Exported {records} records to {}", path.display())
            }
            Ok(MetadataExportStatus::Failed(error)) => format!("Export failed: {error}"),
            Err(error) => format!("Export status unavailable: {error}"),
        };
        let error = self.input_error.clone();
        let panel = div()
            .v_flex()
            .w_full()
            .gap_2()
            .p_3()
            .border_t_1()
            .border_color(rgb(0x343943))
            .child(div().text_sm().font_semibold().child("Metadata backup"))
            .child(
                div()
                    .h_flex()
                    .gap_2()
                    .child(Input::new(&self.path).flex_1())
                    .child(
                        Button::new("metadata-export-browse")
                            .label("Browse…")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.choose_destination(window, cx)
                            })),
                    )
                    .child(
                        Button::new("metadata-export-run")
                            .label("Export JSON")
                            .on_click(cx.listener(|this, _, _, cx| this.export(cx))),
                    ),
            )
            .child(div().text_xs().text_color(rgb(0x9aa0ad)).child(status));
        if let Some(message) = error {
            panel.child(div().text_xs().text_color(rgb(0xef7777)).child(message))
        } else {
            panel
        }
    }
}
