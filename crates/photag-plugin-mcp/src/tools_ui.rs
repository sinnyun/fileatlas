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
    action: String,
    /// Tab: appearance/toolbar/shortcuts/tags/ai/updates. Language: en/zh-CN.
    value: Option<String>,
}

#[tool_router(router = ui_tool_router, vis = "pub(super)")]
impl PhoTagMcpServer {
    #[tool(
        description = "Read Settings, language, Preview opacity, production scan progress and bounded Gallery animation frame diagnostics without changing state."
    )]
    fn get_ui_state(&self) -> String {
        json!({
            "gallery_popover": self.surfaces.value("gallery.popover").unwrap_or_default(),
            "tools_open": self.surfaces.is_open("tools"),
            "gallery": self.gallery.ui_snapshot().map(|state| json!({
                "search": state.filter.search, "media_type": format!("{:?}", state.filter.media_type),
                "folder_id": state.filter.folder_id, "path_prefix": state.filter.path_prefix,
                "sort": format!("{:?}", state.filter.sort), "extensions": state.filter.extensions,
                "excluded_extensions": state.filter.excluded_extensions, "included_tags": state.filter.selected_tag_ids,
                "excluded_tags": state.filter.excluded_tag_ids, "rating_min": state.filter.rating_min,
                "rating_max": state.filter.rating_max, "date_from": state.filter.date_from.map(|d| d.to_string()),
                "date_to": state.filter.date_to.map(|d| d.to_string()), "selected_media_ids": state.selected_media_ids,
                "matched_count": state.matched_count, "group_by_month": state.group_by_month,
                "window_offset": state.window_offset, "loaded_count": state.loaded_count,
                "inspector_sections": state.inspector_sections,
                "bulk_action": state.bulk_action, "bulk_source": state.bulk_source,
                "statistics_open": state.statistics_open, "tag_picker_open": state.tag_picker_open,
                "media_actions_open": state.media_actions_open, "media_actions_id": state.media_actions_id,
            })),
            "settings_open": self.surfaces.is_open("settings"),
            "export_open": self.surfaces.is_open("metadata-export"),
            "sidebar_open": self.surfaces.value("sidebar.visible").as_deref() != Some("false"),
            "inspector_open": self.surfaces.is_open("inspector"),
            "update_checking": self.surfaces.is_open("settings.update-checking"),
            "update_available": self.surfaces.is_open("settings.update-available"),
            "update_offer_open": self.surfaces.is_open("settings.update-offer"),
            "settings_tab": self.surfaces.value("settings.tab").unwrap_or_else(|| "toolbar".into()),
            "language": self.i18n.as_ref().map(|service| service.language().code()),
            "translation_revision": photag_ui_bridge::i18n::revision(),
            "preview_chrome_alpha": self.preview.chrome_alpha(),
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
            "writeback_failures": photag_plugin_tagging::writeback::failures().iter().map(|failure| json!({"path":failure.path,"error":failure.error})).collect::<Vec<_>>(),
        })
        .to_string()
    }

    #[tool(
        description = "Control Settings, language and Gallery service states; hover_media injects a Gallery hover target, not an OS pointer. Requires PHOTAG_MCP_ALLOW_MUTATIONS=1."
    )]
    fn control_ui(&self, Parameters(request): Parameters<UiRequest>) -> String {
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        let result = match request.action.as_str() {
            "writeback_retry" | "writeback_dismiss" => {
                let Some(path) = request.value.as_deref() else {
                    return error_json("writeback action requires a recorded failure path");
                };
                if !photag_plugin_tagging::writeback::failures()
                    .iter()
                    .any(|failure| failure.path.to_string_lossy() == path)
                {
                    return error_json("writeback failure not found");
                }
                if request.action == "writeback_retry" {
                    if let Err(error) = photag_plugin_tagging::writeback::retry_failure(path) {
                        return error_json(&error.to_string());
                    }
                } else {
                    photag_plugin_tagging::writeback::clear_failure(path);
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
                    "toolbar",
                    "shortcuts",
                    "tags",
                    "ai",
                    "updates",
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
            _ => return error_json("unsupported UI action"),
        };
        match result {
            Ok(()) => self.get_ui_state(),
            Err(error) => error_json(&error),
        }
    }
}
