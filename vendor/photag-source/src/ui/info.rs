//! Left-side information panel for the preview view. Shows everything we
//! know about the currently-previewed media: filesystem facts, image
//! dimensions, EXIF/metadata (best-effort), tags, and engagement stats.
//!
//! Rendered as an `egui::Area` (not a Window) anchored to the left edge of
//! the screen, just below the "i" toggle button. Lives only while
//! `preview_state.show_info` is true.

use crate::app::AppState;
use crate::db::queries::{MediaItem, Tag};
use crate::ui::sidebar::tag_color32;
use egui::{self, Color32, RichText, Ui};
use std::path::Path;

/// Render the panel. Caller is responsible for the show/hide flag.
/// `_open` is currently unused (no built-in close button on the inline
/// panel) but kept for API symmetry with the previous Window-based version.
pub fn show_info_panel(ctx: &egui::Context, state: &mut AppState, _open: &mut bool) {
    let Some(media) = state.preview_media.clone() else { return };
    let media_id = media.id;

    let screen = ctx.screen_rect();
    // Panel sits flush against the left edge, starting just below the "i"
    // button (which is at y ≈ 25 with radius 14 → bottom ≈ 39). Add a small
    // gap so the panel doesn't visually merge with the button.
    let panel_x = screen.min.x + 12.0;
    let panel_y = screen.min.y + 50.0;
    let panel_w = 320.0_f32.min(screen.width() * 0.32);
    let panel_h = (screen.height() - panel_y - 16.0).max(120.0);

    egui::Area::new(egui::Id::new("preview_info_panel"))
        .fixed_pos(egui::pos2(panel_x, panel_y))
        .order(egui::Order::Foreground)
        .interactable(true)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(Color32::from_rgba_premultiplied(20, 22, 28, 230))
                .rounding(egui::Rounding::same(8.0))
                .stroke(egui::Stroke::new(1.0, crate::theme::BORDER))
                .inner_margin(egui::Margin::symmetric(12.0, 10.0))
                .shadow(egui::epaint::Shadow {
                    offset: egui::Vec2::new(0.0, 4.0),
                    blur: 14.0,
                    spread: 0.0,
                    color: Color32::from_black_alpha(180),
                })
                .show(ui, |ui| {
                    ui.set_width(panel_w);
                    ui.set_max_height(panel_h);
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, true])
                        .max_height(panel_h)
                        .show(ui, |ui| {
                            section(ui, "File");
                            file_section(ui, &media);

                            if media.media_type == "image" {
                                section(ui, "Image");
                                image_section(ui, &media);
                            } else if media.media_type == "video" {
                                section(ui, "Video");
                                video_section(ui, &media);
                            }

                            section(ui, "Dates");
                            dates_section(ui, &media);

                            let tags = state.media_tags.get(&media_id).cloned().unwrap_or_default();
                            section(ui, &format!("Tags ({})", tags.len()));
                            tags_section(ui, &tags);

                            section(ui, "Engagement");
                            engagement_section(ui, state, media_id);

                            if media.media_type == "image" {
                                section(ui, "Metadata (EXIF)");
                                exif_section(ui, &media.full_path);
                            }
                        });
                });
        });
}

fn section(ui: &mut Ui, title: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(title).strong().color(Color32::from_rgb(170, 200, 255)));
    ui.separator();
}

fn kv(ui: &mut Ui, key: &str, value: impl Into<String>) {
    let value: String = value.into();
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{}:", key)).color(Color32::GRAY));
        ui.label(value);
    });
}

fn kv_mono(ui: &mut Ui, key: &str, value: impl Into<String>) {
    let value: String = value.into();
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(format!("{}:", key)).color(Color32::GRAY));
        ui.label(RichText::new(value).monospace());
    });
}

