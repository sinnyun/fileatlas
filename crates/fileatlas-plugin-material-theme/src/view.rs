//! Settings schema owns layout; this view only owns generator draft/input state.
use super::*;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
use fileatlas_ui_bridge::{UiViewContribution, settings_page::*, theme};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::color_picker::{ColorPickerEvent, ColorPickerState, ColorSelect};
use gpui_kit::*;
use std::rc::Rc;
struct GeneratorView {
    service: Arc<MaterialThemeService>,
    seed: Entity<ColorPickerState>,
    _seed_subscription: Subscription,
    draft: Preferences,
    error: Option<String>,
}
pub fn contribution(service: Arc<MaterialThemeService>) -> UiViewContribution {
    UiViewContribution::new(
        "material-theme/settings",
        "settings.pages",
        0,
        move |window, app| {
            let draft = service.snapshot().preferences;
            let initial = theme::parse_hex_color(&draft.seed)
                .map(|[r, g, b]| rgb(u32::from_be_bytes([0, r, g, b])).into())
                .unwrap_or_else(|| app.theme().primary);
            let seed = app.new(|cx| ColorPickerState::new(window, cx).default_value(initial));
            app.new(|cx| {
                let subscription = cx.subscribe(&seed, |view: &mut GeneratorView, _, event, cx| {
                    if let ColorPickerEvent::Change(Some(color)) = event {
                        view.draft.seed = format!("#{:06x}", theme::packed(*color));
                        view.error = None;
                        cx.notify();
                    }
                });
                GeneratorView {
                    service: service.clone(),
                    seed,
                    _seed_subscription: subscription,
                    draft,
                    error: None,
                }
            })
            .into()
        },
    )
    .with_settings_page(
        "Material 主题生成",
        "生成原生 GPUI Kit 主题 JSON，并统一应用到所有插件。",
    )
}
impl Render for GeneratorView {
    fn render(&mut self, _: &mut Window, cx: &mut gpui_kit::Context<Self>) -> impl IntoElement {
        let weak = cx.entity().downgrade();
        let state = self.service.snapshot();
        let error = self.error.clone().or(state.error);
        SettingsPage::new(
            "Material 主题生成",
            "Material 负责生成配色，GPUI Kit 负责解析与应用。",
            vec![
                SettingsSection::new(
                    "生成参数",
                    vec![
                        SettingRow::new(
                            "material.seed",
                            "种子颜色",
                            SettingControl::Custom(
                                ColorSelect::new(&self.seed)
                                    .accessibility_label("种子颜色")
                                    .w(px(240.))
                                    .into_any_element(),
                            ),
                        )
                        .description("点击色块从色板选择；用于生成完整的亮色和暗色主题。"),
                        SettingRow::new(
                            "material.variant",
                            "配色风格",
                            SettingControl::Select {
                                value: self.draft.variant.clone(),
                                options: vec![
                                    Choice::new("neutral", "低饱和中性"),
                                    Choice::new("tonal-spot", "柔和色调"),
                                    Choice::new("monochrome", "单色"),
                                ],
                            },
                        ),
                        SettingRow::new(
                            "material.contrast",
                            "对比度",
                            SettingControl::Select {
                                value: self.draft.contrast.to_string(),
                                options: vec![
                                    Choice::new("0", "标准"),
                                    Choice::new("0.5", "较高"),
                                    Choice::new("1", "最高"),
                                ],
                            },
                        ),
                    ],
                ),
                SettingsSection::new(
                    "主题文件",
                    vec![
                        SettingRow::new(
                            "material.output",
                            "生成位置",
                            SettingControl::ReadOnly(state.output.display().to_string()),
                        ),
                        SettingRow::new(
                            "material.generate",
                            "生成主题",
                            SettingControl::Action {
                                label: "生成并应用".into(),
                                primary: true,
                            },
                        )
                        .error(error),
                        SettingRow::new(
                            "material.apply",
                            "已保存主题",
                            SettingControl::Action {
                                label: "应用已生成主题".into(),
                                primary: false,
                            },
                        )
                        .disabled(!state.generated),
                    ],
                ),
            ],
        )
        .render_body(Rc::new(move |id, value, _, cx| {
            let _ = weak.update(cx, |view, cx| {
                view.error = None;
                match (id, value) {
                    ("material.variant", SettingValue::Choice(value)) => view.draft.variant = value,
                    ("material.contrast", SettingValue::Choice(value)) => {
                        view.draft.contrast = value.parse().unwrap_or(0.)
                    }
                    ("material.generate", SettingValue::Invoke) => {
                        if let Some(color) = view.seed.read(cx).value() {
                            // Material seeds are opaque RGB; picker alpha is not a theme opacity.
                            view.draft.seed = format!("#{:06x}", theme::packed(color));
                            view.error = view.service.generate(view.draft.clone(), true).err();
                        } else {
                            view.error = Some("请先选择种子颜色。".into());
                        }
                    }
                    ("material.apply", SettingValue::Invoke) => {
                        view.error = view.service.apply_saved().err()
                    }
                    _ => {}
                }
                cx.notify();
            });
        }))
        .workbench_body()
    }
}
