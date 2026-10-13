//! Cordis service wrapper around PhoTag's original image thumbnail pipeline.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use cordis::{Context, Plugin, Service};

pub mod thumbs;

/// Shared thumbnail generation service available to feature plugins.
pub struct ThumbnailService {
    directory: PathBuf,
}

impl Service for ThumbnailService {
    const NAME: &'static str = "photag/thumbnails";
}

impl ThumbnailService {
    /// Create a thumbnail service with an explicit cache directory.
    pub fn open_at(directory: impl Into<PathBuf>) -> std::io::Result<Self> {
        let directory = directory.into();
        std::fs::create_dir_all(&directory)?;
        Ok(Self { directory })
    }

    /// Directory holding PhoTag's path-hash named JPEG thumbnail cache.
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Run the unchanged PhoTag thumbnail pipeline and return cache path,
    /// source dimensions, and the 8-pixel micro-thumbnail bytes.
    pub fn generate_image(
        &self,
        source: impl AsRef<Path>,
    ) -> Option<(String, u32, u32, Option<Vec<u8>>)> {
        thumbs::generate_thumbnail_with_dims(source.as_ref(), &self.directory)
    }
}

/// Optional cache location. `None` uses `%LOCALAPPDATA%/PhoTag/thumbs`.
#[derive(Debug, Clone, Default)]
pub struct ThumbnailConfig {
    pub directory: Option<PathBuf>,
}

/// Initializes the thumbnail cache and publishes its API for the plugin fiber.
pub struct ThumbnailPlugin;

impl Plugin for ThumbnailPlugin {
    type Config = ThumbnailConfig;
    type Input = ThumbnailConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = ThumbnailPluginError;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        "photag.thumbnails".into()
    }

    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }

    async fn apply(&self, context: Context, config: &Self::Input) -> Result<(), Self::ApplyError> {
        let directory = config.directory.clone().unwrap_or_else(default_thumb_dir);
        let service = Arc::new(ThumbnailService::open_at(directory)?);
        let _publication = context
            .provide(service)
            .map_err(|error| ThumbnailPluginError::Publish(error.to_string()))?;
        Ok(())
    }
}

fn default_thumb_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("PhoTag")
        .join("thumbs")
}

/// Error while initializing the cache service.
#[derive(Debug, thiserror::Error)]
pub enum ThumbnailPluginError {
    #[error("thumbnail cache directory could not be created: {0}")]
    Io(#[from] std::io::Error),
    #[error("thumbnail service publication failed: {0}")]
    Publish(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    #[test]
    fn original_pipeline_creates_and_reuses_cached_thumbnail() {
        let root = std::env::temp_dir().join(format!(
            "photag-thumbnails-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let source = root.join("source.png");
        let cache = root.join("cache");
        std::fs::create_dir_all(&cache).unwrap();
        ImageBuffer::from_pixel(48, 24, Rgb([24u8, 120, 220]))
            .save(&source)
            .unwrap();
        let service = ThumbnailService { directory: cache };

        let first = service.generate_image(&source).unwrap();
        assert_eq!((first.1, first.2), (48, 24));
        assert!(Path::new(&first.0).is_file());
        assert!(first.3.is_some());

        let cached = service.generate_image(&source).unwrap();
        assert_eq!((cached.1, cached.2), (0, 0));
        assert!(cached.3.is_some());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn original_pipeline_decodes_common_still_image_formats() {
        let root = std::env::temp_dir().join(format!(
            "photag-thumbnail-formats-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let cache = root.join("cache");
        std::fs::create_dir_all(&cache).unwrap();
        let service = ThumbnailService { directory: cache };
        let pixels = ImageBuffer::from_fn(64, 40, |x, y| {
            Rgb([(x * 3) as u8, (y * 5) as u8, ((x + y) * 2) as u8])
        });

        for (extension, format) in [
            ("jpg", image::ImageFormat::Jpeg),
            ("png", image::ImageFormat::Png),
            ("webp", image::ImageFormat::WebP),
            ("tiff", image::ImageFormat::Tiff),
            ("bmp", image::ImageFormat::Bmp),
        ] {
            let source = root.join(format!("sample.{extension}"));
            pixels.save_with_format(&source, format).unwrap();
            let thumbnail = service
                .generate_image(&source)
                .unwrap_or_else(|| panic!("thumbnail generation failed for {extension}"));
            assert_eq!((thumbnail.1, thumbnail.2), (64, 40), "{extension}");
            assert!(Path::new(&thumbnail.0).is_file(), "{extension}");
        }

        let _ = std::fs::remove_dir_all(root);
    }
}
