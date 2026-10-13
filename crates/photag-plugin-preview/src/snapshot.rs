//! Preview snapshot types: the transient view state owned by the preview service.

use std::path::PathBuf;

/// Current preview item and the source transform controls.
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewSnapshot {
    pub source: Option<PathBuf>,
    pub media_kind: Option<PreviewMediaKind>,
    /// Database identity when opened by a caller that has an indexed media row.
    pub media_id: Option<i64>,
    /// Clockwise quarter-turn count, normalized to 0..=3.
    pub quarter_turns: u8,
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
    pub fit_to_window: bool,
    /// Percent zoom when fit-to-window is disabled; source presets are retained.
    pub zoom_percent: u16,
    /// Continuous zoom multiplier used by the source wheel interaction.
    pub zoom_scale: f32,
    /// Image-center offset from the viewport center in screen pixels.
    pub pan_x: f32,
    pub pan_y: f32,
    /// Source default is center-anchored wheel zoom; cursor mode preserves the point under the cursor.
    pub zoom_at_cursor: bool,
    pub show_info: bool,
    /// One-shot request from Gallery's AI Suggest Tags action.
    pub ai_request_media_id: Option<i64>,
    pub revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewMediaKind {
    Image,
    Video,
    Audio,
    Other,
}

pub fn classify_media_path(path: &std::path::Path) -> PreviewMediaKind {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if [
        "jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif", "svg", "ico", "heic", "heif",
        "avif",
    ]
    .contains(&extension.as_str())
    {
        PreviewMediaKind::Image
    } else if [
        "mp4", "mov", "avi", "mkv", "wmv", "flv", "webm", "m4v", "mpg", "mpeg", "3gp",
    ]
    .contains(&extension.as_str())
    {
        PreviewMediaKind::Video
    } else if [
        "mp3", "wav", "flac", "aac", "ogg", "wma", "m4a", "opus", "ape", "aiff", "aif",
    ]
    .contains(&extension.as_str())
    {
        PreviewMediaKind::Audio
    } else {
        PreviewMediaKind::Other
    }
}

impl Default for PreviewSnapshot {
    fn default() -> Self {
        Self {
            source: None,
            media_kind: None,
            media_id: None,
            quarter_turns: 0,
            flip_horizontal: false,
            flip_vertical: false,
            fit_to_window: true,
            zoom_percent: 100,
            zoom_scale: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            zoom_at_cursor: false,
            show_info: false,
            ai_request_media_id: None,
            revision: 0,
        }
    }
}
