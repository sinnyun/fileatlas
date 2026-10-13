//! GPUI media tile rendering and click selection.

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::*;
use photag_contracts::TagDragPayload;
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
    micro_thumb: Option<&[u8]>,
    rating_target_ids: Vec<i64>,
    tag_tree: Arc<Vec<photag_plugin_database::db::queries::Tag>>,
    cx: &mut Context<GalleryView>,
) -> impl IntoElement + 'static {
    let media_id = media.id;
    let payload = FileDragPayload {
        service: drag_drop,
        paths: drag_paths,
        allowed: drag_allowed.clone(),
    };
    let mut card = div()
        .relative()
        .w(px(thumb_size))
        .h(px(thumb_size))
        .rounded(px(theme::THUMB_ROUND))
        .overflow_hidden()
        .bg(rgba(if selected {
            theme::accent_alpha(theme::accent(), 0.42)
        } else {
            theme::base_rgba(theme::SURFACE_ELEVATED)
        }));
    // PhoTag paints a square image target. Selection reveals the accent
    // backplate by insetting the image seven source points on every edge.
    let image_inset = if selected { 7.0 } else { 0.0 };
    let mut thumbnail = div()
        .relative()
        .absolute()
        .top(px(image_inset))
        .left(px(image_inset))
        .right(px(image_inset))
        .bottom(px(image_inset))
        .overflow_hidden()
        .rounded(px(theme::THUMB_ROUND));
    let micro_image = micro_thumb.and_then(micro_render_image);
    if let Some(image) = micro_image.clone() {
        thumbnail = thumbnail.child(
            img(image)
                .absolute()
                .top(px(0.))
                .left(px(0.))
                .right(px(0.))
                .bottom(px(0.))
                .object_fit(ObjectFit::Cover),
        );
    }
    if let Some(path) = media.thumb_path.as_ref() {
        thumbnail = thumbnail.child(
            img(std::path::PathBuf::from(path))
                .absolute()
                .top(px(0.))
                .left(px(0.))
                .right(px(0.))
                .bottom(px(0.))
                .object_fit(ObjectFit::Cover)
                .with_fallback(move || {
                    div()
                        .absolute()
                        .top(px(0.))
                        .left(px(0.))
                        .right(px(0.))
                        .bottom(px(0.))
                        .into_any_element()
                }),
        );
    } else if micro_image.is_none() {
        thumbnail = thumbnail
            .items_center()
            .justify_center()
            .text_xs()
            .text_color(rgb(theme::TEXT_MUTED))
            .bg(rgba(theme::panel_rgba(theme::SURFACE_PANEL)))
            .child(media.media_type.to_uppercase());
    }
    let menu_view = cx.entity().downgrade();
    let menu_targets = rating_target_ids.clone();
    let drop_targets = rating_target_ids.clone();
    let menu = Button::new(format!("gallery-card-actions-{media_id}"))
        .label(photag_ui_bridge::i18n::tr(&("⋯").to_string()))
        .dropdown_menu(move |mut menu, _, _| {
            let ai_view = menu_view.clone();
            menu = menu.item(
                PopupMenuItem::new(photag_ui_bridge::i18n::tr("AI Suggest Tags")).on_click(
                    move |_, _, cx| {
                        let _ = ai_view.update(cx, |this, cx| {
                            this.suggest_tags(media_id, cx);
                        });
                    },
                ),
            );
            let preview_view = menu_view.clone();
            menu = menu.item(
                PopupMenuItem::new(photag_ui_bridge::i18n::tr("Preview")).on_click(
                    move |_, _, cx| {
                        let _ = preview_view.update(cx, |this, cx| {
                            this.preview_media(media_id, cx);
                        });
                    },
                ),
            );
            let open_view = menu_view.clone();
            menu = menu.item(
                PopupMenuItem::new(photag_ui_bridge::i18n::tr("Open in default app")).on_click(
                    move |_, _, cx| {
                        let _ = open_view.update(cx, |this, cx| {
                            this.open_media_action(media_id, false, cx);
                        });
                    },
                ),
            );
            let reveal_view = menu_view.clone();
            menu = menu.item(
                PopupMenuItem::new(photag_ui_bridge::i18n::tr("Show in File Explorer")).on_click(
                    move |_, _, cx| {
                        let _ = reveal_view.update(cx, |this, cx| {
                            this.open_media_action(media_id, true, cx);
                        });
                    },
                ),
            );
            menu = menu.item(PopupMenuItem::separator());
            for tag in flatten_tags(&tag_tree) {
                let tag_view = menu_view.clone();
                let tag_targets = menu_targets.clone();
                menu = menu.item(
                    PopupMenuItem::new(format!("Assign tag: {}", tag.label)).on_click(
                        move |_, _, cx| {
                            let _ = tag_view.update(cx, |this, cx| {
                                this.assign_context_tag(&tag_targets, tag.id, cx);
                            });
                        },
                    ),
                );
            }
            menu = menu.item(PopupMenuItem::separator());
            let copy_view = menu_view.clone();
            let copy_targets = menu_targets.clone();
            menu = menu.item(
                PopupMenuItem::new(photag_ui_bridge::i18n::tr("Copy to…")).on_click(
                    move |_, _, cx| {
                        let _ = copy_view.update(cx, |this, cx| {
                            this.selection.selected_media_ids = copy_targets.clone();
                            this.copy_selected_with_dialog(cx);
                        });
                    },
                ),
            );
            let move_view = menu_view.clone();
            let move_targets = menu_targets.clone();
            menu = menu.item(
                PopupMenuItem::new(photag_ui_bridge::i18n::tr("Move to…")).on_click(
                    move |_, _, cx| {
                        let _ = move_view.update(cx, |this, cx| {
                            this.selection.selected_media_ids = move_targets.clone();
                            this.move_selected_with_dialog(cx);
                        });
                    },
                ),
            );
            let delete_view = menu_view.clone();
            let delete_targets = menu_targets.clone();
            menu = menu.item(
                PopupMenuItem::new(photag_ui_bridge::i18n::tr("Delete…")).on_click(
                    move |_, window, cx| {
                        let _ = delete_view.update(cx, |this, cx| {
                            this.selection.selected_media_ids = delete_targets.clone();
                            this.selection.anchor_id = delete_targets.first().copied();
                            this.request_delete_confirmation(window, cx);
                        });
                    },
                ),
            );
            menu = menu.item(PopupMenuItem::separator());
            for rating in 0..=5 {
                let rating_view = menu_view.clone();
                let rating_target_ids = rating_target_ids.clone();
                let label = if rating == 0 {
                    "No rating".to_string()
                } else {
                    format!("Rate {rating}★")
                };
                menu = menu.item(PopupMenuItem::new(label).on_click(move |_, _, cx| {
                    let _ = rating_view.update(cx, |this, cx| {
                        this.apply_rating_to(&rating_target_ids, rating);
                        cx.notify();
                    });
                }));
            }
            menu
        });
    thumbnail = thumbnail.child(div().absolute().top(px(7.)).right(px(7.)).child(menu));
    if media.rating > 0 {
        thumbnail = thumbnail.child(
            div()
                .absolute()
                .left(px(7.))
                .bottom(px(7.))
                .h(px(18.))
                .rounded_full()
                .bg(rgba(0x00000096))
                .px_2()
                .items_center()
                .text_xs()
                .text_color(rgb(theme::STAR))
                .child(format!("★ {}", media.rating)),
        );
    }
    if media.media_type == "video" || media.media_type == "audio" {
        thumbnail = thumbnail.child(
            div()
                .absolute()
                .right(px(7.))
                .bottom(px(7.))
                .size(px(22.))
                .rounded_full()
                .bg(rgba(0x00000096))
                .items_center()
                .justify_center()
                .text_xs()
                .text_color(rgb(theme::TEXT_PRIMARY))
                .child(if media.media_type == "video" {
                    "▶"
                } else {
                    "♪"
                }),
        );
    }
    card = card.child(thumbnail);
    card.id(format!("gallery-media-{media_id}"))
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
}

fn flatten_tags(tags: &[photag_plugin_database::db::queries::Tag]) -> Vec<TagMenuEntry> {
    fn visit(
        tags: &[photag_plugin_database::db::queries::Tag],
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

struct TagMenuEntry {
    id: i64,
    label: String,
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
