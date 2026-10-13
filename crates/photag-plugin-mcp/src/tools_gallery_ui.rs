//! Plugin MCP controls exercise the live Gallery actions and its real popovers.
use super::*;
use rmcp::schemars;
use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct GalleryControl {
    /// popover/search/media_type/sort/rating/extension/tag/tag_mode/extension_mode/date_from/date_to/clear_filters/inspector_section/tag_picker/month_grouping/tools_open/tools_close.
    action: String,
    /// Popover: type/tags/extensions/date/rating/sort/close. Media type: all/photos/videos/audio. Sort uses search_media names. Dates: YYYY-MM-DD or empty.
    value: Option<String>,
    exclude: Option<bool>,
    tag_id: Option<i64>,
    rating_min: Option<i32>,
    rating_max: Option<i32>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct NavigationControl {
    /// create/edit/delete/merge/close: open the corresponding real tag dialog. Opening a dialog does not commit a tag mutation.
    action: String,
    tag_id: Option<i64>,
}

#[tool_router(router = gallery_ui_tool_router, vis = "pub(super)")]
impl PhoTagMcpServer {
    #[tool(
        description = "Open or close the live Navigation tag dialogs using the feature's public request queue. Requires PHOTAG_MCP_ALLOW_MUTATIONS=1. No tag mutation is committed by this tool."
    )]
    fn control_navigation(&self, Parameters(request): Parameters<NavigationControl>) -> String {
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        use photag_plugin_navigation::NavigationUiRequest as R;
        let action = match request.action.as_str() {
            "create" => R::OpenCreate,
            "close" => R::CloseModal,
            "edit" | "delete" | "merge" => {
                let Some(id) = request.tag_id else {
                    return error_json("tag_id required");
                };
                match request.action.as_str() {
                    "edit" => R::OpenEdit(id),
                    "delete" => R::OpenDelete(id),
                    _ => R::OpenMerge(id),
                }
            }
            _ => return error_json("unsupported navigation UI action"),
        };
        match self.navigation.request_ui(action) {
            Ok(()) => json!({"accepted": true}).to_string(),
            Err(e) => error_json(&e.to_string()),
        }
    }

    #[tool(
        description = "Control the live Gallery filters, native popovers and inspector sections through existing feature actions. Requires PHOTAG_MCP_ALLOW_MUTATIONS=1; wait for the GPUI poller and read get_ui_state/screenshot_window."
    )]
    fn control_gallery(&self, Parameters(request): Parameters<GalleryControl>) -> String {
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        use photag_plugin_gallery::{GalleryUiRequest as R, filter::MediaTypeFilter as M};
        let value = request.value.as_deref().unwrap_or("");
        let action = match request.action.as_str() {
            "popover" => {
                if ![
                    "type",
                    "tags",
                    "extensions",
                    "date",
                    "rating",
                    "sort",
                    "close",
                ]
                .contains(&value)
                {
                    return error_json("unsupported gallery popover");
                }
                return match self
                    .surfaces
                    .set_value("gallery.popover", if value == "close" { "" } else { value })
                {
                    Ok(()) => json!({"accepted": true}).to_string(),
                    Err(e) => error_json(&e),
                };
            }
            "tools_open" | "tools_close" => {
                return match self
                    .surfaces
                    .set_open("tools", request.action == "tools_open")
                {
                    Ok(()) => json!({"accepted": true}).to_string(),
                    Err(e) => error_json(&e),
                };
            }
            "search" => R::Search(value.to_string()),
            "media_type" => R::MediaType(match value {
                "all" => M::All,
                "photos" => M::Photos,
                "videos" => M::Videos,
                "audio" => M::Audio,
                _ => return error_json("unsupported media type"),
            }),
            "sort" => R::Sort(match value {
                "newest" => SortOrder::NewestFirst,
                "oldest" => SortOrder::OldestFirst,
                "largest" => SortOrder::BiggestFirst,
                "smallest" => SortOrder::SmallestFirst,
                "name_asc" => SortOrder::NameAsc,
                "name_desc" => SortOrder::NameDesc,
                "rated" => SortOrder::HighestRated,
                "unrated_first" => SortOrder::LowestRated,
                _ => return error_json("unsupported sort order"),
            }),
            "rating" => {
                let (Some(min), Some(max)) = (request.rating_min, request.rating_max) else {
                    return error_json("rating requires min/max");
                };
                if !(0..=5).contains(&min) || !(min..=5).contains(&max) {
                    return error_json("rating bounds must be ordered in 0..5");
                }
                R::RatingRange(min, max)
            }
            "extension" => {
                if value.is_empty()
                    || value.len() > 32
                    || !value.chars().all(|c| c.is_ascii_alphanumeric())
                {
                    return error_json("extension must be a bare suffix");
                }
                R::Extension(value.to_ascii_lowercase(), request.exclude.unwrap_or(false))
            }
            "tag" => {
                let Some(id) = request.tag_id else {
                    return error_json("tag_id required");
                };
                let exists = self
                    .tagging
                    .tag_tree()
                    .ok()
                    .is_some_and(|tags| contains_tag(&tags, id));
                if !exists {
                    return error_json("tag not found");
                }
                R::TagFilter(id, request.exclude.unwrap_or(false))
            }
            "tag_mode" => R::TagFilterMode(request.exclude.unwrap_or(false)),
            "extension_mode" => R::ExtensionMode(request.exclude.unwrap_or(false)),
            "date_from" | "date_to" => {
                let Some(date) = photag_plugin_gallery::filter::parse_date_input(value) else {
                    return error_json("date must be YYYY-MM-DD or empty");
                };
                R::DateBound(request.action == "date_from", date)
            }
            "clear_filters" => R::ClearFilters,
            "inspector_section" => R::InspectorSection(match value {
                "basic" => "basic",
                "tags" => "tags",
                "exif" => "exif",
                "path" => "path",
                _ => return error_json("unsupported inspector section"),
            }),
            "tag_picker" => R::TagPicker,
            "month_grouping" => R::MonthGrouping(request.enabled.unwrap_or(true)),
            _ => return error_json("unsupported gallery action"),
        };
        self.gallery.request_ui(action);
        json!({"accepted": true}).to_string()
    }
}

fn contains_tag(tags: &[photag_plugin_database::db::queries::Tag], id: i64) -> bool {
    tags.iter()
        .any(|tag| tag.id == id || contains_tag(&tag.children, id))
}
