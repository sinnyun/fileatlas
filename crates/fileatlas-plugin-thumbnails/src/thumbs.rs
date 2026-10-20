use image::{GenericImageView, ImageDecoder};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub fn thumb_path_for(source: &Path, thumb_dir: &Path) -> PathBuf {
    let hash = {
        let mut hasher = Sha256::new();
        hasher.update(source.to_string_lossy().as_bytes());
        if let Ok(metadata) = std::fs::metadata(source) {
            hasher.update(metadata.len().to_le_bytes());
            if let Ok(modified) = metadata.modified().and_then(|time| {
                time.duration_since(std::time::UNIX_EPOCH)
                    .map_err(std::io::Error::other)
            }) {
                hasher.update(modified.as_nanos().to_le_bytes());
            }
        }
        let result = hasher.finalize();
        format!("{:x}", result)[..16].to_string()
    };
    thumb_dir.join(format!("{}.jpg", hash))
}

const THUMB_SIZE: u32 = 300;

// ══════════════════════════════════════════════════════════════════
//  Fast EXIF parser — extracts orientation + embedded thumbnail
// ══════════════════════════════════════════════════════════════════

struct ExifInfo {
    orientation: u8,
    thumb_jpeg: Option<Vec<u8>>,
}

fn parse_exif(data: &[u8]) -> ExifInfo {
    let default = ExifInfo {
        orientation: 1,
        thumb_jpeg: None,
    };
    if data.len() < 12 || data[0] != 0xFF || data[1] != 0xD8 {
        return default;
    }
    let mut pos = 2;
    while pos + 4 < data.len() {
        if data[pos] != 0xFF {
            return default;
        }
        let marker = data[pos + 1];
        if marker == 0xFF {
            pos += 1;
            continue;
        }
        if marker == 0x00 {
            pos += 2;
            continue;
        }
        if marker == 0xDA {
            break;
        }
        if pos + 4 > data.len() {
            break;
        }
        let seg_len = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
        if seg_len < 2 {
            break;
        }
        if marker == 0xE1 {
            let seg_start = pos + 4;
            let seg_end = (pos + 2 + seg_len).min(data.len());
            if seg_end - seg_start >= 14 && &data[seg_start..seg_start + 6] == b"Exif\0\0" {
                let tiff_start = seg_start + 6;
                let tiff = &data[tiff_start..seg_end];
                return parse_tiff_exif(tiff);
            }
        }
        pos += 2 + seg_len;
    }
    default
}

fn parse_tiff_exif(tiff: &[u8]) -> ExifInfo {
    let mut info = ExifInfo {
        orientation: 1,
        thumb_jpeg: None,
    };
    if tiff.len() < 8 {
        return info;
    }
    let le = tiff[0] == b'I' && tiff[1] == b'I';
    let read_u16 = |off: usize| -> u16 {
        if off + 2 > tiff.len() {
            return 0;
        }
        if le {
            u16::from_le_bytes([tiff[off], tiff[off + 1]])
        } else {
            u16::from_be_bytes([tiff[off], tiff[off + 1]])
        }
    };
    let read_u32 = |off: usize| -> u32 {
        if off + 4 > tiff.len() {
            return 0;
        }
        if le {
            u32::from_le_bytes([tiff[off], tiff[off + 1], tiff[off + 2], tiff[off + 3]])
        } else {
            u32::from_be_bytes([tiff[off], tiff[off + 1], tiff[off + 2], tiff[off + 3]])
        }
    };
    let ifd0_offset = read_u32(4) as usize;
    if ifd0_offset + 2 > tiff.len() {
        return info;
    }
    let count = read_u16(ifd0_offset) as usize;
    for i in 0..count {
        let entry = ifd0_offset + 2 + i * 12;
        if entry + 12 > tiff.len() {
            break;
        }
        if read_u16(entry) == 0x0112 {
            info.orientation = read_u16(entry + 8) as u8;
        }
    }
    let next_ifd_ptr = ifd0_offset + 2 + count * 12;
    if next_ifd_ptr + 4 > tiff.len() {
        return info;
    }
    let ifd1_offset = read_u32(next_ifd_ptr) as usize;
    if ifd1_offset == 0 || ifd1_offset + 2 > tiff.len() {
        return info;
    }
    let count1 = read_u16(ifd1_offset) as usize;
    let mut thumb_offset: u32 = 0;
    let mut thumb_length: u32 = 0;
    for i in 0..count1 {
        let entry = ifd1_offset + 2 + i * 12;
        if entry + 12 > tiff.len() {
            break;
        }
        match read_u16(entry) {
            0x0201 => {
                thumb_offset = read_u32(entry + 8);
            }
            0x0202 => {
                thumb_length = read_u32(entry + 8);
            }
            _ => {}
        }
    }
    if thumb_offset > 0 && thumb_length > 100 {
        let start = thumb_offset as usize;
        let end = start + thumb_length as usize;
        if end <= tiff.len() {
            let jpeg_data = &tiff[start..end];
            if jpeg_data.len() >= 2 && jpeg_data[0] == 0xFF && jpeg_data[1] == 0xD8 {
                info.thumb_jpeg = Some(jpeg_data.to_vec());
            }
        }
    }
    info
}

