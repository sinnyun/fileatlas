use super::*;
use photag_ui_bridge::settings_page::{
    Choice, SettingControl, SettingRow, SettingValue, SettingsPage, SettingsSection,
};
impl SettingsView {
    pub(super) fn render_appearance(&self, s: &Settings, cx: &mut Context<Self>) -> Div {
        let language = div()
            .w_full()
            .children(self.language_views.iter().map(|(_, _, view)| view.clone()));
        let colors = theme::ACCENT_PRESETS
            .iter()
            .map(|p| Choice::new(p.name, p.name).swatch(p.color))
            .collect();
        let weak = cx.entity().downgrade();
        SettingsPage::new(
            "",
            "",
            vec![
                SettingsSection::new(
                    "Interface language",
                    vec![SettingRow::new(
                        "settings-language",
                        "",
                        SettingControl::Custom(language.into_any_element()),
                    )],
                ),
                SettingsSection::new(
                    "Accent color",
                    vec![
                        SettingRow::new(
                            "settings-accent",
                            "Application accent",
                            SettingControl::Swatches {
                                value: s.accent_name.clone(),
                                options: colors,
                            },
                        ),
                        SettingRow::new(
                            "settings-accent-system",
                            "",
                            SettingControl::InlineToggle {
                                value: s.accent_name == theme::SYSTEM_ACCENT_NAME,
                                label: "Follow the Windows accent color".into(),
                            },
                        ),
                    ],
                ),
                SettingsSection::new(
                    "Interface layers",
                    vec![
                        SettingRow::new(
                            "settings-region",
                            "Region separation",
                            SettingControl::Segments {
                                value: s.region_separation.clone(),
                                options: vec![
                                    Choice::new("border", "Border"),
                                    Choice::new("shadow", "Shadow"),
                                ],
                            },
                        )
                        .description("Choose how the sidebar and content regions are separated."),
                    ],
                ),
            ],
        )
        .render_body(std::rc::Rc::new(move |id, value, _, cx| {
            let _ = weak.update(cx, |view, cx| {
                view.edit(
                    |settings| match (id, &value) {
                        ("settings-accent", SettingValue::Choice(value)) => {
                            settings.accent_name = value.clone()
                        }
                        ("settings-accent-system", SettingValue::Bool(value)) => {
                            settings.accent_name = if *value {
                                theme::SYSTEM_ACCENT_NAME
                            } else {
                                theme::DEFAULT_ACCENT_NAME
                            }
                            .into()
                        }
                        ("settings-region", SettingValue::Choice(value)) => {
                            settings.region_separation = value.clone()
                        }
                        _ => {}
                    },
                    cx,
                )
            });
        }))
    }
}
