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
    /// Optional exact media type: image, video, or audio.
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
    /// Existing local directory to register and recursively index.
    pub(super) path: String,
    /// Must be true after the caller has shown the folder path to the user.
    pub(super) confirm: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct MediaIdRequest {
    pub(super) media_id: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct PreviewControlRequest {
    /// rotate_clockwise, rotate_counter_clockwise, flip_horizontal, flip_vertical,
    /// fit, toggle_info, or zoom.
    pub(super) action: String,
    /// Used only by the zoom action; accepted presets are 25, 50, 100, 125, 150, and 200.
    pub(super) zoom_percent: Option<u16>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct PlaybackControlRequest {
    /// play_pause, seek, step, set_speed, or close.
    pub(super) action: String,
    /// Used by seek.
    pub(super) seconds: Option<f64>,
    /// Used by step; limited to -300 through 300 frames.
    pub(super) frames: Option<i32>,
    /// Used by set_speed; accepted range is 0.1 through 8.
    pub(super) speed: Option<f32>,
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
