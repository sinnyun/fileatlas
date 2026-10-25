//! GPUI media tile rendering and click selection.

use fileatlas_contracts::FileManagerFileDragPayload;
use fileatlas_contracts::TagDragPayload;
use fileatlas_plugin_drag_drop::DragDropService;
use fileatlas_ui_bridge::components::WorkbenchForeground as _;
use fileatlas_ui_bridge::components::WorkbenchPaint as _;
use fileatlas_ui_bridge::components::WorkbenchStyle as _;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
use fileatlas_ui_bridge::theme;
use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::menu::ContextMenuExt;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::FileManagerView;

/// Fixed caption block under the image: filename line plus metadata line.
/// Rows reserve this height so uniform_list keeps a uniform item size.
pub(super) const CARD_CAPTION_HEIGHT: f32 = 38.;

pub(super) fn render_media_card(
    media: &fileatlas_plugin_database::MediaItem,
    hover: f32,
    selection: f32,
    keyboard_focus: bool,
    drag_drop: Arc<DragDropService>,
    drag_paths: Vec<PathBuf>,
    drag_ids: Vec<i64>,
    native_drag: bool,
    drag_allowed: Arc<AtomicBool>,
    thumb_size: f32,
    list_width: Option<f32>,
    micro_thumb: Option<&[u8]>,
    version_state: Option<&fileatlas_plugin_versions::FileVersionState>,
    rating_target_ids: Vec<i64>,
    media_tags: &[fileatlas_plugin_database::Tag],
    tag_tree: Arc<Vec<fileatlas_plugin_database::Tag>>,
    cx: &mut Context<FileManagerView>,
) -> impl IntoElement + 'static {
    let media_id = media.id;
    let payload = FileManagerFileDragPayload {
        paths: drag_paths
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
        ids: drag_ids,
        allowed: drag_allowed.clone(),
        native_handoff_started: Arc::new(AtomicBool::new(false)),
    };
    let source_allowed = Arc::clone(&payload.allowed);
    let media_type = media.media_type.as_str();
    let micro_image = micro_thumb.and_then(micro_render_image);
    let mut frame = fileatlas_ui_bridge::components::thumbnail_frame();
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
            fileatlas_ui_bridge::components::status_badge(
                div().child(fileatlas_ui_bridge::i18n::tr("File missing")),
                fileatlas_ui_bridge::components::StatusTone::Danger,
            )
            .absolute()
            .bottom_1()
            .left_1(),
        );
    }
    let drop_targets = rating_target_ids.clone();
    if media.rating > 0 {
        frame = frame.child(
            fileatlas_ui_bridge::components::media_badge(
                div()
                    .child(theme::small_icon(IconName::Star).workbench_foreground(
                        fileatlas_ui_bridge::components::ForegroundRole::StarOnMediaBadge,
                    ))
                    .child(media.rating.to_string()),
            )
            .absolute()
            .left(px(6.))
            .top(px(6.)),
        );
    }
    if media_type == "video" {
        frame = frame.child(
            fileatlas_ui_bridge::components::media_badge(
                div().child(theme::small_icon(IconName::Play)),
            )
            .absolute()
            .right(px(6.))
            .bottom(px(6.)),
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
                .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                .overflow_hidden();
            for tag in visible_tags.iter().take(shown_count) {
                chips = chips.child(
                    fileatlas_ui_bridge::tag_capsule::TagCapsule::new(tag.name.clone())
                        .color(tag.color.as_deref())
                        .icon(tag.icon.as_deref())
                        .compact()
                        .render()
                        .max_w(px(capsule_width)),
                );
            }
            if overflow > 0 {
                chips = chips.child(fileatlas_ui_bridge::components::media_badge(
                    div().child(format!("+{overflow}")),
                ));
            }
            frame = frame.child(chips);
        }
    }
    if hover > 0.01 {
        frame = frame.child(fileatlas_ui_bridge::components::media_hover_overlay(hover));
    }
    frame = frame.child(super::file_format::badge(&media.filename, thumb_size));
    let mut image_area = div()
        .relative()
        .w(px(thumb_size))
        .h(px(thumb_size))
        .child(frame);
    if selection > 0.01 || (keyboard_focus && selection > 0.5) {
        image_area = image_area.child(fileatlas_ui_bridge::components::media_selection_overlay(
            thumb_size,
            selection,
            keyboard_focus,
        ));
    }
    if let Some(state) = version_state
        && state.status == fileatlas_plugin_versions::FileVersionStatus::Modified
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
        .pt(px(fileatlas_ui_bridge::theme::space_compact()))
        .v_flex()
        .items_center()
        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
        .child(
            div()
                .w_full()
                .workbench_body()
                .overflow_hidden()
                .text_ellipsis()
                .text_center()
                .child(media.filename.clone()),
        );
    if let Some(meta) = caption_metadata(media) {
        caption = caption.child(
            div()
                .w_full()
                .workbench_assistant()
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
        let mut metadata = vec![fileatlas_ui_bridge::i18n::tr(kind).to_string()];
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
            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
            .px(px(fileatlas_ui_bridge::theme::space_expanded()))
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                    .when_some(version_state, |row, state| {
                        row.when(
                            state.status == fileatlas_plugin_versions::FileVersionStatus::Modified,
                            |row| row.child(super::version_badges::badge(media_id, state, cx)),
                        )
                    })
                    .child(
                        div()
                            .workbench_assistant()
                            .text_ellipsis()
                            .overflow_hidden()
                            .child(media.filename.clone()),
                    ),
            )
            .child(
                div()
                    .workbench_assistant()
                    .text_ellipsis()
                    .overflow_hidden()
                    .child(metadata.join(" · ")),
            );
    }
    let context_view = cx.entity().downgrade();
    let context_media = media.clone();
    let context_targets = rating_target_ids.clone();
    let context_tags = tag_tree.clone();
    let card = div()
        .relative()
        .w(px(list_width.unwrap_or(thumb_size)))
        .h(px(if list_width.is_some() {
            56.
        } else {
            thumb_size + CARD_CAPTION_HEIGHT
        }))
        .when(list_width.is_some(), |row| {
            row.h_flex()
                .workbench_corner(fileatlas_ui_bridge::components::CornerRole::Small)
                .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::CardOverlay)
        })
        .when(list_width.is_none(), |row| row.v_flex())
        .when(list_width.is_none(), |card| card.workbench_media_card())
        .id(format!("file_manager-media-{media_id}"))
        .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
            this.set_card_hover(media_id, *hovered, cx)
        }))
        .drag_over::<TagDragPayload>(|style, _, _, _| {
            fileatlas_ui_bridge::components::drop_target_style(style)
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
        .on_drag(payload, move |_, _, _, cx| cx.new(|_| FileDragGhost))
        .on_drag_move::<FileManagerFileDragPayload>(move |event, window, cx| {
            let payload = event.drag(cx);
            let viewport = gpui::Bounds::new(gpui::Point::default(), window.viewport_size());
            if native_drag
                && payload.allows_drop()
                && Arc::ptr_eq(&payload.allowed, &source_allowed)
                && !viewport.contains(&event.event.position)
                && !payload.native_handoff_started.swap(true, Ordering::AcqRel)
            {
                let paths = payload.paths.iter().map(PathBuf::from).collect::<Vec<_>>();
                if let Err(error) = drag_drop.start_file_drag(
                    window,
                    &paths,
                    fileatlas_plugin_drag_drop::DragImageStyle {
                        background: theme::channels(theme::surface_card()),
                        foreground: theme::channels(theme::text_primary()),
                        border: theme::channels(theme::separator()),
                        accent: theme::channels(theme::accent()),
                    },
                ) {
                    tracing::warn!(%error, "could not start native file drag");
                }
            }
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
        });
    fileatlas_ui_bridge::components::review_card(
        card,
        fileatlas_ui_bridge::geometry_audit::ComponentTokens::default(),
    )
}

/// Second caption line. The source schema stores no media duration, so
/// video/audio rows keep the type glyph and omit a fabricated time.
fn caption_metadata(media: &fileatlas_plugin_database::MediaItem) -> Option<String> {
    match media.media_type.as_str() {
        "video" | "audio" => None,
        _ => media
            .width
            .zip(media.height)
            .map(|(width, height)| format!("{width} × {height}")),
    }
}

pub(super) fn flatten_tags(tags: &[fileatlas_plugin_database::Tag]) -> Vec<TagMenuEntry> {
    fn visit(
        tags: &[fileatlas_plugin_database::Tag],
        depth: usize,
        entries: &mut Vec<TagMenuEntry>,
    ) {
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

struct FileDragGhost;

impl Render for FileDragGhost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size(px(1.))
    }
}

pub(super) fn render_media_details(
    media: &fileatlas_plugin_database::MediaItem,
    tags: &[fileatlas_plugin_database::Tag],
    open_count: i64,
    filesystem: Option<&FileMetadataSnapshot>,
    show_exif: bool,
    exif_fields: Option<&Result<Vec<(String, String)>, String>>,
    cx: &mut Context<FileManagerView>,
) -> impl IntoElement {
    let extension = std::path::Path::new(&media.filename)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{}", value.to_lowercase()))
        .unwrap_or_else(|| "—".to_string());
    let mut details = fileatlas_ui_bridge::components::card_surface(
        div()
            .v_flex()
            .gap(px(fileatlas_ui_bridge::theme::space_standard()))
            .p(px(fileatlas_ui_bridge::theme::space_expanded())),
        false,
    )
    .child(fileatlas_ui_bridge::components::heading_block(
        fileatlas_ui_bridge::i18n::tr("File"),
        media.filename.clone(),
    ))
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
                    .workbench_assistant()
                    .child(fileatlas_ui_bridge::i18n::tr("Image")),
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
                    .workbench_assistant()
                    .child(fileatlas_ui_bridge::i18n::tr("Dates")),
            )
            .child(detail_line("Date taken", &format_timestamp(timestamp)));
    }
    if !tags.is_empty() {
        details = details
            .child(
                div()
                    .workbench_assistant()
                    .child(format!("Tags ({})", tags.len())),
            )
            .child(div().workbench_assistant().child(format!(
                "Tags: {}",
                tags.iter()
                    .map(|tag| tag.full_path.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
    } else {
        details = details.child(
            div()
                .workbench_assistant()
                .child(fileatlas_ui_bridge::i18n::tr("No tags assigned.")),
        );
    }
    if media.media_type == "image" {
        details = details
            .child(
                div()
                    .workbench_assistant()
                    .child(fileatlas_ui_bridge::i18n::tr("Metadata (EXIF)")),
            )
            .child(
                fileatlas_ui_bridge::components::button("file_manager-toggle-exif")
                    .label(fileatlas_ui_bridge::i18n::tr(
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
                            .workbench_assistant()
                            .child(fileatlas_ui_bridge::i18n::tr("No EXIF metadata.")),
                    );
                }
                Some(Ok(fields)) => {
                    for (label, value) in fields {
                        details = details.child(detail_line(label, value));
                    }
                }
                Some(Err(message)) => {
                    details = details.child(div().workbench_assistant().child(message.clone()));
                }
                None => {
                    details = details.child(div().workbench_assistant().child(
                        fileatlas_ui_bridge::i18n::tr("Select one image to read EXIF metadata."),
                    ));
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
        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
        .workbench_assistant()
        .child(
            div()
                .workbench_foreground(fileatlas_ui_bridge::components::ForegroundRole::Secondary)
                .child(format!("{}:", fileatlas_ui_bridge::i18n::tr(key))),
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