fn file_section(ui: &mut Ui, m: &MediaItem) {
    kv(ui, "Name", &m.filename);
    kv_mono(ui, "Path", &m.full_path);
    kv(ui, "Type", &m.media_type);
    if let Some(ext) = Path::new(&m.filename).extension().and_then(|e| e.to_str()) {
        kv(ui, "Extension", format!(".{}", ext.to_lowercase()));
    }
    kv(ui, "Size", format_bytes(m.file_size));
    if let Ok(meta) = std::fs::metadata(&m.full_path) {
        kv(ui, "Read-only", if meta.permissions().readonly() { "yes" } else { "no" });
    }
}

fn image_section(ui: &mut Ui, m: &MediaItem) {
    if let (Some(w), Some(h)) = (m.width, m.height) {
        kv(ui, "Dimensions", format!("{} × {} px", w, h));
        kv(ui, "Megapixels", format!("{:.2} MP", (w * h) as f64 / 1_000_000.0));
        let ratio = aspect_ratio(w as u32, h as u32);
        kv(ui, "Aspect", ratio);
    } else {
        kv(ui, "Dimensions", "—");
    }
}

fn video_section(ui: &mut Ui, m: &MediaItem) {
    // Try ffprobe for live dimensions/duration/codec info. This is the same
    // binary the player uses, so if videos play at all this will succeed.
    if let Some(info) = probe_video(&m.full_path) {
        kv(ui, "Dimensions", format!("{} × {} px", info.width, info.height));
        kv(ui, "Duration", crate::video::format_time(info.duration));
        kv(ui, "FPS", format!("{:.2}", info.fps));
        if let Some(c) = info.codec { kv(ui, "Codec", c); }
        if let Some(b) = info.bitrate { kv(ui, "Bitrate", format!("{:.0} kbps", b / 1000.0)); }
        if let Some(a) = info.audio_codec { kv(ui, "Audio", a); }
    } else if let (Some(w), Some(h)) = (m.width, m.height) {
        kv(ui, "Dimensions", format!("{} × {} px", w, h));
    }
}

fn dates_section(ui: &mut Ui, m: &MediaItem) {
    kv(ui, "Modified", format_unix(m.modified_at));
    if let Some(d) = m.date_taken {
        kv(ui, "Date taken", format_unix(d));
    }
    if let Ok(meta) = std::fs::metadata(&m.full_path) {
        if let Ok(c) = meta.created() {
            if let Ok(d) = c.duration_since(std::time::UNIX_EPOCH) {
                kv(ui, "Created", format_unix(d.as_secs() as i64));
            }
        }
    }
}

fn tags_section(ui: &mut Ui, tags: &[Tag]) {
    if tags.is_empty() {
        ui.colored_label(Color32::GRAY, "No tags assigned.");
        return;
    }
    ui.horizontal_wrapped(|ui| {
        for t in tags {
            let icon = t.icon.as_deref().filter(|s| !s.is_empty()).unwrap_or("🏷");
            let col = tag_color32(t);
            let label = format!("{} {}", icon, t.name);
            ui.label(RichText::new(label).color(col));
        }
    });
}

fn engagement_section(ui: &mut Ui, state: &AppState, media_id: i64) {
    let count = {
        let conn = state.read_db.lock().unwrap();
        crate::db::get_open_count(&conn, media_id)
    };
    kv(ui, "Times opened", count.to_string());
}

fn exif_section(ui: &mut Ui, path: &str) {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => { ui.colored_label(Color32::GRAY, "Couldn't read file."); return; }
    };
    let mut reader = std::io::BufReader::new(&file);
    let exif = match exif::Reader::new().read_from_container(&mut reader) {
        Ok(e) => e,
        Err(_) => { ui.colored_label(Color32::GRAY, "No EXIF metadata."); return; }
    };

    // Show a curated subset that's actually useful to humans, then a
    // collapsible "All fields" section for completeness.
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

    let mut shown = 0;
    for (tag, label) in interesting {
        for ifd in [exif::In::PRIMARY, exif::In::THUMBNAIL] {
            if let Some(field) = exif.get_field(tag, ifd) {
                kv(ui, label, field.display_value().with_unit(&exif).to_string());
                shown += 1;
                break;
            }
        }
    }

    if shown == 0 {
        ui.colored_label(Color32::GRAY, "No common EXIF tags found.");
    }

    egui::CollapsingHeader::new("All EXIF fields")
        .default_open(false)
        .show(ui, |ui| {
            for f in exif.fields() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!("{}:", f.tag))
                            .color(Color32::GRAY).small(),
                    );
                    ui.label(
                        RichText::new(f.display_value().with_unit(&exif).to_string())
                            .small(),
                    );
                });
            }
        });
}

