use super::*;
use fileatlas_ui_bridge::settings_page::{self, SettingControl, SettingRow, SettingValue};
impl SettingsView {
    pub(super) fn toggle_button(
        &self,
        key: &'static str,
        label: &str,
        value: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let weak = cx.entity().downgrade();
        settings_page::render_row(
            SettingRow::new(
                format!("settings-{key}"),
                label,
                SettingControl::Toggle(value),
            ),
            std::rc::Rc::new(move |_, value, _, cx| {
                if matches!(value, SettingValue::Bool(_)) {
                    let _ = weak.update(cx, |this, cx| this.toggle(key, cx));
                }
            }),
        )
    }
}
