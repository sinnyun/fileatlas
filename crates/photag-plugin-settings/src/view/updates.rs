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
    pub(super) fn begin_update_check(&mut self, manual: bool) {
        let Some(updater) = &self.updater else {
            self.update_error = Some("Updater service is unavailable.".into());
            return;
        };
        self.update_error = None;
        self.update_check_manual = manual;
        if manual {
            self.update_offer_dismissed = false;
        }
        self.update_status = Some("Checking GitHub for a newer release…".into());
        self.update_check = Some(updater.check_async());
        let _ = self.surfaces.set_open("settings.update-checking", true);
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
            let _ = self.surfaces.set_open("settings.update-checking", false);
            match result {
                Ok(Some(info)) => {
                    self.update_status = Some(format!(
                        "PhoTag {} is available ({:.1} MB).",
                        info.version,
                        info.size_mb()
                    ));
                    self.update_info = Some(info);
                    let _ = self.surfaces.set_open("settings.update-available", true);
                    // Startup checks populate Settings quietly. Only an explicit
                    // manual check may open the update offer.
                    if self.update_check_manual && !self.update_offer_dismissed {
                        self.update_offer_open = true;
                    }
                    let _ = self
                        .surfaces
                        .set_open("settings.update-offer", self.update_offer_open);
                }
                Ok(None) => {
                    self.update_info = None;
                    let _ = self.surfaces.set_open("settings.update-available", false);
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
        let _ = self
            .surfaces
            .set_open("settings.update-offer", self.update_offer_open);
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
        let dialog_width = (f32::from(viewport.width) - 40.).clamp(440., 820.);
        let dialog_height = (f32::from(viewport.height) - 24.).clamp(320., 580.);
        let body_height = dialog_height - 48. - 56.;
        let inner_width = dialog_width - 32.;
        let left_width = inner_width * 0.58 - 8.;
        let right_width = inner_width * 0.42 - 8.;
        let notes_height = (body_height - 140.).clamp(80., 250.);

        let mut left = div()
            .v_flex()
            .w(px(left_width))
            .h_full()
            .min_h_0()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child(photag_ui_bridge::i18n::tr("Current version")),
            )
            .child(
                div()
                    .p_2()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgba(theme::HAIRLINE.to_rgba()))
                    .bg(rgb(theme::SURFACE_BASE))
                    .child(format!(
                        "PhoTag {}",
                        photag_plugin_updater::current_version()
                    )),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child(photag_ui_bridge::i18n::tr("Update available")),
            )
            .child(
                div()
                    .v_flex()
                    .gap_1()
                    .p_2()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgba(theme::HAIRLINE.to_rgba()))
                    .bg(rgb(theme::SURFACE_BASE))
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .font_semibold()
                                    .child(format!("PhoTag {}", info.version)),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(theme::TEXT_SECONDARY))
                                    .child(format!("{:.1} MB", info.size_mb())),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(theme::TEXT_SECONDARY))
                            .child(format!(
                                "You're running PhoTag {}.",
                                photag_plugin_updater::current_version()
                            )),
                    ),
            );
        let notes = info.notes_plain();
        if !notes.is_empty() {
            left = left
                .child(
                    div()
                        .font_semibold()
                        .child(photag_ui_bridge::i18n::tr("Release notes")),
                )
                .child(
                    div()
                        .h(px(notes_height))
                        .max_h(px(notes_height))
                        .overflow_y_scrollbar()
                        .text_sm()
                        .text_color(rgb(theme::TEXT_SECONDARY))
                        .child(notes),
                );
        }

        let mut right = div()
            .v_flex()
            .w(px(right_width))
            .h_full()
            .min_h_0()
            .gap_2()
            .child(
                div()
                    .font_semibold()
                    .child(photag_ui_bridge::i18n::tr("Update status")),
            );
        if downloading {
            let done = self
                .update_progress
                .load(std::sync::atomic::Ordering::Relaxed);
            let total = info.asset_size;
            let percent = if total == 0 {
                0.0
            } else {
                (done as f64 * 100.0 / total as f64).clamp(0.0, 100.0)
            };
            let bar_width = (right_width - 24.).max(1.) * percent as f32 / 100.0;
            right = right.child(
                div()
                    .v_flex()
                    .gap_2()
                    .p_3()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgba(theme::HAIRLINE.to_rgba()))
                    .bg(rgb(theme::SURFACE_BASE))
                    .child(
                        div()
                            .font_semibold()
                            .child(photag_ui_bridge::i18n::tr("Downloading…")),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(theme::TEXT_SECONDARY))
                            .child(format!(
                                "{:.1} MB / {:.1} MB ({percent:.0}%)",
                                done as f64 / 1_048_576.0,
                                info.size_mb(),
                            )),
                    )
                    .child(
                        div()
                            .w_full()
                            .h(px(8.))
                            .rounded_full()
                            .bg(rgb(theme::SURFACE_ELEVATED))
                            .child(
                                div()
                                    .w(px(bar_width))
                                    .h_full()
                                    .rounded_full()
                                    .bg(rgb(theme::accent())),
                            ),
                    )
                    .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(
                        photag_ui_bridge::i18n::tr(
                            "The update offer stays open while downloading.",
                        ),
                    )),
            );
        } else if staged {
            right = right.child(
                div()
                    .v_flex()
                    .gap_2()
                    .p_3()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgba(theme::HAIRLINE.to_rgba()))
                    .bg(rgb(theme::SURFACE_BASE))
                    .child(div().font_semibold().child(photag_ui_bridge::i18n::tr("Download verified")))
                    .child(div().text_sm().text_color(rgb(theme::TEXT_SECONDARY)).child(
                        photag_ui_bridge::i18n::tr("The update is staged. Restart and install only when you are ready."),
                    ))
                    .child(div().text_xs().text_color(rgb(theme::SUCCESS)).child(
                        photag_ui_bridge::i18n::tr("The downloaded file is checked against the published SHA-256 checksum before it is staged."),
                    )),
            );
        } else {
            right = right.child(
                div()
                    .v_flex()
                    .gap_2()
                    .p_3()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgba(theme::HAIRLINE.to_rgba()))
                    .bg(rgb(theme::SURFACE_BASE))
                    .child(div().font_semibold().child(photag_ui_bridge::i18n::tr("Download")))
                    .child(div().text_sm().text_color(rgb(theme::TEXT_SECONDARY)).child(
                        photag_ui_bridge::i18n::tr("The downloaded file is checked against the published SHA-256 checksum before it is staged."),
                    )),
            );
        }
        if let Some(error) = &self.update_error {
            right = right.child(
                div()
                    .p_2()
                    .rounded_sm()
                    .bg(rgb(theme::DANGER))
                    .text_color(rgb(theme::TEXT_PRIMARY))
                    .child(error.clone()),
            );
        }
        if staged && self.confirm_install {
            right = right.child(
                div()
                    .text_sm()
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child(photag_ui_bridge::i18n::tr(
                        "PhoTag will replace its executable and launch the updated version.",
                    )),
            );
        }

        let header = div()
            .h_flex()
            .h(px(48.))
            .flex_shrink_0()
            .items_center()
            .justify_between()
            .px_4()
            .border_b_1()
            .border_color(rgba(theme::HAIRLINE.to_rgba()))
            .bg(rgb(theme::SURFACE_MODAL))
            .child(
                div()
                    .text_lg()
                    .font_semibold()
                    .child(photag_ui_bridge::i18n::tr("Update available")),
            );
        let header = if downloading {
            header
        } else {
            header.child(
                Button::new("settings-update-offer-close")
                    .w(px(32.))
                    .h(px(32.))
                    .label(photag_ui_bridge::i18n::tr(&("×").to_string()))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.update_offer_open = false;
                        this.update_offer_dismissed = true;
                        cx.notify();
                    })),
            )
        };

        let mut actions = div().h_flex().items_center().gap_2();
        if staged {
            actions = actions
                .child(
                    Button::new("settings-update-offer-restart")
                        .label(photag_ui_bridge::i18n::tr(
                            &("Restart & install").to_string(),
                        ))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.confirm_install = true;
                            this.install_update(cx);
                        })),
                )
                .child(
                    Button::new("settings-update-offer-later")
                        .label(photag_ui_bridge::i18n::tr(&("Later").to_string()))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.update_offer_open = false;
                            this.update_offer_dismissed = true;
                            cx.notify();
                        })),
                );
        } else if downloading {
            actions = actions.child(div().text_sm().text_color(rgb(theme::TEXT_MUTED)).child(
                photag_ui_bridge::i18n::tr("The update offer stays open while downloading."),
            ));
        } else {
            actions = actions
                .child(
                    Button::new("settings-update-offer-download")
                        .label(photag_ui_bridge::i18n::tr("Update now"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.begin_update_download();
                            this.monitor_updates(cx);
                        })),
                )
                .child(
                    Button::new("settings-update-offer-later")
                        .label(photag_ui_bridge::i18n::tr("Not now"))
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
                    .label(photag_ui_bridge::i18n::tr("View on GitHub"))
                    .on_click(move |_, _, _| {
                        let _ = open::that_detached(&url);
                    }),
            );
        }
        let footer = div()
            .h_flex()
            .h(px(56.))
            .flex_shrink_0()
            .items_center()
            .justify_end()
            .gap_2()
            .px_4()
            .border_t_1()
            .border_color(rgba(theme::HAIRLINE.to_rgba()))
            .bg(rgb(theme::SURFACE_MODAL))
            .child(actions);
        let body = div()
            .h_flex()
            .h(px(body_height))
            .flex_shrink_0()
            .gap_4()
            .px_4()
            .py_3()
            .bg(rgb(theme::SURFACE_MODAL))
            .child(left)
            .child(right);
        let dialog = div()
            .v_flex()
            .w(px(dialog_width))
            .h(px(dialog_height))
            .max_w(px((f32::from(viewport.width) - 32.).max(1.)))
            .max_h(px((f32::from(viewport.height) - 24.).max(1.)))
            .rounded_md()
            .border_1()
            .border_color(rgba(theme::HAIRLINE.to_rgba()))
            .bg(rgb(theme::SURFACE_MODAL))
            .text_color(rgb(theme::TEXT_PRIMARY))
            .child(header)
            .child(body)
            .child(footer);
        div()
            .absolute()
            .inset_0()
            .h_flex()
            .items_center()
            .justify_center()
            .child(div().absolute().inset_0().bg(rgba(0x0000_00b0)))
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
