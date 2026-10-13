//! Service-level UI validation; no OS pointer/keyboard injection.
use super::*;
use rmcp::schemars;
use serde::Deserialize;

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub(super) struct UiRequest {
    /// settings_open/close/tab, language, statistics, bulk_actions,
    /// gallery_dialogs_close, select_media, deselect, hover_media, hover_clear,
    /// inspector_open, inspector_close, settings_check_updates.
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
                "sort": format!("{:?}", state.filter.sort), "extensions": state.filter.extensions,
                "excluded_extensions": state.filter.excluded_extensions, "included_tags": state.filter.selected_tag_ids,
                "excluded_tags": state.filter.excluded_tag_ids, "rating_min": state.filter.rating_min,
                "rating_max": state.filter.rating_max, "date_from": state.filter.date_from.map(|d| d.to_string()),
                "date_to": state.filter.date_to.map(|d| d.to_string()), "selected_media_ids": state.selected_media_ids,
                "matched_count": state.matched_count, "group_by_month": state.group_by_month,
                "inspector_sections": state.inspector_sections,
            })),
            "settings_open": self.surfaces.is_open("settings"),
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
                "phase": p.phase, "scanned": p.scanned, "total": p.total,
            })),
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
            "settings_open" => self.surfaces.set_open("settings", true),
            "settings_close" => self.surfaces.set_open("settings", false),
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
