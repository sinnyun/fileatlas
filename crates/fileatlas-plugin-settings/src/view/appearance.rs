use super::*;
use fileatlas_ui_bridge::settings_page::{
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
                    "全局布局密度",
                    vec![SettingRow::new(
                        "settings-ui-density",
                        "间距与常用高度",
                        SettingControl::Segments {
                            value: s.ui_density.clone(),
                            options: vec![
                                Choice::new("compact", "紧凑"),
                                Choice::new("standard", "标准"),
                                Choice::new("comfortable", "宽松"),
                            ],
                        },
                    )
                    .description("统一调整全局三档间距、共享按钮、输入控件和导航行高度；插件不单独覆盖。")],
                ),
                SettingsSection::new(
                    "界面审查",
                    vec![SettingRow::new(
                        "settings-ui-audit-frames",
                        "显示组件审查框",
                        SettingControl::Toggle(s.show_ui_audit_frames),
                    )
                    .description("控制基础组件边界标记与结构化尺寸采集；不影响字体层级颜色标记。")],
                ),
                SettingsSection::new(
                    "字体层级审查",
                    vec![SettingRow::new(
                        "settings-typography-audit-colors",
                        "显示字体层级审查色",
                        SettingControl::Toggle(s.show_typography_audit_colors),
                    )
                    .description("标题为红色、正文为绿色、辅助说明为蓝色；关闭后恢复当前主题文字颜色，不影响组件审查框。")],
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
                        ("settings-ui-density", SettingValue::Choice(value))
                            if ["compact", "standard", "comfortable"].contains(&value.as_str()) =>
                        {
                            settings.ui_density = value.clone()
                        }
                        ("settings-ui-audit-frames", SettingValue::Bool(value)) => {
                            settings.show_ui_audit_frames = *value
                        }
                        ("settings-typography-audit-colors", SettingValue::Bool(value)) => {
                            settings.show_typography_audit_colors = *value
                        }
                        _ => {}
                    },
                    cx,
                )
            });
        }))
    }
}
