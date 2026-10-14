//! Preview information panel adapted from PhoTag's `src/ui/info.rs`.
//! Combines filesystem/image metadata with the indexed media details retained by Preview.

use std::path::Path;
use std::sync::{Mutex, OnceLock};

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::*;
use photag_ui_bridge::theme;

pub(super) struct IndexedInfo {
    pub rating: i32,
    pub date_taken: Option<i64>,
    pub open_count: i64,
    pub tags: Vec<String>,
}

pub(super) fn info_panel(
    path: &Path,
    indexed: Option<IndexedInfo>,
    collapsed: &std::collections::HashSet<&'static str>,
    cx: &mut Context<crate::view::PreviewView>,
) -> impl IntoElement {
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let mut panel = div()
        .v_flex()
        .gap_2()
        .w(px(310.0))
        .max_h_full()
        .p_3()
        .bg(rgba(theme::modal_rgba(theme::SURFACE_MODAL)))
        .border_1()
        .border_color(rgb(theme::SURFACE_FAINT))
        .rounded_md()
        .overflow_y_scrollbar()
        .child(
            div()
                .h_flex()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_base()
                        .font_medium()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(name.clone()),
                )
                .child(
                    Button::new("preview-info-close")
                        .ghost()
                        .compact()
                        .icon(theme::icon(gpui_kit::assets::IconName::X))
                        .on_click(cx.listener(|view, _, _, cx| {
                            view.toggle_information(cx);
                        })),
                ),
        );
    if let Some(indexed) = &indexed {
        let mut stars = div().h_flex().gap_1();
        for rating in 1..=5u8 {
            let active = i32::from(rating) <= indexed.rating;
            let target = if i32::from(rating) == indexed.rating {
                0
            } else {
                rating
            };
            stars = stars.child(
                Button::new(format!("preview-info-rating-{rating}"))
                    .ghost()
                    .compact()
                    .icon(theme::icon(gpui_kit::assets::IconName::Star))
                    .text_color(rgb(if active {
                        theme::STAR
                    } else {
                        theme::TEXT_MUTED
                    }))
                    .on_click(
                        cx.listener(move |view, _, _, cx| view.apply_rating_shortcut(target, cx)),
                    ),
            );
        }
        panel = panel.child(stars);
    }
    panel = panel.child(section_title("File", collapsed.contains("File"), cx));
    if !collapsed.contains("File") {
        panel = panel.child(detail(
            "Type",
            if is_video(path) { "video" } else { "image" },
        ));
        if let Some(extension) = path.extension().and_then(|value| value.to_str()) {
            panel = panel.child(detail(
                "Extension",
                &format!(".{}", extension.to_lowercase()),
            ));
        }
        match std::fs::metadata(path) {
            Ok(metadata) => {
                panel = panel
                    .child(detail("Size", &format_bytes(metadata.len())))
                    .child(detail(
                        "Read-only",
                        if metadata.permissions().readonly() {
                            "yes"
                        } else {
                            "no"
                        },
                    ))
                    .child(detail("Modified", &format_time(metadata.modified().ok())))
                    .child(detail("Created", &format_time(metadata.created().ok())));
            }
            Err(_) => panel = panel.child(detail("File", "Couldn't read file metadata.")),
        }
    }

    panel = panel.child(section_title(
        "File path",
        collapsed.contains("File path"),
        cx,
    ));
    if !collapsed.contains("File path") {
        panel = panel.child(detail("Path", &path.display().to_string()));
    }

    if let Some(indexed) = &indexed {
        panel = panel.child(section_title("Library", collapsed.contains("Library"), cx));
        if !collapsed.contains("Library") {
            let tags = if indexed.tags.is_empty() {
                "No tags assigned.".to_owned()
            } else {
                indexed.tags.join(", ")
            };
            panel = panel
                .child(detail("Times opened", &indexed.open_count.to_string()))
                .child(detail(
                    "Date taken",
                    &indexed
                        .date_taken
                        .map(format_unix)
                        .unwrap_or_else(|| "—".into()),
                ))
                .child(detail("Tags", &tags));
        }
    }

    if is_video(path) {
        panel = panel.child(section_title("Video", collapsed.contains("Video"), cx));
        if !collapsed.contains("Video") {
            match probe_video(path) {
                Some(info) => {
                    panel = panel
                        .child(detail(
                            "Dimensions",
                            &format!("{} × {} px", info.width, info.height),
                        ))
                        .child(detail("Duration", &format_video_time(info.duration)))
                        .child(detail("FPS", &format!("{:.2}", info.fps)));
                    if let Some(codec) = info.codec.as_deref() {
                        panel = panel.child(detail("Codec", codec));
                    }
                    if let Some(bitrate) = info.bitrate {
                        panel = panel
                            .child(detail("Bitrate", &format!("{:.0} kbps", bitrate / 1000.0)));
                    }
                    if let Some(codec) = info.audio_codec.as_deref() {
                        panel = panel.child(detail("Audio", codec));
                    }
                }
                None => panel = panel.child(mut_text("ffprobe metadata is unavailable.")),
            }
        }
        return panel;
    }
    panel = panel.child(section_title("Image", collapsed.contains("Image"), cx));
    if !collapsed.contains("Image") {
        match image::image_dimensions(path) {
            Ok((width, height)) => {
                panel = panel
                    .child(detail("Dimensions", &format!("{width} × {height} px")))
                    .child(detail(
                        "Megapixels",
                        &format!(
                            "{:.2} MP",
                            f64::from(width) * f64::from(height) / 1_000_000.0
                        ),
                    ))
                    .child(detail("Aspect", &aspect_ratio(width, height)));
            }
            Err(_) => panel = panel.child(detail("Dimensions", "—")),
        }
    }

    panel = panel.child(section_title(
        "Metadata (EXIF)",
        collapsed.contains("Metadata (EXIF)"),
        cx,
    ));
    if !collapsed.contains("Metadata (EXIF)") {
        match read_exif(path) {
            Ok(fields) if fields.is_empty() => {
                panel = panel.child(mut_text("No common EXIF tags found."));
            }
            Ok(fields) => {
                for (label, value) in fields {
                    panel = panel.child(detail(&label, &value));
                }
            }
            Err(message) => panel = panel.child(mut_text(message)),
        }
    }
    panel
}

