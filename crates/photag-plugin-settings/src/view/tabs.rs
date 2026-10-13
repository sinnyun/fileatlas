use super::*;

impl SettingsView {
    pub(super) fn render_tab(&self, s: &Settings, cx: &mut Context<Self>) -> impl IntoElement {
        let mut b = div().v_flex().gap_2().p_3().line_height(relative(1.));
        match self.tab {
            Tab::Appearance => {
                b = b.children(self.language_views.clone());
                b = b.child(self.section("Accent color"));
                for name in [
                    "System",
                    "Slate Blue",
                    "Dusty Teal",
                    "Muted Copper",
                    "Dusty Rose",
                    "Sage Green",
                    "Dusty Mauve",
                    "Warm Taupe",
                ] {
                    let selected = s.accent_name == name;
                    let value = name.to_owned();
                    let swatch = Button::new(format!("settings-accent-{name}"))
                        .compact()
                        .w(px(22.))
                        .h(px(22.))
                        .rounded(px(4.))
                        .bg(rgb(theme::resolve_accent(name)))
                        .border_color(rgb(if selected {
                            theme::TEXT_PRIMARY
                        } else {
                            theme::SURFACE_MODAL
                        }))
                        .border(px(if selected { 1.5 } else { 0. }))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let value = value.clone();
                            this.edit(move |s| s.accent_name = value.clone(), cx);
                        }));
                    b = b.child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap(px(10.))
                            .h(px(22.))
                            .child(swatch)
                            .child(
                                div()
                                    .text_size(px(13.5))
                                    .child(photag_ui_bridge::i18n::tr(name)),
                            ),
                    );
                }
                b = b.child(div().text_xs().text_color(rgb(theme::TEXT_SECONDARY)).child(
                    photag_ui_bridge::i18n::tr("System follows the Windows accent color. Preview and feature panels use the shared accent palette."),
                ));
                b = b.child(self.toggle_button(
                    "date",
                    "Group date-sorted gallery by month",
                    s.show_date_dividers,
                    cx,
                ));
            }
            Tab::Toolbar => {
                for (group, controls) in [
                    (
                        "Zoom controls",
                        vec![
                            (
                                "tb.zoom_mode",
                                "Zoom mode toggle",
                                s.toolbar.zoom_mode_button,
                            ),
                            ("tb.zoom_reset", "Reset zoom", s.toolbar.zoom_reset_button),
                            ("tb.zoom_presets", "Zoom presets", s.toolbar.zoom_presets),
                        ],
                    ),
                    (
                        "Photo manipulation",
                        vec![
                            ("tb.rotate", "Rotation controls", s.toolbar.rotate_buttons),
                            ("tb.flip", "Mirror controls", s.toolbar.flip_buttons),
                        ],
                    ),
                    (
                        "Tags",
                        vec![("tb.tags", "Tags dropdown", s.toolbar.tags_button)],
                    ),
                    (
                        "File",
                        vec![
                            (
                                "tb.delete",
                                "Delete to recycle bin",
                                s.toolbar.delete_button,
                            ),
                            ("tb.copy", "Copy to…", s.toolbar.copy_button),
                            ("tb.move", "Move to…", s.toolbar.move_button),
                            ("tb.explorer", "Show in Explorer", s.toolbar.explorer_button),
                            (
                                "tb.external",
                                "Open in default app",
                                s.toolbar.open_external_button,
                            ),
                        ],
                    ),
                    (
                        "View",
                        vec![(
                            "tb.filmstrip",
                            "Filmstrip toggle",
                            s.toolbar.filmstrip_toggle,
                        )],
                    ),
                ] {
                    let mut section = div().v_flex().gap(px(6.)).child(
                        div()
                            .font_semibold()
                            .child(photag_ui_bridge::i18n::tr(group)),
                    );
                    for (k, n, v) in controls {
                        section = section.child(self.toggle_button(k, n, v, cx));
                    }
                    b = b.child(section);
                }
                b = b
                    .child(self.toggle_button(
                        "filmstrip",
                        "Show filmstrip by default",
                        s.show_filmstrip,
                        cx,
                    ))
                    .child(self.toggle_button(
                        "pos.locked",
                        "Lock floating toolbar position",
                        s.toolbar_pos.locked,
                        cx,
                    ))
                    .child(div().child(format!(
                        "Toolbar offset: {:.0}, {:.0} px",
                        s.toolbar_pos.offset_x, s.toolbar_pos.offset_y
                    )))
                    .child(div().child(photag_ui_bridge::i18n::tr(
                        "Adjust the preview toolbar position in 10 px steps.",
                    )))
                    .child(
                        div()
                            .h_flex()
                            .gap_1()
                            .child(self.enum_button(
                                "pos-left",
                                "← 10 px".into(),
                                |s| s.toolbar_pos.offset_x -= 10.0,
                                cx,
                            ))
                            .child(self.enum_button(
                                "pos-right",
                                "10 px →".into(),
                                |s| s.toolbar_pos.offset_x += 10.0,
                                cx,
                            ))
                            .child(self.enum_button(
                                "pos-up",
                                "↑ 10 px".into(),
                                |s| s.toolbar_pos.offset_y -= 10.0,
                                cx,
                            ))
                            .child(self.enum_button(
                                "pos-down",
                                "10 px ↓".into(),
                                |s| s.toolbar_pos.offset_y += 10.0,
                                cx,
                            ))
                            .child(self.enum_button(
                                "pos-reset",
                                "Reset position".into(),
                                |s| {
                                    s.toolbar_pos.offset_x = 0.0;
                                    s.toolbar_pos.offset_y = 0.0;
                                },
                                cx,
                            )),
                    );
            }
            Tab::Shortcuts => {
                b = b.child(self.section("Tag sidebar modifier"));
                for opt in [
                    ExcludeModifier::None,
                    ExcludeModifier::Ctrl,
                    ExcludeModifier::Shift,
                    ExcludeModifier::Alt,
                ] {
                    let selected = s.exclude_modifier == opt;
                    b = b.child(self.enum_button(
                        format!("exclude-modifier-{}", opt.label()),
                        format!("{}{}", if selected { "● " } else { "" }, opt.label()),
                        move |s| s.exclude_modifier = opt,
                        cx,
                    ));
                }
                b = b
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap_2()
                            .child(photag_ui_bridge::i18n::tr_args("Open selected media: {key}", &[("key", s.open_key.clone())]))
                            .child(self.open_key_capture_button(cx)),
                    )
                    .child(div().child(
                        photag_ui_bridge::i18n::tr("Gallery ratings use number-row keys. Preview applies 0–5 to the current item; F1–F9 toggles its assigned tag."),
                    ));
                b = b.child(self.section("Tag hotkeys F1–F9"));
                b = b.child(div().child(photag_ui_bridge::i18n::tr(
                    "Press an F-key in the gallery or preview to toggle its assigned tag.",
                )));
                match self
                    .navigation
                    .as_ref()
                    .and_then(Weak::upgrade)
                    .map(|navigation| navigation.snapshot())
                {
                    Some(Ok(snapshot)) => {
                        let mut flat_tags = Vec::new();
                        let mut pending: Vec<_> = snapshot.tags.into_iter().rev().collect();
                        while let Some(tag) = pending.pop() {
                            let icon = tag
                                .icon
                                .as_deref()
                                .filter(|icon| !icon.is_empty())
                                .unwrap_or("🏷");
                            flat_tags.push((
                                tag.id,
                                format!("{icon} {}", tag.full_path.replace('>', " > ")),
                            ));
                            pending.extend(tag.children.into_iter().rev());
                        }
                        for key in ["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9"] {
                            let selected = s
                                .tag_hotkeys
                                .iter()
                                .find(|hotkey| hotkey.key == key)
                                .map(|hotkey| hotkey.tag_id);
                            b = b.child(tag_hotkeys::button(
                                key,
                                selected,
                                flat_tags.clone(),
                                cx.entity().downgrade(),
                            ));
                        }
                    }
                    Some(Err(error)) => {
                        b = b.child(div().child(format!("Could not load tags: {error}")));
                    }
                    None => {
                        b = b.child(div().child(photag_ui_bridge::i18n::tr(
                            "Tag choices are unavailable while the navigation plugin is stopped.",
                        )));
                        for key in ["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9"] {
                            let selected = s
                                .tag_hotkeys
                                .iter()
                                .find(|hotkey| hotkey.key == key)
                                .map(|hotkey| hotkey.tag_id);
                            let assigned = selected
                                .map(|id| format!("tag #{id}"))
                                .unwrap_or_else(|| "None".into());
                            b = b.child(div().child(format!("{key}: {assigned}")));
                        }
                    }
                }
            }
            Tab::Tags => {
                b = b.child(self.section("Global search scope"));
                for (k, n, v) in [
                    ("search.files", "Filenames", s.search_scopes.filenames),
                    ("search.paths", "Paths", s.search_scopes.paths),
                    ("search.tags", "Tags", s.search_scopes.tags),
                ] {
                    b = b.child(self.toggle_button(k, n, v, cx));
                }
                b = b.child(self.toggle_button(
                    "persist",
                    "Keep tag search query when switching tabs",
                    s.tag_search_persist,
                    cx,
                ));
                for opt in [TagSearchMatch::Contains, TagSearchMatch::StartsWith] {
                    let chosen = s.tag_search_match == opt;
                    b = b.child(self.enum_button(
                        format!("tag-match-{}", opt.label()),
                        format!("{}{}", if chosen { "● " } else { "" }, opt.label()),
                        move |s| s.tag_search_match = opt,
                        cx,
                    ));
                }
                b = b.child(self.section("Source tag children after merge"));
                for opt in [TagMergeChildren::Reparent, TagMergeChildren::Delete] {
                    let chosen = s.tag_merge_children == opt;
                    b = b.child(self.enum_button(
                        format!("tag-merge-{}", opt.label()),
                        format!("{}{}", if chosen { "● " } else { "" }, opt.label()),
                        move |s| s.tag_merge_children = opt,
                        cx,
                    ));
                }
            }
            Tab::Ai => {
                b = b
                    .child(self.section("AI analysis"))
                    .child(self.toggle_button(
                        "ai",
                        "Automatically analyze opened photos",
                        s.ai.auto_analyze,
                        cx,
                    ))
                    .child(self.section("Mandatory tag groups"));
                b = b.child(self.ai_groups(s, cx));
            }
            Tab::Updates => {
                b = b
                    .child(self.section("Updates"))
                    .child(div().child(format!(
                        "PhoTag {}",
                        photag_plugin_updater::current_version()
                    )))
                    .child(self.toggle_button(
                        "updates",
                        "Check for updates on startup",
                        s.auto_check_updates,
                        cx,
                    ));
                if self.updater.is_some() {
                    let checking = self.update_check.is_some();
                    b = b.child(
                        Button::new("settings-update-check")
                            .label(photag_ui_bridge::i18n::tr(
                                &(if checking { "Checking…" } else { "Check now" }).to_string(),
                            ))
                            .on_click(cx.listener(|this, _, _, cx| {
                                if this.update_check.is_none() {
                                    this.begin_update_check(true);
                                    this.monitor_updates(cx);
                                }
                            })),
                    );
                    if let Some(status) = &self.update_status {
                        b = b.child(div().child(status.clone()));
                    }
                    if let Some(error) = &self.update_error {
                        b = b.child(
                            div()
                                .text_color(rgb(theme::DANGER))
                                .child(format!("Update failed: {error}")),
                        );
                    }
                    if let Some(info) = &self.update_info {
                        b = b
                            .child(div().child(format!("Release notes\n{}", info.notes_plain())))
                            .child(
                                div()
                                    .text_sm()
                                    .child(format!("Release page: {}", info.html_url)),
                            );
                        if self.update_download.is_some() {
                            let got = self.update_progress.load(Ordering::Relaxed);
                            let total = info.asset_size;
                            b = b.child(div().child(format!(
                                "Downloading: {:.1} / {:.1} MB ({:.0}%)",
                                got as f64 / 1_048_576.0,
                                info.size_mb(),
                                if total == 0 {
                                    0.0
                                } else {
                                    got as f64 * 100.0 / total as f64
                                }
                            )));
                        } else if self.update_staged.is_none() {
                            b = b.child(
                                Button::new("settings-update-download")
                                    .label(photag_ui_bridge::i18n::tr("Download and verify update"))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if this.update_download.is_none() {
                                            this.begin_update_download();
                                            this.monitor_updates(cx);
                                        }
                                    })),
                            );
                        }
                        if self.update_offer_dismissed {
                            b = b.child(
                                Button::new("settings-update-offer-reopen")
                                    .label(photag_ui_bridge::i18n::tr("Show update offer…"))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.update_offer_open = true;
                                        this.update_offer_dismissed = false;
                                        cx.notify();
                                    })),
                            );
                        }
                    }
                    if self.update_staged.is_some() {
                        b = b.child(
                            Button::new("settings-update-install")
                                .label(photag_ui_bridge::i18n::tr(
                                    &(if self.confirm_install {
                                        "Confirm restart and install"
                                    } else {
                                        "Restart and install…"
                                    })
                                    .to_string(),
                                ))
                                .on_click(cx.listener(|this, _, _, cx| this.install_update(cx))),
                        );
                        if self.confirm_install {
                            b = b.child(div().child(photag_ui_bridge::i18n::tr("PhoTag will replace its executable and launch the updated version.")));
                        }
                    }
                } else {
                    b = b.child(div().child(photag_ui_bridge::i18n::tr(
                        "Updater service is unavailable.",
                    )));
                }
            }
        }
        b
    }
}
