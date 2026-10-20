use super::SettingsView;
use crate::Settings;
use fileatlas_ui_bridge::theme;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::*;
use std::sync::atomic::Ordering;

impl SettingsView {
    pub(super) fn render_updates_page(&self, settings: &Settings, cx: &mut Context<Self>) -> Div {
        let mut page = div().v_flex().gap(px(theme::SPACE_4));
        page = page.child(self.section("Current version"));
        page = page.child(
            div()
                .h_flex()
                .items_center()
                .justify_between()
                .gap_2()
                .p_3()
                .border_b_1()
                .border_color(rgb(theme::separator()))
                .child(
                    div()
                        .v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(theme::text_secondary()))
                                .child(fileatlas_ui_bridge::i18n::tr("Current version")),
                        )
                        .child(div().font_semibold().child(format!(
                            "FileAtlas {}",
                            fileatlas_plugin_updater::current_version()
                        ))),
                )
                .child(self.toggle_button(
                    "updates",
                    "Check for updates on startup",
                    settings.auto_check_updates,
                    cx,
                )),
        );

        if self.updater.is_none() {
            return page.child(div().child(fileatlas_ui_bridge::i18n::tr(
                "Updater service is unavailable.",
            )));
        }

        page = page.child(self.section("Check for updates"));
        let checking = self.update_check.is_some();
        let mut check_row = div().h_flex().items_center().gap_2().child(
            Button::new("settings-update-check")
                .label(fileatlas_ui_bridge::i18n::tr(
                    &(if checking { "Checking…" } else { "Check now" }).to_string(),
                ))
                .on_click(cx.listener(|this, _, _, cx| {
                    if this.update_check.is_none() {
                        this.begin_update_check(true);
                        this.monitor_updates(cx);
                    }
                })),
        );
        if let Some(info) = &self.update_info {
            let url = if info.html_url.is_empty() {
                fileatlas_plugin_updater::releases_url()
            } else {
                info.html_url.clone()
            };
            check_row = check_row.child(
                Button::new("settings-update-release-page")
                    .label(fileatlas_ui_bridge::i18n::tr("Open release page"))
                    .on_click(move |_, _, _| {
                        let _ = open::that_detached(&url);
                    }),
            );
        }

        let (status, status_color) = if checking {
            (
                "Checking GitHub for a newer release…".to_owned(),
                theme::text_secondary(),
            )
        } else if let Some(error) = &self.update_error {
            (format!("Update error: {error}"), theme::danger_text())
        } else if let Some(status) = &self.update_status {
            (status.clone(), theme::text_secondary())
        } else {
            (
                "No update check has run in this session.".to_owned(),
                theme::text_secondary(),
            )
        };
        page = page.child(
            div()
                .h_flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(theme::FONT_SIZE_SMALL))
                        .text_color(rgb(status_color))
                        .child(status),
                )
                .child(check_row),
        );

        if let Some(info) = &self.update_info {
            page = page.child(self.section("Release"));
            let mut release = div().v_flex().gap_2().py(px(theme::SPACE_2)).child(
                div()
                    .h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .font_semibold()
                            .child(format!("FileAtlas {}", info.version)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(theme::text_secondary()))
                            .child(format!("{:.1} MB", info.size_mb())),
                    ),
            );

            let notes = info.notes_plain();
            if !notes.is_empty() {
                release = release.child(
                    div()
                        .h(px(150.))
                        .max_h(px(150.))
                        .overflow_y_scrollbar()
                        .text_sm()
                        .text_color(rgb(theme::text_secondary()))
                        .child(notes),
                );
            }
            release = release.child(
                div()
                    .text_xs()
                    .text_color(rgb(theme::text_secondary()))
                    .child(fileatlas_ui_bridge::i18n::tr(
                        "The downloaded file is checked against the published SHA-256 checksum before it is staged.",
                    )),
            );

            if self.update_download.is_some() {
                let downloaded = self.update_progress.load(Ordering::Relaxed);
                let total = info.asset_size;
                release = release.child(div().text_sm().child(format!(
                    "Downloading: {:.1} / {:.1} MB ({:.0}%)",
                    downloaded as f64 / 1_048_576.0,
                    info.size_mb(),
                    if total == 0 {
                        0.0
                    } else {
                        downloaded as f64 * 100.0 / total as f64
                    }
                )));
            } else if self.update_staged.is_none() {
                release = release.child(
                    div().h_flex().justify_end().child(
                        Button::new("settings-update-download")
                            .primary()
                            .h(px(theme::ACTION_HEIGHT))
                            .label(fileatlas_ui_bridge::i18n::tr("Download and verify update"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                if this.update_download.is_none() {
                                    this.begin_update_download();
                                    this.monitor_updates(cx);
                                }
                            })),
                    ),
                );
            }
            if self.update_staged.is_some() {
                release = release.child(
                    div()
                        .h_flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(
                            div()
                                .v_flex()
                                .gap_1()
                                .child(
                                    div().font_semibold().child(
                                        fileatlas_ui_bridge::i18n::tr("Download verified"),
                                    ),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(theme::text_secondary()))
                                        .child(fileatlas_ui_bridge::i18n::tr(
                                            "The update is staged. Restart and install only when you are ready.",
                                        )),
                                ),
                        )
                        .child(
                            Button::new("settings-update-install")
                                .label(fileatlas_ui_bridge::i18n::tr(
                                    &(if self.confirm_install {
                                        "Confirm restart and install"
                                    } else {
                                        "Restart and install…"
                                    })
                                    .to_string(),
                                ))
                                .on_click(cx.listener(|this, _, _, cx| this.install_update(cx))),
                        ),
                );
                if self.confirm_install {
                    release = release.child(
                        div()
                            .text_sm()
                            .text_color(rgb(theme::text_secondary()))
                            .child(fileatlas_ui_bridge::i18n::tr(
                                "FileAtlas will replace its executable and launch the updated version.",
                            )),
                    );
                }
            }
            page = page.child(release);
            if self.update_offer_dismissed {
                page = page.child(
                    Button::new("settings-update-offer-reopen")
                        .label(fileatlas_ui_bridge::i18n::tr("Show update offer…"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.update_offer_open = true;
                            this.update_offer_dismissed = false;
                            cx.notify();
                        })),
                );
            }
        }
        page
    }
}
