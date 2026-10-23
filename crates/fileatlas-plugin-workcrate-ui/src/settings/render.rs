use super::*;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
use fileatlas_ui_bridge::theme;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
impl Render for PresetSettings {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.refresh_requested {
            self.refresh_requested = false;
            if !self.busy
                && !self.auto_pending
                && self.error.is_none()
                && self.stored().ok().as_ref() != Some(&self.snapshot(cx))
            {
                self.change_scope(self.project_id.clone(), window, cx);
            }
        }
        let state = serde_json::json!({"project_id": self.project_id, "presets": self.snapshot(cx), "busy": self.busy, "pending": self.auto_pending, "error": self.error, "notice": self.notice, "config_error": self.config_error, "config_busy": self.config_busy, "config_saved": self.config_saved, "preferences": self.preferences.snapshot().ok().map(|s| s.workcrate), "executable": self.executable.read(cx).value().to_string(), "cli": self.cli.read(cx).value().to_string(), "port": self.port.read(cx).value().to_string()}).to_string();
        if self.surfaces.value("workcrate.settings").as_deref() != Some(&state) {
            let _ = self.surfaces.set_value("workcrate.settings", &state);
        }
        let executable = div()
            .w(px(theme::CONTROL_WIDTH_EXTRA_WIDE))
            .max_w_full()
            .h_flex()
            .items_center()
            .gap(px(fileatlas_ui_bridge::theme::space_standard()))
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
        let weak = cx.entity().downgrade();
        let projects = self.service.projects(true).unwrap_or_default();
        let label = self
            .project_id
            .as_ref()
            .and_then(|id| projects.iter().find(|p| &p.id == id))
            .map(|p| format!("项目：{}", p.name))
            .unwrap_or_else(|| "程序默认信息".into());
        let current = self.project_id.clone();
        let scope = fileatlas_ui_bridge::components::button("workcrate-preset-scope")
            .outline()
            .label(label)
            .disabled(self.busy || self.auto_pending || self.error.is_some())
            .dropdown_menu(move |mut menu, _, _| {
                let global = weak.clone();
                menu = menu.item(
                    PopupMenuItem::new("程序默认信息")
                        .checked(current.is_none())
                        .on_click(move |_, window, cx| {
                            let _ =
                                global.update(cx, |view, cx| view.change_scope(None, window, cx));
                        }),
                );
                for project in &projects {
                    let weak = weak.clone();
                    let id = project.id.clone();
                    menu = menu.item(
                        PopupMenuItem::new(format!("项目：{}", project.name))
                            .checked(current.as_ref() == Some(&id))
                            .on_click(move |_, window, cx| {
                                let _ = weak.update(cx, |view, cx| {
                                    view.change_scope(Some(id.clone()), window, cx)
                                });
                            }),
                    );
                }
                menu
            });
        let mut sections = vec![SettingsSection::new(
            "默认信息范围",
            vec![
                SettingRow::new(
                    "scope",
                    "编辑范围",
                    SettingControl::Custom(
                        control("settings-preset-scope", scope, &self.surfaces).into_any_element(),
                    ),
                )
                .description(self.notice.as_deref().unwrap_or(
                    "程序选项供所有项目使用；项目选项只在该项目出现。两者合并去重，不覆盖页面值。",
                ))
                .error(self.error.clone()),
            ],
        )];
        if self.project_id.is_none() {
            match self.preferences.snapshot() {
                Ok(saved) => sections.push(SettingsSection::new(
                    "浏览与导出默认值",
                    vec![
                        SettingRow::new(
                            "default-sort",
                            "项目默认排序",
                            SettingControl::Select {
                                value: if saved.workcrate.sort_by_name {
                                    "name"
                                } else {
                                    "created"
                                }
                                .into(),
                                options: vec![
                                    Choice::new("created", "最近创建"),
                                    Choice::new("name", "项目名称"),
                                ],
                            },
                        )
                        .description(
                            "新打开的 WorkCrate 面板使用此排序；面板中的切换只影响当前浏览。",
                        ),
                        SettingRow::new(
                            "default-export",
                            "默认图片格式",
                            SettingControl::Select {
                                value: if saved.workcrate.image_export_jpeg {
                                    "jpeg"
                                } else {
                                    "png"
                                }
                                .into(),
                                options: vec![
                                    Choice::new("png", "PNG"),
                                    Choice::new("jpeg", "JPEG"),
                                ],
                            },
                        )
                        .description("新打开的面板使用此格式；导出前仍可临时切换。"),
                    ],
                )),
                Err(error) => sections.push(SettingsSection::new(
                    "浏览与导出默认值",
                    vec![SettingRow::new(
                        "preferences-error",
                        "读取失败",
                        SettingControl::ReadOnly(error.to_string()),
                    )],
                )),
            }
        }
        if self.project_id.is_none() {
            sections.push(SettingsSection::new(
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
                SettingRow::new("vectorcraft-cli", "后台 CLI 路径", SettingControl::Custom(control("settings-vectorcraft-cli", components::text_input(&self.cli).w(px(theme::CONTROL_WIDTH_EXTRA_WIDE)), &self.surfaces).into_any_element()))
                    .description("可留空，自动使用程序同目录的 vectorcraft-cli.exe；可填写独立安装的官方 CLI。"),
                SettingRow::new(
                    "vectorcraft-port",
                    "本机控制端口",
                    SettingControl::Custom(
                        control(
                            "settings-vectorcraft-port",
                            components::text_input(&self.port).w(px(theme::CONTROL_WIDTH_STANDARD)),
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
        ));
        }
        for field in &self.fields {
            let mut choices = div()
                .v_flex()
                .w(px(theme::CONTROL_WIDTH_EXTRA_WIDE))
                .max_w_full()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()));
            if field.values.is_empty() {
                choices = choices.child(div().workbench_assistant().child("暂无默认信息"));
            }
            for (index, value) in field.values.iter().enumerate() {
                let key = field.key.clone();
                choices = choices.child(
                    div()
                        .h_flex()
                        .items_center()
                        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
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
                            fileatlas_ui_bridge::components::button(format!(
                                "preset-remove-value-{key}-{index}"
                            ))
                            .ghost()
                            .label("删除")
                            .on_click(cx.listener(
                                move |view, _, _, cx| {
                                    if let Some(f) = view.fields.iter_mut().find(|f| f.key == key) {
                                        if index < f.values.len() {
                                            f.values.remove(index);
                                        }
                                    }
                                    view.saved = false;
                                    view.subscribe_values(cx);
                                    view.queue_save(cx);
                                    cx.notify();
                                },
                            )),
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
            if self.project_id.is_some() {
                let key = field.key.clone();
                rows.push(
                    SettingRow::new(
                        format!("promote:{key}"),
                        "提升此列到程序",
                        SettingControl::Custom(
                            control(
                                &format!("settings-promote-{key}"),
                                fileatlas_ui_bridge::components::button(format!("promote-{key}"))
                                    .outline()
                                    .label("合并到程序默认信息")
                                    .disabled(
                                        self.busy
                                            || self.auto_pending
                                            || self.error.is_some()
                                            || field
                                                .values
                                                .iter()
                                                .all(|v| v.read(cx).value().trim().is_empty()),
                                    )
                                    .on_click(cx.listener(move |view, _, _, cx| {
                                        view.promote(key.clone(), cx)
                                    })),
                                &self.surfaces,
                            )
                            .into_any_element(),
                        ),
                    )
                    .description("只合并这一列并去重，保留程序原有选项；其他列不变。"),
                );
            }
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
                            components::text_input(&self.name).w(px(theme::CONTROL_WIDTH_WIDE)),
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
                .description(if let Some(notice) = &self.notice {
                    notice
                } else if self.saved {
                    "预设已保存"
                } else {
                    "有效修改自动保存，并同步到编辑页下拉菜单；不会覆盖页面已填写的值。"
                })
                .error(self.error.clone()),
            ],
        ));
        let weak = cx.entity().downgrade();
        div()
            .id("workcrate-defaults-form")
            .size_full()
            .overflow_y_scroll()
            .p(px(fileatlas_ui_bridge::theme::space_expanded()))
            .child(
                SettingsPage::new("WorkCrate 字段预设", "", sections)
                    .render_body(Rc::new(move |id, value, window, cx| {
                        let _ = weak.update(cx, |view, cx| {
                            if matches!(value, SettingValue::Invoke) {
                                view.apply(id, window, cx);
                            } else {
                                view.preference(id, value, cx);
                            }
                        });
                    }))
                    .workbench_body(),
            )
    }
}
