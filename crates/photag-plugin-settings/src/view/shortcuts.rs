//! Layout contract: docs/reference-ui/concepts/shortcuts-settings-2026-10-18.md.
use super::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use photag_ui_bridge::{CommandKeybinding, RegisteredCommand, ShortcutScope};

pub(super) fn command_label(command: &RegisteredCommand) -> String {
    match command.id.as_str() {
        "program-launcher/open" => "程序启动台".into(),
        "clipboard-workbench/open" => "剪贴板工作台".into(),
        _ => photag_ui_bridge::i18n::tr(&command.label),
    }
}

fn heading(label: &'static str) -> Div {
    div()
        .h(px(28.))
        .h_flex()
        .items_center()
        .font_semibold()
        .text_size(px(16.))
        .border_b_1()
        .border_color(rgba(theme::HAIRLINE.to_rgba()))
        .child(label)
}

fn keycap(label: String) -> Div {
    div()
        .w(px(130.))
        .h(px(32.))
        .flex_shrink_0()
        .h_flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .border_1()
        .border_color(rgba(theme::HAIRLINE.to_rgba()))
        .bg(rgb(theme::SURFACE_BASE))
        .child(label)
}

fn setting_row() -> Div {
    div()
        .h_flex()
        .w_full()
        .min_h(px(56.))
        .items_center()
        .gap_3()
        .border_b_1()
        .border_color(rgba(theme::HAIRLINE.to_rgba()))
}

