//! WorkCrate preferences use shared SettingsPage forms and project-owned JSON.
use crate::view::tracking::control;
use fileatlas_plugin_workcrate_boards::WorkCrateBoardsService;
use fileatlas_plugin_workcrate_projects::{FieldPreset, FieldPresets, WorkCrateProjectsService};
use fileatlas_ui_bridge::{
    UiSurfaceState, UiViewContribution, components, settings_page::*, theme,
};
use gpui_kit::{
    base::{Disableable as _, StyledExt as _},
    component::{
        button::{Button, ButtonVariants as _},
        input::{InputEvent, InputState},
    },
    *,
};
use std::{rc::Rc, sync::Arc};

struct DraftField {
    key: String,
    label: String,
    values: Vec<Entity<InputState>>,
}
struct PresetSettings {
    service: Arc<WorkCrateProjectsService>,
    boards: Arc<WorkCrateBoardsService>,
    surfaces: Arc<UiSurfaceState>,
    fields: Vec<DraftField>,
    name: Entity<InputState>,
    executable: Entity<InputState>,
    port: Entity<InputState>,
    config_busy: bool,
    config_saved: bool,
    config_error: Option<String>,
    busy: bool,
    error: Option<String>,
    saved: bool,
    auto_pending: bool,
    _subscriptions: Vec<Subscription>,
    _config_subscriptions: Vec<Subscription>,
}

