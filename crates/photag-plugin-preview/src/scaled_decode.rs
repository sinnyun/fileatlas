//! JPEG DCT scaling follows the pinned PhoTag gallery::decode_jpeg_scaled
//! pipeline; retain its orientation and generic-format fallback semantics.
//! Source: PhoTag, Copyright (c) 2026 Oleksandr Shabaldas, MIT License;
//! see vendor/photag-source/LICENSE.
use std::path::Path;

use super::PreviewService;

#[derive(Default, Debug)]
pub(super) struct DecodeMetrics {
    pub read_ms: f64,
    pub header_ms: f64,
    pub decode_ms: f64,
    pub orientation_ms: f64,
    pub resize_ms: f64,
    pub source_dimensions: Option<(u32, u32)>,
    pub output_dimensions: Option<(u32, u32)>,
    pub decoder: &'static str,
    pub fallback_error: Option<String>,
    pub error_stage: &'static str,
}
pub(super) fn millis(duration: std::time::Duration) -> f64 {
    (duration.as_secs_f64() * 1_000_000.).round() / 1000.
}
fn elapsed(start: std::time::Instant) -> f64 {
    millis(start.elapsed())
}

pub(super) fn decode_preview(
    path: &Path,
    max_dim: u32,
    metrics: &mut DecodeMetrics,
) -> Result<image::DynamicImage, String> {
    metrics.error_stage = "read";
    let started = std::time::Instant::now();
    let bytes = std::fs::read(path).map_err(|e| e.to_string());
    metrics.read_ms = elapsed(started);
    let bytes = bytes?;
    let jpeg = path
        .extension()
        .and_then(|v| v.to_str())
        .is_some_and(|v| v.eq_ignore_ascii_case("jpg") || v.eq_ignore_ascii_case("jpeg"));
    let mut image = if jpeg {
        match scaled_jpeg_bytes(path, &bytes, max_dim, metrics) {
            Ok(image) => image,
            Err(error) => {
                metrics.fallback_error = Some(error);
                generic_decode(path, &bytes, metrics)?
            }
        }
    } else {
        generic_decode(path, &bytes, metrics)?
    };
    metrics.error_stage = "resize";
    let started = std::time::Instant::now();
    if image.width().max(image.height()) > max_dim {
        image = image.thumbnail(max_dim, max_dim);
    }
    metrics.resize_ms = elapsed(started);
    metrics.output_dimensions = Some((image.width(), image.height()));
    metrics.error_stage = "none";
    Ok(image)
}

fn generic_decode(
    path: &Path,
    bytes: &[u8],
    metrics: &mut DecodeMetrics,
) -> Result<image::DynamicImage, String> {
    use image::ImageDecoder as _;
    metrics.decoder = "generic";
    metrics.error_stage = "header";
    let started = std::time::Instant::now();
    let reader = image::ImageReader::with_format(
        std::io::Cursor::new(bytes),
        image::ImageFormat::from_path(path).map_err(|e| e.to_string())?,
    );
    let result = reader.into_decoder().map_err(|e| e.to_string());
    metrics.header_ms += elapsed(started);
    let mut decoder = result?;
    metrics.source_dimensions = Some(decoder.dimensions());
    let started = std::time::Instant::now();
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    metrics.orientation_ms += elapsed(started);
    metrics.error_stage = "decode";
    let started = std::time::Instant::now();
    let result = image::DynamicImage::from_decoder(decoder).map_err(|e| e.to_string());
    metrics.decode_ms += elapsed(started);
    let mut image = result?;
    let started = std::time::Instant::now();
    image.apply_orientation(orientation);
    metrics.orientation_ms += elapsed(started);
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timed_generic_decode_matches_existing_pixels_and_reports_failures() {
        let directory =
            std::env::temp_dir().join(format!("photag-timed-decode-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        for extension in ["png", "bmp", "tiff", "webp"] {
            let path = directory.join(format!("source.{extension}"));
            image::RgbImage::from_pixel(80, 60, image::Rgb([25, 75, 125]))
                .save(&path)
                .unwrap();
            let expected = PreviewService::open_oriented(&path).unwrap();
            let mut metrics = DecodeMetrics::default();
            let actual = decode_preview(&path, 100, &mut metrics).unwrap();
            assert_eq!(expected.to_rgba8(), actual.to_rgba8());
            assert_eq!(metrics.source_dimensions, Some((80, 60)));
            assert_eq!(metrics.decoder, "generic");
        }
        let mut metrics = DecodeMetrics::default();
        assert!(decode_preview(&directory.join("absent.jpg"), 96, &mut metrics).is_err());
        assert_eq!(metrics.error_stage, "read");
        let broken = directory.join("broken.jpg");
        std::fs::write(&broken, b"invalid").unwrap();
        let mut metrics = DecodeMetrics::default();
        assert!(decode_preview(&broken, 96, &mut metrics).is_err());
        assert!(metrics.fallback_error.is_some());
        assert_eq!(metrics.error_stage, "header");
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn jpeg_fast_pass_uses_scaled_decode_and_preserves_aspect() {
        let path = std::env::temp_dir().join(format!("photag-dct-{}.jpg", std::process::id()));
        image::RgbImage::from_pixel(512, 384, image::Rgb([20, 80, 180]))
            .save(&path)
            .unwrap();
        let mut metrics = DecodeMetrics::default();
        let image = decode_preview(&path, 64, &mut metrics).unwrap();
        assert_eq!(metrics.source_dimensions, Some((512, 384)));
        assert_eq!(metrics.output_dimensions, Some((64, 48)));
        assert_eq!(metrics.error_stage, "none");
        assert_eq!((image.width(), image.height()), (64, 48));
        let _ = std::fs::remove_file(path);
    }
}

fn scaled_jpeg_bytes(
    path: &Path,
    bytes: &[u8],
    max_dim: u32,
    metrics: &mut DecodeMetrics,
) -> Result<image::DynamicImage, String> {
    use jpeg_decoder::{Decoder, PixelFormat};
    metrics.decoder = "jpeg_dct";
    metrics.error_stage = "header";
    let mut decoder = Decoder::new(std::io::Cursor::new(bytes));
    let started = std::time::Instant::now();
    let result = decoder.read_info().map_err(|error| error.to_string());
    metrics.header_ms += elapsed(started);
    result?;
    let info = decoder.info().ok_or("JPEG dimensions unavailable")?;
    metrics.source_dimensions = Some((info.width as u32, info.height as u32));
    let target = max_dim.clamp(1, u16::MAX as u32) as u16;
    let (width, height) = decoder
        .scale(target.min(info.width), target.min(info.height))
        .map_err(|error| error.to_string())?;
    metrics.error_stage = "decode";
    let started = std::time::Instant::now();
    let result = decoder.decode().map_err(|error| error.to_string());
    metrics.decode_ms += elapsed(started);
    let pixels = result?;
    let mut image = match decoder
        .info()
        .ok_or("JPEG pixel format unavailable")?
        .pixel_format
    {
        PixelFormat::RGB24 => image::RgbImage::from_raw(width as u32, height as u32, pixels)
            .map(image::DynamicImage::ImageRgb8),
        PixelFormat::L8 => image::GrayImage::from_raw(width as u32, height as u32, pixels)
            .map(image::DynamicImage::ImageLuma8),
        _ => None,
    }
    .ok_or("Unsupported scaled JPEG layout")?;
    let started = std::time::Instant::now();
    image.apply_orientation(PreviewService::read_orientation(path));
    metrics.orientation_ms += elapsed(started);
    Ok(image)
}
