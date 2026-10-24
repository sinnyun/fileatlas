//! Material generator settings and per-role color editor.
use super::*;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
use fileatlas_ui_bridge::{UiViewContribution, settings_page::*, theme};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::color_picker::{ColorPickerEvent, ColorPickerState, ColorSelect};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::*;
use std::{collections::BTreeMap, rc::Rc};

const CORE: &[(&str, &str)] = &[
    ("background", "中性表面"),
    ("foreground", "正文文字"),
    ("secondary.background", "次要色"),
    ("success.background", "成功色"),
    ("warning.background", "警告色"),
    ("danger.background", "错误色"),
    ("info.background", "信息色"),
];
struct GeneratorView {
    service: Arc<MaterialThemeService>,
    draft: Preferences,
    pickers: BTreeMap<String, Entity<ColorPickerState>>,
    _subscriptions: Vec<Subscription>,
    seed: Entity<ColorPickerState>,
    name: Entity<InputState>,
    _name_subscription: Subscription,
    error: Option<String>,
}
pub fn contribution(service: Arc<MaterialThemeService>) -> UiViewContribution {
    UiViewContribution::new(
        "material-theme/settings",
        "settings.pages",
        0,
        move |window, app| {
            let draft = service.snapshot().preferences;
            let generated_light: serde_json::Value = generate::generate(&draft)
                .ok()
                .and_then(|json| serde_json::from_str(&json).ok())
                .unwrap_or_default();
            let generated_colors = generated_light["themes"][0]["colors"].clone();
            let mut pickers = BTreeMap::new();
            for (role, _) in CORE {
                let role = (*role).to_owned();
                let color = draft
                    .manual_colors
                    .get(&role)
                    .and_then(|v| theme::parse_hex_color(v))
                    .or_else(|| {
                        generated_colors[&role]
                            .as_str()
                            .and_then(theme::parse_hex_color)
                    })
                    .map(|[r, g, b]| rgb(u32::from_be_bytes([0, r, g, b])).into())
                    .unwrap_or_else(|| app.theme().primary);
                pickers.insert(
                    role,
                    app.new(|cx| ColorPickerState::new(window, cx).default_value(color)),
                );
            }
            let seed_color = theme::parse_hex_color(&draft.seed)
                .map(|[r, g, b]| rgb(u32::from_be_bytes([0, r, g, b])).into())
                .unwrap_or_else(|| app.theme().primary);
            let seed = app.new(|cx| ColorPickerState::new(window, cx).default_value(seed_color));
            let name = app.new(|cx| InputState::new(window, cx).placeholder("主题名称"));
            app.new(|cx| {
                let mut subscriptions = Vec::new();
                for (role, picker) in &pickers {
                    let role = role.clone();
                    subscriptions.push(cx.subscribe_in(
                        picker,
                        window,
                        move |view: &mut GeneratorView, _, event, window, cx| {
                            if let ColorPickerEvent::Change(Some(color)) = event {
                                view.draft.manual_colors.insert(
                                    role.clone(),
                                    format!("#{:06x}", theme::packed(*color)),
                                );
                                view.error = None;
                                refresh_generated_pickers(view, window, cx);
                                cx.notify();
                            }
                        },
                    ));
                }
                let seed_subscription = cx.subscribe_in(
                    &seed,
                    window,
                    |view: &mut GeneratorView, _, event, window, cx| {
                        if let ColorPickerEvent::Change(Some(color)) = event {
                            view.draft.seed = format!("#{:06x}", theme::packed(*color));
                            refresh_generated_pickers(view, window, cx);
                            cx.notify();
                        }
                    },
                );
                subscriptions.push(seed_subscription);
                let name_subscription = cx.subscribe(&name, |_, _, event, cx| {
                    if matches!(event, InputEvent::Change) {
                        cx.notify();
                    }
                });
                GeneratorView {
                    service: service.clone(),
                    draft: draft.clone(),
                    pickers: pickers.clone(),
                    _subscriptions: subscriptions,
                    seed: seed.clone(),
                    name,
                    _name_subscription: name_subscription,
                    error: None,
                }
            })
            .into()
        },
    )
    .with_settings_page(
        "Material 主题自定义",
        "只配置 8 类共享语义色；其余原生角色由 Material 色阶统一生成。",
    )
}
fn refresh_generated_pickers(
    view: &GeneratorView,
    window: &mut Window,
    cx: &mut gpui_kit::Context<GeneratorView>,
) {
    let Ok(json) = generate::generate(&view.draft) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&json) else {
        return;
    };
    let colors = &value["themes"][0]["colors"];
    for (role, picker) in &view.pickers {
        if view.draft.manual_colors.contains_key(role) {
            continue;
        }
        let Some(color) = colors[role]
            .as_str()
            .and_then(theme::parse_hex_color)
            .map(|[r, g, b]| Hsla::from(rgb(u32::from_be_bytes([0, r, g, b]))))
        else {
            continue;
        };
        picker.update(cx, |state, cx| state.set_value(color, window, cx));
    }
}
impl Render for GeneratorView {
    fn render(&mut self, _: &mut Window, cx: &mut gpui_kit::Context<Self>) -> impl IntoElement {
        let weak = cx.entity().downgrade();
        let state = self.service.snapshot();
        let requested_name = self.name.read(cx).value().to_string();
        let target_path = state
            .output
            .parent()
            .unwrap_or(state.output.as_path())
            .join(format!("{}.json", requested_name.trim()));
        let seed_row = SettingRow::new(
            "seed",
            "主色 / 生成种子",
            SettingControl::Custom(
                ColorSelect::new(&self.seed)
                    .accessibility_label("主色 / 生成种子")
                    .w(px(220.))
                    .into_any_element(),
            ),
        )
        .description(format!(
            "主按钮、选中和悬停共享此色系 · {}",
            self.draft.seed
        ));
        let mut palette_rows = vec![seed_row];
        palette_rows.extend(CORE.iter().filter_map(|(role, label)| {
            self.pickers.get(*role).map(|picker| {
                SettingRow::new(
                    format!("color.{role}"),
                    *label,
                    SettingControl::Custom(
                        ColorSelect::new(picker)
                            .accessibility_label(*label)
                            .w(px(220.))
                            .into_any_element(),
                    ),
                )
                .description(format!(
                    "{} · {} · {}",
                    if *role == "background" {
                        "同步工作区、面板、卡片与弹窗表面"
                    } else {
                        "共享语义色"
                    },
                    if self.draft.manual_colors.contains_key(*role) {
                        "手动"
                    } else {
                        "生成"
                    },
                    picker
                        .read(cx)
                        .displayed_color()
                        .map(|color| format!("#{:06x}", theme::packed(color)))
                        .unwrap_or_else(|| "—".into())
                ))
            })
        }));
        let sections = vec![
            SettingsSection::new(
                "生成参数",
                vec![
                    SettingRow::new(
                        "variant",
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
                        "contrast",
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
            SettingsSection::new("全局色板（8 类）", palette_rows),
            SettingsSection::new(
                "主题文件（同名覆盖并备份）",
                vec![
                    SettingRow::new(
                        "theme-name",
                        "主题名称",
                        SettingControl::Text(self.name.clone()),
                    ),
                    SettingRow::new(
                        "output",
                        "目标主题路径",
                        SettingControl::ReadOnly(if requested_name.trim().is_empty() {
                            "输入名称后显示保存路径".into()
                        } else {
                            target_path.display().to_string()
                        }),
                    )
                    .description("覆盖同名主题前，旧文件会保留为 .json.bak"),
                    SettingRow::new(
                        "apply-theme",
                        "生成并应用",
                        SettingControl::Action {
                            label: "生成并应用主题".into(),
                            primary: true,
                        },
                    )
                    .error(self.error.clone().or(state.error.clone())),
                ],
            ),
        ];
        SettingsPage::new(
            "Material 主题自定义",
            "全局仅配置 8 类语义色；GPUI Kit 将共享色阶应用到所有插件、组件与亮暗主题。",
            sections,
        )
        .render_body(Rc::new(move |id, value, window, cx| {
            let _ = weak.update(cx, |view, cx| {
                match (id, value) {
                    ("variant", SettingValue::Choice(v)) => view.draft.variant = v,
                    ("contrast", SettingValue::Choice(v)) => {
                        view.draft.contrast = v.parse().unwrap_or(0.)
                    }
                    ("apply-theme", SettingValue::Invoke) => {
                        let name = view.name.read(cx).value().to_string();
                        if name.trim().is_empty() {
                            view.error = Some("请输入主题名称".into())
                        } else {
                            view.error = view
                                .service
                                .generate_named(view.draft.clone(), name, true)
                                .err();
                        }
                    }
                    _ => {}
                }
                if matches!(id, "variant" | "contrast") {
                    refresh_generated_pickers(view, window, cx);
                }
                cx.notify();
            });
        }))
        .workbench_body()
    }
}
