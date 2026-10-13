//! GPUI media tile rendering and click selection.

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::*;
use photag_plugin_drag_drop::DragDropService;
use photag_ui_bridge::theme;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::GalleryView;

pub(super) fn render_media_card(
    media: &photag_plugin_database::db::queries::MediaItem,
    selected: bool,
    drag_drop: Arc<DragDropService>,
    drag_paths: Vec<PathBuf>,
    drag_allowed: Arc<AtomicBool>,
    thumb_size: f32,
    cx: &mut Context<GalleryView>,
) -> impl IntoElement + 'static {
    let media_id = media.id;
    let payload = FileDragPayload {
        service: drag_drop,
        paths: drag_paths,
        allowed: drag_allowed.clone(),
    };
    let mut card = div()
        .v_flex()
        .w(px(thumb_size + 32.))
        .h(px(thumb_size + 64.))
        .justify_between()
        .rounded_md()
        .border_1()
        .border_color(if selected {
            rgb(theme::accent())
        } else {
            rgb(theme::SURFACE_FAINT)
        })
        .bg(if selected {
            rgb(theme::accent_alpha(theme::accent(), 0.42))
        } else {
            rgb(theme::SURFACE_PANEL)
        })
        .p_2();
    if let Some(path) = media.thumb_path.as_ref() {
        card = card.child(
            img(std::path::PathBuf::from(path))
                .w_full()
                .h(px(thumb_size - 35.))
                .object_fit(ObjectFit::Cover)
                .with_fallback(move || {
                    div()
                        .w_full()
                        .h(px(thumb_size - 35.))
                        .bg(rgb(theme::SURFACE_PANEL))
                        .into_any_element()
                }),
        );
    } else {
        card = card.child(
            div()
                .w_full()
                .h(px(thumb_size - 35.))
                .rounded_sm()
                .bg(rgb(theme::SURFACE_PANEL))
                .items_center()
                .justify_center()
                .text_xs()
                .text_color(rgb(theme::TEXT_MUTED))
                .child(media.media_type.to_uppercase()),
        );
    }
    card.id(format!("gallery-media-{media_id}"))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |_, event: &MouseDownEvent, _, _| {
                drag_allowed.store(
                    !event.modifiers.control && !event.modifiers.shift,
                    Ordering::Release,
                );
            }),
        )
        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
            this.select_media(media_id, event.modifiers(), window, cx);
            if event.click_count() >= 2 {
                this.preview_media(media_id, cx);
            }
        }))
        .on_drag(payload, |payload, _, _, cx| {
            if payload.allowed.load(Ordering::Acquire) {
                let _ = payload.service.start_file_drag(&payload.paths);
            }
            cx.new(|_| FileDragGhost)
        })
        .child(div().text_sm().font_medium().child(media.filename.clone()))
        .child(
            div()
                .text_xs()
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child(format!("{}  ·  ★ {}", media.media_type, media.rating)),
        )
}

struct FileDragPayload {
    service: Arc<DragDropService>,
    paths: Vec<PathBuf>,
    allowed: Arc<AtomicBool>,
}

struct FileDragGhost;

impl Render for FileDragGhost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size(px(1.))
    }
}

