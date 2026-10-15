use super::*;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};

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
                b = b.child(self.section("Accent source"));
                for name in [
                    "Slate Blue",
                    "Dusty Teal",
                    "Muted Copper",
                    "Dusty Rose",
                    "Sage Green",
                    "Dusty Mauve",
                    "Warm Taupe",
                    "System",
                ] {
                    let selected = s.accent_name == name;
                    let value = name.to_owned();
                    let mut swatch = Button::new(format!("settings-accent-{name}"))
                        .compact()
                        .p_0()
                        .min_w(px(22.))
                        .w(px(22.))
                        .h(px(22.))
                        .rounded_full()
                        .bg(rgb(theme::resolve_accent(name)))
                        .border_color(rgb(if selected {
                            theme::TEXT_PRIMARY
                        } else {
                            theme::TEXT_SECONDARY
                        }))
                        .border(px(if selected { 2. } else { 1. }));
                    if selected {
                        swatch = swatch.child(
                            div()
                                .size(px(6.))
                                .rounded_full()
                                .bg(rgb(theme::TEXT_PRIMARY)),
                        );
                    }
                    swatch = swatch.on_click(cx.listener(move |this, _, _, cx| {
                        let value = value.clone();
                        this.edit(move |s| s.accent_name = value.clone(), cx);
                    }));
                    b = b.child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap(px(9.))
                            .h(px(25.))
                            .child(swatch)
                            .child(
                                div()
                                    .text_size(px(13.5))
                                    .child(photag_ui_bridge::i18n::tr(name)),
                            ),
                    );
                }
                b = b.child(
                    div()
                        .text_xs()
                        .text_color(rgb(theme::TEXT_SECONDARY))
                        .child(photag_ui_bridge::i18n::tr(
                            "System follows the Windows accent color.",
                        )),
                );
                b = b.child(self.section("Interface preview")).child(
                    div()
                        .h_flex()
                        .items_center()
                        .gap(px(16.))
                        .h(px(50.))
                        .px_3()
                        .rounded_md()
                        .border_1()
                        .border_color(rgba(theme::HAIRLINE.to_rgba()))
                        .bg(rgba(theme::modal_rgba(theme::SURFACE_MODAL)))
                        .child(
                            div()
                                .h_flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    theme::icon(gpui_kit::assets::IconName::Aperture)
                                        .text_color(rgb(theme::accent())),
                                )
                                .child(div().font_semibold().child("PhoTag")),
                        )
                        .child(
                            div()
                                .w(px(1.))
                                .h(px(22.))
                                .bg(rgba(theme::HAIRLINE.to_rgba())),
                        )
                        .child(theme::icon(gpui_kit::assets::IconName::House))
                        .child(theme::icon(gpui_kit::assets::IconName::Folder))
                        .child(div().flex_1())
                        .child(theme::icon(gpui_kit::assets::IconName::Star))
                        .child(
                            div()
                                .h_flex()
                                .items_center()
                                .justify_center()
                                .w(px(30.))
                                .h_full()
                                .border_b_1()
                                .border_color(rgb(theme::accent()))
                                .child(
                                    theme::icon(gpui_kit::assets::IconName::Image)
                                        .text_color(rgb(theme::accent())),
                                ),
                        ),
                );
            }
            Tab::Toolbar => {
                b = b.child(self.toolbar_settings(s, cx));
            }
            Tab::Shortcuts => {
                b = b.child(self.section("Tag sidebar modifier"));
                let modifier = s.exclude_modifier;
                let weak = cx.entity().downgrade();
                b = b.child(
                    div()
                        .h_flex()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .child(
                            div()
                                .v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .child(photag_ui_bridge::i18n::tr("Exclude shortcut")),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(theme::TEXT_SECONDARY))
                                        .child(photag_ui_bridge::i18n::tr(
                                            "Hold this key while clicking a tag to exclude it.",
                                        )),
                                ),
                        )
                        .child(
                            Button::new("settings-exclude-modifier")
                                .w(px(100.))
                                .label(photag_ui_bridge::i18n::tr(modifier.label()))
                                .dropdown_caret(true)
                                .dropdown_menu(move |mut menu, _, _| {
                                    for option in [
                                        ExcludeModifier::None,
                                        ExcludeModifier::Ctrl,
                                        ExcludeModifier::Shift,
                                        ExcludeModifier::Alt,
                                    ] {
                                        let weak = weak.clone();
                                        menu = menu.item(
                                            PopupMenuItem::new(photag_ui_bridge::i18n::tr(
                                                option.label(),
                                            ))
                                            .checked(option == modifier)
                                            .on_click(move |_, _, cx| {
                                                let _ = weak.update(cx, |view, cx| {
                                                    view.edit(
                                                        |settings| {
                                                            settings.exclude_modifier = option
                                                        },
                                                        cx,
                                                    )
                                                });
                                            }),
                                        );
                                    }
                                    menu
                                }),
                        ),
                );
                b = b.child(self.section("Open media file"));
                b = b.child(div().h_flex().items_center().justify_between().gap_3()
                    .child(div().v_flex().gap_1()
                        .child(div().text_sm().child(photag_ui_bridge::i18n::tr("Open selected media")))
                        .child(div().text_xs().text_color(rgb(theme::TEXT_SECONDARY))
                            .child(photag_ui_bridge::i18n::tr("Use this key in the gallery or preview to open the selected file."))))
                    .child(div().h_flex().items_center().gap_2()
                        .child(div().px_3().py_1().rounded_sm().border_1()
                            .border_color(rgba(theme::HAIRLINE.to_rgba())).bg(rgb(theme::SURFACE_BASE))
                            .child(s.open_key.clone()))
                        .child(self.open_key_capture_button(cx))));
                b = b.child(self.section("Rating shortcuts"));
                b = b.child(div().text_sm().child(photag_ui_bridge::i18n::tr(
                    "Use number keys 0–5 to set a rating; in preview, 0 clears the rating.",
                )));
                b = b.child(self.section("Tag target shortcuts (F1–F9)"));
                b = b.child(div().text_xs().text_color(rgb(theme::TEXT_SECONDARY)).child(photag_ui_bridge::i18n::tr(
                    "Choose a tag for each key. Press that key in the gallery or preview to toggle the tag.",
                )));
                match self.tag_choices.value() {
                    Some(Ok(choices)) => {
                        let flat_tags = choices
                            .iter()
                            .map(|tag| (tag.id, tag.label.clone()))
                            .collect::<Vec<_>>();
                        let mut rows = div().v_flex().gap(px(3.));
                        for key in ["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9"] {
                            let selected = s
                                .tag_hotkeys
                                .iter()
                                .find(|hotkey| hotkey.key == key)
                                .map(|hotkey| hotkey.tag_id);
                            rows = rows.child(
                                div()
                                    .h_flex()
                                    .w_full()
                                    .flex_1()
                                    .min_w_0()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .w(px(42.))
                                            .h(px(24.))
                                            .flex_shrink_0()
                                            .h_flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(rgba(theme::HAIRLINE.to_rgba()))
                                            .bg(rgb(theme::SURFACE_BASE))
                                            .text_sm()
                                            .font_semibold()
                                            .child(key),
                                    )
                                    .child(tag_hotkeys::button(
                                        key,
                                        selected,
                                        flat_tags.clone(),
                                        cx.entity().downgrade(),
                                    )),
                            );
                        }
                        b = b.child(rows);
                    }
                    Some(Err(error)) => {
                        b = b.child(div().child(format!("Could not load tags: {error}")));
                    }
                    None => {
                        b = b.child(div().child(photag_ui_bridge::i18n::tr("Loading file info…")));
                        let mut rows = div().v_flex().gap(px(3.));
                        for key in ["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9"] {
                            let selected = s
                                .tag_hotkeys
                                .iter()
                                .find(|hotkey| hotkey.key == key)
                                .map(|hotkey| hotkey.tag_id);
                            let assigned = selected
                                .map(|id| format!("tag #{id}"))
                                .unwrap_or_else(|| "None".into());
                            rows = rows.child(
                                div()
                                    .h_flex()
                                    .w_full()
                                    .flex_1()
                                    .min_w_0()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .w(px(42.))
                                            .h(px(24.))
                                            .flex_shrink_0()
                                            .h_flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(rgba(theme::HAIRLINE.to_rgba()))
                                            .bg(rgb(theme::SURFACE_BASE))
                                            .text_sm()
                                            .font_semibold()
                                            .child(key),
                                    )
                                    .child(div().flex_1().child(assigned)),
                            );
                        }
                        b = b.child(rows);
                    }
                }
            }
            Tab::Tags => {
                b = b.child(self.section("Search match"));
                b = b.child(div().text_xs().text_color(rgb(theme::TEXT_SECONDARY)).child(
                    photag_ui_bridge::i18n::tr("Choose whether search matches anywhere in a tag name or only from its beginning."),
                ));
                let match_mode = s.tag_search_match;
                let weak = cx.entity().downgrade();
                b = b.child(div().h_flex().items_center().justify_between().gap_3()
                    .child(div().v_flex().gap_1()
                        .child(div().text_sm().child(photag_ui_bridge::i18n::tr("Tag name matching")))
                        .child(div().text_xs().text_color(rgb(theme::TEXT_SECONDARY))
                            .child(photag_ui_bridge::i18n::tr("Choose whether a query can match anywhere or only at the start of a tag name."))))
                    .child(Button::new("settings-tag-match")
                        .label(photag_ui_bridge::i18n::tr(match_mode.label()))
                        .dropdown_menu(move |mut menu, _, _| {
                            for option in [TagSearchMatch::Contains, TagSearchMatch::StartsWith] {
                                let weak = weak.clone();
                                menu = menu.item(PopupMenuItem::new(photag_ui_bridge::i18n::tr(option.label()))
                                    .checked(option == match_mode)
                                    .on_click(move |_, _, cx| {
                                        let _ = weak.update(cx, |view, cx| view.edit(|settings| settings.tag_search_match = option, cx));
                                    }));
                            }
                            menu
                        })));
                b = b.child(self.section("Search scope"));
                for (key, label, enabled) in [
                    ("search.files", "Filenames", s.search_scopes.filenames),
                    ("search.paths", "Paths", s.search_scopes.paths),
                    ("search.tags", "Tags", s.search_scopes.tags),
                ] {
                    b = b.child(self.toggle_button(key, label, enabled, cx));
                }
                b = b.child(self.toggle_button(
                    "persist",
                    "Keep search query when switching tabs",
                    s.tag_search_persist,
                    cx,
                ));
                b = b.child(self.section("Source tag children after merge"));
                b = b.child(div().text_xs().text_color(rgb(theme::TEXT_SECONDARY)).child(
                    photag_ui_bridge::i18n::tr("Choose whether child tags move to the target tag or are removed with the merged tag."),
                ));
                let mut merge_options = div().v_flex().gap_1();
                for opt in [TagMergeChildren::Reparent, TagMergeChildren::Delete] {
                    let chosen = s.tag_merge_children == opt;
                    merge_options = merge_options.child(
                        Button::new(format!("settings-tag-merge-{}", opt.label()))
                            .icon(theme::small_icon(if chosen {
                                gpui_kit::assets::IconName::CircleDot
                            } else {
                                gpui_kit::assets::IconName::Circle
                            }))
                            .label(photag_ui_bridge::i18n::tr(opt.label()))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.edit(|settings| settings.tag_merge_children = opt, cx);
                            })),
                    );
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
