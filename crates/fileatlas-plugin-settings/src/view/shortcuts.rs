//! Layout contract: docs/reference-ui/concepts/shortcuts-settings-2026-10-18.md.
use super::*;
use fileatlas_ui_bridge::components::WorkbenchPaint as _;
use fileatlas_ui_bridge::components::{WorkbenchStyle as _, WorkbenchTypography as _};
use fileatlas_ui_bridge::{CommandKeybinding, RegisteredCommand, ShortcutScope};
use gpui_kit::assets::IconName;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};

pub(super) fn command_label(command: &RegisteredCommand) -> String {
    match command.id.as_str() {
        "program-launcher/open" => "程序启动台".into(),
        "clipboard-workbench/open" => "剪贴板工作台".into(),
        _ => fileatlas_ui_bridge::i18n::tr(&command.label),
    }
}

fn heading(label: &'static str) -> Div {
    super::design::heading(label)
}

fn setting_row() -> Div {
    div()
        .h_flex()
        .w_full()
        .min_h(px(theme::ui_height(theme::CONTROL_HEIGHT_FOOTER)))
        .items_center()
        .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
        .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Bottom)
}

fn description(title: &'static str, note: &'static str) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .h_flex()
        .items_center()
        .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
        .child(
            div()
                .w(px(130.))
                .flex_shrink_0()
                .workbench_body()
                .child(title),
        )
        .child(div().flex_1().min_w_0().workbench_assistant().child(note))
}

impl SettingsView {
    pub(super) fn publish_shortcut_capture(&self) {
        let _ = self.surfaces.set_value(
            "settings.shortcut-capture",
            self.capturing_shortcut.as_deref().unwrap_or(""),
        );
        let _ = self.surfaces.set_value(
            "settings.shortcut-error",
            self.shortcut_error.as_deref().unwrap_or(""),
        );
    }

    pub(super) fn cancel_shortcut_capture(&mut self) {
        self.capturing_shortcut = None;
        self.shortcut_error = None;
        let _ = self.surfaces.set_capturing_key("settings", false);
        self.publish_shortcut_capture();
    }

