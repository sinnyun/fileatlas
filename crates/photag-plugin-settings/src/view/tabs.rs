use super::*;
use gpui_kit::base::Disableable;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};

impl SettingsView {
    pub(super) fn render_tab(
        &self,
        s: &Settings,
        width: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        if matches!(self.tab, Tab::Plugins) {
            return self.render_plugin_cards(s, cx).into_any_element();
        }
        if self.tab == Tab::PluginSettings {
            return self.render_plugin_settings(s, cx).into_any_element();
        }
        if self.tab == Tab::Shortcuts {
            return self.render_shortcuts(s, width, cx).into_any_element();
        }
        if self.tab == Tab::Updates {
            return self.render_updates_page(s, cx).into_any_element();
        }
        if self.tab == Tab::Software {
            return self.render_software_page(cx, false).into_any_element();
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
            Tab::Software => ("Software", "Check the external software used by PhoTag."),
            Tab::Plugins => ("Plugins", "Manage optional feature plugins."),
            Tab::PluginSettings => ("Plugin settings", "Configure this feature plugin."),
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
                b = b.children(self.language_views.iter().map(|(_, _, view)| view.clone()));
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
            Tab::Software | Tab::Plugins | Tab::PluginSettings => {}
        }
        b.into_any_element()
    }

    fn render_plugin_cards(&self, _s: &Settings, cx: &mut Context<Self>) -> Div {
        let runtime = self.service.plugin_runtime();
        let statuses = runtime.snapshot_now();
        let mut body = div().v_flex().gap(px(theme::PANEL_GAP)).child(
            div().v_flex().gap_1()
                .child(div().text_lg().font_semibold().child(photag_ui_bridge::i18n::tr("Plugins")))
                .child(div().text_sm().text_color(rgb(theme::TEXT_SECONDARY)).child(photag_ui_bridge::i18n::tr("Configure feature plugins. Plugin lifecycle controls apply to the running Cordis plugin."))),
        );
        for plugin in statuses {
            let id = plugin.id.clone();
            let name = plugin.title.clone();
            let description = plugin.description.clone();
            let icon = match id.as_str() {
                "photag.launcher" => gpui_kit::assets::IconName::LayoutGrid,
                "photag.clipboard" => gpui_kit::assets::IconName::Clipboard,
                "photag.screenshot" => gpui_kit::assets::IconName::Aperture,
                _ => gpui_kit::assets::IconName::Puzzle,
            };
            let enabled = plugin.enabled;
            let busy = plugin.busy;
            let plugin_id = id.clone();
            let settings_id = id.clone();
            body = body.child(
                div()
                    .v_flex()
                    .gap_2()
                    .workbench_card(false)
                    .p(px(theme::SPACE_3))
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .size(px(44.))
                                    .rounded(px(theme::ROUND_MD))
                                    .bg(rgb(theme::SURFACE_PANEL))
                                    .items_center()
                                    .justify_center()
                                    .child(theme::icon(icon)),
                            )
                            .child(
                                div()
                                    .v_flex()
                                    .gap_1()
                                    .flex_1()
                                    .child(
                                        div()
                                            .text_size(px(theme::FONT_SIZE_HEADING))
                                            .font_semibold()
                                            .child(photag_ui_bridge::i18n::tr(&name)),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(rgb(theme::TEXT_SECONDARY))
                                            .child(photag_ui_bridge::i18n::tr(&description)),
                                    ),
                            )
                            .child(
                                Button::new(format!("settings-open-plugin-{id}"))
                                    .label(photag_ui_bridge::i18n::tr("Settings"))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.tab = Tab::PluginSettings;
                                        this.selected_plugin_id = Some(settings_id.clone());
                                        let _ = this.surfaces.set_value("settings.tab", "plugin");
                                        let _ = this
                                            .surfaces
                                            .set_value("settings.plugin_id", &settings_id);
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new(format!("settings-toggle-plugin-{id}"))
                                    .label(photag_ui_bridge::i18n::tr(if busy {
                                        "Working…"
                                    } else if enabled {
                                        "Disable"
                                    } else {
                                        "Enable"
                                    }))
                                    .disabled(busy)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        let runtime = this.service.plugin_runtime();
                                        let plugin_id = plugin_id.clone();
                                        let next = !enabled;
                                        cx.spawn(async move |this, cx| {
                                            let result =
                                                runtime.set_enabled(&plugin_id, next).await;
                                            let persisted_id = plugin_id.clone();
                                            let _ = this.update(cx, |view, cx| {
                                                match result {
                                                    Ok(()) => view.edit(
                                                        |settings| {
                                                            settings
                                                                .plugin_enabled
                                                                .insert(persisted_id.clone(), next);
                                                        },
                                                        cx,
                                                    ),
                                                    Err(error) => {
                                                        view.error = Some(format!(
                                                            "Could not change plugin state: {error}"
                                                        ))
                                                    }
                                                }
                                                cx.notify();
                                            });
                                        })
                                        .detach();
                                    })),
                            ),
                    ),
            );
        }
        body
    }

    fn render_plugin_settings(&self, s: &Settings, cx: &mut Context<Self>) -> Div {
        let Some(id) = self.selected_plugin_id.as_deref() else {
            return div().child(photag_ui_bridge::i18n::tr(
                "Select a plugin from the navigation.",
            ));
        };
        let Some(plugin) = self
            .service
            .plugin_runtime()
            .snapshot_now()
            .into_iter()
            .find(|plugin| plugin.id == id)
        else {
            return div().child(photag_ui_bridge::i18n::tr(
                "This plugin is no longer available.",
            ));
        };
        let mode = s.plugin_launch_mode(id);
        let mut body = div()
            .v_flex()
            .gap(px(theme::PANEL_GAP))
            .child(
                div()
                    .v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_lg()
                            .font_semibold()
                            .child(photag_ui_bridge::i18n::tr(&plugin.title)),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(theme::TEXT_SECONDARY))
                            .child(photag_ui_bridge::i18n::tr(&plugin.description)),
                    ),
            )
            .child(self.section("Shortcut presentation"));
        for value in plugin.launch_modes {
            let chosen = value == mode;
            let plugin_id = id.to_owned();
            let label = match value {
                crate::PluginLaunchMode::WorkspaceAndPanel => "Workspace and floating panel",
                crate::PluginLaunchMode::PanelOnly => "Floating panel only",
                crate::PluginLaunchMode::Direct => "Launch capture directly",
            };
            body = body.child(
                Button::new(format!("settings-plugin-mode-{id}-{label}"))
                    .icon(theme::small_icon(if chosen {
                        gpui_kit::assets::IconName::CircleDot
                    } else {
                        gpui_kit::assets::IconName::Circle
                    }))
                    .label(photag_ui_bridge::i18n::tr(label))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.edit(
                            |settings| {
                                settings
                                    .plugin_launch_modes
                                    .insert(plugin_id.clone(), value);
                            },
                            cx,
                        );
                    })),
            );
        }
        body = body.child(self.render_plugin_shortcuts(id, s, cx));
        if id == "photag.screenshot" {
            body = body
                .child(self.section("External software"))
                .child(self.render_software_page(cx, true));
        }
        let command_id = match id {
            "photag.launcher" => "program-launcher/open",
            "photag.clipboard" => "clipboard-workbench/open",
            "photag.screenshot" => "screenshot-tool/open-manager",
            _ => "",
        };
        let cordis = self.cordis.clone();
        let command_id = command_id.to_owned();
        body = body.child(
            Button::new(format!("settings-open-plugin-panel-{id}"))
                .label(photag_ui_bridge::i18n::tr("Open plugin panel"))
                .disabled(!plugin.enabled || plugin.busy || command_id.is_empty())
                .on_click(move |_, _, _| {
                    let cordis = cordis.clone();
                    let command_id = command_id.clone();
                    tokio::spawn(async move {
                        if let Err(error) = cordis
                            .emit::<photag_ui_bridge::UiCommandInvoked>(
                                cordis::Routing::Unscoped,
                                command_id,
                            )
                            .await
                        {
                            tracing::error!(%error, "could not open plugin panel from Settings");
                        }
                    });
                }),
        );
        body
    }
}
