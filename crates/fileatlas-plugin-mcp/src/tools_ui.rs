//! Service-level UI validation; no OS pointer/keyboard injection.
use super::*;
use rmcp::schemars;
use serde::Deserialize;

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub(super) struct UiRequest {
    /// settings_open/close/tab, language, statistics, bulk_actions,
    /// file_manager_dialogs_close, select_media, deselect, hover_media, hover_clear,
    /// inspector_open/close, sidebar_open/close, export_open/close,
    /// settings_check_updates, writeback_retry/dismiss (value: exact failure path).
    /// sidebar_width/inspector_width (logical pixels), navigation_tab, inspector_tab.
    /// region_separation: value border|shadow, saved through the Settings service.
    /// columns_layout: value single|dual|quad; activate_column: zero-based index;
    /// workspace_select: existing workspace id; workspace_manager: open|close.
    /// pointer_down/pointer_drag/pointer_up: value is logical client coordinates "x,y".
    /// pointer_scroll: value "x,y,delta_y", negative delta scrolls down; bounded logical pixels.
    /// sidebar_request: `request` is one SidebarUiRequest JSON object, e.g.
    /// {"kind":"remove_quick_access","path":"F:\\library"}.
    action: String,
    /// Tab: appearance/shortcuts/tags/ai/updates. Language: en/zh-CN.
    value: Option<String>,
    /// Structured SidebarUiRequest payload for the sidebar_request action.
    request: Option<serde_json::Value>,
    /// Required acknowledgement for high-impact retry/accessibility actions.
    confirm: Option<bool>,
}

