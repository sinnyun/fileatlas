//! GPUI media tile rendering and click selection.

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::menu::ContextMenuExt;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use photag_contracts::TagDragPayload;
use photag_plugin_drag_drop::DragDropService;
use photag_ui_bridge::theme;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::GalleryView;

/// Fixed caption block under the image: filename line plus metadata line.
/// Rows reserve this height so uniform_list keeps a uniform item size.
pub(super) const CARD_CAPTION_HEIGHT: f32 = 38.;

pub(super) fn render_media_card(
    media: &photag_plugin_database::MediaItem,
    hover: f32,
    selection: f32,
    keyboard_focus: bool,
    drag_drop: Arc<DragDropService>,
    drag_paths: Vec<PathBuf>,
    drag_allowed: Arc<AtomicBool>,
    thumb_size: f32,
    list_width: Option<f32>,
    micro_thumb: Option<&[u8]>,
    version_state: Option<&photag_plugin_versions::FileVersionState>,
    rating_target_ids: Vec<i64>,
    media_tags: &[photag_plugin_database::Tag],
    tag_tree: Arc<Vec<photag_plugin_database::Tag>>,
    cx: &mut Context<GalleryView>,
) -> impl IntoElement + 'static {
    let media_id = media.id;
    let payload = FileDragPayload {
        service: drag_drop,
        paths: drag_paths,
        allowed: drag_allowed.clone(),
    };
    let media_type = media.media_type.as_str();
    let micro_image = micro_thumb.and_then(micro_render_image);
    let mut frame = div()
        .relative()
        .absolute()
        .inset_0()
        .overflow_hidden()
        .rounded(px(theme::THUMB_ROUND))
        .bg(rgb(theme::SURFACE_FAINT));
    if let Some(image) = micro_image.clone() {
        frame = frame.child(img(image).absolute().inset_0().object_fit(ObjectFit::Cover));
    }
    if let Some(path) = media.thumb_path.as_ref() {
        let filename = media.filename.clone();
        let kind = media.media_type.clone();
        frame = frame.child(
            img(std::path::PathBuf::from(path))
                .absolute()
                .inset_0()
                .object_fit(ObjectFit::Cover)
                .with_fallback(move || {
                    super::file_format::placeholder(&filename, &kind, 32.).into_any_element()
                }),
        );
    } else if micro_image.is_none() {
        frame = frame.child(super::file_format::placeholder(
            &media.filename,
            media_type,
            32.,
        ));
    }
    if media.missing {
        frame = frame.child(
            div()
                .absolute()
                .bottom_1()
                .left_1()
                .px_2()
                .bg(rgb(theme::DANGER))
                .child(photag_ui_bridge::i18n::tr("File missing")),
        );
    }
    let drop_targets = rating_target_ids.clone();
    if media.rating > 0 {
        frame = frame.child(
            div()
                .absolute()
                .left(px(6.))
                .top(px(6.))
                .h(px(18.))
                .rounded_full()
                .bg(rgba(0x00000096))
                .px_2()
                .flex()
                .items_center()
                .text_xs()
                .text_color(rgb(theme::STAR))
                .child(theme::small_icon(IconName::Star).text_color(rgb(theme::STAR)))
                .child(media.rating.to_string()),
        );
    }
    if media_type == "video" {
        frame = frame.child(
            div()
                .absolute()
                .right(px(6.))
                .bottom(px(6.))
                .h(px(18.))
                .rounded_full()
                .bg(rgba(0x00000096))
                .px_2()
                .flex()
                .items_center()
                .text_color(rgb(theme::TEXT_PRIMARY))
                .child(theme::small_icon(IconName::Play)),
        );
    }
    if list_width.is_none() {
        let visible_tags = media_tags
            .iter()
            .filter(|tag| tag.display)
            .cloned()
            .collect::<Vec<_>>();
        if !visible_tags.is_empty() {
            let shown_count = if thumb_size < 112. && visible_tags.len() > 2 {
                2
            } else {
                visible_tags.len().min(3)
            };
            let overflow = visible_tags.len().saturating_sub(shown_count);
            let available_width = (thumb_size - 10.).max(24.);
            let count_width = if overflow > 0 { 24. } else { 0. };
            let slot_gaps = 3. * (shown_count + usize::from(overflow > 0)).saturating_sub(1) as f32;
            let capsule_width = ((available_width - count_width - slot_gaps)
                / shown_count.max(1) as f32)
                .clamp(22., 48.);
            let mut chips = div()
                .absolute()
                .right(px(5.))
                .bottom(px(5.))
                .max_w(px(available_width))
                .h_flex()
                .items_center()
                .justify_end()
                .gap(px(3.))
                .overflow_hidden();
            for tag in visible_tags.iter().take(shown_count) {
                chips = chips.child(
                    photag_ui_bridge::tag_capsule::TagCapsule::new(tag.name.clone())
                        .color(tag.color.as_deref())
                        .icon(tag.icon.as_deref())
                        .compact()
                        .render()
                        .max_w(px(capsule_width)),
                );
            }
            if overflow > 0 {
                chips = chips.child(
                    div()
                        .h(px(18.))
                        .px(px(4.))
                        .rounded_full()
                        .bg(rgba(0x000000d8))
                        .text_size(px(theme::FONT_SIZE_TINY))
                        .text_color(rgb(theme::TEXT_PRIMARY))
                        .child(format!("+{overflow}")),
                );
            }
            frame = frame.child(chips);
        }
    }
    if hover > 0.01 {
        frame = frame.child(
            div()
                .absolute()
                .inset_0()
                .bg(rgba(0xffffff00 | (18. * hover) as u32)),
        );
    }
    frame = frame.child(super::file_format::badge(&media.filename, thumb_size));
    let mut image_area = div()
        .relative()
        .w(px(thumb_size))
        .h(px(thumb_size))
        .child(frame);
    if selection > 0.01 {
        image_area = image_area.child(
            div()
                .absolute()
                .inset_0()
                .rounded(px(theme::THUMB_ROUND))
                .border_2()
                .border_color(rgba((theme::accent() << 8) | (242. * selection) as u32)),
        );
    }
    if keyboard_focus && selection > 0.5 {
        image_area = image_area.child(
            div()
                .absolute()
                .top(px(-1.))
                .left(px(-1.))
                .right(px(-1.))
                .bottom(px(-1.))
                .rounded(px(theme::THUMB_ROUND + 1.))
                .border_1()
                .border_color(rgba((theme::accent() << 8) | 0x80)),
        );
    }
    if let Some(state) = version_state
        && state.status == photag_plugin_versions::FileVersionStatus::Modified
        && list_width.is_none()
    {
        image_area = image_area.child(
            super::version_badges::badge(media_id, state, cx)
                .absolute()
                .left(px(5.))
                .top(px(5.)),
        );
    }
    let mut caption = div()
        .w_full()
        .pt(px(5.))
        .v_flex()
        .items_center()
        .gap(px(1.))
        .child(
            div()
                .w_full()
                .text_size(px(theme::FONT_SIZE_SMALL))
                .text_color(rgb(theme::TEXT_PRIMARY))
                .overflow_hidden()
                .text_ellipsis()
                .text_center()
                .child(media.filename.clone()),
        );
    if let Some(meta) = caption_metadata(media) {
        caption = caption.child(
            div()
                .w_full()
                .text_size(px(theme::FONT_SIZE_TINY))
                .text_color(rgb(theme::TEXT_MUTED))
                .overflow_hidden()
                .text_ellipsis()
                .text_center()
                .child(meta),
        );
    }
    if list_width.is_some() {
        let kind = match media.media_type.as_str() {
            "video" => "Video",
            "audio" => "Audio",
            "image" => "Photos",
            _ => "File",
        };
        let mut metadata = vec![photag_ui_bridge::i18n::tr(kind).to_string()];
        if let Some(dimensions) = caption_metadata(media) {
            metadata.push(dimensions);
        }
        metadata.push(format_bytes(media.file_size));
        metadata.push(format!("Rating {}", media.rating));
        caption = div()
            .flex_1()
            .min_w_0()
            .v_flex()
            .justify_center()
            .gap_1()
            .px_3()
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap(px(6.))
                    .when_some(version_state, |row, state| {
                        row.when(
                            state.status == photag_plugin_versions::FileVersionStatus::Modified,
                            |row| row.child(super::version_badges::badge(media_id, state, cx)),
                        )
                    })
                    .child(
                        div()
                            .text_sm()
                            .text_ellipsis()
                            .overflow_hidden()
                            .child(media.filename.clone()),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(theme::TEXT_MUTED))
                    .text_ellipsis()
                    .overflow_hidden()
                    .child(metadata.join(" · ")),
            );
    }
    let context_view = cx.entity().downgrade();
    let context_media = media.clone();
    let context_targets = rating_target_ids.clone();
    let context_tags = tag_tree.clone();
    div()
        .relative()
        .w(px(list_width.unwrap_or(thumb_size)))
        .h(px(if list_width.is_some() {
            56.
        } else {
            thumb_size + CARD_CAPTION_HEIGHT
        }))
        .when(list_width.is_some(), |row| {
            row.h_flex()
                .rounded(px(4.))
                .bg(rgba(theme::panel_rgba(theme::SURFACE_CARD)))
        })
        .when(list_width.is_none(), |row| row.v_flex())
        .id(format!("gallery-media-{media_id}"))
        .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
            this.set_card_hover(media_id, *hovered, cx)
        }))
        .drag_over::<TagDragPayload>(|style, _, _, _| {
            style.border_color(rgb(theme::accent_hover()))
        })
        .on_drop::<TagDragPayload>(cx.listener(move |this, payload: &TagDragPayload, _, cx| {
            let targets = if drop_targets.is_empty() {
                vec![media_id]
            } else {
                drop_targets.clone()
            };
            this.assign_context_tag(&targets, payload.source_id, cx);
        }))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |_, event: &MouseDownEvent, _, cx| {
                drag_allowed.store(
                    !event.modifiers.control && !event.modifiers.shift,
                    Ordering::Release,
                );
                cx.stop_propagation();
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
        .child(image_area)
        .child(caption)
        .context_menu(move |menu, window, cx| {
            super::card_menu::build(
                menu,
                context_media.clone(),
                context_targets.clone(),
                context_targets.clone(),
                context_tags.clone(),
                context_view.clone(),
                window,
                cx,
            )
        })
}

