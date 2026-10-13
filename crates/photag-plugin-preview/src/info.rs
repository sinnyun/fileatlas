//! Preview information panel adapted from PhoTag's `src/ui/info.rs`.
//! The path-only preview contract can provide filesystem and image/EXIF facts;
//! database-backed tags and engagement counts are intentionally omitted.

use std::path::Path;

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::*;

pub(super) fn info_panel(path: &Path) -> impl IntoElement {
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
        .bg(rgb(0x14161b))
        .border_1()
        .border_color(rgb(0x343943))
        .rounded_md()
        .overflow_y_scrollbar()
        .child(div().text_base().font_medium().child("Information"))
        .child(section_title("File"))
        .child(detail("Name", &name))
        .child(detail("Path", &path.display().to_string()))
        .child(detail("Type", "image"));

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
                .child(section_title("Dates"))
                .child(detail("Modified", &format_time(metadata.modified().ok())))
                .child(detail("Created", &format_time(metadata.created().ok())));
        }
        Err(_) => panel = panel.child(detail("File", "Couldn't read file metadata.")),
    }

    panel = panel.child(section_title("Image"));
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

    panel = panel.child(section_title("Metadata (EXIF)"));
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
    panel
}

fn section_title(title: &str) -> Div {
    div()
        .mt_2()
        .text_sm()
        .font_medium()
        .text_color(rgb(0xaac8ff))
        .child(title.to_string())
}

fn detail(key: &str, value: &str) -> Div {
    div()
        .v_flex()
        .gap_1()
        .text_xs()
        .child(div().text_color(rgb(0x9aa0ad)).child(format!("{key}:")))
        .child(div().child(value.to_string()))
}

fn mut_text(value: &str) -> Div {
    div()
        .text_xs()
        .text_color(rgb(0x9aa0ad))
        .child(value.to_string())
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