fn exif_to_orientation(val: u8) -> image::metadata::Orientation {
    use image::metadata::Orientation::*;
    match val {
        1 => NoTransforms,
        2 => FlipHorizontal,
        3 => Rotate180,
        4 => FlipVertical,
        5 => Rotate90FlipH,
        6 => Rotate90,
        7 => Rotate270FlipH,
        8 => Rotate270,
        _ => NoTransforms,
    }
}

/// Parse JPEG dimensions from SOF marker without full decode.
fn parse_jpeg_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return None;
    }
    let mut pos = 2;
    while pos + 4 < data.len() {
        if data[pos] != 0xFF {
            return None;
        }
        let marker = data[pos + 1];
        if (0xC0..=0xCF).contains(&marker) && marker != 0xC4 && marker != 0xC8 && marker != 0xCC {
            if pos + 9 > data.len() {
                return None;
            }
            let h = u16::from_be_bytes([data[pos + 5], data[pos + 6]]) as u32;
            let w = u16::from_be_bytes([data[pos + 7], data[pos + 8]]) as u32;
            if w > 0 && h > 0 {
                return Some((w, h));
            }
        }
        if marker == 0xDA {
            break;
        }
        let seg_len = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
        pos += 2 + seg_len;
    }
    None
}

// ══════════════════════════════════════════════════════════════════
//  Stage 1: EXIF embedded thumbnail (reads only first 65KB)
// ══════════════════════════════════════════════════════════════════

/// Try EXIF thumbnail by reading ONLY the first 65KB of the file.
/// Returns (oriented_thumbnail, original_w, original_h).
fn try_exif_thumbnail(source: &Path) -> Option<(image::DynamicImage, u32, u32)> {
    let mut file = std::fs::File::open(source).ok()?;
    use std::io::Read;
    let file_len = file.metadata().ok()?.len() as usize;
    let read_len = file_len.min(65 * 1024);
    let mut header = vec![0u8; read_len];
    file.read_exact(&mut header).ok()?;
    let data = &header[..];

    let exif = parse_exif(data);
    let thumb_data = exif.thumb_jpeg?;

    // Decode embedded thumbnail with zune-jpeg (tiny image, fast)
    use zune_jpeg::zune_core::colorspace::ColorSpace;
    use zune_jpeg::zune_core::options::DecoderOptions;
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGB);
    let mut decoder =
        zune_jpeg::JpegDecoder::new_with_options(std::io::Cursor::new(&thumb_data), options);
    decoder.decode_headers().ok()?;
    let info = decoder.info()?;
    let tw = info.width as u32;
    let th = info.height as u32;
    if tw.max(th) < 160 {
        return None;
    }
    let pixels = decoder.decode().ok()?;
    let img = image::RgbImage::from_raw(tw, th, pixels)?;
    let mut dyn_img = image::DynamicImage::ImageRgb8(img);
    dyn_img.apply_orientation(exif_to_orientation(exif.orientation));

    let (ow, oh) = parse_jpeg_dimensions(data).unwrap_or((tw, th));
    let (ow, oh) = if exif.orientation >= 5 {
        (oh, ow)
    } else {
        (ow, oh)
    };

    Some((dyn_img, ow, oh))
}

