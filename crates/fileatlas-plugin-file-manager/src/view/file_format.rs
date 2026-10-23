//! File identity is independent of thumbnail decoder support.
use fileatlas_ui_bridge::components::WorkbenchForeground as _;
use fileatlas_ui_bridge::theme;
use gpui_kit::assets::IconName;
use gpui_kit::*;
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
    let (background, foreground) = theme::file_format_colors(&extension);
    fileatlas_ui_bridge::components::file_format_badge(
        div().child(if extension.is_empty() {
            "FILE".into()
        } else {
            extension.to_uppercase()
        }),
        background,
        foreground,
    )
    .absolute()
    .top(px(5.))
    .right(px(5.))
    .max_w(px((width - 10.).max(16.)))
    .overflow_hidden()
    .text_ellipsis()
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
        .workbench_foreground(fileatlas_ui_bridge::components::ForegroundRole::Muted)
        .child(theme::icon(icon).size(px(size)))
}
