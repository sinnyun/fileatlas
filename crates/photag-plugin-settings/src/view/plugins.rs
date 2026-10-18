use super::*;
use gpui_kit::base::Disableable;

impl SettingsView {
    pub(super) fn render_plugin_cards(&self, _s: &Settings, cx: &mut Context<Self>) -> Div {
        let runtime = self.service.plugin_runtime();
        let statuses = runtime.snapshot_now();
        let mut body = div().v_flex().gap(px(theme::SPACE_2));
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
                    .py(px(theme::SPACE_4))
                    .min_h(px(100.))
                    .justify_center()
                    .border_b_1()
                    .border_color(rgb(theme::SEPARATOR))
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .size(px(30.))
                                    .rounded(px(theme::ROUND_MD))
                                    .bg(rgb(theme::SURFACE_PANEL))
                                    .h_flex()
                                    .items_center()
                                    .justify_center()
                                    .child(theme::large_icon(icon)),
                            )
                            .child(
                                div()
                                    .v_flex()
                                    .gap_1()
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .text_size(px(theme::FONT_SIZE_HEADING))
                                            .font_medium()
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
                                div()
                                    .text_size(px(theme::FONT_SIZE_SMALL))
                                    .text_color(rgb(if enabled {
                                        theme::SUCCESS
                                    } else {
                                        theme::TEXT_MUTED
                                    }))
                                    .child(if busy {
                                        "处理中"
                                    } else if enabled {
                                        "已启用"
                                    } else {
                                        "已停用"
                                    }),
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
        body.child(
            div()
                .text_size(px(theme::FONT_SIZE_SMALL))
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child("停用会停止相关任务并移除命令和面板，数据文件会保留。"),
        )
    }

    pub(super) fn render_plugin_settings(&self, s: &Settings, cx: &mut Context<Self>) -> Div {
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
            return div()
                .v_flex()
                .child(self.render_plugin_shortcuts(id, s, cx))
                .children(
                    self.plugin_settings_views
                        .iter()
                        .filter(|v| v.0 == id)
                        .map(|v| v.5.clone()),
                );
        };
        let mode = s.plugin_launch_mode(id);
        use photag_ui_bridge::settings_page::{
            Choice, SettingControl, SettingRow, SettingValue, SettingsPage, SettingsSection,
        };
        let choices = plugin
            .launch_modes
            .iter()
            .map(|value| {
                let (key, label, note) = match value {
                    crate::PluginLaunchMode::WorkspaceAndPanel => (
                        "workspace",
                        "Workspace and floating panel",
                        "打开工具时保留工作台。",
                    ),
                    crate::PluginLaunchMode::PanelOnly => (
                        "panel",
                        "Floating panel only",
                        "独立打开工具窗口，不唤起主界面。",
                    ),
                    crate::PluginLaunchMode::Direct => (
                        "direct",
                        "Launch capture directly",
                        "由 Shotoshot 完成选区和标注。",
                    ),
                };
                Choice::new(key, label).description(note)
            })
            .collect();
        let selected = match mode {
            crate::PluginLaunchMode::WorkspaceAndPanel => "workspace",
            crate::PluginLaunchMode::PanelOnly => "panel",
            crate::PluginLaunchMode::Direct => "direct",
        };
        let weak = cx.entity().downgrade();
        let owner = id.to_owned();
        let mut body = SettingsPage::new(
            "",
            "",
            vec![
                SettingsSection::new(
                    "",
                    vec![SettingRow::new(
                        "settings-plugin-status",
                        "运行状态",
                        SettingControl::Status {
                            value: if plugin.enabled {
                                "Enabled"
                            } else {
                                "Disabled"
                            }
                            .into(),
                            enabled: plugin.enabled,
                        },
                    )],
                ),
                SettingsSection::new(
                    "Shortcut presentation",
                    vec![SettingRow::new(
                        "settings-plugin-mode",
                        "",
                        SettingControl::Radios {
                            value: selected.into(),
                            options: choices,
                        },
                    )],
                ),
            ],
        )
        .render_body(std::rc::Rc::new(move |_, value, _, cx| {
            if let SettingValue::Choice(value) = value {
                let mode = match value.as_str() {
                    "workspace" => crate::PluginLaunchMode::WorkspaceAndPanel,
                    "panel" => crate::PluginLaunchMode::PanelOnly,
                    "direct" => crate::PluginLaunchMode::Direct,
                    _ => return,
                };
                let _ = weak.update(cx, |view, cx| {
                    view.edit(
                        |settings| {
                            settings.plugin_launch_modes.insert(owner.clone(), mode);
                        },
                        cx,
                    )
                });
            }
        }));
        body = body.child(self.render_plugin_shortcuts(id, s, cx));
        if id == "photag.screenshot" {
            body = body
                .child(
                    div()
                        .h_flex()
                        .items_center()
                        .gap_3()
                        .child(self.section("External software").flex_1())
                        .child(self.software_refresh_button(cx)),
                )
                .child(self.render_software_page(cx, true));
        }
        body = body.children(
            self.plugin_settings_views
                .iter()
                .filter(|v| v.0 == id)
                .map(|v| v.5.clone()),
        );
        let (command_id, entry_label, entry_note) = match id {
            "photag.launcher" => (
                "program-launcher/open",
                "管理程序和分类",
                "在启动台面板中整理程序。",
            ),
            "photag.clipboard" => (
                "clipboard-workbench/open",
                "查看和管理历史",
                "在剪贴板面板中管理历史内容。",
            ),
            "photag.screenshot" => (
                "screenshot-tool/open-manager",
                "打开插件面板",
                "截图保存和软件路径在管理器中调整。",
            ),
            _ => ("", "Open plugin panel", ""),
        };
        let cordis = self.cordis.clone();
        let command_id = command_id.to_owned();
        if id != "photag.screenshot" {
            body = body.child(self.section("Tool entry"));
        }
        body = body.child(design::row().child(design::label(entry_label,entry_note)).child(
            Button::new(format!("settings-open-plugin-panel-{id}"))
                .outline()
                .h(px(36.))
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
        ));
        body
    }
}
