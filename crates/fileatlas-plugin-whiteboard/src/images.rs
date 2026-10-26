//! Native ImageFn adapter: keyed background reads, linked sources, cached failures.
use fileatlas_plugin_media_resources::MediaResourceService;
use fileatlas_plugin_thumbnails::ThumbnailService;
use fileatlas_ui_bridge::{read_model::ReadModel, refresh::Wake};
use gpui::{ImageSource, RenderImage};
use std::{collections::HashMap, path::PathBuf, sync::Arc};

type Key = (String, i32);
pub struct ImageCache {
    models: HashMap<Key, ReadModel<Key, Arc<RenderImage>>>,
    thumbnails: Arc<ThumbnailService>,
    resources: Arc<MediaResourceService>,
    pub wake: Wake,
}
impl ImageCache {
    pub fn new(thumbnails: Arc<ThumbnailService>, resources: Arc<MediaResourceService>) -> Self {
        Self {
            models: HashMap::new(),
            thumbnails,
            resources,
            wake: Wake::default(),
        }
    }
    pub fn get(&mut self, source: &str, rotation: f32) -> Option<ImageSource> {
        // Upstream snaps images to quarter turns before calling ImageFn.
        let quarter = (rotation / std::f32::consts::FRAC_PI_2).round() as i32;
        let key = (source.to_owned(), quarter.rem_euclid(4));
        let model = self.models.entry(key.clone()).or_insert_with(|| {
            let mut m = ReadModel::default();
            m.set_wake(self.wake.clone());
            m
        });
        let source = source.to_owned();
        let thumbnails = Arc::clone(&self.thumbnails);
        let resources = Arc::clone(&self.resources);
        let turn = key.1;
        model.request(key, move || {
            let path = resources
                .resolve(&source)?
                .local_path
                .ok_or("image resource has no local preview source")?;
            let mut decoded = image::open(&path)
                .or_else(|error| {
                    let Some((path, ..)) = thumbnails.generate_image(&path) else {
                        return Err(error);
                    };
                    image::open(path)
                })
                .map_err(|e| format!("{source}: {e}"))?;
            if decoded.width().max(decoded.height()) > 4096 {
                decoded = decoded.thumbnail(4096, 4096);
            }
            let rotated = match turn {
                1 => decoded.rotate90(),
                2 => decoded.rotate180(),
                3 => decoded.rotate270(),
                _ => decoded,
            };
            Ok(device_image(rotated.into_rgba8()))
        });
        model
            .value()
            .and_then(|r| r.as_ref().ok())
            .cloned()
            .map(Into::into)
    }
    pub fn poll(&mut self) -> bool {
        self.models
            .values_mut()
            .fold(false, |changed, m| m.poll() | changed)
    }
    pub fn errors(&self) -> Vec<String> {
        self.models
            .values()
            .filter_map(|m| m.value().and_then(|v| v.as_ref().err()).cloned())
            .collect()
    }
    pub fn clear(&mut self) {
        self.models.clear();
    }
}
/// GPUI 0.3.8's RenderImage device contract is BGRA; decoders return RGBA.
fn device_image(mut pixels: image::RgbaImage) -> Arc<RenderImage> {
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    Arc::new(RenderImage::new(vec![image::Frame::new(pixels)]))
}
pub fn linked_images(
    paths: Vec<PathBuf>,
    thumbnails: &ThumbnailService,
    resources: &MediaResourceService,
) -> Result<Vec<(String, u32, u32)>, String> {
    paths
        .into_iter()
        .map(|path| {
            let link = resources.link_file(&path)?;
            if link.kind != fileatlas_contracts::resources::ResourceKind::Image {
                return Err("请选择图片文件".into());
            }
            let source = resources
                .resolve(&link.uri)?
                .local_path
                .ok_or("image resource has no local source")?;
            let dimensions = image::image_dimensions(&source)
                .or_else(|error| {
                    thumbnails
                        .generate_image(&source)
                        .map(|(_, w, h, _)| (w, h))
                        .ok_or(error)
                })
                .map_err(|e| format!("{}: {e}", source.display()))?;
            Ok((link.uri, dimensions.0, dimensions.1))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn image_callback_supplies_native_device_channels_without_changing_alpha() {
        let source = image::RgbaImage::from_pixel(1, 1, image::Rgba([10, 130, 250, 77]));
        let rendered = super::device_image(source);
        assert_eq!(rendered.as_bytes(0), Some([250u8, 130, 10, 77].as_slice()));
    }
}
