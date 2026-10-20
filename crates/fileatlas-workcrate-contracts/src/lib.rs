//! Shared WorkCrate data and event contracts.

use cordis::Event;
use serde::{Deserialize, Serialize};

pub type WorkCrateProjectId = String;
pub type WorkCrateAssetId = String;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkCrateProject {
    pub id: WorkCrateProjectId,
    pub name: String,
    pub created_at: u64,
    pub archived: bool,
    #[serde(default)]
    pub main_image: Option<WorkCratePreview>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkCratePreview {
    /// Path relative to the owning project's directory.
    pub path: String,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkCrateFileReference {
    pub id: WorkCrateAssetId,
    pub path: String,
    pub display_title: String,
    pub order: u32,
    pub size_bytes: u64,
    pub modified_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_preview: Option<WorkCratePreview>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasImageFormat {
    Jpeg,
    Png,
}

/// The first board baseline only places JPEG/JPG and PNG source images.
pub fn canvas_image_format(path: &std::path::Path) -> Option<CanvasImageFormat> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => Some(CanvasImageFormat::Jpeg),
        "png" => Some(CanvasImageFormat::Png),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkCrateAddReferencesRequest {
    pub project_id: WorkCrateProjectId,
    pub paths: Vec<String>,
}

pub struct WorkCrateAddReferencesRequested;

impl Event for WorkCrateAddReferencesRequested {
    const NAME: &'static str = "fileatlas/workcrate/add-references-requested";
    type Args = WorkCrateAddReferencesRequest;
    type Output = ();
}

pub struct WorkCrateProjectsChanged;

impl Event for WorkCrateProjectsChanged {
    const NAME: &'static str = "fileatlas/workcrate/projects-changed";
    type Args = ();
    type Output = ();
}

pub struct WorkCrateProjectChanged;

impl Event for WorkCrateProjectChanged {
    const NAME: &'static str = "fileatlas/workcrate/project-changed";
    type Args = WorkCrateProjectId;
    type Output = ();
}

#[cfg(test)]
mod tests {
    use super::{CanvasImageFormat, canvas_image_format};
    use std::path::Path;

    #[test]
    fn board_accepts_only_jpeg_and_png_extensions_case_insensitively() {
        assert_eq!(
            canvas_image_format(Path::new("sample.jpg")),
            Some(CanvasImageFormat::Jpeg)
        );
        assert_eq!(
            canvas_image_format(Path::new("sample.JPEG")),
            Some(CanvasImageFormat::Jpeg)
        );
        assert_eq!(
            canvas_image_format(Path::new("sample.PNG")),
            Some(CanvasImageFormat::Png)
        );
        assert_eq!(canvas_image_format(Path::new("sample.psd")), None);
        assert_eq!(canvas_image_format(Path::new("sample.pdf")), None);
    }
}
