//! Plugin MCP controls exercise the live file-management actions and its real popovers.
use super::*;
use rmcp::schemars;
use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct FileManagerControl {
    /// popover/search/search_scope/search_scopes_open/media_type/sort/rating/extension/tag/tag_mode/extension_mode/date_from/date_to/clear_filters/inspector_section/tag_picker/month_grouping/grouping/toggle_group/open_directory/directory_up/refresh/tools_open/tools_close/bulk_open/bulk_close/bulk_configure/open_media_menu/close_media_menu/select_many/move_files_to_subfolder/scroll/preview.
    action: String,
    /// Popover: type/tags/extensions/date/rating/sort/close. Search scopes: filenames/paths/tags/time. Media type: all/photos/videos/audio. Sort uses search_media names. Dates: YYYY-MM-DD or empty. Bulk action: delete/copy/move. open_media_menu requires media_id. Scroll value is a zero-based result index.
    value: Option<String>,
    /// Bulk source: all/folder/tag/selection.
    bulk_source: Option<String>,
    exclude: Option<bool>,
    tag_id: Option<i64>,
    media_id: Option<i64>,
    /// select_many: 1–1000 unique indexed IDs; move_files_to_subfolder: 1–100 existing IDs.
    media_ids: Option<Vec<i64>>,
    /// move_files_to_subfolder: existing destination directory and unique indexed IDs.
    destination: Option<String>,
    rating_min: Option<i32>,
    rating_max: Option<i32>,
    enabled: Option<bool>,
    /// bulk_field only: input index 0..7 (older/newer days, min/max rating, name,
    /// extensions, min/max MB); text is carried in value.
    field_index: Option<usize>,
    /// Explicitly acknowledge the high-impact delete-missing-records action.
    confirm: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct NavigationControl {
    /// all, expand_folder, expand_path, select_path, folder_search, or create/edit/delete/merge/move/close.
    action: String,
    tag_id: Option<i64>,
    folder_id: Option<i64>,
    /// Existing directory for select_path, or a search term for folder_search.
    value: Option<String>,
    /// tag_appearance sets both color and icon; omitted values clear them.
    color: Option<String>,
    icon: Option<String>,
}

#[tool_router(router = file_manager_ui_tool_router, vis = "pub(super)")]
impl FileAtlasMcpServer {
    #[tool(
        description = "Control live Navigation dialogs and selection through public requests: all, create, edit, delete, merge, move, close, select_path, expand_path, expand_folder. tag_appearance directly sets both color and icon on an existing tag through NavigationService; omitted values clear the field. Requires Settings → MCP → AI control to be enabled."
    )]
    fn control_navigation(&self, Parameters(request): Parameters<NavigationControl>) -> String {
        tracing::info!(?request, "MCP control request");
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        use fileatlas_plugin_navigation::NavigationUiRequest as R;
        let action =
            match request.action.as_str() {
                "tag_appearance" => {
                    let Some(id) = request.tag_id.filter(|id| *id > 0) else {
                        return error_json("positive tag_id required");
                    };
                    if !self
                        .tagging
                        .tag_tree()
                        .ok()
                        .is_some_and(|tags| contains_tag(&tags, id))
                    {
                        return error_json("tag not found");
                    }
                    if request
                        .color
                        .as_deref()
                        .is_some_and(|c| fileatlas_ui_bridge::theme::parse_hex_color(c).is_none())
                    {
                        return error_json("invalid tag color");
                    }
                    return match self.navigation.set_tag_appearance(
                        id,
                        request.color.as_deref(),
                        request.icon.as_deref(),
                    ) {
                        Ok(()) => json!({"accepted": true}).to_string(),
                        Err(error) => error_json(&error.to_string()),
                    };
                }
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
                "folder_picker_cancel" => {
                    return match self.navigation.cancel_folder_picker() {
                        Ok(()) => json!({"accepted":true}).to_string(),
                        Err(error) => error_json(&error),
                    };
                }
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
        description = "Control file-management filters, popovers, bulk configuration, preview opening and file actions through feature requests. move_files_to_subfolder exercises the same validated move used by dropping selected media onto a subfolder group header; pass unique indexed media_ids and an existing destination. Directory browser: open_directory with a displayed folder path, directory_up, refresh. Grouping: action grouping with value none/month/file_type/modified/created/folder/subfolder; toggle_group with exact group label; popover value grouping. Requires Settings → MCP → AI control to be enabled; wait for the GPUI poller and read get_ui_state/screenshot_window."
    )]
    fn control_file_manager(&self, Parameters(request): Parameters<FileManagerControl>) -> String {
        tracing::info!(?request, "MCP control request");
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        if request.action == "delete_missing_records" {
            if let Err(error) = self.full_control_guard() {
                return error_json(&error);
            }
            if request.confirm != Some(true) {
                return error_json("This high-impact action requires confirm=true.");
            }
        }
        use fileatlas_plugin_file_manager::{
            BulkActionKind as A, BulkActionSource as S, FileManagerUiRequest as R,
            MediaTypeFilter as M,
        };
        let value = request.value.as_deref().unwrap_or("");
        let action = match request.action.as_str() {
            "debug_internal_file_drag" => {
                if std::env::var_os("FILEATLAS_DEV_TEST_DB").is_none() {
                    return error_json("internal file drag requires an isolated dev-test profile");
                }
                R::DebugInternalFileDrag(request.enabled.unwrap_or(false))
            }
            "inspector_tag_name" => {
                if value.trim().is_empty() || value.len() > 1024 {
                    return error_json("tag name must contain 1–1024 bytes");
                }
                R::InspectorTagName(value.to_string())
            }
            "layout" => match value {
                "grid" => R::Layout(false),
                "list" => R::Layout(true),
                _ => return error_json("layout requires grid/list"),
            },
            "scroll" => {
                let Ok(index) = value.parse::<usize>() else {
                    return error_json("scroll requires a zero-based index in value");
                };
                let Some(snapshot) = self.file_manager.ui_snapshot() else {
                    return error_json("File management window is not mounted");
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
                        *id <= 0 || self.file_manager.media_by_id(*id).ok().flatten().is_none()
                    })
                {
                    return error_json("select_many requires unique existing positive media IDs");
                }
                R::SelectMany(ids)
            }
            "move_files_to_subfolder" => {
                let Some(ids) = request.media_ids else {
                    return error_json("move_files_to_subfolder requires media_ids");
                };
                let Some(destination) = request.destination.filter(|path| !path.trim().is_empty())
                else {
                    return error_json("move_files_to_subfolder requires destination");
                };
                if ids.is_empty()
                    || ids.len() > 100
                    || ids.iter().any(|id| *id <= 0)
                    || ids
                        .iter()
                        .copied()
                        .collect::<std::collections::HashSet<_>>()
                        .len()
                        != ids.len()
                    || ids
                        .iter()
                        .any(|id| self.file_manager.media_by_id(*id).ok().flatten().is_none())
                {
                    return error_json("media_ids must be 1–100 unique indexed IDs");
                }
                if !std::path::Path::new(&destination).is_dir() {
                    return error_json("destination must be an existing directory");
                }
                let Some(snapshot) = self.file_manager.ui_snapshot() else {
                    return error_json("File management window is not mounted");
                };
                if snapshot.grouping_mode.key() != "subfolder"
                    || !snapshot.groups.iter().any(|(path, _)| path == &destination)
                {
                    return error_json("destination is not a visible subfolder group");
                }
                R::MoveFilesToSubfolder(ids, destination)
            }

            "popover" => {
                if ![
                    "type",
                    "tags",
                    "extensions",
                    "date",
                    "rating",
                    "sort",
                    "grouping",
                    "close",
                ]
                .contains(&value)
                {
                    return error_json("unsupported file_manager popover");
                }
                return match self.surfaces.set_value(
                    "file_manager.popover",
                    if value == "close" { "" } else { value },
                ) {
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
            "bulk_field" => {
                let Some(index) = request.field_index.filter(|index| *index < 8) else {
                    return error_json("bulk_field requires field_index from 0 to 7");
                };
                R::BulkField(index, value.to_owned())
            }
            "bulk_close" | "close_dialogs" => R::CloseDialogs,
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
                return match self.file_manager.open_media_actions(id) {
                    Ok(()) => json!({"accepted":true,"media_id":id}).to_string(),
                    Err(error) => error_json(&error),
                };
            }
            "close_media_menu" => R::CloseMediaActions,
            "preview" => {
                let Some(id) = request.media_id.filter(|id| *id > 0) else {
                    return error_json("preview requires a positive media_id");
                };
                if self.file_manager.media_by_id(id).ok().flatten().is_none() {
                    return error_json("preview media is not indexed");
                }
                R::Preview(id)
            }
            "search" => R::Search(value.to_string()),
            "search_scopes_open" => R::SearchScopesOpen(request.enabled.unwrap_or(true)),
            "search_entire_computer" => R::SearchEntireComputer(request.enabled.unwrap_or(true)),
            "everything_recheck" => R::EverythingRecheck,
            "search_scope" => R::SearchScope(match value {
                "filenames" => fileatlas_plugin_file_manager::SearchScope::Filenames,
                "paths" => fileatlas_plugin_file_manager::SearchScope::Paths,
                "tags" => fileatlas_plugin_file_manager::SearchScope::Tags,
                "time" => fileatlas_plugin_file_manager::SearchScope::Time,
                _ => return error_json("search_scope must be filenames, paths, tags, or time"),
            }),
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
                let Some(date) = fileatlas_plugin_file_manager::parse_date_input(value) else {
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
            "directory_up" => {
                if !self
                    .file_manager
                    .ui_snapshot()
                    .is_some_and(|s| s.browsing && s.current_directory.is_some())
                {
                    return error_json("no parent in the current view");
                }
                R::DirectoryUp
            }
            "record_candidate" => match request.media_id {
                Some(id) => R::RecordCandidate(id, value.to_owned()),
                None => return error_json("media_id required"),
            },
            "relink_record" => match request.media_id {
                Some(id) => R::RelinkRecord(id),
                None => return error_json("media_id required"),
            },
            "delete_missing_records" => R::DeleteMissingRecords,
            "missing_records" => R::MissingRecords,
            "refresh" => R::Refresh,
            "cancel_directory" => R::CancelDirectory,
            "thumbnails" => R::Thumbnails(request.enabled.unwrap_or(true)),
            "show_descendants" => R::ShowDescendants(request.enabled.unwrap_or(false)),
            "locate_file" => match request.media_id {
                Some(id) => R::LocateFile(id),
                None => return error_json("media_id required"),
            },
            "navigate" => R::OpenDirectory(value.to_string()),
            "open_directory" => {
                if !self
                    .file_manager
                    .ui_snapshot()
                    .is_some_and(|s| s.browsing && s.directories.iter().any(|f| f.path == value))
                {
                    return error_json("directory must be a visible folder card");
                }
                R::OpenDirectory(value.to_string())
            }
            "grouping" => {
                let Some(mode) = fileatlas_plugin_file_manager::GroupingMode::parse(value) else {
                    return error_json("unsupported grouping mode");
                };
                R::Grouping(mode)
            }
            "toggle_group" => R::ToggleGroup(value.to_string()),
            "month_grouping" => R::MonthGrouping(request.enabled.unwrap_or(true)),
            _ => return error_json("unsupported file_manager action"),
        };
        self.file_manager.request_ui(action);
        json!({"accepted": true}).to_string()
    }
}

fn contains_tag(tags: &[fileatlas_plugin_database::Tag], id: i64) -> bool {
    tags.iter()
        .any(|tag| tag.id == id || contains_tag(&tag.children, id))
}