// ══════════════════════════════════════════════════════════════════
//  Stage 2: DCT-scaled JPEG decode (jpeg-decoder at 1/4 resolution)
//
//  Instead of decoding all 12M pixels (4000x3000) and then resizing,
//  this decodes directly at 1000x750 by computing only the low-
//  frequency DCT coefficients. 16x fewer pixels to reconstruct.
// ══════════════════════════════════════════════════════════════════

/// Decode JPEG at reduced resolution using DCT scaling.
/// Returns (scaled_image, original_w, original_h, exif_orientation).
fn decode_jpeg_dct_scaled(data: &[u8]) -> Option<(image::DynamicImage, u32, u32, u8)> {
    let exif = parse_exif(data);

    // Get original dimensions up front so we can request a 1/N scale.
    // jpeg-decoder picks the closest 1/N (1, 2, 4, 8) where the result
    // is >= the requested size.
    let (ow, oh) = parse_jpeg_dimensions(data)?;

    let mut decoder = jpeg_decoder::Decoder::new(std::io::Cursor::new(data));

    // Read headers so we know the dimensions and can call scale().
    decoder.read_info().ok()?;

    // Request ~1/4 dimensions. jpeg-decoder will round up to the nearest
    // supported 1/N scaling (1, 2, 4, 8) — passing ow/4 typically yields
    // the 1/4 scale path. If scale fails, the decoder will just decode
    // at full resolution.
    let req_w = ((ow / 4) as u16).max(1);
    let req_h = ((oh / 4) as u16).max(1);
    let _ = decoder.scale(req_w, req_h);

    let pixels = decoder.decode().ok()?;
    let info = decoder.info()?;
    let w = info.width as u32;
    let h = info.height as u32;

    // jpeg-decoder returns pixels based on component count
    let dyn_img = match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => {
            let img = image::RgbImage::from_raw(w, h, pixels)?;
            image::DynamicImage::ImageRgb8(img)
        }
        jpeg_decoder::PixelFormat::L8 => {
            let img = image::GrayImage::from_raw(w, h, pixels)?;
            image::DynamicImage::ImageLuma8(img)
        }
        jpeg_decoder::PixelFormat::CMYK32 => {
            // Convert CMYK to RGB
            let rgb: Vec<u8> = pixels
                .chunks_exact(4)
                .flat_map(|cmyk| {
                    let c = cmyk[0] as f32 / 255.0;
                    let m = cmyk[1] as f32 / 255.0;
                    let y = cmyk[2] as f32 / 255.0;
                    let k = cmyk[3] as f32 / 255.0;
                    let r = (255.0 * (1.0 - c) * (1.0 - k)) as u8;
                    let g = (255.0 * (1.0 - m) * (1.0 - k)) as u8;
                    let b = (255.0 * (1.0 - y) * (1.0 - k)) as u8;
                    [r, g, b]
                })
                .collect();
            let img = image::RgbImage::from_raw(w, h, rgb)?;
            image::DynamicImage::ImageRgb8(img)
        }
        _ => return None,
    };

    Some((dyn_img, ow, oh, exif.orientation))
}

// ══════════════════════════════════════════════════════════════════
//  Stage 3: zune-jpeg full decode fallback (for broken JPEGs)
// ══════════════════════════════════════════════════════════════════