// ── helpers ─────────────────────────────────────────────────────────────

fn format_bytes(bytes: i64) -> String {
    let b = bytes as f64;
    const K: f64 = 1024.0;
    if b < K { format!("{} B", bytes) }
    else if b < K * K { format!("{:.1} KB", b / K) }
    else if b < K * K * K { format!("{:.2} MB", b / (K * K)) }
    else { format!("{:.2} GB", b / (K * K * K)) }
}

fn format_unix(secs: i64) -> String {
    use chrono::TimeZone;
    chrono::Local.timestamp_opt(secs, 0)
        .single()
        .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "—".to_string())
}

fn aspect_ratio(w: u32, h: u32) -> String {
    let g = gcd(w, h);
    if g == 0 { return "—".into(); }
    let aw = w / g;
    let ah = h / g;
    // Collapse very large reduced ratios into a decimal so we don't show
    // something like "5760:3840" — that's not useful information.
    if aw > 30 || ah > 30 {
        format!("{:.2}:1", w as f64 / h as f64)
    } else {
        format!("{}:{}", aw, ah)
    }
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

// ── ffprobe shell-out for video metadata ────────────────────────────────

struct VideoProbe {
    width: u32,
    height: u32,
    fps: f64,
    duration: f64,
    codec: Option<String>,
    bitrate: Option<f64>,
    audio_codec: Option<String>,
}

fn probe_video(path: &str) -> Option<VideoProbe> {
    use std::process::{Command, Stdio};
    #[cfg(windows)]
    use std::os::windows::process::CommandExt;

    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let candidates = [
        dir.join("ffprobe.exe"),
        dir.join("ffmpeg").join("bin").join("ffprobe.exe"),
        std::path::PathBuf::from("ffprobe"),
    ];
    let bin = candidates.iter().find(|p| p.exists())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "ffprobe".to_string());

    let mut cmd = Command::new(bin);
    #[cfg(windows)]
    { cmd.creation_flags(0x08000000); }
    let out = cmd
        .args([
            "-v", "error",
            "-show_entries",
            "stream=codec_type,codec_name,width,height,r_frame_rate,bit_rate:format=duration,bit_rate",
            "-of", "json",
            path,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    let json: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let streams = json["streams"].as_array()?;

    let v = streams.iter().find(|s| s["codec_type"] == "video")?;
    let width = v["width"].as_u64()? as u32;
    let height = v["height"].as_u64()? as u32;
    let fps_str = v["r_frame_rate"].as_str().unwrap_or("0/1");
    let fps = fps_str.split_once('/')
        .and_then(|(n, d)| Some(n.parse::<f64>().ok()? / d.parse::<f64>().ok()?.max(1.0)))
        .unwrap_or(0.0);
    let codec = v["codec_name"].as_str().map(String::from);
    let bitrate = v["bit_rate"].as_str().and_then(|s| s.parse::<f64>().ok())
        .or_else(|| json["format"]["bit_rate"].as_str().and_then(|s| s.parse::<f64>().ok()));
    let duration = json["format"]["duration"].as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let audio_codec = streams.iter()
        .find(|s| s["codec_type"] == "audio")
        .and_then(|s| s["codec_name"].as_str())
        .map(String::from);

    Some(VideoProbe { width, height, fps, duration, codec, bitrate, audio_codec })
}
