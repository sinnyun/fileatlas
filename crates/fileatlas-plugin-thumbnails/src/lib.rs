//! Cordis service wrapper around the original PhoTag image thumbnail pipeline.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use cordis::{Context, Plugin, Service};

mod budget;
mod thumbs;

/// Shared thumbnail generation service available to feature plugins.
pub struct ThumbnailService {
    directory: PathBuf,
    accesses: std::sync::atomic::AtomicUsize,
    system_cache_revision: std::sync::atomic::AtomicU64,
}

impl Service for ThumbnailService {
    const NAME: &'static str = "fileatlas/thumbnails";
}

impl ThumbnailService {
    /// Create a thumbnail service with an explicit cache directory.
    pub fn open_at(directory: impl Into<PathBuf>) -> std::io::Result<Self> {
        let directory = directory.into();
        std::fs::create_dir_all(&directory)?;
        Ok(Self {
            directory,
            accesses: Default::default(),
            system_cache_revision: Default::default(),
        })
    }

    /// Directory holding FileAtlas's path-hash named JPEG thumbnail cache.
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Remove selected Shell transport copies and advance their path revision,
    /// so GPUI cannot reuse a decoded image keyed by the previous path.
    pub fn refresh_system_cache_paths(&self, paths: &[String]) -> std::io::Result<usize> {
        self.system_cache_revision
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        let cache_root = self.directory.join("system-shell");
        let normalized_root = format!(
            "{}\\",
            cache_root
                .to_string_lossy()
                .replace('/', "\\")
                .trim_end_matches('\\')
                .to_lowercase()
        );
        let mut removed = 0;
        for value in paths {
            let path = Path::new(value);
            let normalized_path = path.to_string_lossy().replace('/', "\\").to_lowercase();
            if !normalized_path.starts_with(&normalized_root) {
                continue;
            }
            match std::fs::remove_file(path) {
                Ok(()) => removed += 1,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(removed)
    }

    /// Clear the FileAtlas-owned Shell transport directory only. Other
    /// thumbnail subdirectories contain durable feature previews.
    pub fn clear_system_cache(&self) -> std::io::Result<usize> {
        self.system_cache_revision
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        let path = self.directory.join("system-shell");
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
            return Ok(0);
        }
        let removed = std::fs::read_dir(&path)?.count();
        std::fs::remove_dir_all(&path)?;
        std::fs::create_dir_all(path)?;
        Ok(removed)
    }

    /// Current cache key revision, used by the Windows Shell copy writer.
    pub(crate) fn system_cache_revision(&self) -> u64 {
        self.system_cache_revision
            .load(std::sync::atomic::Ordering::Acquire)
    }

    /// Reads a thumbnail already present in the Windows Shell thumbnail cache.
    /// The bitmap is copied losslessly into FileAtlas's transport cache for GPUI;
    /// this method never decodes the source file or asks Shell to create a thumbnail.
    #[cfg(windows)]
    pub fn system_thumbnail(&self, source: impl AsRef<Path>) -> Option<(String, u32, u32)> {
        let result = system::cached_thumbnail(
            source.as_ref(),
            &self.directory,
            self.system_cache_revision(),
        );
        if self
            .accesses
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            % 256
            == 255
        {
            budget::prune_system(
                &self.directory,
                result.as_ref().map(|value| value.0.as_str()),
            );
        }
        result
    }

    #[cfg(not(windows))]
    pub fn system_thumbnail(&self, _source: impl AsRef<Path>) -> Option<(String, u32, u32)> {
        None
    }

    /// Run the unchanged FileAtlas thumbnail pipeline and return cache path,
    /// source dimensions, and the 8-pixel micro-thumbnail bytes.
    #[tracing::instrument(skip_all)]
    pub fn generate_image(
        &self,
        source: impl AsRef<Path>,
    ) -> Option<(String, u32, u32, Option<Vec<u8>>)> {
        let result = thumbs::generate_thumbnail_with_dims(source.as_ref(), &self.directory);
        self.maintain_cache(result.as_ref().map(|v| v.0.as_str()));
        result
    }

    /// Return the cache path used for a video poster thumbnail.
    pub fn video_thumbnail_path(&self, source: impl AsRef<Path>) -> PathBuf {
        thumbs::thumb_path_for(source.as_ref(), &self.directory)
    }

    /// Produce FileAtlas's compact card thumbnail bytes from an already-decoded image.
    pub fn generate_micro_thumbnail(&self, image: &image::DynamicImage) -> Vec<u8> {
        thumbs::generate_micro_thumb(image)
    }

    /// Called on the requesting worker after image or video thumbnail demand.
    pub fn maintain_cache(&self, protected: Option<&str>) {
        if self
            .accesses
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            % 256
            == 255
        {
            budget::prune(&self.directory, protected);
        }
    }
}

#[cfg(windows)]
mod system;

/// Optional cache location. `None` uses `%LOCALAPPDATA%/FileAtlas/thumbs`.
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
        "fileatlas.thumbnails".into()
    }

    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }

    #[tracing::instrument(skip_all, err)]
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
        .join("FileAtlas")
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
            "fileatlas-thumbnails-{}-{}",
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
        let service = ThumbnailService::open_at(cache).unwrap();

        let first = service.generate_image(&source).unwrap();
        assert_eq!((first.1, first.2), (48, 24));
        assert!(Path::new(&first.0).is_file());
        assert!(first.3.is_some());

        let cached = service.generate_image(&source).unwrap();
        assert_eq!((cached.1, cached.2), (48, 24));
        assert!(cached.3.is_some());
        ImageBuffer::from_pixel(64, 32, Rgb([220u8, 60, 20]))
            .save(&source)
            .unwrap();
        let replaced = service.generate_image(&source).unwrap();
        assert_ne!(replaced.0, cached.0);
        assert_eq!((replaced.1, replaced.2), (64, 32));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn system_cache_clear_keeps_other_feature_owned_cache_directories()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!(
            "fileatlas-system-cache-scope-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        let service = ThumbnailService::open_at(&root)?;
        let shell = root.join("system-shell");
        let boards = root.join("boards");
        std::fs::create_dir_all(&shell)?;
        std::fs::create_dir_all(&boards)?;
        std::fs::write(shell.join("a.png"), b"shell")?;
        std::fs::write(shell.join("b.png"), b"shell")?;
        std::fs::write(boards.join("project.png"), b"persistent")?;

        assert_eq!(
            service.refresh_system_cache_paths(&[
                shell.join("a.png").to_string_lossy().into_owned(),
                boards.join("project.png").to_string_lossy().into_owned(),
            ])?,
            1
        );
        assert_eq!(service.system_cache_revision(), 1);
        assert!(!shell.join("a.png").exists());
        assert!(shell.join("b.png").is_file());
        assert!(boards.join("project.png").is_file());
        assert_eq!(service.clear_system_cache()?, 1);
        assert!(boards.join("project.png").is_file());
        assert!(shell.is_dir());
        let _ = std::fs::remove_dir_all(root);
        Ok(())
    }

    #[test]
    fn original_pipeline_decodes_common_still_image_formats() {
        let root = std::env::temp_dir().join(format!(
            "fileatlas-thumbnail-formats-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let cache = root.join("cache");
        std::fs::create_dir_all(&cache).unwrap();
        let service = ThumbnailService::open_at(cache).unwrap();
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