pub(super) fn render_media_details(
    media: &photag_plugin_database::db::queries::MediaItem,
    tags: &[photag_plugin_database::db::queries::Tag],
    open_count: i64,
    show_exif: bool,
    exif_fields: Option<&Result<Vec<(String, String)>, String>>,
    cx: &mut Context<GalleryView>,
) -> impl IntoElement {
    let filesystem = std::fs::metadata(&media.full_path).ok();
    let extension = std::path::Path::new(&media.filename)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{}", value.to_lowercase()))
        .unwrap_or_else(|| "—".to_string());
    let mut details = div()
        .v_flex()
        .gap_1()
        .rounded_md()
        .bg(rgb(theme::SURFACE_PANEL))
        .p_3()
        .child(div().text_lg().font_semibold().child("File"))
        .child(div().text_sm().font_medium().child(media.filename.clone()))
        .child(detail_line("Path", &media.full_path))
        .child(detail_line("Type", &media.media_type))
        .child(detail_line("Extension", &extension))
        .child(detail_line("Size", &format_bytes(media.file_size)))
        .child(detail_line("Rating", &format!("{} / 5", media.rating)))
        .child(detail_line("Times opened", &open_count.to_string()));
    if let Some(metadata) = filesystem {
        details = details
            .child(detail_line(
                "Read-only",
                if metadata.permissions().readonly() {
                    "Yes"
                } else {
                    "No"
                },
            ))
            .child(detail_line(
                "Modified",
                &format_system_time(metadata.modified().ok()),
            ));
        if let Ok(created) = metadata.created() {
            details = details.child(detail_line("Created", &format_system_time(Some(created))));
        }
    }
    if let (Some(width), Some(height)) = (media.width, media.height) {
        let megapixels = width.saturating_mul(height) as f64 / 1_000_000.0;
        details = details
            .child(div().text_sm().font_semibold().child("Image"))
            .child(detail_line("Dimensions", &format!("{width} × {height} px")))
            .child(detail_line("Megapixels", &format!("{megapixels:.2} MP")))
            .child(detail_line(
                "Aspect",
                &aspect_ratio(width as u32, height as u32),
            ));
    }
    if let Some(timestamp) = media.date_taken {
        details = details
            .child(div().text_sm().font_semibold().child("Dates"))
            .child(detail_line("Date taken", &format_timestamp(timestamp)));
    }
    if !tags.is_empty() {
        details = details
            .child(
                div()
                    .text_sm()
                    .font_semibold()
                    .child(format!("Tags ({})", tags.len())),
            )
            .child(div().text_sm().child(format!(
                "Tags: {}",
                tags.iter()
                    .map(|tag| tag.full_path.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
    } else {
        details = details.child(
            div()
                .text_sm()
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child("No tags assigned."),
        );
    }
    if media.media_type == "image" {
        details = details
            .child(div().text_sm().font_semibold().child("Metadata (EXIF)"))
            .child(
                Button::new("gallery-toggle-exif")
                    .label(if show_exif {
                        "Hide EXIF fields"
                    } else {
                        "Show EXIF fields"
                    })
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_exif(cx))),
            );
        if show_exif {
            match exif_fields {
                Some(Ok(fields)) if fields.is_empty() => {
                    details = details.child(
                        div()
                            .text_sm()
                            .text_color(rgb(theme::TEXT_SECONDARY))
                            .child("No EXIF metadata."),
                    );
                }
                Some(Ok(fields)) => {
                    for (label, value) in fields {
                        details = details.child(detail_line(label, value));
                    }
                }
                Some(Err(message)) => {
                    details = details.child(
                        div()
                            .text_sm()
                            .text_color(rgb(theme::TEXT_SECONDARY))
                            .child(message.clone()),
                    );
                }
                None => {
                    details = details.child(
                        div()
                            .text_sm()
                            .text_color(rgb(theme::TEXT_SECONDARY))
                            .child("Select one image to read EXIF metadata."),
                    );
                }
            }
        }
    }
    details
}

pub(super) fn read_exif_fields(path: &str) -> Result<Vec<(String, String)>, &'static str> {
    let file = std::fs::File::open(path).map_err(|_| "Couldn't read file.")?;
    let mut reader = std::io::BufReader::new(file);
    let exif = exif::Reader::new()
        .read_from_container(&mut reader)
        .map_err(|_| "No EXIF metadata.")?;
    let interesting = [
        (exif::Tag::Make, "Camera make"),
        (exif::Tag::Model, "Camera model"),
        (exif::Tag::LensModel, "Lens"),
        (exif::Tag::FocalLength, "Focal length"),
        (exif::Tag::FNumber, "Aperture"),
        (exif::Tag::ExposureTime, "Shutter"),
        (exif::Tag::PhotographicSensitivity, "ISO"),
        (exif::Tag::ExposureBiasValue, "Exposure bias"),
        (exif::Tag::Flash, "Flash"),
        (exif::Tag::WhiteBalance, "White balance"),
        (exif::Tag::Orientation, "Orientation"),
        (exif::Tag::ColorSpace, "Color space"),
        (exif::Tag::Software, "Software"),
        (exif::Tag::GPSLatitude, "GPS latitude"),
        (exif::Tag::GPSLongitude, "GPS longitude"),
        (exif::Tag::GPSAltitude, "GPS altitude"),
    ];
    let mut fields = Vec::new();
    for (tag, label) in interesting {
        for ifd in [exif::In::PRIMARY, exif::In::THUMBNAIL] {
            if let Some(field) = exif.get_field(tag, ifd) {
                fields.push((
                    label.to_string(),
                    field.display_value().with_unit(&exif).to_string(),
                ));
                break;
            }
        }
    }
    for field in exif.fields() {
        fields.push((
            field.tag.to_string(),
            field.display_value().with_unit(&exif).to_string(),
        ));
    }
    Ok(fields)
}

fn detail_line(key: &str, value: &str) -> Div {
    div()
        .h_flex()
        .gap_2()
        .text_sm()
        .child(
            div()
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child(format!("{key}:")),
        )
        .child(div().flex_1().child(value.to_string()))
}

fn format_bytes(bytes: i64) -> String {
    let value = bytes as f64;
    if value < 1024.0 {
        format!("{bytes} B")
    } else if value < 1024.0_f64.powi(2) {
        format!("{:.1} KB", value / 1024.0)
    } else if value < 1024.0_f64.powi(3) {
        format!("{:.2} MB", value / 1024.0_f64.powi(2))
    } else {
        format!("{:.2} GB", value / 1024.0_f64.powi(3))
    }
}

fn format_system_time(time: Option<std::time::SystemTime>) -> String {
    time.and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| format_timestamp(duration.as_secs() as i64))
        .unwrap_or_else(|| "—".to_string())
}

fn format_timestamp(timestamp: i64) -> String {
    use chrono::TimeZone;
    chrono::Local
        .timestamp_opt(timestamp, 0)
        .single()
        .map(|date| date.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "—".to_string())
}

fn aspect_ratio(width: u32, height: u32) -> String {
    let divisor = gcd(width, height);
    if divisor == 0 {
        return "—".to_string();
    }
    let (width, height) = (width / divisor, height / divisor);
    if width > 30 || height > 30 {
        format!("{:.2}:1", width as f64 / height as f64)
    } else {
        format!("{width}:{height}")
    }
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}
