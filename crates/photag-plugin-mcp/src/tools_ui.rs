//! Service-level UI validation; no OS pointer/keyboard injection.
use super::*;
use rmcp::schemars;
use serde::Deserialize;

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub(super) struct UiRequest {
    /// settings_open/close/tab, language, statistics, bulk_actions,
    /// gallery_dialogs_close, select_media, deselect, hover_media, hover_clear,
    /// inspector_open/close, sidebar_open/close, export_open/close,
    /// settings_check_updates, writeback_retry/dismiss (value: exact failure path).
    /// sidebar_width/inspector_width (logical pixels), navigation_tab, inspector_tab.
    /// region_separation: value border|shadow, saved through the Settings service.
    /// pointer_down/pointer_drag/pointer_up: value is logical client coordinates "x,y".
    /// sidebar_request: `request` is one SidebarUiRequest JSON object, e.g.
    /// {"kind":"remove_quick_access","path":"F:\\library"}.
    action: String,
    /// Tab: appearance/shortcuts/tags/ai/updates. Language: en/zh-CN.
    value: Option<String>,
    /// Structured SidebarUiRequest payload for the sidebar_request action.
    request: Option<serde_json::Value>,
}

#[tool_router(router = ui_tool_router, vis = "pub(super)")]
impl PhoTagMcpServer {
    #[tool(
        description = "Read Settings, language, region separation mode, Preview opacity, production scan progress and bounded Gallery animation frame diagnostics without changing state."
    )]
    fn get_ui_state(&self) -> String {
        let gallery = self.gallery.ui_snapshot().map(|state| {
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
            snapshot["search_scopes_open"] = json!(state.search_scopes_open);
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
            snapshot["version_states"] = json!(state.version_states.iter().map(|item| json!({"media_id":item.media_id,"path":item.path,"state":item.state,"error":item.error,"badge_visible":item.state.as_ref().is_some_and(|state| state.status == photag_plugin_versions::FileVersionStatus::Modified)})).collect::<Vec<_>>());
            snapshot
        });
        let history_visible = self.surfaces.is_open("inspector")
            && self.surfaces.value("gallery.inspector.tab").as_deref() == Some("history")
            && gallery
                .as_ref()
                .and_then(|state| state["inspector_context"].as_str())
                == Some("file");
        let mut state = json!({
            "browser": {"filesystem":self.gallery.browser_enabled(), "show_descendants":self.gallery.show_descendants(),"worker_active":self.gallery.directory_worker_active(),"status":self.gallery.directory_status()},
            "input_activity": self.capture.input_snapshot(),
            "window_status": crate::screenshot::window_status(self.capture.get()),
            "library_refresh": self.navigation.library_refresh().map(|state| json!({"running":state.running,"total_folders":state.total_folders,"completed_folders":state.completed_folders,"failures":state.failures})),
            "folder_picker": {"pending": self.navigation.folder_picker_pending(), "native_open": self.navigation.folder_picker_window().is_some()},
            "gallery_popover": self.surfaces.value("gallery.popover").unwrap_or_default(),
            "tools_open": self.surfaces.is_open("tools"),
            "gallery": gallery,
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
            "inspector_tab": self.surfaces.value("gallery.inspector.tab").unwrap_or_else(|| "information".into()),
            "inspector_history": {
                "visible": history_visible,
                "enabled": self.surfaces.value("gallery.inspector.history-enabled").as_deref() == Some("true"),
                "selected": self.surfaces.value("gallery.inspector.history-selected"),
                "active": self.surfaces.value("gallery.inspector.history-active"),
                "count": self.surfaces.value("gallery.inspector.history-count").and_then(|s| s.parse::<usize>().ok()),
                "last_created": self.surfaces.value("gallery.inspector.history-last-created"),
            },
            "sidebar_width": self.surfaces.value("shell.sidebar-width").and_then(|value| value.parse::<f32>().ok()),
            "inspector_width": self.surfaces.value("shell.inspector-width").and_then(|value| value.parse::<f32>().ok()),
            "language": self.i18n.as_ref().map(|service| service.language().code()),
            "translation_revision": photag_ui_bridge::i18n::revision(),
            "region_separation": photag_ui_bridge::theme::region_depth_name(),
            "quicklook": self.quicklook.snapshot(),
            "quicklook_notice": self.surfaces.value("quicklook.notice"),
            "gallery_cards": self.gallery.card_states().iter().map(|state| json!({
                "media_id": state.media_id, "hover": state.hover, "selection": state.selection,
            })).collect::<Vec<_>>(),
            "gallery_animation_frames": self.gallery.card_animation_frames().iter().map(|state| json!({
                "media_id": state.media_id, "hover": state.hover, "selection": state.selection, "timestamp_ms": state.timestamp_ms,
            })).collect::<Vec<_>>(),
            "scan_progress": self.gallery.scan_progress().map(|p| json!({
                "phase": p.phase, "scanned": p.scanned, "total": p.total, "error": p.error,
            })),
            "metadata_export": self.metadata_export.status().ok().map(|status| match status {
                photag_plugin_metadata_export::MetadataExportStatus::Idle => json!({"state":"idle"}),
                photag_plugin_metadata_export::MetadataExportStatus::Exporting => json!({"state":"exporting"}),
                photag_plugin_metadata_export::MetadataExportStatus::Succeeded { path, records } => json!({"state":"succeeded","path":path,"records":records}),
                photag_plugin_metadata_export::MetadataExportStatus::Failed(error) => json!({"state":"failed","error":error}),
            }),
            "writeback_failures": self.tagging.writeback_failures().iter().map(|failure| json!({"path":failure.path,"error":failure.error})).collect::<Vec<_>>(),
        });
        state["gallery_search_provider"] = json!(self.gallery.search_provider());
        state["gallery_search_provider_detail"] = json!(self.gallery.search_provider_detail());
        state.to_string()
    }

    #[tool(
        description = "Control Settings, language and Gallery service states; hover_media injects a Gallery hover target, not an OS pointer. Requires PHOTAG_MCP_ALLOW_MUTATIONS=1."
    )]
    fn control_ui(&self, Parameters(request): Parameters<UiRequest>) -> String {
        tracing::info!(?request, "MCP control request");
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        let result = match request.action.as_str() {
            "key_press" => {
                let key = request.value.unwrap_or_default();
                if !matches!(
                    key.as_str(),
                    "space" | "enter" | "escape" | "ctrl-l" | "alt-left" | "alt-right" | "alt-up"
                ) {
                    return error_json("unsupported navigation key");
                }
                return match self.capture.queue_key(key) {
                    Ok(()) => json!({"queued":true}).to_string(),
                    Err(e) => error_json(&e),
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
                    .try_service::<photag_ui_bridge::UiBridgeService>()
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
                if coordinates.len() != 2
                    || coordinates
                        .iter()
                        .any(|value| !value.is_finite() || !(0. ..=4096.).contains(value))
                {
                    return error_json("pointer coordinates must be finite and bounded 0–4096");
                }
                self.capture.queue_pointer(crate::McpPointerInput {
                    kind: match request.action.as_str() {
                        "pointer_down" => "down",
                        "pointer_double_down" => "double_down",
                        "pointer_double_up" => "double_up",
                        "pointer_drag" => "drag",
                        "pointer_move" => "move",
                        "pointer_right_down" => "right_down",
                        "pointer_right_up" => "right_up",
                        _ => "up",
                    },
                    x: coordinates[0],
                    y: coordinates[1],
                })
            }
            "address_bar" => {
                let Some(payload) = request.request else {
                    return error_json("address_bar requires a request payload");
                };
                let request = match serde_json::from_value::<
                    photag_plugin_navigation::AddressBarRequest,
                >(payload)
                {
                    Ok(r) => r,
                    Err(e) => return error_json(&e.to_string()),
                };
                return match self.navigation.request_ui(
                    photag_plugin_navigation::NavigationUiRequest::AddressBar(request),
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
                    photag_plugin_navigation::SidebarUiRequest,
                >(payload)
                {
                    Ok(request) => request,
                    Err(error) => return error_json(&format!("invalid sidebar request: {error}")),
                };
                if let Err(error) = self.navigation.request_ui(
                    photag_plugin_navigation::NavigationUiRequest::Sidebar(request),
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
                    "gallery.inspector.tab",
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
            | "gallery_dialogs_close"
            | "select_media"
            | "hover_media"
            | "hover_clear"
            | "deselect" => {
                use photag_plugin_gallery::GalleryUiRequest;
                let action = match request.action.as_str() {
                    "statistics" => GalleryUiRequest::Statistics,
                    "bulk_actions" => GalleryUiRequest::BulkActions,
                    "deselect" => GalleryUiRequest::Deselect,
                    "hover_clear" => GalleryUiRequest::Hover(None),
                    "hover_media" => {
                        let Some(id) = request
                            .value
                            .as_deref()
                            .and_then(|value| value.parse::<i64>().ok())
                        else {
                            return error_json("hover_media requires a media ID");
                        };
                        if self.gallery.media_by_id(id).ok().flatten().is_none() {
                            return error_json("media not found");
                        }
                        GalleryUiRequest::Hover(Some(id))
                    }
                    "select_media" => {
                        let Some(id) = request
                            .value
                            .as_deref()
                            .and_then(|value| value.parse::<i64>().ok())
                        else {
                            return error_json("select_media requires a media ID");
                        };
                        if self.gallery.media_by_id(id).ok().flatten().is_none() {
                            return error_json("media not found");
                        }
                        GalleryUiRequest::Select(id)
                    }
                    _ => GalleryUiRequest::CloseDialogs,
                };
                self.gallery.request_ui(action);
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
                ]
                .contains(&value)
                {
                    return error_json("unsupported Settings tab");
                }
                self.surfaces.set_value("settings.tab", value)
            }
            "language" => {
                let Some(service) = &self.i18n else {
                    return error_json("i18n plugin unavailable");
                };
                photag_plugin_i18n::Language::parse(request.value.as_deref().unwrap_or(""))
                    .and_then(|language| service.set_language(language))
                    .map_err(|error| error.to_string())
            }
            "region_separation" => {
                let value = request.value.as_deref().unwrap_or("");
                if ![
                    photag_ui_bridge::theme::REGION_DEPTH_BORDER,
                    photag_ui_bridge::theme::REGION_DEPTH_SHADOW,
                ]
                .contains(&value)
                {
                    return error_json("region_separation must be border or shadow");
                }
                let Ok(settings) = self
                    .context
                    .try_service::<photag_plugin_settings::SettingsService>()
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
