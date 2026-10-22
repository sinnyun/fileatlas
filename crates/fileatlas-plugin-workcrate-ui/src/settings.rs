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
    preferences: Arc<fileatlas_plugin_settings::SettingsService>,
    service: Arc<WorkCrateProjectsService>,
    boards: Arc<WorkCrateBoardsService>,
    surfaces: Arc<UiSurfaceState>,
    fields: Vec<DraftField>,
    project_id: Option<String>,
    name: Entity<InputState>,
    executable: Entity<InputState>,
    cli: Entity<InputState>,
    port: Entity<InputState>,
    config_busy: bool,
    config_saved: bool,
    config_error: Option<String>,
    busy: bool,
    error: Option<String>,
    saved: bool,
    auto_pending: bool,
    notice: Option<String>,
    _tasks: Vec<Task<()>>,
    refresh_requested: bool,
    _subscriptions: Vec<Subscription>,
    _config_subscriptions: Vec<Subscription>,
}

pub(super) fn contribution(
    service: Arc<WorkCrateProjectsService>,
    boards: Arc<WorkCrateBoardsService>,
    surfaces: Arc<UiSurfaceState>,
    preferences: Arc<fileatlas_plugin_settings::SettingsService>,
) -> UiViewContribution {
    UiViewContribution::new(
        "workcrate/settings",
        "settings.pages",
        0,
        move |window, app| {
            editor(
                service.clone(),
                boards.clone(),
                surfaces.clone(),
                None,
                preferences.clone(),
                window,
                app,
            )
        },
    )
    .with_settings_page("WorkCrate", "配置官方 VectorCraft 程序与页面字段预设。")
}
pub(super) fn editor(
    service: Arc<WorkCrateProjectsService>,
    boards: Arc<WorkCrateBoardsService>,
    surfaces: Arc<UiSurfaceState>,
    project_id: Option<String>,
    preferences: Arc<fileatlas_plugin_settings::SettingsService>,
    window: &mut Window,
    app: &mut App,
) -> AnyView {
    let stored_result = match &project_id {
        Some(id) => service.project_field_presets(id),
        None => service.field_presets(),
    };
    let preset_error = stored_result.as_ref().err().map(ToString::to_string);
    let stored = stored_result.unwrap_or_default();
    let config_result = boards.configuration();
    let config_error = config_result.as_ref().err().map(ToString::to_string);
    let config = config_result.unwrap_or_default();
    let name = app.new(|cx| InputState::new(window, cx).placeholder("例如：制作工艺"));
    let executable = input(config.executable, window, app);
    let cli = input(config.cli_executable, window, app);
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
            preferences,
            service: service.clone(),
            boards: boards.clone(),
            surfaces: surfaces.clone(),
            fields,
            project_id,
            name,
            executable,
            cli,
            port,
            config_busy: false,
            config_saved: config_error.is_none(),
            config_error,
            busy: false,
            error: preset_error,
            saved: false,
            auto_pending: false,
            notice: None,
            _tasks: vec![],
            refresh_requested: false,
            _subscriptions: vec![],
            _config_subscriptions: vec![],
        };
        view.subscribe_values(cx);
        view.subscribe_config(cx);
        view._tasks.push(fileatlas_ui_bridge::refresh::listen(
            cx,
            service.wake(),
            |view: &mut PresetSettings, cx| {
                if view.busy || view.auto_pending || view.error.is_some() {
                    return false;
                }
                if view.stored().ok().as_ref() == Some(&view.snapshot(cx)) {
                    return false;
                }
                view.refresh_requested = true;
                true
            },
        ));
        view
    })
    .into()
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
    fn preference(&mut self, id: &str, value: SettingValue, cx: &mut Context<Self>) {
        let choice = match value {
            SettingValue::Choice(value) => value,
            _ => return,
        };
        let sort = match (id, choice.as_str()) {
            ("default-sort", "name") => Some(true),
            ("default-sort", "created") => Some(false),
            _ => None,
        };
        let jpeg = match (id, choice.as_str()) {
            ("default-export", "jpeg") => Some(true),
            ("default-export", "png") => Some(false),
            _ => None,
        };
        if sort.is_none() && jpeg.is_none() {
            return;
        }
        let preferences = self.preferences.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    preferences
                        .update(|s| {
                            if let Some(value) = sort {
                                s.workcrate.sort_by_name = value;
                            }
                            if let Some(value) = jpeg {
                                s.workcrate.image_export_jpeg = value;
                            }
                        })
                        .map_err(|e| e.to_string())
                })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.config_error = result.err();
                cx.notify();
            });
        })
        .detach();
    }
    fn apply(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.error = None;
        self.notice = None;
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
                            view.notice = None;
                            view.saved = false;
                            view.queue_save(cx);
                        }
                    }));
            }
        }
    }
    fn subscribe_config(&mut self, cx: &mut Context<Self>) {
        self._config_subscriptions = [self.executable.clone(), self.cli.clone(), self.port.clone()]
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
        let cli = self.cli.read(cx).value().trim().to_owned();
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
                    config.cli_executable = cli;
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
    fn stored(
        &self,
    ) -> Result<FieldPresets, fileatlas_plugin_workcrate_projects::WorkCrateProjectsError> {
        match &self.project_id {
            Some(id) => self.service.project_field_presets(id),
            None => self.service.field_presets(),
        }
    }
    fn change_scope(
        &mut self,
        project: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.busy || self.auto_pending || self.error.is_some() {
            return;
        }
        let result = match &project {
            Some(id) => self.service.project_field_presets(id),
            None => self.service.field_presets(),
        };
        match result {
            Ok(value) => {
                self.fields = value
                    .fields
                    .into_iter()
                    .map(|f| DraftField {
                        key: f.key,
                        label: f.label,
                        values: f.values.into_iter().map(|v| input(v, window, cx)).collect(),
                    })
                    .collect();
                self.project_id = project;
                self.notice = None;
                self.saved = true;
                self.subscribe_values(cx);
            }
            Err(error) => self.error = Some(error.to_string()),
        }
        cx.notify();
    }
    fn promote(&mut self, key: String, cx: &mut Context<Self>) {
        if self.busy || self.auto_pending || self.error.is_some() {
            return;
        }
        let Some(id) = self.project_id.clone() else {
            return;
        };
        let service = self.service.clone();
        self.busy = true;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    service
                        .promote_project_field(&id, &key)
                        .map_err(|e| e.to_string())
                })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                view.error = result.err();
                view.saved = view.error.is_none();
                view.notice = view.saved.then(|| "此列已合并到程序默认信息".into());
                cx.notify();
            });
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
            && self.stored().ok().as_ref() != Some(&self.snapshot(cx))
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
                if view.stored().ok().as_ref() == Some(&view.snapshot(cx)) {
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
        if self.busy {
            return;
        }
        let service = self.service.clone();
        let project = self.project_id.clone();
        self.busy = true;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    match project {
                        Some(id) => service.save_project_field_presets(&id, value),
                        None => service.save_field_presets(value),
                    }
                    .map_err(|e| e.to_string())
                })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                view.saved =
                    result.is_ok() && view.stored().ok().as_ref() == Some(&view.snapshot(cx));
                view.error = result.err();
                cx.notify();
            });
        })
        .detach();
    }
}
mod render;
