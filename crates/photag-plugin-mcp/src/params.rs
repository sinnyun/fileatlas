//! Typed tool input contracts exposed in MCP input schemas.

use rmcp::schemars;
use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct SearchMediaRequest {
    /// Case-insensitive phrase matched against filename, path, and tags.
    pub(super) search: Option<String>,
    /// Optional registered-folder ID.
    pub(super) folder_id: Option<i64>,
    /// Optional tag IDs; media with any listed tag is included.
    pub(super) tag_ids: Option<Vec<i64>>,
    /// Optional exact media type: image, video, audio, or file.
    pub(super) media_type: Option<String>,
    /// Result offset, from 0 through 1,000,000.
    pub(super) offset: Option<usize>,
    /// Result count, from 1 through 100.
    pub(super) limit: Option<usize>,
    /// Sort: newest, oldest, largest, smallest, name_asc, name_desc, rated, unrated_first.
    pub(super) sort: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct DevTestInjectRequest {
    /// Only 1000, 10000, or 100000 are accepted.
    pub(super) count: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct MediaIdsRequest {
    /// Media IDs, limited to 100 per call.
    pub(super) media_ids: Vec<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct RatingRequest {
    /// Media IDs, limited to 100 per call.
    pub(super) media_ids: Vec<i64>,
    /// Rating from 0 (unrated) through 5.
    pub(super) rating: u8,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct TagMutationRequest {
    /// Media IDs, limited to 100 per call.
    pub(super) media_ids: Vec<i64>,
    /// Tag IDs, limited to 50 per call.
    pub(super) tag_ids: Vec<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct RenameRequest {
    pub(super) media_id: i64,
    /// New filename, not a path. Existing file-operation validation applies.
    pub(super) new_filename: String,
    /// Must be true after the caller has shown the proposed name to the user.
    pub(super) confirm: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct MoveRequest {
    /// Media IDs, limited to 100 per call.
    pub(super) media_ids: Vec<i64>,
    /// Destination folder. PhoTag's registered-library move behavior applies.
    pub(super) destination_folder: String,
    /// Must be true after the caller has shown the destination and affected media to the user.
    pub(super) confirm: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct CopyRequest {
    /// Media IDs, limited to 100 per call.
    pub(super) media_ids: Vec<i64>,
    /// Destination folder.
    pub(super) destination_folder: String,
    /// Must be true after the caller has shown the destination and affected media to the user.
    pub(super) confirm: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct DeleteRequest {
    /// Media IDs, limited to 100 per call. Items go to the OS Recycle Bin.
    pub(super) media_ids: Vec<i64>,
    /// Must be true after the caller has shown the affected media to the user.
    pub(super) confirm: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct AddFolderRequest {
    /// Existing local directory to bookmark. Browsing does not recursively index.
    pub(super) path: String,
    /// Must be true after the caller has shown the folder path to the user.
    pub(super) confirm: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct MediaIdRequest {
    pub(super) media_id: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct CreateVersionRequest {
    pub(super) media_id: i64,
    pub(super) title: String,
    /// Must be true after the title and target file have been presented to the user.
    pub(super) confirm: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct ActivateVersionRequest {
    pub(super) media_id: i64,
    pub(super) version_id: String,
    /// Must be true after the target historical revision has been presented to the user.
    pub(super) confirm: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct BranchGraphRequest {
    pub(super) media_id: i64,
    #[serde(default)]
    pub(super) include_archived: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct ManageFileBranchRequest {
    pub(super) media_id: i64,
    /// create, switch, archive, merge, resolve, or abort
    pub(super) action: String,
    pub(super) name: Option<String>,
    pub(super) source_branch: Option<String>,
    pub(super) strategy: Option<String>,
    #[serde(default)]
    pub(super) overwrite: bool,
    /// Must be true after the caller has presented the branch and file operation.
    pub(super) confirm: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct CreateTagRequest {
    pub(super) name: String,
    pub(super) parent_id: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct RenameTagRequest {
    pub(super) tag_id: i64,
    pub(super) name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct DeleteTagRequest {
    pub(super) tag_id: i64,
    /// Must be true to acknowledge removal of this tag and its associations.
    pub(super) confirm: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct FolderRequest {
    pub(super) folder_id: i64,
    /// Must be true to acknowledge the indexed folder will be removed from PhoTag.
    pub(super) confirm: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct FolderIdRequest {
    pub(super) folder_id: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct ExportRequest {
    /// Destination JSON file path.
    pub(super) path: String,
    /// Must be true after the caller has shown the destination path to the user.
    pub(super) confirm: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct WindowSizeRequest {
    /// Requested outer window width, in physical pixels.
    pub(super) width: u32,
    /// Requested outer window height, in physical pixels.
    pub(super) height: u32,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub(super) struct TagRecordRequest {
    pub action: String,
    pub record_id: Option<i64>,
    pub path: Option<String>,
    pub record_ids: Option<Vec<i64>>,
    pub confirm: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct FavoriteRequest {
    pub(super) action: String,
    pub(super) folder_id: i64,
    pub(super) name: Option<String>,
    pub(super) delta: Option<isize>,
}
