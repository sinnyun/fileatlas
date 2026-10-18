use super::*;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};

impl SettingsView {
    pub(super) fn render_tab(
        &self,
        s: &Settings,
        width: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        if self.tab == Tab::Shortcuts {
            return self.render_shortcuts(s, width, cx).into_any_element();
        }
        if self.tab == Tab::Updates {
            return self.render_updates_page(s, cx).into_any_element();
        }
        if self.tab == Tab::Software {
            return self.render_software_page(cx).into_any_element();
        }
        let (title, description) = match self.tab {
            Tab::Appearance => ("Appearance", "Choose the accent color used across PhoTag."),
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
            Tab::Software => ("Software", "Check optional software used by PhoTag."),
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
                for name in theme::ACCENT_PRESETS
                    .iter()
                    .map(|preset| preset.name)
                    .chain(std::iter::once(theme::SYSTEM_ACCENT_NAME))
                {
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
                b = b.child(self.section("Region separation"));
                let separation_view = cx.entity().downgrade();
                let separation = s.region_separation.clone();
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
                                        .text_xs()
                                        .text_color(rgb(theme::TEXT_SECONDARY))
                                        .child(photag_ui_bridge::i18n::tr(
                                            "Separate the sidebar, content and preview regions with a hairline border or a shadow.",
                                        )),
                                ),
                        )
                        .child(
                            Button::new("settings-region-separation")
                                .w(px(100.))
                                .label(photag_ui_bridge::i18n::tr(
                                    if s.region_separation == "shadow" {
                                        "Shadow"
                                    } else {
                                        "Border"
                                    },
                                ))
                                .dropdown_caret(true)
                                .dropdown_menu(move |mut menu, _, _| {
                                    for (value, label) in
                                        [("border", "Border"), ("shadow", "Shadow")]
                                    {
                                        let weak = separation_view.clone();
                                        menu = menu.item(
                                            PopupMenuItem::new(photag_ui_bridge::i18n::tr(label))
                                                .checked(separation == value)
                                                .on_click(move |_, _, cx| {
                                                    let _ = weak.update(cx, |view, cx| {
                                                        view.edit(
                                                            |settings| {
                                                                settings.region_separation =
                                                                    value.to_owned()
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
            Tab::Shortcuts => {}
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
                    ("search.time", "Time range", s.search_scopes.time),
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
            Tab::Software => {}
        }
        b.into_any_element()
    }
}