pub(super) fn contribution(
    service: Arc<WorkCrateProjectsService>,
    boards: Arc<WorkCrateBoardsService>,
    surfaces: Arc<UiSurfaceState>,
) -> UiViewContribution {
    UiViewContribution::new(
        "workcrate/settings",
        "settings.pages",
        0,
        move |window, app| {
            let stored = service.field_presets().unwrap_or_default();
            let config_result = boards.configuration();
            let config_error = config_result.as_ref().err().map(ToString::to_string);
            let config = config_result.unwrap_or_default();
            let name = app.new(|cx| InputState::new(window, cx).placeholder("例如：制作工艺"));
            let executable = input(config.executable, window, app);
            let port = input(config.port.to_string(), window, app);
            let fields = stored
                .fields
                .into_iter()
                .map(|f| DraftField {
                    key: f.key,
                    label: f.label,
                    values: f
                        .values
                        .into_iter()
                        .map(|v| input(v, window, app))
                        .collect(),
                })
                .collect();
            app.new(|cx| {
                let mut view = PresetSettings {
                    service: service.clone(),
                    boards: boards.clone(),
                    surfaces: surfaces.clone(),
                    fields,
                    name,
                    executable,
                    port,
                    config_busy: false,
                    config_saved: config_error.is_none(),
                    config_error,
                    busy: false,
                    error: None,
                    saved: false,
                    auto_pending: false,
                    _subscriptions: vec![],
                    _config_subscriptions: vec![],
                };
                view.subscribe_values(cx);
                view.subscribe_config(cx);
                view
            })
            .into()
        },
    )
    .with_settings_page("WorkCrate", "配置官方 VectorCraft 程序与页面字段预设。")
}
fn input(value: String, window: &mut Window, app: &mut App) -> Entity<InputState> {
    app.new(|cx| {
        let mut s = InputState::new(window, cx);
        s.set_value(value, window, cx);
        s
    })
}
impl PresetSettings {
    fn program_action(
        &self,
        id: &'static str,
        label: &str,
        primary: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let measure = match id {
            "browse" => "settings-vectorcraft-browse",
            _ => "settings-vectorcraft-save",
        };
        let mut button = Button::new(format!("settings-vectorcraft-{id}"))
            .label(label.to_owned())
            .disabled(self.config_busy);
        button = if primary {
            button.primary()
        } else {
            button.ghost()
        };
        control(
            measure,
            button.on_click(cx.listener(move |view, _, window, cx| {
                if id == "browse" {
                    view.browse_executable(window, cx);
                } else {
                    view.save_config(cx);
                }
            })),
            &self.surfaces,
        )
        .into_any_element()
    }
    fn action(
        &self,
        id: String,
        label: &str,
        primary: bool,
        cx: &mut Context<Self>,
    ) -> SettingControl {
        let measure = match id.as_str() {
            "create" => "settings-preset-add-field".into(),
            "save" => "settings-preset-save".into(),
            _ => format!("settings-{id}"),
        };
        let mut button = Button::new(format!("workcrate-settings-{id}"))
            .label(label.to_owned())
            .disabled(self.busy && id == "save");
        if primary {
            button = button.primary();
        } else {
            button = button.ghost();
        }
        SettingControl::Custom(
            control(
                &measure,
                button
                    .on_click(cx.listener(move |view, _, window, cx| view.apply(&id, window, cx))),
                &self.surfaces,
            )
            .into_any_element(),
        )
    }
    fn apply(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.error = None;
        self.saved = false;
        if let Some(key) = id.strip_prefix("add:") {
            let entry = input(String::new(), window, cx);
            if let Some(field) = self.fields.iter_mut().find(|f| f.key == key) {
                field.values.push(entry);
            }
        } else if let Some(key) = id.strip_prefix("remove:") {
            self.fields
                .retain(|f| f.key != key || !key.starts_with("extra:"));
        } else if id == "create" {
            let name = self.name.read(cx).value().trim().to_owned();
            if name.is_empty() || self.fields.iter().any(|f| f.label == name) {
                self.error = Some("字段名称不能为空或重复".into());
            } else {
                self.fields.push(DraftField {
                    key: format!("extra:{name}"),
                    label: name,
                    values: vec![],
                });
                self.name.update(cx, |i, cx| i.set_value("", window, cx));
            }
        } else if id == "save" {
            self.save(cx);
        }
        self.subscribe_values(cx);
        self.queue_save(cx);
        cx.notify();
    }
    fn subscribe_values(&mut self, cx: &mut Context<Self>) {
        self._subscriptions.clear();
        for field in &self.fields {
            for value in &field.values {
                self._subscriptions
                    .push(cx.subscribe(value, |view, _, event, cx| {
                        if matches!(event, InputEvent::Change) {
                            view.error = None;
                            view.saved = false;
                            view.queue_save(cx);
                        }
                    }));
            }
        }
    }
    fn subscribe_config(&mut self, cx: &mut Context<Self>) {
        self._config_subscriptions = [self.executable.clone(), self.port.clone()]
            .into_iter()
            .map(|input| {
                cx.subscribe(&input, |view, _, event, cx| {
                    if matches!(event, InputEvent::Change) {
                        view.config_error = None;
                        view.config_saved = false;
                        cx.notify();
                    }
                })
            })
            .collect();
    }
    fn save_config(&mut self, cx: &mut Context<Self>) {
        let executable = self.executable.read(cx).value().trim().to_owned();
        let Ok(port) = self.port.read(cx).value().trim().parse::<u16>() else {
            self.config_error = Some("端口应为 1–65535".into());
            self.config_saved = false;
            cx.notify();
            return;
        };
        if port == 0 {
            self.config_error = Some("端口应为 1–65535".into());
            self.config_saved = false;
            cx.notify();
            return;
        }
        let boards = self.boards.clone();
        self.config_busy = true;
        self.config_error = None;
        self.config_saved = false;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let mut config = boards.configuration().map_err(|e| e.to_string())?;
                    config.executable = executable;
                    config.port = port;
                    boards.configure(config).map_err(|e| e.to_string())
                })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.config_busy = false;
                view.config_saved = result.is_ok();
                view.config_error = result.err();
                cx.notify();
            });
        })
        .detach();
    }
    fn browse_executable(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, async move |this, cx| {
            let picked = rfd::AsyncFileDialog::new()
                .add_filter("VectorCraft 程序", &["exe"])
                .pick_file()
                .await;
            if let Some(file) = picked {
                let path = file.path().to_string_lossy().into_owned();
                let _ = cx.update(|window, app| {
                    let _ = this.update(app, |view, cx| {
                        view.executable
                            .update(cx, |input, cx| input.set_value(path, window, cx));
                        view.config_error = None;
                        view.config_saved = false;
                        cx.notify();
                    });
                });
            }
        })
        .detach();
    }
    fn snapshot(&self, cx: &Context<Self>) -> FieldPresets {
        FieldPresets {
            version: 1,
            fields: self
                .fields
                .iter()
                .map(|f| FieldPreset {
                    key: f.key.clone(),
                    label: f.label.clone(),
                    values: f
                        .values
                        .iter()
                        .map(|i| i.read(cx).value().trim().to_owned())
                        .filter(|v| !v.is_empty())
                        .collect(),
                })
                .collect(),
        }
    }
    fn queue_save(&mut self, cx: &mut Context<Self>) {
        if self.auto_pending {
            return;
        }
        self.auto_pending = true;
        if !self.busy
            && self.error.is_none()
            && self.service.field_presets().ok().as_ref() != Some(&self.snapshot(cx))
        {
            self.save(cx);
        }
        fileatlas_ui_bridge::refresh::poll(
            cx,
            std::time::Duration::from_millis(200),
            |view, cx| {
                use fileatlas_ui_bridge::refresh::PollResult;
                if view.busy {
                    return PollResult::until(false, true);
                }
                if view.service.field_presets().ok().as_ref() == Some(&view.snapshot(cx)) {
                    view.auto_pending = false;
                    view.saved = true;
                    return PollResult::until(true, false);
                }
                if view.error.is_some() {
                    view.auto_pending = false;
                    return PollResult::until(false, false);
                }
                view.save(cx);
                PollResult::until(true, true)
            },
        );
    }
    fn save(&mut self, cx: &mut Context<Self>) {
        let value = self.snapshot(cx);
        let service = self.service.clone();
        self.busy = true;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { service.save_field_presets(value).map_err(|e| e.to_string()) })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                view.saved = result.is_ok()
                    && view.service.field_presets().ok().as_ref() == Some(&view.snapshot(cx));
                view.error = result.err();
                cx.notify();
            });
        })
        .detach();
    }
}
impl Render for PresetSettings {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let executable = div()
            .h_flex()
            .items_center()
            .gap_2()
            .child(
                control(
                    "settings-vectorcraft-executable",
                    components::text_input(&self.executable).w_full(),
                    &self.surfaces,
                )
                .flex_1()
                .min_w_0(),
            )
            .child(self.program_action("browse", "浏览…", false, cx));
        let mut sections = vec![SettingsSection::new(
            "VectorCraft 程序",
            vec![
                SettingRow::new(
                    "vectorcraft-executable",
                    "程序路径",
                    SettingControl::Custom(executable.into_any_element()),
                )
                .description(
                    "后台任务默认使用同目录的 vectorcraft-cli.exe；保存路径不会启动程序。",
                ),
                SettingRow::new(
                    "vectorcraft-port",
                    "本机控制端口",
                    SettingControl::Custom(
                        control(
                            "settings-vectorcraft-port",
                            components::text_input(&self.port).w(px(180.)),
                            &self.surfaces,
                        )
                        .into_any_element(),
                    ),
                )
                .description("配置仅保存；浏览和保存设置不会连接端口或启动 VectorCraft。"),
                SettingRow::new(
                    "vectorcraft-save",
                    "保存并应用",
                    SettingControl::Custom(self.program_action("save", "保存程序设置", true, cx)),
                )
                .description(if self.config_saved {
                    "程序设置已加载；保存后的配置会立即供后续后台任务使用。"
                } else {
                    "输入 1–65535 范围内的端口后保存。"
                })
                .error(self.config_error.clone()),
            ],
        )];
        for field in &self.fields {
            let mut choices = div().v_flex().w(px(320.)).max_w_full().gap_2();
            if field.values.is_empty() {
                choices = choices.child(
                    div()
                        .text_xs()
                        .text_color(rgb(theme::text_muted()))
                        .child("暂无默认信息"),
                );
            }
            for (index, value) in field.values.iter().enumerate() {
                let key = field.key.clone();
                choices = choices.child(
                    div()
                        .h_flex()
                        .items_center()
                        .gap_2()
                        .child(
                            control(
                                &format!("settings-value-{key}-{index}"),
                                components::text_input(value).w_full(),
                                &self.surfaces,
                            )
                            .flex_1()
                            .min_w_0(),
                        )
                        .child(
                            Button::new(format!("preset-remove-value-{key}-{index}"))
                                .ghost()
                                .label("删除")
                                .on_click(cx.listener(move |view, _, _, cx| {
                                    if let Some(f) = view.fields.iter_mut().find(|f| f.key == key) {
                                        if index < f.values.len() {
                                            f.values.remove(index);
                                        }
                                    }
                                    view.saved = false;
                                    view.subscribe_values(cx);
                                    view.queue_save(cx);
                                    cx.notify();
                                })),
                        ),
                );
            }
            let mut rows = vec![
                SettingRow::new(
                    format!("values:{}", field.key),
                    "默认信息",
                    SettingControl::Custom(choices.into_any_element()),
                )
                .description("每项信息可在页面内容的下拉菜单中选择。"),
                SettingRow::new(
                    format!("add:{}", field.key),
                    "添加默认信息",
                    self.action(format!("add:{}", field.key), "＋ 添加选项", false, cx),
                ),
            ];
            if field.key.starts_with("extra:") {
                rows.push(SettingRow::new(
                    format!("remove:{}", field.key),
                    "移除自定义字段",
                    self.action(format!("remove:{}", field.key), "删除字段预设", false, cx),
                ));
            }
            sections.push(SettingsSection::new(field.label.clone(), rows));
        }
        sections.push(SettingsSection::new(
            "管理字段",
            vec![
                SettingRow::new(
                    "new-name",
                    "新字段名称",
                    SettingControl::Custom(
                        control(
                            "settings-preset-name",
                            components::text_input(&self.name).w(px(240.)),
                            &self.surfaces,
                        )
                        .into_any_element(),
                    ),
                ),
                SettingRow::new(
                    "create",
                    "新建自定义字段",
                    self.action("create".into(), "＋ 添加字段", false, cx),
                ),
                SettingRow::new(
                    "save",
                    "保存字段与默认信息",
                    self.action("save".into(), "保存预设", true, cx),
                )
                .description(if self.saved {
                    "预设已保存"
                } else {
                    "有效修改自动保存，并同步到编辑页下拉菜单；不会覆盖页面已填写的值。"
                })
                .error(self.error.clone()),
            ],
        ));
        let weak = cx.entity().downgrade();
        SettingsPage::new("WorkCrate 字段预设", "", sections)
            .render_body(Rc::new(move |id, value, window, cx| {
                if matches!(value, SettingValue::Invoke) {
                    let _ = weak.update(cx, |view, cx| view.apply(id, window, cx));
                }
            }))
            .text_color(rgb(theme::text_primary()))
    }
}
