//! Update-check/download lifecycle for the settings surface.

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::*;
use photag_ui_bridge::theme;
use std::sync::Arc;
use std::time::Duration;

use super::SettingsView;

impl SettingsView {
    pub(super) fn begin_update_check(&mut self, _manual: bool) {
        let Some(updater) = &self.updater else {
            self.update_error = Some("Updater service is unavailable.".into());
            return;
        };
        self.update_error = None;
        self.update_status = Some("Checking GitHub for a newer release…".into());
        self.update_check = Some(updater.check_async());
    }

    pub(super) fn begin_update_download(&mut self) {
        let (Some(updater), Some(info)) = (&self.updater, self.update_info.clone()) else {
            return;
        };
        self.update_error = None;
        self.update_status = Some(format!("Downloading PhoTag {}…", info.version));
        self.update_progress
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.update_download =
            Some(updater.download_async(info, Arc::clone(&self.update_progress)));
    }

    pub(super) fn monitor_updates(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                let active = this
                    .update(cx, |view, cx| {
                        view.poll_updates();
                        cx.notify();
                        view.update_check.is_some() || view.update_download.is_some()
                    })
                    .unwrap_or(false);
                if !active {
                    break;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(120))
                    .await;
            }
        })
        .detach();
    }

    fn poll_updates(&mut self) {
        let check_result = self
            .update_check
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());
        if let Some(result) = check_result {
            self.update_check = None;
            match result {
                Ok(Some(info)) => {
                    self.update_status = Some(format!(
                        "PhoTag {} is available ({:.1} MB).",
                        info.version,
                        info.size_mb()
                    ));
                    self.update_info = Some(info);
                    if !self.update_offer_dismissed {
                        self.update_offer_open = true;
                    }
                }
                Ok(None) => {
                    self.update_info = None;
                    self.update_status = Some(format!(
                        "PhoTag {} is up to date.",
                        photag_plugin_updater::current_version()
                    ));
                }
                Err(error) => {
                    self.update_status = None;
                    self.update_error = Some(error);
                }
            }
        }
        let download_result = self
            .update_download
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());
        if let Some(result) = download_result {
            self.update_download = None;
            match result {
                Ok(path) => {
                    self.update_staged = Some(path);
                    self.confirm_install = false;
                    self.update_status =
                        Some("Download verified. PhoTag is ready to install and restart.".into());
                }
                Err(error) => {
                    self.update_status = None;
                    self.update_error = Some(error);
                }
            }
        }
    }

    pub(super) fn render_update_offer(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        if !self.update_offer_open {
            return div();
        }
        let Some(info) = self.update_info.clone() else {
            self.update_offer_open = false;
            return div();
        };
        let downloading = self.update_download.is_some();
        let staged = self.update_staged.is_some();
        let viewport = window.bounds().size;
        let mut offer_header = div()
            .h_flex()
            .items_center()
            .justify_between()
            .child(div().text_lg().font_semibold().child("Update available"));
        if !downloading {
            offer_header = offer_header.child(
                Button::new("settings-update-offer-close")
                    .label("×")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.update_offer_open = false;
                        this.update_offer_dismissed = true;
                        cx.notify();
                    })),
            );
        }
        let mut dialog = div()
            .v_flex()
            .gap_2()
            .w(px(440.))
            .max_h(px((f32::from(viewport.height) - 32.).max(180.)))
            .rounded_md()
            .border_1()
            .border_color(rgba(theme::HAIRLINE.to_rgba()))
            .bg(rgba(theme::modal_rgba(theme::SURFACE_MODAL)))
            .text_color(rgb(theme::TEXT_PRIMARY))
            .p_4()
            .child(offer_header)
            .child(div().child(format!(
                "PhoTag {} · {:.1} MB",
                info.version,
                info.size_mb()
            )))
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child(format!(
                        "You're running PhoTag {}.",
                        photag_plugin_updater::current_version()
                    )),
            );
        if !info.notes_plain().is_empty() {
            dialog = dialog.child(
                div()
                    .max_h(px(180.))
                    .overflow_y_scrollbar()
                    .text_sm()
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child(info.notes_plain()),
            );
        }
        if let Some(error) = &self.update_error {
            dialog = dialog.child(
                div()
                    .text_sm()
                    .text_color(rgb(theme::DANGER))
                    .child(error.clone()),
            );
        }
        if downloading {
            let done = self
                .update_progress
                .load(std::sync::atomic::Ordering::Relaxed);
            let total = info.asset_size;
            dialog = dialog.child(
                div()
                    .text_sm()
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child(format!(
                        "Downloading… {:.1} of {:.1} MB ({:.0}%)",
                        done as f64 / 1_048_576.0,
                        info.size_mb(),
                        if total == 0 {
                            0.0
                        } else {
                            done as f64 * 100.0 / total as f64
                        }
                    )),
            );
        }
        let mut actions = div().h_flex().gap_2();
        if staged {
            actions = actions
                .child(
                    Button::new("settings-update-offer-restart")
                        .label("Restart & install")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.confirm_install = true;
                            this.install_update(cx);
                        })),
                )
                .child(
                    Button::new("settings-update-offer-later")
                        .label("Later")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.update_offer_open = false;
                            this.update_offer_dismissed = true;
                            cx.notify();
                        })),
                );
        } else if downloading {
            actions = actions.child(
                div()
                    .text_sm()
                    .text_color(rgb(theme::TEXT_MUTED))
                    .child("The update offer stays open while downloading."),
            );
        } else {
            actions = actions
                .child(
                    Button::new("settings-update-offer-download")
                        .label("Update now")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.begin_update_download();
                            this.monitor_updates(cx);
                        })),
                )
                .child(
                    Button::new("settings-update-offer-later")
                        .label("Not now")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.update_offer_open = false;
                            this.update_offer_dismissed = true;
                            cx.notify();
                        })),
                );
        }
        let url = if info.html_url.is_empty() {
            photag_plugin_updater::releases_url()
        } else {
            info.html_url
        };
        if !downloading {
            actions = actions.child(
                Button::new("settings-update-offer-github")
                    .label("View on GitHub")
                    .on_click(move |_, _, _| {
                        let _ = open::that_detached(&url);
                    }),
            );
        }
        dialog = dialog.child(actions);
        div()
            .absolute()
            .inset_0()
            .h_flex()
            .items_center()
            .justify_center()
            .child(div().absolute().inset_0().bg(rgba(0x0000_00a6)))
            .child(dialog)
    }

    pub(super) fn install_update(&mut self, cx: &mut Context<Self>) {
        if !self.confirm_install {
            self.confirm_install = true;
            self.update_status = Some("Confirm restart and install the verified update.".into());
            cx.notify();
            return;
        }
        let Some(path) = self.update_staged.clone() else {
            return;
        };
        let session_save = self
            .session
            .as_ref()
            .ok_or_else(|| {
                "Session service is unavailable; update installation was stopped.".to_owned()
            })
            .and_then(|session| {
                let snapshot = session.snapshot().map_err(|error| error.to_string())?;
                session.replace(snapshot).map_err(|error| error.to_string())
            });
        if let Err(error) = session_save {
            self.update_error = Some(error);
            cx.notify();
            return;
        }
        let result = self
            .updater
            .as_ref()
            .ok_or_else(|| "Updater service is unavailable.".to_owned())
            .and_then(|updater| updater.schedule_install(path.clone()));
        match result {
            Ok(()) => {
                self.update_staged = None;
                self.update_status = Some("Closing PhoTag to install the verified update…".into());
                cx.quit();
            }
            Err(error) => {
                let log = dirs::data_local_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join("PhoTag")
                    .join("update-error.txt");
                let _ = std::fs::create_dir_all(
                    log.parent().unwrap_or_else(|| std::path::Path::new(".")),
                );
                let _ = std::fs::write(
                    &log,
                    format!(
                        "PhoTag {} could not install update from {}\n\n{}\n",
                        photag_plugin_updater::current_version(),
                        path.display(),
                        error
                    ),
                );
                self.update_error = Some(format!("{error} (details: {})", log.display()));
            }
        }
        cx.notify();
    }
}
