use super::*;
use photag_ui_bridge::settings_page::{
    Choice, SettingControl, SettingRow, SettingValue, SettingsPage, SettingsSection,
};
impl SettingsView {
    pub(super) fn render_appearance(&self, s: &Settings, cx: &mut Context<Self>) -> Div {
        let language = div()
            .w_full()
            .children(self.language_views.iter().map(|(_, _, view)| view.clone()));
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
                    "主题来源",
                    vec![
                        SettingRow::new(
                            "settings-native-theme",
                            "GPUI Kit 主题",
                            SettingControl::Select {
                                value: if s.theme_file.is_some() {
                                    "material"
                                } else {
                                    "default"
                                }
                                .into(),
                                options: if s.theme_file.is_some() {
                                    vec![
                                        Choice::new("default", "框架默认主题"),
                                        Choice::new("material", "Material 生成主题"),
                                    ]
                                } else {
                                    vec![Choice::new("default", "框架默认主题")]
                                },
                            },
                        )
                        .description("在插件设置 → Material 主题生成中生成并应用主题。"),
                    ],
                ),
                SettingsSection::new(
                    "Color mode",
                    vec![SettingRow::new(
                        "settings-color-mode",
                        "Workbench theme",
                        SettingControl::Segments {
                            value: s.color_mode.clone(),
                            options: vec![
                                Choice::new("dark", "Dark"),
                                Choice::new("light", "Light"),
                            ],
                        },
                    )],
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
                        ("settings-native-theme", SettingValue::Choice(value))
                            if value == "default" =>
                        {
                            settings.theme_file = None;
                        }
                        ("settings-color-mode", SettingValue::Choice(value))
                            if value == "dark" || value == "light" =>
                        {
                            settings.color_mode = value.clone()
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