/// Second caption line. The source schema stores no media duration, so
/// video/audio rows keep the type glyph and omit a fabricated time.
fn caption_metadata(media: &photag_plugin_database::MediaItem) -> Option<String> {
    match media.media_type.as_str() {
        "video" | "audio" => None,
        _ => media
            .width
            .zip(media.height)
            .map(|(width, height)| format!("{width} × {height}")),
    }
}

pub(super) fn flatten_tags(tags: &[photag_plugin_database::Tag]) -> Vec<TagMenuEntry> {
    fn visit(tags: &[photag_plugin_database::Tag], depth: usize, entries: &mut Vec<TagMenuEntry>) {
        for tag in tags {
            entries.push(TagMenuEntry {
                id: tag.id,
                label: format!("{}{}", "  ".repeat(depth), tag.full_path),
            });
            visit(&tag.children, depth + 1, entries);
        }
    }
    let mut entries = Vec::new();
    visit(tags, 0, &mut entries);
    entries
}

pub(super) struct TagMenuEntry {
    pub(super) id: i64,
    pub(super) label: String,
}

fn micro_render_image(data: &[u8]) -> Option<Arc<::gpui::RenderImage>> {
    if data.len() != 8 * 8 * 3 {
        return None;
    }
    let rgba = data
        .chunks_exact(3)
        .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
        .collect::<Vec<_>>();
    let buffer = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(8, 8, rgba)?;
    Some(Arc::new(::gpui::RenderImage::new(vec![image::Frame::new(
        buffer,
    )])))
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
    media: &photag_plugin_database::MediaItem,
    tags: &[photag_plugin_database::Tag],
    open_count: i64,
    filesystem: Option<&FileMetadataSnapshot>,
    show_exif: bool,
    exif_fields: Option<&Result<Vec<(String, String)>, String>>,
    cx: &mut Context<GalleryView>,
) -> impl IntoElement {
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
        .child(
            div()
                .text_lg()
                .font_semibold()
                .child(photag_ui_bridge::i18n::tr("File")),
        )
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
                if metadata.read_only { "Yes" } else { "No" },
            ))
            .child(detail_line("Modified", &metadata.modified));
        if let Some(created) = &metadata.created {
            details = details.child(detail_line("Created", created));
        }
    }
    if let (Some(width), Some(height)) = (media.width, media.height) {
        let megapixels = width.saturating_mul(height) as f64 / 1_000_000.0;
        details = details
            .child(
                div()
                    .text_sm()
                    .font_semibold()
                    .child(photag_ui_bridge::i18n::tr("Image")),
            )
            .child(detail_line("Dimensions", &format!("{width} × {height} px")))
            .child(detail_line("Megapixels", &format!("{megapixels:.2} MP")))
            .child(detail_line(
                "Aspect",
                &aspect_ratio(width as u32, height as u32),
            ));
    }
    if let Some(timestamp) = media.date_taken {
        details = details
            .child(
                div()
                    .text_sm()
                    .font_semibold()
                    .child(photag_ui_bridge::i18n::tr("Dates")),
            )
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
                .child(photag_ui_bridge::i18n::tr("No tags assigned.")),
        );
    }
    if media.media_type == "image" {
        details = details
            .child(
                div()
                    .text_sm()
                    .font_semibold()
                    .child(photag_ui_bridge::i18n::tr("Metadata (EXIF)")),
            )
            .child(
                Button::new("gallery-toggle-exif")
                    .label(photag_ui_bridge::i18n::tr(
                        &(if show_exif {
                            "Hide EXIF fields"
                        } else {
                            "Show EXIF fields"
                        })
                        .to_string(),
                    ))
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_exif(cx))),
            );
        if show_exif {
            match exif_fields {
                Some(Ok(fields)) if fields.is_empty() => {
                    details = details.child(
                        div()
                            .text_sm()
                            .text_color(rgb(theme::TEXT_SECONDARY))
                            .child(photag_ui_bridge::i18n::tr("No EXIF metadata.")),
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
                            .child(photag_ui_bridge::i18n::tr(
                                "Select one image to read EXIF metadata.",
                            )),
                    );
                }
            }
        }
    }
    details
}

#[derive(Clone)]
pub(super) struct FileMetadataSnapshot {
    read_only: bool,
    modified: String,
    created: Option<String>,
}

pub(super) fn read_file_metadata(path: &str) -> Option<FileMetadataSnapshot> {
    let metadata = std::fs::metadata(path).ok()?;
    Some(FileMetadataSnapshot {
        read_only: metadata.permissions().readonly(),
        modified: format_system_time(metadata.modified().ok()),
        created: metadata
            .created()
            .ok()
            .map(|value| format_system_time(Some(value))),
    })
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
                .child(format!("{}:", photag_ui_bridge::i18n::tr(key))),
        )
        .child(div().flex_1().child(value.to_string()))
}

pub(super) fn format_bytes(bytes: i64) -> String {
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

pub(super) fn format_timestamp(timestamp: i64) -> String {
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
