use super::*;
use fileatlas_ui_bridge::components::WorkbenchPaint as _;
use fileatlas_ui_bridge::components::{WorkbenchStyle as _, WorkbenchTypography as _};

impl SettingsView {
    fn tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut rail = div()
            .v_flex()
            .w(px(theme::SETTINGS_NAV_WIDTH))
            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
            .p(px(theme::space_expanded()))
            .flex_shrink_0();
        use gpui_kit::assets::IconName;
        let add_group = |rail: Div, title: &'static str| {
            rail.child(
                div()
                    .workbench_title()
                    .px(px(fileatlas_ui_bridge::theme::space_standard()))
                    .pt(px(fileatlas_ui_bridge::theme::space_standard()))
                    .pb(px(fileatlas_ui_bridge::theme::space_compact()))
                    .child(fileatlas_ui_bridge::i18n::tr(title)),
            )
        };
        let add_tab =
            |rail: Div, tab: Tab, name: &'static str, icon: IconName| {
                let active = self.tab == tab;
                rail.child(
                    div()
                        .id(format!("settings-tab-{name}"))
                        .workbench_tree_row(active)
                        .h(px(theme::ui_height(theme::NAV_ROW_HEIGHT)))
                        .px(px(fileatlas_ui_bridge::theme::space_standard()))
                        .gap(px(theme::space_standard()))
                        .workbench_assistant()
                        .child(theme::icon(icon))
                        .child(div().flex_1().workbench_body().child(
                            fileatlas_ui_bridge::i18n::tr(match tab {
                                Tab::Tags => "Tags and search",
                                Tab::Ai => "AI analysis",
                                Tab::Updates => "Software updates",
                                Tab::Software => "External software",
                                Tab::Plugins => "Plugin management",
                                _ => name,
                            }),
                        ))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.cancel_shortcut_capture();
                            this.capturing_open_key = false;
                            this.tab = tab;
                            let name = match tab {
                                Tab::Appearance => "appearance",
                                Tab::Shortcuts => "shortcuts",
                                Tab::Tags => "tags",
                                Tab::Ai => "ai",
                                Tab::Updates => "updates",
                                Tab::Software => "software",
                                Tab::Plugins => "plugins",
                                Tab::PluginSettings => "plugin",
                            };
                            let _ = this.surfaces.set_value("settings.tab", name);
                            cx.notify()
                        })),
                )
            };
        rail = add_group(rail, "Program settings");
        for (tab, name, icon) in [
            (Tab::Appearance, "Appearance", IconName::Monitor),
            (Tab::Shortcuts, "Shortcuts", IconName::Keyboard),
            (Tab::Tags, "Tags", IconName::Tag),
            (Tab::Ai, "AI", IconName::Sparkles),
            (Tab::Updates, "Updates", IconName::RefreshCw),
            (Tab::Software, "Software", IconName::Wrench),
            (Tab::Plugins, "Plugins", IconName::Puzzle),
        ] {
            rail = add_tab(rail, tab, name, icon);
        }
        rail = rail.child(
            div()
                .w_full()
                .h(px(1.))
                .my(px(fileatlas_ui_bridge::theme::space_standard()))
                .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Transparent(
                    theme::hairline().to_rgba(),
                )),
        );
        rail = add_group(rail, "Plugin settings");
        let mut plugins = self
            .service
            .plugin_runtime()
            .snapshot_now()
            .into_iter()
            .map(|p| (p.id, p.title))
            .collect::<Vec<_>>();
        for (owner, title, _, _, _, _) in &self.plugin_settings_views {
            if !plugins.iter().any(|(id, _)| id == owner) {
                plugins.push((owner.clone(), title.clone()));
            }
        }
        for (id, title) in plugins {
            let active =
                self.tab == Tab::PluginSettings && self.selected_plugin_id.as_deref() == Some(&id);
            rail = rail.child(
                div()
                    .id(format!("settings-plugin-page-{id}"))
                    .workbench_tree_row(active)
                    .h(px(theme::ui_height(theme::NAV_ROW_HEIGHT)))
                    .px(px(fileatlas_ui_bridge::theme::space_standard()))
                    .gap(px(theme::space_standard()))
                    .workbench_body()
                    .child(theme::icon(match id.as_str() {
                        "fileatlas.clipboard" => IconName::Clipboard,
                        "fileatlas.launcher" => IconName::LayoutGrid,
                        _ => IconName::ScanLine,
                    }))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .child(fileatlas_ui_bridge::i18n::tr(&title)),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.cancel_shortcut_capture();
                        this.capturing_open_key = false;
                        this.tab = Tab::PluginSettings;
                        this.selected_plugin_id = Some(id.clone());
                        let _ = this.surfaces.set_value("settings.tab", "plugin");
                        let _ = this.surfaces.set_value("settings.plugin_id", &id);
                        cx.notify();
                    })),
            );
        }
        rail.h_full().overflow_y_scrollbar()
    }
}
impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_count = self.render_count.wrapping_add(1);
        let _ = self
            .surfaces
            .set_value("settings.render-count", &self.render_count.to_string());
        self.sync_language_views(window, cx);
        let locale_revision = fileatlas_ui_bridge::i18n::revision();
        if self.locale_revision != locale_revision {
            self.locale_revision = locale_revision;
            self.group_name_input.update(cx, |input, cx| {
                input.set_placeholder(fileatlas_ui_bridge::i18n::tr("Group name"), window, cx)
            });
        }

        if !self.surface_visible {
            return self.render_update_offer(window, cx).into_any_element();
        }
        if let Some(tab) = self.surfaces.value("settings.tab") {
            self.tab = match tab.as_str() {
                "shortcuts" => Tab::Shortcuts,
                "tags" => Tab::Tags,
                "ai" => Tab::Ai,
                "updates" => Tab::Updates,
                "software" => Tab::Software,
                "plugins" => Tab::Plugins,
                "plugin" => Tab::PluginSettings,
                _ => Tab::Appearance,
            };
            if self.tab == Tab::PluginSettings {
                self.selected_plugin_id = self.surfaces.value("settings.plugin_id");
            }
        }
        let bounds =
            fileatlas_ui_bridge::panels::content_bounds(&self.surfaces, "settings", window);
        let card_width = bounds.width;
        let card_height = bounds.height;
        let error_height = if self.error.is_some() { 40. } else { 0. };
        let body_height = (card_height - 48. - error_height).max(8.);
        let content_width = (card_width - theme::SETTINGS_NAV_WIDTH - 1.).max(1.);
        let mut card = div()
            .absolute()
            .inset_0()
            .v_flex()
            .w(px(card_width))
            .h(px(card_height))
            .max_h(px(card_height))
            .workbench_body()
            .track_focus(&self.focus_handle)
            .child(
                div()
                    .h_flex()
                    .w_full()
                    .h(px(body_height))
                    .flex_shrink_0()
                    .child(
                        div()
                            .v_flex()
                            .w(px(theme::SETTINGS_NAV_WIDTH))
                            .h(px(body_height))
                            .flex_shrink_0()
                            .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Base)
                            .child(self.tabs(cx)),
                    )
                    .child(fileatlas_ui_bridge::components::vertical_divider())
                    .child(
                        div()
                            .v_flex()
                            .w(px(content_width))
                            .h(px(body_height))
                            .flex_shrink_0()
                            .child(
                                div()
                                    .v_flex()
                                    .size_full()
                                    .overflow_y_scrollbar()
                                    .px(px(theme::SETTINGS_CONTENT_PADDING))
                                    .py(px(fileatlas_ui_bridge::theme::space_expanded()))
                                    .child(self.page_header())
                                    .child(match self.service.snapshot() {
                                        Ok(s) => self
                                            .render_tab(
                                                &s,
                                                content_width
                                                    - theme::SETTINGS_CONTENT_PADDING * 2.,
                                                cx,
                                            )
                                            .into_any_element(),
                                        Err(error) => {
                                            div().child(error.to_string()).into_any_element()
                                        }
                                    }),
                            ),
                    ),
            );
        card = card.child(
            div()
                .h_flex()
                .h(px(theme::ui_height(theme::CONTROL_HEIGHT_FOOTER)))
                .flex_shrink_0()
                .items_center()
                .justify_between()
                .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Top)
                .px(px(fileatlas_ui_bridge::theme::space_expanded()))
                .py(px(fileatlas_ui_bridge::theme::space_standard()))
                .child(
                    div()
                        .h_flex()
                        .items_center()
                        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                        .workbench_assistant()
                        .child(theme::small_icon(gpui_kit::assets::IconName::Check))
                        .child(fileatlas_ui_bridge::i18n::tr(
                            "Settings changes are saved automatically.",
                        )),
                )
                .child(
                    fileatlas_ui_bridge::components::button("settings-close-footer")
                        .label(fileatlas_ui_bridge::i18n::tr("Close"))
                        .on_click(cx.listener(|this, _, _, cx| this.close_surface(cx))),
                ),
        );
        if let Some(e) = &self.error {
            card = card.child(
                div()
                    .h(px(theme::ui_height(theme::CONTROL_HEIGHT_NAV)))
                    .flex_shrink_0()
                    .px(px(fileatlas_ui_bridge::theme::space_expanded()))
                    .py(px(fileatlas_ui_bridge::theme::space_standard()))
                    .workbench_assistant()
                    .child(format!("Could not save: {e}")),
            );
        }
        let mut root = div()
            .absolute()
            .inset_0()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event, _, cx| this.handle_key_down(event, cx)))
            .child(fileatlas_ui_bridge::components::modal_frame(card));
        if self.update_offer_open {
            root = root.child(self.render_update_offer(window, cx));
        }
        root.into_any_element()
    }
}
