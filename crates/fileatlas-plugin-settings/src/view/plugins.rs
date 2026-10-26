use super::*;
use fileatlas_ui_bridge::components::WorkbenchPaint as _;
use fileatlas_ui_bridge::components::WorkbenchStyle as _;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
use gpui_kit::base::Disableable;
use std::path::{Path, PathBuf};

#[derive(Clone)]
struct StorageLocation {
    label: &'static str,
    note: &'static str,
    path: Option<PathBuf>,
    related: Vec<PathBuf>,
}

impl StorageLocation {
    fn new(label: &'static str, note: &'static str, path: PathBuf) -> Self {
        Self {
            label,
            note,
            path: Some(path),
            related: Vec::new(),
        }
    }

    fn unregistered() -> Self {
        Self {
            label: "存储位置",
            note: "此插件未声明独立存储位置。",
            path: None,
            related: Vec::new(),
        }
    }
}

pub(super) struct StorageUsage {
    pub(super) bytes: Option<u64>,
    pub(super) complete: bool,
}

pub(super) struct StorageCatalogEntry {
    plugin_id: String,
    plugin_title: String,
    location: StorageLocation,
}

pub(super) struct StorageSummaryRow {
    pub(super) plugin_id: String,
    pub(super) plugin_title: String,
    pub(super) label: &'static str,
    pub(super) note: &'static str,
    pub(super) path: Option<PathBuf>,
    pub(super) usage: StorageUsage,
}

fn measure_storage(locations: &[StorageLocation]) -> Vec<StorageUsage> {
    locations
        .iter()
        .map(|location| {
            let Some(path) = location.path.as_deref() else {
                return StorageUsage {
                    bytes: None,
                    complete: true,
                };
            };
            let mut total = 0u64;
            let mut complete = true;
            for candidate in
                std::iter::once(path).chain(location.related.iter().map(PathBuf::as_path))
            {
                measure_path(candidate, &mut total, &mut complete);
            }
            StorageUsage {
                bytes: Some(total),
                complete,
            }
        })
        .collect()
}

fn measure_path(path: &Path, total: &mut u64, complete: &mut bool) {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(_) => {
            *complete = false;
            return;
        }
    };
    if metadata.file_type().is_symlink() {
        return;
    }
    if metadata.is_file() {
        *total = total.saturating_add(metadata.len());
        return;
    }
    if !metadata.is_dir() {
        return;
    }
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => {
            *complete = false;
            return;
        }
    };
    for entry in entries {
        match entry {
            Ok(entry) => measure_path(&entry.path(), total, complete),
            Err(_) => *complete = false,
        }
    }
}