    fn shortcut_row(
        &self,
        command: &RegisteredCommand,
        binding: &CommandKeybinding,
        settings: &Settings,
        cx: &mut Context<Self>,
    ) -> Div {
        let id = binding.id.clone();
        let capture = self.capturing_shortcut.as_ref() == Some(&id);
        let current = settings.keybindings.get(&id).unwrap_or(&binding.key);
        let (scope, help) = match binding.scope {
            ShortcutScope::Application => ("程序内", "仅在 FileAtlas 程序内生效"),
            ShortcutScope::System => ("系统级", "在其他程序中也可以触发此快捷键"),
        };
        let label = command_label(command);
        let mut row = div()
            .h_flex()
            .min_h(px(theme::ui_height(theme::CONTROL_HEIGHT_FOOTER)))
            .items_center()
            .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
            .child(
                div()
                    .w(px(130.))
                    .flex_shrink_0()
                    .min_w_0()
                    .h_flex()
                    .items_center()
                    .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                    .child(div().truncate().workbench_body().child(label.clone()))
                    .child(
                        fileatlas_ui_bridge::components::button(format!("shortcut-info-{id}"))
                            .ghost()
                            .icon(theme::small_icon(IconName::Info))
                            .tooltip(format!(
                                "{}\n{}\n{}\n默认按键：{}",
                                label, command.owner, help, binding.key
                            )),
                    ),
            )
            .child(
                div()
                    .w(px(60.))
                    .flex_shrink_0()
                    .workbench_corner(fileatlas_ui_bridge::components::CornerRole::Small)
                    .py(px(fileatlas_ui_bridge::theme::space_compact()))
                    .text_center()
                    .workbench_assistant()
                    .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Elevated)
                    .child(scope),
            );
        let controls = if capture {
            div()
                .flex_1()
                .min_w_0()
                .v_flex()
                .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                .child(
                    div()
                        .h_flex()
                        .h(px(theme::ui_height(theme::CONTROL_HEIGHT_TOOLBAR)))
                        .items_center()
                        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                        .px(px(fileatlas_ui_bridge::theme::space_expanded()))
                        .workbench_card(true)
                        .child(theme::small_icon(IconName::Keyboard))
                        .child(div().flex_1().child("按下组合键…"))
                        .child(
                            fileatlas_ui_bridge::components::button(format!(
                                "shortcut-cancel-{id}"
                            ))
                            .ghost()
                            .label("Esc 取消")
                            .on_click(cx.listener(
                                |this, _, _, cx| {
                                    this.cancel_shortcut_capture();
                                    cx.notify();
                                },
                            )),
                        ),
                )
        } else {
            let change_id = id.clone();
            let reset_id = id.clone();
            let disable_id = id.clone();
            div()
                .flex_1()
                .min_w_0()
                .h_flex()
                .items_center()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                .child(fileatlas_ui_bridge::components::keycap(
                    if current.is_empty() {
                        "已禁用".into()
                    } else {
                        current.replace('+', " + ")
                    },
                    fileatlas_ui_bridge::components::KeycapSize::Standard,
                ))
                .child(div().flex_1())
                .child(
                    fileatlas_ui_bridge::components::button(format!("shortcut-change-{id}"))
                        .label("更改")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.capturing_open_key = false;
                            this.capturing_shortcut = Some(change_id.clone());
                            this.shortcut_error = None;
                            this.error = None;
                            let _ = this.surfaces.set_capturing_key("settings", true);
                            this.publish_shortcut_capture();
                            window.focus(&this.focus_handle, cx);
                            cx.notify();
                        })),
                )
                .child(
                    fileatlas_ui_bridge::components::button(format!("shortcut-reset-{id}"))
                        .ghost()
                        .label("恢复")
                        .tooltip("恢复插件提供的默认按键")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.cancel_shortcut_capture();
                            this.edit(
                                |s| {
                                    s.keybindings.remove(&reset_id);
                                },
                                cx,
                            );
                        })),
                )
                .child(
                    fileatlas_ui_bridge::components::button(format!("shortcut-disable-{id}"))
                        .ghost()
                        .label("禁用")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.cancel_shortcut_capture();
                            this.edit(
                                |s| {
                                    s.keybindings.insert(disable_id.clone(), String::new());
                                },
                                cx,
                            );
                        })),
                )
        };
        row = row.child(controls);
        let mut container = div()
            .v_flex()
            .py(px(fileatlas_ui_bridge::theme::space_compact()))
            .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Bottom)
            .child(row);
        if capture && let Some(error) = &self.shortcut_error {
            container = container.child(
                div()
                    .h_flex()
                    .pl(px(fileatlas_ui_bridge::theme::space_expanded()))
                    .child(
                        div()
                            .flex_1()
                            .pb(px(fileatlas_ui_bridge::theme::space_compact()))
                            .workbench_assistant()
                            .child(error.clone()),
                    ),
            );
        }
        container
    }

    pub(super) fn render_shortcuts(
        &self,
        settings: &Settings,
        width: f32,
        cx: &mut Context<Self>,
    ) -> Div {
        let registered = self
            .cordis
            .try_service::<fileatlas_ui_bridge::UiBridgeService>()
            .ok()
            .and_then(|bridge| bridge.catalog.commands().ok())
            .unwrap_or_default();
        let mut page = div()
            .v_flex()
            .w_full()
            .min_w_0()
            .workbench_body()
            .child(heading("程序快捷键"));
        let mut count = 0;
        for command in &registered {
            if matches!(
                command.owner.as_str(),
                "fileatlas.launcher" | "fileatlas.clipboard" | "fileatlas.screenshot"
            ) {
                continue;
            }
            for binding in &command.keybindings {
                page = page.child(self.shortcut_row(command, binding, settings, cx));
                count += 1;
            }
        }
        if count == 0 {
            page = page.child(
                div()
                    .py(px(fileatlas_ui_bridge::theme::space_expanded()))
                    .workbench_assistant()
                    .child("暂无其他程序快捷键。工具快捷键可在各插件设置中调整。"),
            );
        }
        page = page.child(
            div()
                .mt(px(fileatlas_ui_bridge::theme::space_expanded()))
                .child(heading("媒体操作")),
        );
        let modifier = settings.exclude_modifier;
        let weak = cx.entity().downgrade();
        page = page.child(
            setting_row()
                .child(description(
                    "排除修饰键",
                    "按住此修饰键点击标签，可将标签排除。",
                ))
                .child(
                    fileatlas_ui_bridge::components::button("settings-exclude-modifier")
                        .label(fileatlas_ui_bridge::i18n::tr(modifier.label()))
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
                                    PopupMenuItem::new(fileatlas_ui_bridge::i18n::tr(
                                        option.label(),
                                    ))
                                    .checked(option == modifier)
                                    .on_click(
                                        move |_, _, cx| {
                                            let _ = weak.update(cx, |this, cx| {
                                                this.edit(|s| s.exclude_modifier = option, cx);
                                            });
                                        },
                                    ),
                                );
                            }
                            menu
                        }),
                ),
        );
        page = page.child(
            setting_row()
                .child(description("打开媒体文件", "空格和 Enter 始终可用。"))
                .child(
                    div()
                        .h_flex()
                        .items_center()
                        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                        .child(fileatlas_ui_bridge::components::keycap(
                            settings.open_key.clone(),
                            fileatlas_ui_bridge::components::KeycapSize::Standard,
                        ))
                        .child(
                            self.open_key_capture_button(cx)
                                .w(px(100.))
                                .h(px(theme::ui_height(theme::CONTROL_HEIGHT_STANDARD))),
                        ),
                ),
        );
        page = page.child(
            setting_row()
                .child(description("评分", "数字键 0–5，0 表示取消评分。"))
                .child(
                    div()
                        .h_flex()
                        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                        .children((0..=5).map(|key| {
                            fileatlas_ui_bridge::components::keycap(
                                key.to_string(),
                                fileatlas_ui_bridge::components::KeycapSize::Compact,
                            )
                        })),
                ),
        );
        page = page.child(
            div()
                .h(px(30.))
                .h_flex()
                .items_center()
                .workbench_title()
                .child("标签目标快捷键（F1–F9）"),
        );
        let mut tags = div()
            .v_flex()
            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
            .py(px(theme::space_standard()));
        for key in ["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9"] {
            let selected = settings
                .tag_hotkeys
                .iter()
                .find(|binding| binding.key == key)
                .map(|binding| binding.tag_id);
            let mut row = div()
                .h_flex()
                .h(px(30.))
                .flex_shrink_0()
                .items_center()
                .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
                .child(
                    div()
                        .w(px(54.))
                        .h(px(24.))
                        .flex_shrink_0()
                        .h_flex()
                        .items_center()
                        .justify_center()
                        .child(fileatlas_ui_bridge::components::keycap(
                            key,
                            fileatlas_ui_bridge::components::KeycapSize::Tag,
                        )),
                );
            row = match self.tag_choices.value() {
                Some(Ok(choices)) => row.child(tag_hotkeys::button(
                    key,
                    selected,
                    choices.clone(),
                    (width - 78.).max(1.),
                    cx.entity().downgrade(),
                )),
                _ => row.child(
                    div().flex_1().workbench_body().child(
                        selected
                            .map(|id| format!("标签 #{id}"))
                            .unwrap_or_else(|| "正在加载标签…".into()),
                    ),
                ),
            };
            tags = tags.child(row);
        }
        page = page.child(tags);
        if let Some(Err(error)) = self.tag_choices.value() {
            page = page.child(
                div()
                    .workbench_assistant()
                    .child(format!("无法加载标签：{error}")),
            );
        }
        page
    }

    pub(super) fn render_plugin_shortcuts(
        &self,
        owner: &str,
        settings: &Settings,
        cx: &mut Context<Self>,
    ) -> Div {
        let commands = self
            .cordis
            .try_service::<fileatlas_ui_bridge::UiBridgeService>()
            .ok()
            .and_then(|bridge| bridge.catalog.commands().ok())
            .unwrap_or_default();
        let mut page = div()
            .v_flex()
            .w_full()
            .min_w_0()
            .gap(px(theme::space_standard()))
            .child(heading("Keyboard shortcuts"));
        let mut count = 0;
        for command in commands.iter().filter(|command| command.owner == owner) {
            for binding in &command.keybindings {
                page = page.child(self.shortcut_row(command, binding, settings, cx));
                count += 1;
            }
        }
        if count == 0 {
            page = page.child(
                div()
                    .py(px(fileatlas_ui_bridge::theme::space_expanded()))
                    .workbench_assistant()
                    .child(fileatlas_ui_bridge::i18n::tr(
                        "No active keyboard shortcuts are registered by this plugin.",
                    )),
            );
        }
        page
    }
}