fn decode_jpeg_zune_full(data: &[u8]) -> Option<(image::DynamicImage, u8)> {
    let exif = parse_exif(data);
    use zune_jpeg::zune_core::colorspace::ColorSpace;
    use zune_jpeg::zune_core::options::DecoderOptions;
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGB);
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(std::io::Cursor::new(data), options);
    decoder.decode_headers().ok()?;
    let info = decoder.info()?;
    let w = info.width as u32;
    let h = info.height as u32;
    let pixels = decoder.decode().ok()?;
    let img = image::RgbImage::from_raw(w, h, pixels)?;
    Some((image::DynamicImage::ImageRgb8(img), exif.orientation))
}

// ══════════════════════════════════════════════════════════════════
//  Non-JPEG fallback (PNG, WebP, GIF, etc.)
// ══════════════════════════════════════════════════════════════════

fn open_other_with_orientation(source: &Path) -> Option<image::DynamicImage> {
    let reader = image::ImageReader::open(source).ok()?;
    let mut decoder = reader.into_decoder().ok()?;
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut img = image::DynamicImage::from_decoder(decoder).ok()?;
    img.apply_orientation(orientation);
    Some(img)
}

// ══════════════════════════════════════════════════════════════════
//  Unified thumbnail generation pipeline
// ══════════════════════════════════════════════════════════════════

pub fn generate_micro_thumb(img: &image::DynamicImage) -> Vec<u8> {
    let tiny = img.thumbnail(8, 8);
    let rgb = tiny.to_rgb8();
    rgb.into_raw()
}

/// Create a properly-sized thumbnail from an image, applying orientation.
fn make_thumbnail(img: image::DynamicImage, orientation: u8) -> image::DynamicImage {
    // Resize FIRST (cheap on already-small DCT-scaled image),
    // THEN apply orientation (on tiny 300px image instead of full-res).
    let thumb = img.thumbnail(THUMB_SIZE, THUMB_SIZE);
    let mut oriented = thumb;
    oriented.apply_orientation(exif_to_orientation(orientation));
    oriented
}

/// Generate thumbnail. Returns (thumb_path, original_w, original_h, micro_thumb_bytes).
pub fn generate_thumbnail_with_dims(
    source: &Path,
    thumb_dir: &Path,
) -> Option<(String, u32, u32, Option<Vec<u8>>)> {
    let thumb_path = thumb_path_for(source, thumb_dir);

    // === Already generated (e.g. rescan or interrupted scan) ===
    if thumb_path.exists() {
        let micro = image::open(&thumb_path)
            .ok()
            .map(|img| generate_micro_thumb(&img));
        let (w, h) = image::ImageReader::open(source)
            .ok()
            .and_then(|r| r.with_guessed_format().ok())
            .and_then(|r| r.into_dimensions().ok())
            .unwrap_or((0, 0));
        return Some((thumb_path.to_string_lossy().to_string(), w, h, micro));
    }

    // === Legacy .webp conversion ===
    let legacy_webp = {
        let hash = {
            let mut hasher = Sha256::new();
            hasher.update(source.to_string_lossy().as_bytes());
            let result = hasher.finalize();
            format!("{:x}", result)[..16].to_string()
        };
        thumb_dir.join(format!("{}.webp", hash))
    };
    if legacy_webp.exists()
        && legacy_webp.metadata().and_then(|m| m.modified()).ok()
            >= source.metadata().and_then(|m| m.modified()).ok()
    {
        if let Ok(img) = image::open(&legacy_webp) {
            let micro = generate_micro_thumb(&img);
            if img.save(&thumb_path).is_ok() {
                std::fs::remove_file(&legacy_webp).ok();
                let dims = image::image_dimensions(source).ok()?;
                return Some((
                    thumb_path.to_string_lossy().to_string(),
                    dims.0,
                    dims.1,
                    Some(micro),
                ));
            }
        }
    }

    let ext = source
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    if ext == "jpg" || ext == "jpeg" {
        return generate_jpeg_thumbnail(source, &thumb_path);
    }

    // === Non-JPEG: use image crate ===
    match open_other_with_orientation(source) {
        Some(img) => {
            let (w, h) = img.dimensions();
            let thumbnail = img.thumbnail(THUMB_SIZE, THUMB_SIZE);
            let micro = generate_micro_thumb(&thumbnail);
            match thumbnail.save(&thumb_path) {
                Ok(_) => Some((thumb_path.to_string_lossy().to_string(), w, h, Some(micro))),
                Err(_) => Some(("".to_string(), w, h, Some(micro))),
            }
        }
        None => image::image_dimensions(source)
            .ok()
            .map(|(w, h)| ("".to_string(), w, h, None)),
    }
}