pub(super) fn format_storage_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024. && unit < UNITS.len() - 1 {
        value /= 1024.;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

pub(super) fn compact_storage_path(path: &Path) -> String {
    let path = path.display().to_string();
    let chars = path.chars().collect::<Vec<_>>();
    if chars.len() <= 76 {
        return path;
    }
    let head = chars.iter().take(30).collect::<String>();
    let tail = chars.iter().skip(chars.len() - 43).collect::<String>();
    format!("{head}…{tail}")
}

pub(super) fn measure_storage_catalog(catalog: Vec<StorageCatalogEntry>) -> Vec<StorageSummaryRow> {
    catalog
        .into_iter()
        .map(|entry| {
            let usage = measure_storage(std::slice::from_ref(&entry.location))
                .into_iter()
                .next()
                .unwrap_or(StorageUsage {
                    bytes: None,
                    complete: false,
                });
            StorageSummaryRow {
                plugin_id: entry.plugin_id,
                plugin_title: entry.plugin_title,
                label: entry.location.label,
                note: entry.location.note,
                path: entry.location.path,
                usage,
            }
        })
        .collect()
}

impl SettingsView {
    pub(super) fn storage_plugin_signature(&self) -> Vec<(String, String)> {
        let mut plugins = self
            .service
            .plugin_runtime()
            .snapshot_now()
            .into_iter()
            .map(|plugin| (plugin.id, plugin.title))
            .collect::<Vec<_>>();
        for (owner, title, _, _, _, _) in &self.plugin_settings_views {
            if !plugins.iter().any(|(id, _)| id == owner) {
                plugins.push((owner.clone(), title.clone()));
            }
        }
        plugins.sort_by(|left, right| left.0.cmp(&right.0));
        plugins
    }

    pub(super) fn storage_catalog(&self) -> Vec<StorageCatalogEntry> {
        self.storage_plugin_signature()
            .into_iter()
            .flat_map(|(plugin_id, plugin_title)| {
                self.plugin_storage_locations(&plugin_id)
                    .into_iter()
                    .map(move |location| StorageCatalogEntry {
                        plugin_id: plugin_id.clone(),
                        plugin_title: plugin_title.clone(),
                        location,
                    })
            })
            .collect()
    }

    fn plugin_storage_locations(&self, id: &str) -> Vec<StorageLocation> {
        let root = self
            .service
            .path()
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();
        match id {
            "fileatlas.whiteboard" => {
                let canvas_root = root.join("whiteboards");
                vec![
                    StorageLocation::new(
                        "画布数据文件（JSON）",
                        "画板、场景图层、文本、文件与 Eagle 链接、模板和颜色；不是 SQLite 数据库。",
                        canvas_root.join("library.json"),
                    ),
                    StorageLocation::new(
                        "画板图片预览",
                        "按画板保存的图片缩略图缓存。",
                        canvas_root.join("previews"),
                    ),
                    StorageLocation::new(
                        "粘贴图片资源",
                        "从剪贴板粘贴进画布的独立图片文件。",
                        canvas_root.join("pasted-images"),
                    ),
                ]
            }
            "fileatlas.launcher" => vec![
                StorageLocation::new(
                    "启动台配置",
                    "程序和分类配置文件。",
                    root.join("launcher").join("launcher.json"),
                ),
                StorageLocation::new(
                    "程序图标",
                    "导入或缓存的程序图标。",
                    root.join("launcher").join("icons"),
                ),
            ],
            "fileatlas.clipboard" => vec![StorageLocation::new(
                "剪贴板记录与图片",
                "剪贴板历史及其中保存的内容。",
                root.join("clipboard").join("clipboard.json"),
            )],
            "fileatlas.screenshot" => vec![
                StorageLocation::new(
                    "插件设置",
                    "FileAtlas 程序设置文件。",
                    self.service.path().to_path_buf(),
                ),
                StorageLocation::new(
                    "截图文件",
                    "Shotoshot 保存的截图文件。",
                    self.service
                        .snapshot()
                        .ok()
                        .and_then(|settings| settings.screenshot_tool.save_directory)
                        .unwrap_or_else(|| root.join("Screenshots")),
                ),
            ],
            "fileatlas.project-tree" => vec![StorageLocation::new(
                "项目根目录清单",
                "已登记的项目根目录。",
                root.join("project-tree").join("project-tree.json"),
            )],
            "fileatlas.workspaces" => vec![StorageLocation::new(
                "工作区清单",
                "保存的工作区和各栏布局。",
                root.join("workspaces").join("workspaces.json"),
            )],
            "fileatlas.material-theme" => vec![
                StorageLocation::new(
                    "主题参数",
                    "Material 主题生成参数。",
                    root.join("themes").join("material-settings.json"),
                ),
                StorageLocation::new(
                    "生成的主题",
                    "生成并应用的主题文件。",
                    root.join("themes").join("material.json"),
                ),
            ],
            "fileatlas.file-staging" => vec![StorageLocation::new(
                "暂存清单",
                "暂存文件路径清单。",
                root.join("file-staging").join("staging.json"),
            )],
            "fileatlas.workcrate-projects" | "fileatlas.workcrate-ui" => {
                vec![StorageLocation::new(
                    "WorkCrate 项目数据",
                    "项目关联、主图与字段预设。",
                    root.join("workcrate").join("projects.json"),
                )]
            }
            "fileatlas.workcrate-boards" => vec![
                StorageLocation::new(
                    "VectorCraft 配置",
                    "VectorCraft 可执行文件和连接配置。",
                    root.join("workcrate").join("vectorcraft.json"),
                ),
                StorageLocation::new(
                    "画板目录与预览缓存",
                    "画板目录、导出/预览缓存与辅助文件。",
                    root.join("workcrate"),
                ),
            ],
            "fileatlas.database"
            | "fileatlas.tagging"
            | "fileatlas.versions"
            | "fileatlas.version-info"
            | "fileatlas.file-manager" => {
                let database = root.join("fileatlas.db");
                let mut location = StorageLocation::new(
                    "共享媒体数据库",
                    "SQLite 文件；统计包含 WAL/SHM 辅助文件。",
                    database.clone(),
                );
                location.related =
                    vec![root.join("fileatlas.db-wal"), root.join("fileatlas.db-shm")];
                vec![location]
            }
            "fileatlas.session" => vec![StorageLocation::new(
                "窗口与视图状态",
                "保存的窗口和视图布局。",
                root.join("session.json"),
            )],
            "fileatlas.settings" => vec![StorageLocation::new(
                "程序设置",
                "FileAtlas 设置文件。",
                self.service.path().to_path_buf(),
            )],
            _ => vec![StorageLocation::unregistered()],
        }
    }

    fn request_plugin_storage(&mut self, id: String) {
        let key = (id.clone(), self.plugin_storage_revision);
        let locations = self.plugin_storage_locations(&id);
        self.plugin_storage
            .request(key, move || Ok(measure_storage(&locations)));
    }

    pub(super) fn poll_plugin_storage(&mut self) -> bool {
        let active = self.surfaces.is_open("settings")
            && self.surfaces.value("settings.tab").as_deref() == Some("plugin");
        if !active {
            self.plugin_storage_active = false;
            return false;
        }
        let Some(id) = self.surfaces.value("settings.plugin_id") else {
            self.plugin_storage_active = false;
            return false;
        };
        let mut changed = false;
        if !self.plugin_storage_active || self.plugin_storage_plugin_id.as_deref() != Some(&id) {
            self.plugin_storage_active = true;
            self.plugin_storage_plugin_id = Some(id.clone());
            self.plugin_storage_revision = self.plugin_storage_revision.wrapping_add(1);
            self.request_plugin_storage(id);
            changed = true;
        }
        self.plugin_storage.poll() | changed
    }

    fn refresh_plugin_storage(&mut self, id: String, cx: &mut Context<Self>) {
        self.plugin_storage_revision = self.plugin_storage_revision.wrapping_add(1);
        self.plugin_storage_plugin_id = Some(id.clone());
        self.plugin_storage_active = true;
        self.request_plugin_storage(id);
        cx.notify();
    }

    fn render_plugin_storage_locations(&self, id: &str, cx: &mut Context<Self>) -> Div {
        let locations = self.plugin_storage_locations(id);
        let key = (id.to_owned(), self.plugin_storage_revision);
        let usage = if self.plugin_storage.key() == Some(&key) {
            self.plugin_storage
                .value()
                .and_then(|result| result.as_ref().ok())
        } else {
            None
        };
        let rows = locations.into_iter().enumerate().map(|(index, location)| {
            let status = if location.path.is_none() {
                "未登记路径".to_owned()
            } else if let Some(usage) = usage.and_then(|values| values.get(index)) {
                match usage.bytes {
                    Some(bytes) if usage.complete => format_storage_size(bytes),
                    Some(bytes) if bytes > 0 => {
                        format!("至少 {}（部分目录不可读取）", format_storage_size(bytes))
                    }
                    Some(_) => "无法完整读取".into(),
                    None => "无法读取".into(),
                }
            } else if self.plugin_storage.is_pending() {
                "统计中…".into()
            } else if matches!(self.plugin_storage.value(), Some(Err(_))) {
                "统计失败".into()
            } else {
                "等待统计…".into()
            };
            let address = location.path.as_ref().map_or_else(
                || "该插件尚无路径声明".to_owned(),
                |path| compact_storage_path(path),
            );
            design::row()
                .child(design::label(location.label, location.note))
                .child(
                    div()
                        .v_flex()
                        .items_end()
                        .min_w_0()
                        .child(div().workbench_assistant().child(address))
                        .child(div().workbench_assistant().child(format!("占用 {status}"))),
                )
        });
        let plugin_id = id.to_owned();
        div()
            .v_flex()
            .gap(px(theme::space_standard()))
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .justify_between()
                    .child(self.section("数据存储位置").flex_1())
                    .child(
                        fileatlas_ui_bridge::components::button("settings-storage-refresh")
                            .outline()
                            .icon(theme::small_icon(gpui_kit::assets::IconName::RefreshCw))
                            .label(fileatlas_ui_bridge::i18n::tr("Refresh"))
                            .on_click(cx.listener(move |view, _, _, cx| {
                                view.refresh_plugin_storage(plugin_id.clone(), cx);
                            })),
                    ),
            )
            .children(rows)
    }

    pub(super) fn render_plugin_cards(&self, _s: &Settings, cx: &mut Context<Self>) -> Div {
        let runtime = self.service.plugin_runtime();
        let statuses = runtime.snapshot_now();
        let mut body = div().v_flex().gap(px(theme::space_standard()));
        for plugin in statuses {
            let id = plugin.id.clone();
            let name = plugin.title.clone();
            let description = plugin.description.clone();
            let icon = match id.as_str() {
                "fileatlas.launcher" => gpui_kit::assets::IconName::LayoutGrid,
                "fileatlas.clipboard" => gpui_kit::assets::IconName::Clipboard,
                "fileatlas.screenshot" => gpui_kit::assets::IconName::Aperture,
                "fileatlas.project-tree" => gpui_kit::assets::IconName::Folder,
                _ => gpui_kit::assets::IconName::Puzzle,
            };
            let enabled = plugin.enabled;
            let busy = plugin.busy;
            let plugin_id = id.clone();
            let settings_id = id.clone();
            body = body.child(
                div()
                    .v_flex()
                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                    .py(px(theme::space_expanded()))
                    .min_h(px(100.))
                    .justify_center()
                    .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Bottom)
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .justify_between()
                            .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
                            .child(
                                div()
                                    .size(px(30.))
                                    .workbench_corner(
                                        fileatlas_ui_bridge::components::CornerRole::Medium,
                                    )
                                    .workbench_surface(
                                        fileatlas_ui_bridge::components::SurfaceRole::Panel,
                                    )
                                    .h_flex()
                                    .items_center()
                                    .justify_center()
                                    .child(theme::large_icon(icon)),
                            )
                            .child(
                                div()
                                    .v_flex()
                                    .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .workbench_title()
                                            .child(fileatlas_ui_bridge::i18n::tr(&name)),
                                    )
                                    .child(
                                        div()
                                            .workbench_assistant()
                                            .child(fileatlas_ui_bridge::i18n::tr(&description)),
                                    ),
                            )
                            .child(div().workbench_assistant().child(if busy {
                                "处理中"
                            } else if enabled {
                                "已启用"
                            } else {
                                "已停用"
                            }))
                            .child(
                                fileatlas_ui_bridge::components::button(format!(
                                    "settings-open-plugin-{id}"
                                ))
                                .label(fileatlas_ui_bridge::i18n::tr("Settings"))
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.tab = Tab::PluginSettings;
                                        this.selected_plugin_id = Some(settings_id.clone());
                                        let _ = this.surfaces.set_value("settings.tab", "plugin");
                                        let _ = this
                                            .surfaces
                                            .set_value("settings.plugin_id", &settings_id);
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                fileatlas_ui_bridge::components::button(format!(
                                    "settings-toggle-plugin-{id}"
                                ))
                                .label(fileatlas_ui_bridge::i18n::tr(if busy {
                                    "Working…"
                                } else if enabled {
                                    "Disable"
                                } else {
                                    "Enable"
                                }))
                                .disabled(busy)
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        let runtime = this.service.plugin_runtime();
                                        let plugin_id = plugin_id.clone();
                                        let next = !enabled;
                                        cx.spawn(async move |this, cx| {
                                            let result =
                                                runtime.set_enabled(&plugin_id, next).await;
                                            let persisted_id = plugin_id.clone();
                                            let _ = this.update(cx, |view, cx| {
                                                match result {
                                                    Ok(()) => view.edit(
                                                        |settings| {
                                                            settings
                                                                .plugin_enabled
                                                                .insert(persisted_id.clone(), next);
                                                        },
                                                        cx,
                                                    ),
                                                    Err(error) => {
                                                        view.error = Some(format!(
                                                            "Could not change plugin state: {error}"
                                                        ))
                                                    }
                                                }
                                                cx.notify();
                                            });
                                        })
                                        .detach();
                                    },
                                )),
                            ),
                    ),
            );
        }
        body.child(
            div()
                .workbench_assistant()
                .child("停用会停止相关任务并移除命令和面板，数据文件会保留。"),
        )
    }

    pub(super) fn render_plugin_settings(&self, s: &Settings, cx: &mut Context<Self>) -> Div {
        let Some(id) = self.selected_plugin_id.as_deref() else {
            return div()
                .workbench_assistant()
                .child(fileatlas_ui_bridge::i18n::tr(
                    "Select a plugin from the navigation.",
                ));
        };
        let Some(plugin) = self
            .service
            .plugin_runtime()
            .snapshot_now()
            .into_iter()
            .find(|plugin| plugin.id == id)
        else {
            return div()
                .v_flex()
                .child(self.render_plugin_shortcuts(id, s, cx))
                .child(self.render_plugin_storage_locations(id, cx))
                .children(
                    self.plugin_settings_views
                        .iter()
                        .filter(|v| v.0 == id)
                        .map(|v| v.5.clone()),
                );
        };
        let mode = s.plugin_launch_mode(id);
        use fileatlas_ui_bridge::settings_page::{
            Choice, SettingControl, SettingRow, SettingValue, SettingsPage, SettingsSection,
        };
        let choices = plugin
            .launch_modes
            .iter()
            .map(|value| {
                let (key, label, note) = match value {
                    crate::PluginLaunchMode::WorkspaceAndPanel => (
                        "workspace",
                        "Workspace and floating panel",
                        "打开工具时保留工作台。",
                    ),
                    crate::PluginLaunchMode::PanelOnly => (
                        "panel",
                        "Floating panel only",
                        "独立打开工具窗口，不唤起主界面。",
                    ),
                    crate::PluginLaunchMode::Direct => (
                        "direct",
                        "Launch capture directly",
                        "由 Shotoshot 完成选区和标注。",
                    ),
                };
                Choice::new(key, label).description(note)
            })
            .collect();
        let selected = match mode {
            crate::PluginLaunchMode::WorkspaceAndPanel => "workspace",
            crate::PluginLaunchMode::PanelOnly => "panel",
            crate::PluginLaunchMode::Direct => "direct",
        };
        let weak = cx.entity().downgrade();
        let owner = id.to_owned();
        let mut body = SettingsPage::new(
            "",
            "",
            vec![
                SettingsSection::new(
                    "",
                    vec![SettingRow::new(
                        "settings-plugin-status",
                        "运行状态",
                        SettingControl::Status {
                            value: if plugin.enabled {
                                "Enabled"
                            } else {
                                "Disabled"
                            }
                            .into(),
                            enabled: plugin.enabled,
                        },
                    )],
                ),
                SettingsSection::new(
                    "Shortcut presentation",
                    vec![SettingRow::new(
                        "settings-plugin-mode",
                        "",
                        SettingControl::Radios {
                            value: selected.into(),
                            options: choices,
                        },
                    )],
                ),
            ],
        )
        .render_body(std::rc::Rc::new(move |_, value, _, cx| {
            if let SettingValue::Choice(value) = value {
                let mode = match value.as_str() {
                    "workspace" => crate::PluginLaunchMode::WorkspaceAndPanel,
                    "panel" => crate::PluginLaunchMode::PanelOnly,
                    "direct" => crate::PluginLaunchMode::Direct,
                    _ => return,
                };
                let _ = weak.update(cx, |view, cx| {
                    view.edit(
                        |settings| {
                            settings.plugin_launch_modes.insert(owner.clone(), mode);
                        },
                        cx,
                    )
                });
            }
        }));
        body = body.child(self.render_plugin_shortcuts(id, s, cx));
        if id == "fileatlas.screenshot" {
            body = body
                .child(
                    div()
                        .h_flex()
                        .items_center()
                        .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
                        .child(self.section("External software").flex_1())
                        .child(self.software_refresh_button(cx)),
                )
                .child(self.render_software_page(cx, true));
        }
        body = body.children(
            self.plugin_settings_views
                .iter()
                .filter(|v| v.0 == id)
                .map(|v| v.5.clone()),
        );
        body = body.child(self.render_plugin_storage_locations(id, cx));
        let (command_id, entry_label, entry_note) = match id {
            "fileatlas.launcher" => (
                "program-launcher/open",
                "管理程序和分类",
                "在启动台面板中整理程序。",
            ),
            "fileatlas.clipboard" => (
                "clipboard-workbench/open",
                "查看和管理历史",
                "在剪贴板面板中管理历史内容。",
            ),
            "fileatlas.screenshot" => (
                "screenshot-tool/open-manager",
                "打开插件面板",
                "截图保存和软件路径在管理器中调整。",
            ),
            "fileatlas.project-tree" => (
                "project-tree/open",
                "打开项目树",
                "浏览已保存的项目根目录及其完整子目录树。",
            ),
            _ => ("", "Open plugin panel", ""),
        };
        let cordis = self.cordis.clone();
        let command_id = command_id.to_owned();
        if id != "fileatlas.screenshot" {
            body = body.child(self.section("Tool entry"));
        }
        body = body.child(design::row().child(design::label(entry_label,entry_note)).child(
            fileatlas_ui_bridge::components::button(format!("settings-open-plugin-panel-{id}"))
                .outline()

                .label(fileatlas_ui_bridge::i18n::tr("Open plugin panel"))
                .disabled(!plugin.enabled || plugin.busy || command_id.is_empty())
                .on_click(move |_, _, _| {
                    let cordis = cordis.clone();
                    let command_id = command_id.clone();
                    tokio::spawn(async move {
                        if let Err(error) = cordis
                            .emit::<fileatlas_ui_bridge::UiCommandInvoked>(
                                cordis::Routing::Unscoped,
                                command_id,
                            )
                            .await
                        {
                            tracing::error!(%error, "could not open plugin panel from Settings");
                        }
                    });
                }),
        ));
        body
    }
}