#[tool_router(router = ui_tool_router, vis = "pub(super)")]
impl FileAtlasMcpServer {
    #[tool(
        description = "Read Settings, language, region separation mode, Preview opacity, production scan progress and bounded FileManager animation frame diagnostics without changing state."
    )]
    fn get_ui_state(&self) -> String {
        let file_manager = self.file_manager.ui_snapshot().map(|state| {
            let statistics = json!({"state":state.dialog_reads.statistics_state,"error":state.dialog_reads.statistics_error,"photos":state.dialog_reads.photos,"videos":state.dialog_reads.videos,"audio":state.dialog_reads.audio,"rating_counts":state.dialog_reads.rating_counts});
                    let bulk = json!({"state":state.dialog_reads.bulk_state,"error":state.dialog_reads.bulk_error,"count":state.dialog_reads.bulk_count,"preview_ids":state.dialog_reads.bulk_preview_ids});
                    let dialog_reads = json!({"statistics":statistics,"bulk":bulk});
                    let mut snapshot = json!({
                "records_open":state.records_open,"records_count":state.records_count,"record_candidates":state.record_candidates,
                "browsing": state.browsing, "current_directory": state.current_directory, "parent_directory": state.parent_directory,
                "directories": state.directories.iter().map(|f| json!({"name":f.name,"path":f.path})).collect::<Vec<_>>(),
                "selected_directory": state.selected_directory, "directory_error": state.directory_error,
                "search": state.filter.search, "media_type": format!("{:?}", state.filter.media_type),
                "folder_id": state.filter.folder_id, "path_prefix": state.filter.path_prefix,
                "sort": format!("{:?}", state.filter.sort), "extensions": state.filter.extensions,
                "excluded_extensions": state.filter.excluded_extensions, "included_tags": state.filter.selected_tag_ids,
                "excluded_tags": state.filter.excluded_tag_ids, "rating_min": state.filter.rating_min,
                "rating_max": state.filter.rating_max, "date_from": state.filter.date_from.map(|d| d.to_string()),
                "date_to": state.filter.date_to.map(|d| d.to_string()), "selected_media_ids": state.selected_media_ids,
                "layout": if state.list_layout { "list" } else { "grid" },
                "grouping_mode": state.grouping_mode.key(), "groups": state.groups, "collapsed_groups": state.collapsed_groups,
                "matched_count": state.matched_count, "group_by_month": state.group_by_month,
                "window_offset": state.window_offset, "loaded_count": state.loaded_count,
                "snapshot_revision":state.snapshot_revision,
                "inspector_sections": state.inspector_sections, "inspector_context": state.inspector_context,
                "bulk_action": state.bulk_action, "bulk_source": state.bulk_source,
                "statistics_open": state.statistics_open, "tag_picker_open": state.tag_picker_open,
                "dialog_reads": dialog_reads,
                "media_actions_open": state.media_actions_open, "media_actions_id": state.media_actions_id,
            });
            snapshot["file_copy_buffer_count"] = json!(state.file_copy_buffer_count);
            snapshot["search_scopes_open"] = json!(state.search_scopes_open);
            snapshot["debug_internal_file_drag"] = json!(self.surfaces.is_open("file_manager.column.0.internal-file-drag"));
            snapshot["group_frames"] = self.surfaces.value("file_manager.column.0.group-frames")
                .and_then(|value| serde_json::from_str::<serde_json::Value>(&value).ok())
                .unwrap_or_else(|| json!({}));
            snapshot["search_entire_computer"] = json!(state.filter.search_entire_computer);
            snapshot["search_scopes"] = json!({
                "filenames":state.filter.search_scopes.filenames,
                "paths":state.filter.search_scopes.paths,
                "tags":state.filter.search_scopes.tags,
                "time":state.filter.search_scopes.time
            });
            snapshot["search_provider"] = json!(state.search_provider);
            snapshot["everything"] = json!({"state":state.everything_state,"version":state.everything_version,
                "path":state.everything_path,"index_loading":state.everything_loading});
            snapshot["version_states_loading"] = json!(state.version_states_loading);
            snapshot["version_states"] = json!(state.version_states.iter().map(|item| json!({"media_id":item.media_id,"path":item.path,"state":item.state,"error":item.error,"badge_visible":item.state.as_ref().is_some_and(|state| serde_json::to_value(state.status).ok().and_then(|value| value.as_str().map(str::to_owned)).as_deref() == Some("modified"))})).collect::<Vec<_>>());
            snapshot
        });
        let history_visible = self.surfaces.is_open("inspector")
            && self.surfaces.value("file_manager.inspector.tab").as_deref() == Some("history")
            && file_manager
                .as_ref()
                .and_then(|state| state["inspector_context"].as_str())
                == Some("file");
        // These are the same role resolvers consumed by cards/capsules, not a separate palette.
        let content_colors = {
            use fileatlas_ui_bridge::theme as t;
            let formats = [
                "png", "psd", "ipk", "zip", "toml", "py", "svg", "", "jpg", "mp4", "txt",
            ]
            .map(|extension| {
                let (background, foreground) = t::file_format_colors(extension);
                json!({"extension":extension,"background":background,"foreground":foreground})
            });
            let tags = [
                None,
                Some(0x000000),
                Some(0xffffff),
                Some(0x763c98),
                Some(0x315ca8),
                Some(0xffff00),
            ]
            .into_iter()
            .flat_map(|tint| {
                [false, true].map(move |selected| {
                    let colors = t::tag_colors(tint, selected);
                    json!({"tint":tint,"selected":selected,"background":colors.background,
                        "foreground":colors.foreground,"icon":colors.icon})
                })
            })
            .collect::<Vec<_>>();
            json!({"formats":formats,"tags":tags,"selection_text":t::selection_text(),
                "media_background":t::media_badge_background(),"media_text":t::text_on_media_badge(),
                "media_star":t::star_on_media_badge()})
        };
        let theme_state = json!({
            "id":fileatlas_ui_bridge::theme::theme_name(),
            "framework":"GPUI Kit ThemeRegistry",
            "mode":if fileatlas_ui_bridge::theme::color_mode() == fileatlas_ui_bridge::theme::ColorMode::Light { "light" } else { "dark" },
            "accent":fileatlas_ui_bridge::theme::accent(),
            "content_colors":content_colors,
            "accent_text":fileatlas_ui_bridge::theme::accent_text(),
            "button_text":fileatlas_ui_bridge::theme::text_on_accent(),
            "selection":fileatlas_ui_bridge::theme::selection_color(),
            "danger":fileatlas_ui_bridge::theme::danger(),
            "danger_text":fileatlas_ui_bridge::theme::text_on_danger(),
            "danger_label":fileatlas_ui_bridge::theme::danger_text(),
            "warning":fileatlas_ui_bridge::theme::warning(),
            "warning_text":fileatlas_ui_bridge::theme::text_on_warning(),
            "warning_label":fileatlas_ui_bridge::theme::warning_text(),
            "success":fileatlas_ui_bridge::theme::success(),
            "success_text":fileatlas_ui_bridge::theme::text_on_success(),
            "success_label":fileatlas_ui_bridge::theme::success_text(),
            "star_label":fileatlas_ui_bridge::theme::star_text(),
            "base":fileatlas_ui_bridge::theme::surface_base(),
            "panel":fileatlas_ui_bridge::theme::surface_panel(),
            "card":fileatlas_ui_bridge::theme::surface_card(),
            "elevated":fileatlas_ui_bridge::theme::surface_elevated(),
            "border":fileatlas_ui_bridge::theme::separator(),
            "text":fileatlas_ui_bridge::theme::text_primary(),
            "text_secondary":fileatlas_ui_bridge::theme::text_secondary(),
            "text_muted":fileatlas_ui_bridge::theme::text_muted(),
            "body_size":fileatlas_ui_bridge::theme::FONT_SIZE_BODY,
        });
        let plugin_settings = self
            .context
            .try_service::<fileatlas_plugin_settings::SettingsService>()
            .ok()
            .map(|settings| {
                let preferences = settings.snapshot().ok();
                json!({
                    "runtime": settings.plugin_runtime().snapshot_now().into_iter().map(|plugin| json!({
                        "id": plugin.id,
                        "title": plugin.title,
                        "description": plugin.description,
                        "launch_modes": plugin.launch_modes,
                        "enabled": plugin.enabled,
                        "busy": plugin.busy,
                    })).collect::<Vec<_>>(),
                    "launch_modes": preferences.as_ref().map(|value| &value.plugin_launch_modes),
                    "enabled": preferences.as_ref().map(|value| &value.plugin_enabled),
                    "mcp_access_level": preferences.map(|value| value.mcp_access_level.key()),
                })
            });
        let mut state = json!({
            "browser": {"filesystem":self.file_manager.browser_enabled(), "show_descendants":self.file_manager.show_descendants(),"worker_active":self.file_manager.directory_worker_active(),"status":self.file_manager.directory_status()},
            "input_activity": self.capture.input_snapshot(),
            "window_status": crate::screenshot::window_status(self.capture.get()),
            "library_refresh": self.navigation.library_refresh().map(|state| json!({"running":state.running,"total_folders":state.total_folders,"completed_folders":state.completed_folders,"failures":state.failures})),
            "folder_picker": {"pending": self.navigation.folder_picker_pending(), "native_open": self.navigation.folder_picker_window().is_some()},
            "file_manager_popover": self.surfaces.value("file_manager.popover").unwrap_or_default(),
            "tools_open": self.surfaces.is_open("tools"),
            "file_manager": file_manager,
            "panels": (["settings"].iter().map(|id| json!({"id":id,"open":self.surfaces.is_open(&format!("panel.{id}")),"bounds":self.surfaces.value(&format!("panel.{id}.bounds")),"client_bounds":self.surfaces.value(&format!("panel.{id}.client_bounds"))})).collect::<Vec<_>>()),
            "open_panel_ids": self.surfaces.open_panel_ids(),
            "panel_instances": self.surfaces.open_panel_ids().iter().map(|id| json!({"id":id,"bounds":self.surfaces.value(&format!("panel.{id}.bounds")),"client_bounds":self.surfaces.value(&format!("panel.{id}.client_bounds"))})).collect::<Vec<_>>(),
            "active_panel": self.surfaces.value("panel.active"),
            "settings_open": self.surfaces.is_open("settings"),

            "export_open": self.surfaces.is_open("metadata-export"),
            "sidebar_open": self.surfaces.value("sidebar.visible").as_deref() != Some("false"),
            "inspector_open": self.surfaces.is_open("inspector"),
            "update_checking": self.surfaces.is_open("settings.update-checking"),
            "update_available": self.surfaces.is_open("settings.update-available"),
            "update_offer_open": self.surfaces.is_open("settings.update-offer"),
            "settings_tab": self.surfaces.value("settings.tab").unwrap_or_else(|| "appearance".into()),
            "settings_tag_choices": self.surfaces.value("settings.tag-choices").and_then(|value| serde_json::from_str::<serde_json::Value>(&value).ok()),
            "settings_software_check": self.surfaces.value("settings.software-check").and_then(|value| serde_json::from_str::<serde_json::Value>(&value).ok()),
            "navigation_tab": self.surfaces.value("navigation.tab").unwrap_or_else(|| "folders".into()),
            "sidebar_folders": self.navigation.sidebar_state(),
            "inspector_tab": self.surfaces.value("file_manager.inspector.tab").unwrap_or_else(|| "information".into()),
            "inspector_history": {
                "visible": history_visible,
                "enabled": self.surfaces.value("file_manager.inspector.history-enabled").as_deref() == Some("true"),
                "selected": self.surfaces.value("file_manager.inspector.history-selected"),
                "active": self.surfaces.value("file_manager.inspector.history-active"),
                "count": self.surfaces.value("file_manager.inspector.history-count").and_then(|s| s.parse::<usize>().ok()),
                "last_created": self.surfaces.value("file_manager.inspector.history-last-created"),
            },
            "sidebar_width": self.surfaces.value("shell.sidebar-width").and_then(|value| value.parse::<f32>().ok()),
            "inspector_width": self.surfaces.value("shell.inspector-width").and_then(|value| value.parse::<f32>().ok()),
            "language": self.i18n.as_ref().map(|service| service.language().code()),
            "translation_revision": fileatlas_ui_bridge::i18n::revision(),
            "region_separation": fileatlas_ui_bridge::theme::region_depth_name(),
            "quicklook": self.quicklook.snapshot(),
            "quicklook_notice": self.surfaces.value("quicklook.notice"),
            "file_manager_cards": self.file_manager.card_states().iter().map(|state| json!({
                "media_id": state.media_id, "hover": state.hover, "selection": state.selection,
            })).collect::<Vec<_>>(),
            "file_manager_animation_frames": self.file_manager.card_animation_frames().iter().map(|state| json!({
                "media_id": state.media_id, "hover": state.hover, "selection": state.selection, "timestamp_ms": state.timestamp_ms,
            })).collect::<Vec<_>>(),
            "scan_progress": self.file_manager.scan_progress().map(|p| json!({
                "phase": p.phase, "scanned": p.scanned, "total": p.total, "error": p.error,
            })),
            "metadata_export": self.metadata_export.status().ok().map(|status| match status {
                fileatlas_plugin_metadata_export::MetadataExportStatus::Idle => json!({"state":"idle"}),
                fileatlas_plugin_metadata_export::MetadataExportStatus::Exporting => json!({"state":"exporting"}),
                fileatlas_plugin_metadata_export::MetadataExportStatus::Succeeded { path, records } => json!({"state":"succeeded","path":path,"records":records}),
                fileatlas_plugin_metadata_export::MetadataExportStatus::Failed(error) => json!({"state":"failed","error":error}),
            }),
            "writeback_failures": self.tagging.writeback_failures().iter().map(|failure| json!({"path":failure.path,"error":failure.error})).collect::<Vec<_>>(),
        });
        state["settings_render_count"] = json!(
            self.surfaces
                .value("settings.render-count")
                .and_then(|count| count.parse::<u64>().ok())
        );
        state["settings_plugin_id"] = json!(self.surfaces.value("settings.plugin_id"));
        state["sidebar_plugin_buttons"] = json!(
            self.context
                .try_service::<fileatlas_ui_bridge::UiBridgeService>()
                .ok()
                .and_then(|bridge| bridge.views.snapshot().ok())
                .unwrap_or_default()
                .into_iter()
                .filter(|entry| entry.contribution.slot == "navigation.launcher-shortcuts")
                .map(|entry| json!({"id":entry.contribution.id,"owner":entry.owner}))
                .collect::<Vec<_>>()
        );
        state["settings_plugins"] = json!(plugin_settings);
        state["selected_window"] = json!(
            self.surfaces
                .value("mcp.window.selected")
                .unwrap_or_else(|| "main".into())
        );
        let main = self
            .surfaces
            .value("mcp.window.main")
            .and_then(|v| v.parse().ok())
            .unwrap_or_default();
        state["main_window_status"] = crate::screenshot::window_status(main);
        state["native_panels"] = json!(self.surfaces.open_panel_ids().into_iter().filter_map(|id| {
            let hwnd = self.surfaces.value(&format!("panel.{id}.hwnd"))?.parse::<isize>().ok()?;
            Some(json!({"id":id,"hwnd":hwnd,"status":crate::screenshot::window_status(hwnd),"render_count":self.surfaces.value(&format!("panel.{id}.render_count"))}))
        }).collect::<Vec<_>>());
        state["theme"] = theme_state;
        state["screenshot_tool"] =
            json!(self.screenshot.as_ref().map(|service| service.mcp_state()));
        state["file_manager_search_provider"] = json!(self.file_manager.search_provider());
        state["settings_shortcuts"] = json!({
            "capturing_key": self.surfaces.is_capturing_key("settings"),
            "binding_id": self.surfaces.value("settings.shortcut-capture").unwrap_or_default(),
            "error": self.surfaces.value("settings.shortcut-error").unwrap_or_default(),
        });
        state["file_manager_search_provider_detail"] =
            json!(self.file_manager.search_provider_detail());
        state["drag_drop"] = json!(
            self.context
                .try_service::<fileatlas_plugin_drag_drop::DragDropService>()
                .ok()
                .map(|service| service.status())
        );
        state["program_workspace"] = match (&self.columns, &self.workspaces) {
            (Some(columns), Some(workspaces)) => {
                let column_state = columns.snapshot().ok();
                let document = workspaces.snapshot().ok();
                let active = workspaces.active().ok();
                json!({
                    "layout": column_state.map(|s| s.layout),
                    "active_column": column_state.map(|s| s.active_index),
                    "revision": column_state.map(|s| s.revision),
                    "active_workspace_id": active.as_ref().map(|w| w.id.clone()),
                    "active_workspace": active,
                    "current_snapshot": self.surfaces
                        .value(fileatlas_plugin_workspaces::CURRENT_SNAPSHOT_SURFACE_KEY)
                        .and_then(|raw| serde_json::from_str::<fileatlas_plugin_workspaces::ProgramWorkspace>(&raw).ok()),
                    "default_workspace_id": document.as_ref().map(|d| d.default_workspace_id.clone()),
                    "workspaces": document.map(|d| d.workspaces),
                    "manager_open": self.surfaces.is_open("workspace-manager"),
                })
            }
            _ => json!(null),
        };
        state.to_string()
    }

    #[tool(
        description = "Control Settings, language and FileManager service states; hover_media injects a FileManager hover target, not an OS pointer. Requires Settings → MCP → AI control to be enabled."
    )]
    fn control_ui(&self, Parameters(request): Parameters<UiRequest>) -> String {
        tracing::info!(?request, "MCP control request");
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        let result = match request.action.as_str() {
            "columns_layout" => {
                let Some(columns) = &self.columns else {
                    return error_json("column layout plugin unavailable");
                };
                let layout = match request.value.as_deref() {
                    Some("single") => fileatlas_plugin_workspaces::ColumnLayout::Single,
                    Some("dual") => fileatlas_plugin_workspaces::ColumnLayout::Dual,
                    Some("quad") => fileatlas_plugin_workspaces::ColumnLayout::Quad,
                    _ => return error_json("columns_layout value must be single, dual, or quad"),
                };
                columns
                    .set_layout(layout)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            }
            "activate_column" => {
                let Some(columns) = &self.columns else {
                    return error_json("column layout plugin unavailable");
                };
                let Some(index) = request
                    .value
                    .as_deref()
                    .and_then(|v| v.parse::<usize>().ok())
                else {
                    return error_json("activate_column requires a zero-based numeric value");
                };
                columns
                    .activate(index)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            }
            "workspace_select" => {
                let Some(workspaces) = &self.workspaces else {
                    return error_json("program workspace plugin unavailable");
                };
                let Some(id) = request.value.as_deref() else {
                    return error_json("workspace_select requires a workspace id");
                };
                let workspace = match workspaces.activate(id) {
                    Ok(workspace) => workspace,
                    Err(error) => return error_json(&error.to_string()),
                };
                if let Some(columns) = &self.columns {
                    if let Err(error) = columns.set_layout(workspace.layout) {
                        return error_json(&error.to_string());
                    }
                }
                Ok(())
            }
            "workspace_manager" => match request.value.as_deref() {
                Some("open") => self.surfaces.set_open("workspace-manager", true),
                Some("close") => self.surfaces.set_open("workspace-manager", false),
                _ => return error_json("workspace_manager value must be open or close"),
            },
            "input_text" => {
                let text = request.value.unwrap_or_default();
                if text.chars().count() > 1000 || text.chars().any(char::is_control) {
                    return error_json("input_text requires at most 1000 printable characters");
                }
                return match self.capture.queue_key(format!("text:{text}")) {
                    Ok(()) => json!({"queued":true}).to_string(),
                    Err(error) => error_json(&error),
                };
            }
            "key_press" => {
                let key = request.value.unwrap_or_default();
                let supported = matches!(
                    key.as_str(),
                    "space"
                        | "enter"
                        | "escape"
                        | "ctrl-l"
                        | "alt-left"
                        | "alt-right"
                        | "alt-up"
                        | "ctrl-a"
                        | "backspace"
                ) || {
                    let components = key.split('-').collect::<Vec<_>>();
                    components.len() >= 2
                        && components.len() <= 6
                        && components[..components.len() - 1].iter().all(|component| {
                            ["ctrl", "alt", "shift", "win", "super", "cmd", "fn"]
                                .contains(component)
                        })
                        && components.last().is_some_and(|key| {
                            !key.is_empty()
                                && key.len() <= 16
                                && key.bytes().all(|byte| byte.is_ascii_alphanumeric())
                        })
                };
                if !supported {
                    return error_json("unsupported navigation key");
                }
                return match self.capture.queue_key(key) {
                    Ok(()) => json!({"queued":true}).to_string(),
                    Err(e) => error_json(&e),
                };
            }
            "window_select" => {
                let id = request.value.unwrap_or_else(|| "main".into());
                let key = if id == "main" {
                    "mcp.window.main".to_owned()
                } else {
                    format!("panel.{id}.hwnd")
                };
                let Some(hwnd) = self
                    .surfaces
                    .value(&key)
                    .and_then(|v| v.parse::<isize>().ok())
                else {
                    return error_json(
                        "Unknown live FileAtlas window; use main or a native_panels id",
                    );
                };
                if crate::screenshot::window_status(hwnd)["owned"] != true {
                    return error_json("Window is no longer owned by this process");
                }
                let _ = self.surfaces.set_value("mcp.window.selected", &id);
                self.capture.set(hwnd);
                return json!({"selected":id}).to_string();
            }
            "window_close" => {
                if self
                    .surfaces
                    .value("mcp.window.selected")
                    .as_deref()
                    .unwrap_or("main")
                    == "main"
                {
                    return error_json("Select a native plugin window before window_close");
                }
                let Some(hwnd) = self
                    .surfaces
                    .value("mcp.window.selected")
                    .and_then(|id| self.surfaces.value(&format!("panel.{id}.hwnd")))
                    .and_then(|value| value.parse::<isize>().ok())
                else {
                    return error_json("Selected native plugin window is no longer registered");
                };
                return match crate::screenshot::close_test_window(hwnd) {
                    Ok(()) => json!({"queued":true}).to_string(),
                    Err(error) => error_json(&error),
                };
            }
            "window_minimize" | "window_activate" => {
                return match crate::screenshot::set_test_window_active(
                    self.capture.get(),
                    request.action == "window_activate",
                ) {
                    Ok(()) => json!({"requested":true}).to_string(),
                    Err(error) => error_json(&error),
                };
            }
            "window_restore" => {
                return match crate::screenshot::restore_test_window(self.capture.get()) {
                    Ok(()) => json!({"restored":true}).to_string(),
                    Err(error) => error_json(&error),
                };
            }
            "accessibility_activate" => {
                if let Err(error) = self.full_control_guard() {
                    return error_json(&error);
                }
                if request.confirm != Some(true) {
                    return error_json("This high-impact action requires confirm=true.");
                }
                return match crate::screenshot::activate_accessibility(self.capture.get()) {
                    Ok(provider) => {
                        json!({"requested":true,"provider_response":provider}).to_string()
                    }
                    Err(error) => error_json(&error),
                };
            }
            "panel_open" | "panel_close" => {
                let Ok(bridge) = self
                    .context
                    .try_service::<fileatlas_ui_bridge::UiBridgeService>()
                else {
                    return error_json("UI Bridge unavailable");
                };
                let id = request.value.as_deref().unwrap_or("");
                return if request.action == "panel_open" {
                    match bridge.panels.open(id) {
                        Ok(instance) => json!({"instance_id":instance}).to_string(),
                        Err(error) => error_json(&error),
                    }
                } else {
                    match bridge.panels.close(id) {
                        Ok(()) => json!({"closed":id}).to_string(),
                        Err(error) => error_json(&error),
                    }
                };
            }
            "pointer_down"
            | "pointer_drag"
            | "pointer_move"
            | "pointer_scroll"
            | "pointer_right_down"
            | "pointer_right_up"
            | "pointer_up"
            | "pointer_double_down"
            | "pointer_double_up" => {
                let coordinates = request
                    .value
                    .as_deref()
                    .unwrap_or("")
                    .split(',')
                    .map(|value| value.trim().parse::<f32>())
                    .collect::<Result<Vec<_>, _>>();
                let Ok(coordinates) = coordinates else {
                    return error_json("pointer requires x,y logical client coordinates");
                };
                let scroll = request.action == "pointer_scroll";
                if coordinates.len() != if scroll { 3 } else { 2 }
                    || coordinates[..2]
                        .iter()
                        .any(|value| !value.is_finite() || !(0. ..=4096.).contains(value))
                    || (scroll && (!coordinates[2].is_finite() || coordinates[2].abs() > 4096.))
                {
                    return error_json(
                        "pointer requires bounded x,y (0–4096), and scroll delta (-4096–4096) for pointer_scroll",
                    );
                }
                self.capture.queue_pointer(crate::McpPointerInput {
                    kind: match request.action.as_str() {
                        "pointer_down" => "down",
                        "pointer_double_down" => "double_down",
                        "pointer_double_up" => "double_up",
                        "pointer_drag" => "drag",
                        "pointer_move" => "move",
                        "pointer_scroll" => "scroll",
                        "pointer_right_down" => "right_down",
                        "pointer_right_up" => "right_up",
                        _ => "up",
                    },
                    x: coordinates[0],
                    y: coordinates[1],
                    scroll_y: if scroll { coordinates[2] } else { 0. },
                })
            }
            "address_bar" => {
                let Some(payload) = request.request else {
                    return error_json("address_bar requires a request payload");
                };
                let request = match serde_json::from_value::<
                    fileatlas_plugin_navigation::AddressBarRequest,
                >(payload)
                {
                    Ok(r) => r,
                    Err(e) => return error_json(&e.to_string()),
                };
                return match self.navigation.request_ui(
                    fileatlas_plugin_navigation::NavigationUiRequest::AddressBar(request),
                ) {
                    Ok(()) => json!({"queued":true}).to_string(),
                    Err(e) => error_json(&e.to_string()),
                };
            }
            "sidebar_request" => {
                let Some(payload) = request.request else {
                    return error_json("sidebar_request requires a request payload");
                };
                let request = match serde_json::from_value::<
                    fileatlas_plugin_navigation::SidebarUiRequest,
                >(payload)
                {
                    Ok(request) => request,
                    Err(error) => return error_json(&format!("invalid sidebar request: {error}")),
                };
                if let Err(error) = self.navigation.request_ui(
                    fileatlas_plugin_navigation::NavigationUiRequest::Sidebar(request),
                ) {
                    return error_json(&error.to_string());
                }
                let mut state = serde_json::json!({"queued": true});
                state["ui_state"] = serde_json::from_str::<serde_json::Value>(&self.get_ui_state())
                    .unwrap_or_default();
                return state.to_string();
            }
            "writeback_retry" | "writeback_dismiss" => {
                let Some(path) = request.value.as_deref() else {
                    return error_json("writeback action requires a recorded failure path");
                };
                if !self
                    .tagging
                    .writeback_failures()
                    .iter()
                    .any(|failure| failure.path.to_string_lossy() == path)
                {
                    return error_json("writeback failure not found");
                }
                if request.action == "writeback_retry" {
                    if let Err(error) = self.full_control_guard() {
                        return error_json(&error);
                    }
                    if request.confirm != Some(true) {
                        return error_json("This high-impact action requires confirm=true.");
                    }
                    if let Err(error) = self.tagging.retry_writeback_failure(path) {
                        return error_json(&error.to_string());
                    }
                } else {
                    self.tagging.dismiss_writeback_failure(path);
                }
                return self.get_ui_state();
            }
            "settings_open" => self.surfaces.set_open("settings", true),
            "settings_close" => self.surfaces.set_open("settings", false),
            "export_open" => self.surfaces.set_open("metadata-export", true),
            "export_close" => self.surfaces.set_open("metadata-export", false),
            "sidebar_open" => self.surfaces.set_value("sidebar.visible", "true"),
            "sidebar_close" => self.surfaces.set_value("sidebar.visible", "false"),
            "inspector_open" => self.surfaces.set_open("inspector", true),
            "inspector_close" => self.surfaces.set_open("inspector", false),
            "sidebar_width" | "inspector_width" => {
                let Some(width) = request
                    .value
                    .as_deref()
                    .and_then(|value| value.parse::<f32>().ok())
                    .filter(|width| width.is_finite() && (100. ..=1000.).contains(width))
                else {
                    return error_json(
                        "panel width requires finite logical pixels from 100 to 1000",
                    );
                };
                self.surfaces.set_value(
                    if request.action == "sidebar_width" {
                        "shell.sidebar-width.request"
                    } else {
                        "shell.inspector-width.request"
                    },
                    &width.to_string(),
                )
            }
            "navigation_tab" => {
                let value = request.value.as_deref().unwrap_or("");
                if !["folders", "tags", "extensions"].contains(&value) {
                    return error_json("navigation_tab requires folders/tags/extensions");
                }
                self.surfaces.set_value("navigation.tab", value)
            }
            "inspector_tab" => {
                let value = request.value.as_deref().unwrap_or("");
                if !["info", "information", "history", "extensions"].contains(&value) {
                    return error_json("inspector_tab requires information/history/extensions");
                }
                self.surfaces.set_value(
                    "file_manager.inspector.tab",
                    if value == "info" {
                        "information"
                    } else {
                        value
                    },
                )
            }
            "settings_check_updates" => self.surfaces.set_open("settings.check-updates", true),
            "statistics"
            | "bulk_actions"
            | "file_manager_dialogs_close"
            | "select_media"
            | "hover_media"
            | "hover_clear"
            | "deselect" => {
                use fileatlas_plugin_file_manager::FileManagerUiRequest;
                let action = match request.action.as_str() {
                    "statistics" => FileManagerUiRequest::Statistics,
                    "bulk_actions" => FileManagerUiRequest::BulkActions,
                    "deselect" => FileManagerUiRequest::Deselect,
                    "hover_clear" => FileManagerUiRequest::Hover(None),
                    "hover_media" => {
                        let Some(id) = request
                            .value
                            .as_deref()
                            .and_then(|value| value.parse::<i64>().ok())
                        else {
                            return error_json("hover_media requires a media ID");
                        };
                        if self.file_manager.media_by_id(id).ok().flatten().is_none() {
                            return error_json("media not found");
                        }
                        FileManagerUiRequest::Hover(Some(id))
                    }
                    "select_media" => {
                        let Some(id) = request
                            .value
                            .as_deref()
                            .and_then(|value| value.parse::<i64>().ok())
                        else {
                            return error_json("select_media requires a media ID");
                        };
                        if self.file_manager.media_by_id(id).ok().flatten().is_none() {
                            return error_json("media not found");
                        }
                        FileManagerUiRequest::Select(id)
                    }
                    _ => FileManagerUiRequest::CloseDialogs,
                };
                self.file_manager.request_ui(action);
                Ok(())
            }
            "settings_tab" => {
                let value = request.value.as_deref().unwrap_or("");
                if ![
                    "appearance",
                    "shortcuts",
                    "tags",
                    "ai",
                    "updates",
                    "software",
                    "plugins",
                    "plugin",
                ]
                .contains(&value)
                {
                    return error_json("unsupported Settings tab");
                }
                self.surfaces.set_value("settings.tab", value)
            }
            "settings_plugin_page" => {
                let id = request.value.as_deref().unwrap_or("");
                let Ok(settings) = self
                    .context
                    .try_service::<fileatlas_plugin_settings::SettingsService>()
                else {
                    return error_json("settings plugin unavailable");
                };
                if !settings
                    .plugin_runtime()
                    .snapshot_now()
                    .iter()
                    .any(|plugin| plugin.id == id)
                    && !self
                        .context
                        .try_service::<fileatlas_ui_bridge::UiBridgeService>()
                        .ok()
                        .and_then(|bridge| bridge.views.snapshot().ok())
                        .unwrap_or_default()
                        .iter()
                        .any(|entry| {
                            entry.owner == id && entry.contribution.settings_page.is_some()
                        })
                {
                    return error_json("unknown plugin settings page");
                }
                self.surfaces
                    .set_value("settings.plugin_id", id)
                    .and_then(|_| self.surfaces.set_value("settings.tab", "plugin"))
            }
            "plugin_launch_mode" => {
                let Some((id, mode)) = request.value.as_deref().unwrap_or("").split_once(':')
                else {
                    return error_json(
                        "plugin_launch_mode requires plugin_id:workspace|panel|direct",
                    );
                };
                let Ok(settings) = self
                    .context
                    .try_service::<fileatlas_plugin_settings::SettingsService>()
                else {
                    return error_json("settings plugin unavailable");
                };
                let status = settings
                    .plugin_runtime()
                    .snapshot_now()
                    .into_iter()
                    .find(|plugin| plugin.id == id);
                let value = match mode {
                    "workspace" => fileatlas_plugin_settings::PluginLaunchMode::WorkspaceAndPanel,
                    "panel" => fileatlas_plugin_settings::PluginLaunchMode::PanelOnly,
                    "direct" => fileatlas_plugin_settings::PluginLaunchMode::Direct,
                    _ => return error_json("mode must be workspace, panel, or direct"),
                };
                if !status.is_some_and(|plugin| plugin.launch_modes.contains(&value)) {
                    return error_json("launch mode is not supported by this plugin");
                }
                settings
                    .update(|state| {
                        state.plugin_launch_modes.insert(id.to_owned(), value);
                    })
                    .map(|_| ())
                    .map_err(|error| error.to_string())
            }
            "plugin_enabled" => {
                let Some((id, value)) = request.value.as_deref().unwrap_or("").split_once(':')
                else {
                    return error_json("plugin_enabled requires plugin_id:on|off");
                };
                let enabled = match value {
                    "on" => true,
                    "off" => false,
                    _ => return error_json("plugin state must be on or off"),
                };
                let Ok(settings) = self
                    .context
                    .try_service::<fileatlas_plugin_settings::SettingsService>()
                else {
                    return error_json("settings plugin unavailable");
                };
                let registry = settings.plugin_runtime();
                let id = id.to_owned();
                let response_id = id.clone();
                let settings = Arc::clone(&settings);
                tokio::spawn(async move {
                    let result = registry.set_enabled(&id, enabled).await;
                    if let Err(error) = result {
                        tracing::error!(%error, plugin=%id, "MCP plugin lifecycle request failed");
                        return;
                    }
                    if let Err(error) = settings.update(|state| {
                        state.plugin_enabled.insert(id.clone(), enabled);
                    }) {
                        tracing::error!(%error, plugin=%id, "could not persist MCP plugin lifecycle state");
                    }
                });
                return json!({"queued":true,"plugin_id":response_id,"enabled":enabled})
                    .to_string();
            }
            "language" => {
                let Some(service) = &self.i18n else {
                    return error_json("i18n plugin unavailable");
                };
                fileatlas_plugin_i18n::Language::parse(request.value.as_deref().unwrap_or(""))
                    .and_then(|language| service.set_language(language))
                    .map_err(|error| error.to_string())
            }
            "region_separation" => {
                let value = request.value.as_deref().unwrap_or("");
                if ![
                    fileatlas_ui_bridge::theme::REGION_DEPTH_BORDER,
                    fileatlas_ui_bridge::theme::REGION_DEPTH_SHADOW,
                ]
                .contains(&value)
                {
                    return error_json("region_separation must be border or shadow");
                }
                let Ok(settings) = self
                    .context
                    .try_service::<fileatlas_plugin_settings::SettingsService>()
                else {
                    return error_json("settings plugin unavailable");
                };
                settings
                    .update(|state| state.region_separation = value.to_owned())
                    .map(|_| ())
                    .map_err(|error| error.to_string())
            }
            _ => return error_json("unsupported UI action"),
        };
        match result {
            Ok(()) => self.get_ui_state(),
            Err(error) => error_json(&error),
        }
    }
}
