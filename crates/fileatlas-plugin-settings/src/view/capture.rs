use super::*;
use gpui_kit::component::button::Button;

impl SettingsView {
    pub(super) fn capture_shortcut(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let Some(id) = self.capturing_shortcut.clone() else {
            return;
        };
        if event.keystroke.key.eq_ignore_ascii_case("escape") {
            self.cancel_shortcut_capture();
            cx.notify();
            return;
        }
        let mut parts = Vec::new();
        if event.keystroke.modifiers.control {
            parts.push("Ctrl".to_owned());
        }
        if event.keystroke.modifiers.alt {
            parts.push("Alt".to_owned());
        }
        if event.keystroke.modifiers.shift {
            parts.push("Shift".to_owned());
        }
        if event.keystroke.modifiers.platform {
            parts.push("Win".to_owned());
        }
        let key = if event.keystroke.key == " " {
            "Space"
        } else {
            event.keystroke.key.trim()
        };
        if key.is_empty()
            || ["control", "ctrl", "alt", "shift", "platform"]
                .contains(&key.to_ascii_lowercase().as_str())
        {
            return;
        }
        parts.push(if key.len() == 1 {
            key.to_ascii_uppercase()
        } else {
            key.to_owned()
        });
        let chord = parts.join("+");
        let registered = self
            .cordis
            .try_service::<fileatlas_ui_bridge::UiBridgeService>()
            .ok()
            .and_then(|bridge| bridge.catalog.commands().ok())
            .unwrap_or_default();
        let binding = registered
            .iter()
            .flat_map(|command| command.keybindings.iter())
            .find(|binding| binding.id == id);
        if let Some(binding) = binding {
            if binding.scope == fileatlas_ui_bridge::ShortcutScope::System && parts.len() < 2 {
                self.shortcut_error =
                    Some("系统级快捷键必须包含 Ctrl、Alt、Shift 或 Win 修饰键。".into());
                self.publish_shortcut_capture();
                cx.notify();
                return;
            }
            if binding.scope == fileatlas_ui_bridge::ShortcutScope::System {
                let normalized_key = key.to_ascii_lowercase();
                let supported = (normalized_key.len() == 1
                    && normalized_key.as_bytes()[0].is_ascii_alphanumeric())
                    || matches!(
                        normalized_key.as_str(),
                        "space"
                            | "enter"
                            | "tab"
                            | "escape"
                            | "esc"
                            | "backspace"
                            | "delete"
                            | "home"
                            | "end"
                            | "pageup"
                            | "pagedown"
                            | "left"
                            | "right"
                            | "up"
                            | "down"
                            | "arrowleft"
                            | "arrowright"
                            | "arrowup"
                            | "arrowdown"
                    )
                    || normalized_key
                        .strip_prefix('f')
                        .and_then(|number| number.parse::<u8>().ok())
                        .is_some_and(|number| (1..=12).contains(&number));
                if !supported {
                    self.shortcut_error =
                        Some("系统级快捷键支持字母、数字、F1–F12 和导航键。".into());
                    self.publish_shortcut_capture();
                    cx.notify();
                    return;
                }
            }
            let overrides = self
                .service
                .snapshot()
                .ok()
                .map(|settings| settings.keybindings)
                .unwrap_or_default();
            let duplicate = registered.iter().find(|command| {
                command.keybindings.iter().any(|other| {
                    other.id != id
                        && other.scope == binding.scope
                        && overrides
                            .get(&other.id)
                            .unwrap_or(&other.key)
                            .eq_ignore_ascii_case(&chord)
                })
            });
            if let Some(command) = duplicate {
                self.shortcut_error = Some(format!(
                    "{chord} 已被“{}”使用，请按其他组合键或取消。",
                    shortcuts::command_label(command)
                ));
                self.publish_shortcut_capture();
                cx.notify();
                return;
            }
        }
        self.edit(
            move |settings| {
                settings.keybindings.insert(id.clone(), chord.clone());
            },
            cx,
        );
        self.cancel_shortcut_capture();
        cx.notify();
    }

    pub(super) fn capture_open_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        if !self.capturing_open_key {
            return;
        }

        let key = match event.keystroke.key.to_ascii_lowercase().as_str() {
            "escape" => {
                self.capturing_open_key = false;
                let _ = self.surfaces.set_capturing_key("settings", false);
                cx.notify();
                return;
            }
            "space" | " " => "Space".to_owned(),
            "enter" => "Enter".to_owned(),
            "tab" => "Tab".to_owned(),
            "backspace" => "Backspace".to_owned(),
            "delete" => "Delete".to_owned(),
            "insert" => "Insert".to_owned(),
            "home" => "Home".to_owned(),
            "end" => "End".to_owned(),
            "pageup" => "PageUp".to_owned(),
            "pagedown" => "PageDown".to_owned(),
            "left" => "ArrowLeft".to_owned(),
            "right" => "ArrowRight".to_owned(),
            "up" => "ArrowUp".to_owned(),
            "down" => "ArrowDown".to_owned(),
            key if key.len() == 1 && key.as_bytes()[0].is_ascii_alphanumeric() => {
                key.to_ascii_uppercase()
            }
            key if key.len() >= 2 && key.starts_with('f') && key[1..].parse::<u8>().is_ok() => {
                key.to_ascii_uppercase()
            }
            _ => {
                self.error = Some("That key cannot be saved as a FileAtlas shortcut.".into());
                self.capturing_open_key = false;
                let _ = self.surfaces.set_capturing_key("settings", false);
                cx.notify();
                return;
            }
        };

        self.edit(|settings| settings.open_key = key.clone(), cx);
        self.capturing_open_key = false;
        let _ = self.surfaces.set_capturing_key("settings", false);
        cx.notify();
    }

    pub(super) fn open_key_capture_button(&self, cx: &mut Context<Self>) -> Button {
        fileatlas_ui_bridge::components::button("settings-open-key-capture")
            .label(fileatlas_ui_bridge::i18n::tr(
                &(if self.capturing_open_key {
                    "Press a key… (Escape cancels)".to_owned()
                } else {
                    "Change key".to_owned()
                })
                .to_string(),
            ))
            .on_click(cx.listener(|this, _, window, cx| {
                this.cancel_shortcut_capture();
                this.capturing_open_key = !this.capturing_open_key;
                if let Err(error) = this
                    .surfaces
                    .set_capturing_key("settings", this.capturing_open_key)
                {
                    this.error = Some(format!("Could not update shortcut capture state: {error}"));
                }
                this.error = None;
                window.focus(&this.focus_handle, cx);
                cx.notify();
            }))
    }
}