fn description(title: &'static str, note: &'static str) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .v_flex()
        .gap(px(2.))
        .child(div().font_medium().child(title))
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child(note),
        )
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
            ShortcutScope::Application => ("程序内", "仅在 PhoTag 程序内生效"),
            ShortcutScope::System => ("系统级", "在其他程序中也可以触发此快捷键"),
        };
        let label = command_label(command);
        let mut row = div()
            .h_flex()
            .min_h(px(44.))
            .items_center()
            .gap(px(12.))
            .child(
                div()
                    .w(px(190.))
                    .flex_shrink_0()
                    .min_w_0()
                    .h_flex()
                    .items_center()
                    .gap_1()
                    .child(div().truncate().font_medium().child(label.clone()))
                    .child(
                        Button::new(format!("shortcut-info-{id}"))
                            .ghost()
                            .compact()
                            .icon(theme::small_icon(IconName::Info))
                            .tooltip(format!(
                                "{}\n{}\n{}\n默认按键：{}",
                                label, command.owner, help, binding.key
                            )),
                    ),
            )
            .child(
                div()
                    .w(px(66.))
                    .flex_shrink_0()
                    .rounded(px(4.))
                    .py_1()
                    .text_center()
                    .text_size(px(12.))
                    .bg(rgb(theme::SURFACE_ELEVATED))
                    .child(scope),
            );
        let controls = if capture {
            div().flex_1().min_w_0().v_flex().gap_1().child(
                div()
                    .h_flex()
                    .h(px(36.))
                    .items_center()
                    .gap_2()
                    .px_3()
                    .rounded(px(4.))
                    .border_1()
                    .border_color(rgb(theme::accent()))
                    .bg(rgba((theme::accent() << 8) | 0x28))
                    .child(theme::small_icon(IconName::Keyboard).text_color(rgb(theme::accent())))
                    .child(div().flex_1().child("按下组合键…"))
                    .child(
                        Button::new(format!("shortcut-cancel-{id}"))
                            .ghost()
                            .compact()
                            .label("Esc 取消")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.cancel_shortcut_capture();
                                cx.notify();
                            })),
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
                .gap_2()
                .child(keycap(if current.is_empty() {
                    "已禁用".into()
                } else {
                    current.replace('+', " + ")
                }))
                .child(div().flex_1())
                .child(
                    Button::new(format!("shortcut-change-{id}"))
                        .w(px(92.))
                        .h(px(32.))
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
                    Button::new(format!("shortcut-reset-{id}"))
                        .ghost()
                        .w(px(56.))
                        .h(px(32.))
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
                    Button::new(format!("shortcut-disable-{id}"))
                        .ghost()
                        .w(px(56.))
                        .h(px(32.))
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
            .py(px(2.))
            .border_b_1()
            .border_color(rgba(theme::HAIRLINE.to_rgba()))
            .child(row);
        if capture && let Some(error) = &self.shortcut_error {
            container = container.child(
                div().h_flex().pl(px(280.)).child(
                    div()
                        .flex_1()
                        .pb_1()
                        .text_size(px(12.))
                        .text_color(rgb(theme::DANGER))
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
            .try_service::<photag_ui_bridge::UiBridgeService>()
            .ok()
            .and_then(|bridge| bridge.catalog.commands().ok())
            .unwrap_or_default();
        let mut page = div()
            .v_flex()
            .w_full()
            .min_w_0()
            .text_size(px(14.))
            .line_height(px(20.))
            .child(
                div()
                    .v_flex()
                    .gap(px(6.))
                    .pb(px(12.))
                    .child(
                        div()
                            .text_size(px(22.))
                            .line_height(px(28.))
                            .font_semibold()
                            .child("快捷键"),
                    )
                    .child(
                        div()
                            .text_color(rgb(theme::TEXT_SECONDARY))
                            .child("管理插件命令和媒体操作按键。系统级快捷键可在其他程序中使用。"),
                    ),
            )
            .child(heading("插件快捷键"));
        let mut count = 0;
        for command in &registered {
            if matches!(
                command.owner.as_str(),
                "photag.launcher" | "photag.clipboard" | "photag.screenshot"
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
                    .py_3()
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child("当前没有插件注册快捷键。启用相关插件后会自动显示。"),
            );
        }
        page = page.child(div().mt_3().child(heading("其他快捷键设置")));
        let modifier = settings.exclude_modifier;
        let weak = cx.entity().downgrade();
        page = page.child(
            setting_row()
                .child(description(
                    "排除修饰键",
                    "按住此修饰键点击标签，可将标签排除。",
                ))
                .child(
                    Button::new("settings-exclude-modifier")
                        .w(px(230.))
                        .h(px(32.))
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
                                    PopupMenuItem::new(photag_ui_bridge::i18n::tr(option.label()))
                                        .checked(option == modifier)
                                        .on_click(move |_, _, cx| {
                                            let _ = weak.update(cx, |this, cx| {
                                                this.edit(|s| s.exclude_modifier = option, cx);
                                            });
                                        }),
                                );
                            }
                            menu
                        }),
                ),
        );
        page = page.child(
            setting_row()
                .child(description(
                    "打开媒体文件",
                    "在图库中使用 QuickLook 打开所选文件；空格和 Enter 始终可用。",
                ))
                .child(
                    div()
                        .h_flex()
                        .items_center()
                        .gap_2()
                        .child(keycap(settings.open_key.clone()))
                        .child(self.open_key_capture_button(cx).w(px(150.)).h(px(32.))),
                ),
        );
        page = page.child(setting_row().child(description(
            "评分快捷键",
            "使用数字键 0–5 设置评分，0 表示取消评分。",
        )));
        page = page.child(
            div()
                .h(px(30.))
                .h_flex()
                .items_center()
                .font_semibold()
                .text_size(px(16.))
                .child("标签目标快捷键（F1–F9）"),
        );
        let mut tags = div()
            .v_flex()
            .gap(px(2.))
            .p(px(4.))
            .border_1()
            .border_color(rgba(theme::HAIRLINE.to_rgba()))
            .rounded(px(5.))
            .bg(rgb(theme::SURFACE_PANEL));
        for key in ["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9"] {
            let selected = settings
                .tag_hotkeys
                .iter()
                .find(|binding| binding.key == key)
                .map(|binding| binding.tag_id);
            let mut row = div()
                .h_flex()
                .h(px(26.))
                .flex_shrink_0()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .w(px(54.))
                        .h(px(24.))
                        .flex_shrink_0()
                        .h_flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.))
                        .border_1()
                        .border_color(rgba(theme::HAIRLINE.to_rgba()))
                        .bg(rgb(theme::SURFACE_ELEVATED))
                        .child(key),
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
                    div().flex_1().text_color(rgb(theme::TEXT_SECONDARY)).child(
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
                    .text_size(px(12.))
                    .text_color(rgb(theme::DANGER))
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
            .try_service::<photag_ui_bridge::UiBridgeService>()
            .ok()
            .and_then(|bridge| bridge.catalog.commands().ok())
            .unwrap_or_default();
        let mut page = div()
            .v_flex()
            .w_full()
            .min_w_0()
            .gap(px(theme::PANEL_GAP))
            .child(heading("Keyboard shortcuts"));
        let mut count = 0;
        for command in commands.iter().filter(|command| command.owner == owner) {
            for binding in &command.keybindings {
                page = page.child(self.shortcut_row(command, binding, settings, cx));
                count += 1;
            }
        }
        if count == 0 {
            page = page.child(div().py_3().text_color(rgb(theme::TEXT_SECONDARY)).child(
                photag_ui_bridge::i18n::tr(
                    "No active keyboard shortcuts are registered by this plugin.",
                ),
            ));
        }
        page
    }
}