fn is_video(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "mp4"
                    | "m4v"
                    | "mov"
                    | "avi"
                    | "mkv"
                    | "webm"
                    | "wmv"
                    | "flv"
                    | "mpeg"
                    | "mpg"
                    | "3gp"
                    | "mts"
                    | "m2ts"
            )
        })
}

#[derive(Clone)]
struct VideoProbe {
    width: u32,
    height: u32,
    fps: f64,
    duration: f64,
    codec: Option<String>,
    bitrate: Option<f64>,
    audio_codec: Option<String>,
}

fn probe_video(path: &Path) -> Option<VideoProbe> {
    static CACHE: OnceLock<
        Mutex<std::collections::HashMap<(std::path::PathBuf, u128, u64), Option<VideoProbe>>>,
    > = OnceLock::new();
    let metadata = std::fs::metadata(path).ok()?;
    let stamp = metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos();
    let key = (path.to_path_buf(), stamp, metadata.len());
    let cache = CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    if let Ok(cache) = cache.lock() {
        if let Some(result) = cache.get(&key) {
            return result.clone();
        }
    }
    let result = probe_video_uncached(path);
    if let Ok(mut cache) = cache.lock() {
        cache.retain(|(cached_path, _, _), _| cached_path != path);
        cache.insert(key, result.clone());
    }
    result
}

