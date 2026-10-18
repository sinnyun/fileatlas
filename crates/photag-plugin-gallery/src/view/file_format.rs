//! File identity is independent of thumbnail decoder support.
use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt as _;
use gpui_kit::*;
use photag_ui_bridge::theme;
use std::path::Path;

pub(super) fn extension(filename: &str) -> String {
    Path::new(filename)
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase()
}

pub(super) fn badge(filename: &str, width: f32) -> Div {
    let extension = extension(filename);
    // A stable per-format hue, saturation and lightness also covers future formats.
    let hash = extension.bytes().fold(2166136261u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(16777619)
    });
    let color = hsla(
        (hash % 360) as f32 / 360.,
        0.42 + ((hash >> 9) % 20) as f32 / 100.,
        0.26 + ((hash >> 17) % 8) as f32 / 100.,
        1.,
    );
    div()
        .absolute()
        .top(px(5.))
        .right(px(5.))
        .max_w(px((width - 10.).max(16.)))
        .px(px(5.))
        .py(px(2.))
        .rounded(px(3.))
        .bg(color)
        .text_color(rgb(theme::TEXT_ON_ACCENT))
        .text_size(px(9.))
        .font_semibold()
        .overflow_hidden()
        .text_ellipsis()
        .child(if extension.is_empty() {
            "FILE".into()
        } else {
            extension.to_uppercase()
        })
}

pub(super) fn placeholder(filename: &str, media_type: &str, size: f32) -> Div {
    let extension = extension(filename);
    let icon = match media_type {
        "image" => IconName::FileImage,
        "video" => IconName::FileVideoCamera,
        "audio" => IconName::FileMusic,
        _ => match extension.as_str() {
            "psd" | "psb" | "ai" | "eps" | "raw" | "cr2" | "cr3" | "nef" | "arw" | "dng" => {
                IconName::FileImage
            }
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" => IconName::FileArchive,
            "rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "html" | "css" | "json" | "xml" | "xmp"
            | "yaml" | "yml" | "toml" | "c" | "cpp" | "h" | "java" => IconName::FileCode,
            "txt" | "md" | "pdf" | "doc" | "docx" | "xls" | "xlsx" | "csv" | "ppt" | "pptx"
            | "rtf" => IconName::FileText,
            "ttf" | "otf" | "woff" | "woff2" => IconName::FileType,
            _ => IconName::File,
        },
    };
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(size))
        .text_color(rgb(theme::TEXT_MUTED))
        .child(icon)
}