/// JPEG thumbnail pipeline — tries 3 stages in order:
///
/// 1. **EXIF embedded thumbnail** (reads 65KB, decodes ~160x120)
///    → ~5ms on HDD. Works for most camera/phone photos.
///
/// 2. **DCT-scaled decode at 1/4** (reads full file, decodes 1000x750)
///    → ~80ms on HDD. 3-4x faster than full decode because jpeg-decoder
///    skips 15/16ths of the IDCT work during decompression.
///
/// 3. **zune-jpeg full decode** (fallback for unusual JPEGs)
///    → ~200ms on HDD. Last resort.
fn generate_jpeg_thumbnail(
    source: &Path,
    thumb_path: &Path,
) -> Option<(String, u32, u32, Option<Vec<u8>>)> {
    // ── Stage 1: EXIF embedded thumbnail (65KB header read only) ──
    // CRITICAL: Most camera/phone JPEGs have an embedded thumbnail in the
    // first 64KB. Reading just the header (~65KB) instead of the full
    // 10-30MB file is the difference between thousands/sec and dozens/sec.
    // Do NOT lift this read up to a "read once" full-file load — that
    // multiplies HDD I/O by 100-500x for the common case.
    if let Some((exif_thumb, ow, oh)) = try_exif_thumbnail(source) {
        let thumbnail = if exif_thumb.width() > THUMB_SIZE || exif_thumb.height() > THUMB_SIZE {
            exif_thumb.thumbnail(THUMB_SIZE, THUMB_SIZE)
        } else {
            exif_thumb
        };
        let micro = generate_micro_thumb(&thumbnail);
        match thumbnail.save(thumb_path) {
            Ok(_) => {
                return Some((
                    thumb_path.to_string_lossy().to_string(),
                    ow,
                    oh,
                    Some(micro),
                ));
            }
            Err(_) => return Some(("".to_string(), ow, oh, Some(micro))),
        }
    }

    // ── Stage 2+3: EXIF missed, fall back to full-file decode ───
    let data = std::fs::read(source).ok()?;

    // Stage 2: DCT 1/4 scaled decode (jpeg-decoder)
    if let Some((scaled_img, ow, oh, orientation)) = decode_jpeg_dct_scaled(&data) {
        let thumbnail = make_thumbnail(scaled_img, orientation);
        let (ow, oh) = if orientation >= 5 { (oh, ow) } else { (ow, oh) };
        let micro = generate_micro_thumb(&thumbnail);
        match thumbnail.save(thumb_path) {
            Ok(_) => {
                return Some((
                    thumb_path.to_string_lossy().to_string(),
                    ow,
                    oh,
                    Some(micro),
                ));
            }
            Err(_) => return Some(("".to_string(), ow, oh, Some(micro))),
        }
    }

    // Stage 3: zune-jpeg full decode fallback
    if let Some((full_img, orientation)) = decode_jpeg_zune_full(&data) {
        let (w, h) = full_img.dimensions();
        let thumbnail = make_thumbnail(full_img, orientation);
        let (w, h) = if orientation >= 5 { (h, w) } else { (w, h) };
        let micro = generate_micro_thumb(&thumbnail);
        match thumbnail.save(thumb_path) {
            Ok(_) => return Some((thumb_path.to_string_lossy().to_string(), w, h, Some(micro))),
            Err(_) => return Some(("".to_string(), w, h, Some(micro))),
        }
    }

    // Couldn't decode at all — at least get dimensions
    image::image_dimensions(source)
        .ok()
        .map(|(w, h)| ("".to_string(), w, h, None))
}
