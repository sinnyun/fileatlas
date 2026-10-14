//! Plugin MCP controls exercise the live Gallery actions and its real popovers.
use super::*;
use rmcp::schemars;
use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct GalleryControl {
    /// popover/search/media_type/sort/rating/extension/tag/tag_mode/extension_mode/date_from/date_to/clear_filters/inspector_section/tag_picker/month_grouping/tools_open/tools_close/bulk_open/bulk_close/bulk_configure/open_media_menu/close_media_menu/select_many/scroll.
    action: String,
    /// Popover: type/tags/extensions/date/rating/sort/close. Media type: all/photos/videos/audio. Sort uses search_media names. Dates: YYYY-MM-DD or empty. Bulk action: delete/copy/move. open_media_menu requires media_id. Scroll value is a zero-based result index.
    value: Option<String>,
    /// Bulk source: all/folder/tag/selection.
    bulk_source: Option<String>,
    exclude: Option<bool>,
    tag_id: Option<i64>,
    media_id: Option<i64>,
    /// select_many: 1–1000 unique indexed IDs, including records outside the loaded page.
    media_ids: Option<Vec<i64>>,
    rating_min: Option<i32>,
    rating_max: Option<i32>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct NavigationControl {
    /// all, expand_folder, expand_path, select_path, folder_search, or create/edit/delete/merge/move/close.
    action: String,
    tag_id: Option<i64>,
    folder_id: Option<i64>,
    /// Existing directory for select_path, or a search term for folder_search.
    value: Option<String>,
}

#[tool_router(router = gallery_ui_tool_router, vis = "pub(super)")]
impl PhoTagMcpServer {
    #[tool(
        description = "Open or close the live Navigation tag dialogs using the feature's public request queue. Actions: all, create, edit, delete, merge, move, close. Requires PHOTAG_MCP_ALLOW_MUTATIONS=1. No tag mutation is committed by this tool."
    )]
    fn control_navigation(&self, Parameters(request): Parameters<NavigationControl>) -> String {
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        use photag_plugin_navigation::NavigationUiRequest as R;
        let action =
            match request.action.as_str() {
                "folder_search" => {
                    return match self.surfaces.set_value(
                        "navigation.folder-search",
                        request.value.as_deref().unwrap_or(""),
                    ) {
                        Ok(()) => json!({"accepted": true}).to_string(),
                        Err(error) => error_json(&error),
                    };
                }
                "expand_folder" => {
                    let Some(id) = request.folder_id.filter(|id| *id > 0) else {
                        return error_json("expand_folder requires positive folder_id");
                    };
                    if !self.navigation.snapshot().is_ok_and(|snapshot| {
                        snapshot.folders.iter().any(|node| node.folder.id == id)
                    }) {
                        return error_json("folder not found");
                    }
                    R::ExpandFolder(id)
                }
                "select_path" | "expand_path" => {
                    let Some(path) = request.value.filter(|value| !value.is_empty()) else {
                        return error_json("select_path requires directory value");
                    };
                    if !std::path::Path::new(&path).is_dir() {
                        return error_json("directory does not exist");
                    }
                    if request.action == "expand_path" {
                        R::ExpandPath(path)
                    } else {
                        R::SelectPath(path)
                    }
                }
                "all" => R::AllFiles,
                "create" => R::OpenCreate,
                "close" => R::CloseModal,
                "edit" | "delete" | "merge" | "move" => {
                    let Some(id) = request.tag_id else {
                        return error_json("tag_id required");
                    };
                    match request.action.as_str() {
                        "edit" => R::OpenEdit(id),
                        "delete" => R::OpenDelete(id),
                        "merge" => R::OpenMerge(id),
                        _ => R::OpenMove(id),
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
        description = "Control live Gallery filters, popovers, bulk configuration and the Gallery-owned media action surface through feature requests. Actions open_media_menu/close_media_menu control the visible action surface; pass media_id to open. Requires PHOTAG_MCP_ALLOW_MUTATIONS=1; wait for the GPUI poller and read get_ui_state/screenshot_window."
    )]
    fn control_gallery(&self, Parameters(request): Parameters<GalleryControl>) -> String {
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        use photag_plugin_gallery::{
            BulkActionKind as A, BulkActionSource as S, GalleryUiRequest as R,
            filter::MediaTypeFilter as M,
        };
        let value = request.value.as_deref().unwrap_or("");
        let action = match request.action.as_str() {
            "layout" => match value {
                "grid" => R::Layout(false),
                "list" => R::Layout(true),
                _ => return error_json("layout requires grid/list"),
            },
            "scroll" => {
                let Ok(index) = value.parse::<usize>() else {
                    return error_json("scroll requires a zero-based index in value");
                };
                let Some(snapshot) = self.gallery.ui_snapshot() else {
                    return error_json("Gallery is not mounted");
                };
                if index >= snapshot.matched_count {
                    return error_json("scroll index is outside the current results");
                }
                R::ScrollTo(index)
            }
            "select_many" => {
                let Some(ids) = request.media_ids else {
                    return error_json("select_many requires media_ids");
                };
                if ids.is_empty() || ids.len() > 1000 {
                    return error_json("select_many requires 1–1000 indexed IDs");
                }
                let unique = ids
                    .iter()
                    .copied()
                    .collect::<std::collections::HashSet<_>>();
                if unique.len() != ids.len()
                    || ids.iter().any(|id| {
                        *id <= 0 || self.gallery.media_by_id(*id).ok().flatten().is_none()
                    })
                {
                    return error_json("select_many requires unique existing positive media IDs");
                }
                R::SelectMany(ids)
            }

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
            "bulk_open" => R::BulkActions,
            "bulk_close" => R::CloseDialogs,
            "bulk_configure" => {
                let action = match value {
                    "delete" => A::Delete,
                    "copy" => A::CopyTo,
                    "move" => A::MoveTo,
                    _ => return error_json("bulk_configure action must be delete, copy, or move"),
                };
                let source = match request.bulk_source.as_deref().unwrap_or("all") {
                    "all" => S::WholeLibrary,
                    "folder" => S::SelectedFolder,
                    "tag" => S::SelectedTag,
                    "selection" => S::CurrentSelection,
                    _ => return error_json("bulk_source must be all, folder, tag, or selection"),
                };
                R::ConfigureBulk(action, source)
            }
            "open_media_menu" => {
                let Some(id) = request.media_id else {
                    return error_json("media_id required");
                };
                return match self.gallery.open_media_actions(id) {
                    Ok(()) => json!({"accepted":true,"media_id":id}).to_string(),
                    Err(error) => error_json(&error),
                };
            }
            "close_media_menu" => R::CloseMediaActions,
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
