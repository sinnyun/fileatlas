use super::*;

impl SettingsView {
    pub(super) fn render_tab(&self, s: &Settings, cx: &mut Context<Self>) -> impl IntoElement {
        if self.tab == Tab::Updates {
            return self.render_updates_page(s, cx);
        }
        let (title, description) = match self.tab {
            Tab::Appearance => ("Appearance", "Choose the accent color used across PhoTag."),
            Tab::Toolbar => (
                "Toolbar",
                "Choose which preview actions and position controls are shown.",
            ),
            Tab::Shortcuts => (
                "Shortcuts",
                "Set the open-media key and assign tags to F1–F9.",
            ),
            Tab::Tags => (
                "Tags",
                "Configure tag search and child-tag handling after a merge.",
            ),
            Tab::Ai => (
                "AI",
                "Configure automatic analysis and required tag groups.",
            ),
            Tab::Updates => (
                "Updates",
                "Check for releases and choose when to install them.",
            ),
        };
        let mut b = div().v_flex().gap_3().line_height(relative(1.)).child(
            div()
                .v_flex()
                .gap_1()
                .child(
                    div()
                        .text_lg()
                        .font_semibold()
                        .child(photag_ui_bridge::i18n::tr(title)),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(theme::TEXT_SECONDARY))
                        .child(photag_ui_bridge::i18n::tr(description)),
                ),
        );
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
                b = b.child(self.section("Accent preview")).child(
                    div()
                        .h_flex()
                        .items_center()
                        .gap_2()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(rgba(theme::HAIRLINE.to_rgba()))
                        .bg(rgba(theme::modal_rgba(theme::SURFACE_MODAL)))
                        .child(div().font_semibold().child("PhoTag"))
                        .child(div().flex_1())
                        .child(div().text_sm().child(photag_ui_bridge::i18n::tr("Library")))
                        .child(
                            div()
                                .px_2()
                                .py_1()
                                .rounded_sm()
                                .bg(rgb(theme::resolve_accent(&s.accent_name)))
                                .text_color(rgb(theme::TEXT_PRIMARY))
                                .child(photag_ui_bridge::i18n::tr("Gallery")),
                        )
                        .child(div().text_sm().child(photag_ui_bridge::i18n::tr("Tags"))),
                );
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
                        vec![
                            (
                                "tb.filmstrip",
                                "Filmstrip toggle",
                                s.toolbar.filmstrip_toggle,
                            ),
                            (
                                "date",
                                "Group date-sorted gallery by month",
                                s.show_date_dividers,
                            ),
                        ],
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
                b = b.child(self.section("Search match"));
                b = b.child(div().text_xs().text_color(rgb(theme::TEXT_SECONDARY)).child(
                    photag_ui_bridge::i18n::tr("Choose whether search matches anywhere in a tag name or only from its beginning."),
                ));
                let mut match_options = div().h_flex().gap_2();
                for opt in [TagSearchMatch::Contains, TagSearchMatch::StartsWith] {
                    let chosen = s.tag_search_match == opt;
                    match_options = match_options.child(self.enum_button(
                        format!("tag-match-{}", opt.label()),
                        format!("{}{}", if chosen { "● " } else { "" }, opt.label()),
                        move |s| s.tag_search_match = opt,
                        cx,
                    ));
                }
                b = b.child(match_options);
                b = b.child(self.section("Source tag children after merge"));
                b = b.child(div().text_xs().text_color(rgb(theme::TEXT_SECONDARY)).child(
                    photag_ui_bridge::i18n::tr("Choose whether child tags move to the target tag or are removed with the merged tag."),
                ));
                let mut merge_options = div().h_flex().gap_2();
                for opt in [TagMergeChildren::Reparent, TagMergeChildren::Delete] {
                    let chosen = s.tag_merge_children == opt;
                    merge_options = merge_options.child(self.enum_button(
                        format!("tag-merge-{}", opt.label()),
                        format!("{}{}", if chosen { "● " } else { "" }, opt.label()),
                        move |s| s.tag_merge_children = opt,
                        cx,
                    ));
                }
                b = b.child(merge_options);
            }
            Tab::Ai => {
                b = b
                    .child(self.toggle_button(
                        "ai",
                        "Automatically analyze opened photos",
                        s.ai.auto_analyze,
                        cx,
                    ))
                    .child(self.section("Mandatory tag groups"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(theme::TEXT_SECONDARY))
                            .child(photag_ui_bridge::i18n::tr(
                                "Each group defines required tags for automatic analysis.",
                            )),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(theme::TEXT_SECONDARY))
                            .child(photag_ui_bridge::i18n::tr(
                                "Add a group, rename it, and select or remove its tags below.",
                            )),
                    );
                b = b.child(self.ai_groups(s, cx));
            }
            Tab::Updates => {}
        }
        b
    }
}