fn probe_video_uncached(path: &Path) -> Option<VideoProbe> {
    #[cfg(windows)]
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    let executable = std::env::current_exe().ok()?.parent()?.to_path_buf();
    let candidates = [
        executable.join("ffprobe.exe"),
        executable.join("ffmpeg").join("bin").join("ffprobe.exe"),
        "ffprobe".into(),
    ];
    let bin = candidates
        .iter()
        .find(|candidate| candidate.exists())
        .unwrap_or(&candidates[2]);
    let mut command = Command::new(bin);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let output = command.args(["-v", "error", "-show_entries", "stream=codec_type,codec_name,width,height,r_frame_rate,bit_rate:format=duration,bit_rate", "-of", "json"]).arg(path).stdout(Stdio::piped()).stderr(Stdio::null()).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let streams = json["streams"].as_array()?;
    let video = streams
        .iter()
        .find(|stream| stream["codec_type"] == "video")?;
    let rate = video["r_frame_rate"].as_str().unwrap_or("0/1");
    let fps = rate
        .split_once('/')
        .and_then(|(n, d)| Some(n.parse::<f64>().ok()? / d.parse::<f64>().ok()?.max(1.0)))
        .unwrap_or(0.0);
    let bitrate = |value: &serde_json::Value| value.as_str().and_then(|s| s.parse::<f64>().ok());
    Some(VideoProbe {
        width: video["width"].as_u64()? as u32,
        height: video["height"].as_u64()? as u32,
        fps,
        duration: json["format"]["duration"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0),
        codec: video["codec_name"].as_str().map(str::to_owned),
        bitrate: bitrate(&video["bit_rate"]).or_else(|| bitrate(&json["format"]["bit_rate"])),
        audio_codec: streams
            .iter()
            .find(|stream| stream["codec_type"] == "audio")
            .and_then(|stream| stream["codec_name"].as_str())
            .map(str::to_owned),
    })
}

fn format_video_time(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    if total >= 3600 {
        format!(
            "{:02}:{:02}:{:02}",
            total / 3600,
            total % 3600 / 60,
            total % 60
        )
    } else {
        format!("{:02}:{:02}", total / 60, total % 60)
    }
}

fn section_title(
    title: &'static str,
    collapsed: bool,
    cx: &mut Context<crate::view::PreviewView>,
) -> Button {
    Button::new(format!("preview-info-section-{title}"))
        .ghost()
        .compact()
        .w_full()
        .border_t_1()
        .border_color(rgba(theme::SEPARATOR_LIGHT))
        .py_2()
        .child(
            div()
                .w_full()
                .h_flex()
                .gap_2()
                .child(theme::icon(if collapsed {
                    gpui_kit::assets::IconName::ChevronRight
                } else {
                    gpui_kit::assets::IconName::ChevronDown
                }))
                .child(photag_ui_bridge::i18n::tr(title)),
        )
        .on_click(cx.listener(move |view, _, _, cx| {
            if !view.collapsed_info_sections.remove(title) {
                view.collapsed_info_sections.insert(title);
            }
            cx.notify();
        }))
}

fn detail(key: &str, value: &str) -> Div {
    div()
        .h_flex()
        .items_start()
        .gap_2()
        .text_xs()
        .child(
            div()
                .w(px(88.))
                .flex_shrink_0()
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child(format!("{}:", photag_ui_bridge::i18n::tr(key))),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .text_ellipsis()
                .child(
                    if ["Type", "Read-only"].contains(&key) || value == "No tags assigned." {
                        photag_ui_bridge::i18n::tr(value)
                    } else {
                        value.to_string()
                    },
                ),
        )
}

fn mut_text(value: &str) -> Div {
    div()
        .text_xs()
        .text_color(rgb(theme::TEXT_SECONDARY))
        .child(photag_ui_bridge::i18n::tr(value))
}

fn read_exif(path: &Path) -> Result<Vec<(String, String)>, &'static str> {
    let file = std::fs::File::open(path).map_err(|_| "Couldn't read file.")?;
    let mut reader = std::io::BufReader::new(file);
    let exif = exif::Reader::new()
        .read_from_container(&mut reader)
        .map_err(|_| "No EXIF metadata.")?;
    // Curated fields and labels match the source info panel.
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

fn format_bytes(bytes: u64) -> String {
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

fn format_time(time: Option<std::time::SystemTime>) -> String {
    use chrono::TimeZone;
    time.and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|duration| {
            chrono::Local
                .timestamp_opt(duration.as_secs() as i64, 0)
                .single()
        })
        .map(|time| time.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "—".to_string())
}

fn format_unix(seconds: i64) -> String {
    use chrono::TimeZone;
    chrono::Local
        .timestamp_opt(seconds, 0)
        .single()
        .map(|time| time.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "—".into())
}

fn aspect_ratio(width: u32, height: u32) -> String {
    let gcd = gcd(width, height);
    if gcd == 0 {
        return "—".into();
    }
    let (w, h) = (width / gcd, height / gcd);
    if w > 30 || h > 30 {
        format!("{:.2}:1", f64::from(width) / f64::from(height))
    } else {
        format!("{w}:{h}")
    }
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}
