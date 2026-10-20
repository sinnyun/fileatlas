use std::path::{Path, PathBuf};

use image::{DynamicImage, ImageBuffer, ImageFormat, Rgb};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-data/fileatlas-fixtures");
    let small = root.join("01-small");
    let mixed = root.join("02-mixed-formats");
    let nested = root.join("03-nested/level-one/level-two");
    let no_thumbnail = root.join("04-no-thumbnail");
    let support = root.join(".dev-support");
    for directory in [&small, &mixed, &nested, &no_thumbnail, &support] {
        std::fs::create_dir_all(directory)?;
    }

    save_image(&small.join("tiny.jpg"), 24, 16, ImageFormat::Jpeg)?;
    save_image(&small.join("small.png"), 180, 120, ImageFormat::Png)?;
    save_image(&mixed.join("medium.webp"), 640, 480, ImageFormat::WebP)?;
    save_image(&mixed.join("medium.gif"), 320, 240, ImageFormat::Gif)?;
    save_image(&mixed.join("large.tiff"), 1280, 960, ImageFormat::Tiff)?;
    save_image(
        &nested.join("nested-large.bmp"),
        1024,
        768,
        ImageFormat::Bmp,
    )?;
    save_image(
        &nested.join("nested-photo.jpeg"),
        900,
        600,
        ImageFormat::Jpeg,
    )?;
    save_image(&support.join("preview.png"), 96, 64, ImageFormat::Png)?;

    std::fs::write(
        no_thumbnail.join("broken-image.jpg"),
        b"deliberately invalid JPEG fixture for missing-thumbnail handling",
    )?;
    write_wave(&no_thumbnail.join("sample.wav"))?;
    std::fs::write(
        root.join("README.txt"),
        "FileAtlas development media fixtures. See test-data/README.md.\n",
    )?;
    println!("Generated fixtures under {}", root.display());
    Ok(())
}

fn save_image(
    path: &Path,
    width: u32,
    height: u32,
    format: ImageFormat,
) -> Result<(), image::ImageError> {
    let image = ImageBuffer::from_fn(width, height, |x, y| {
        let red = ((x * 255) / width.max(1)) as u8;
        let green = ((y * 255) / height.max(1)) as u8;
        let blue = ((x.wrapping_add(y) * 13) % 256) as u8;
        Rgb([red, green, blue])
    });
    DynamicImage::ImageRgb8(image).save_with_format(path, format)
}

fn write_wave(path: &Path) -> std::io::Result<()> {
    let sample_rate = 8_000u32;
    let samples = 2_000u32;
    let data_size = samples * 2;
    let mut bytes = Vec::with_capacity((44 + data_size) as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());
    for index in 0..samples {
        let phase = index as f32 * std::f32::consts::TAU * 440.0 / sample_rate as f32;
        let sample = (phase.sin() * i16::MAX as f32 * 0.2) as i16;
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    std::fs::write(path, bytes)
}
