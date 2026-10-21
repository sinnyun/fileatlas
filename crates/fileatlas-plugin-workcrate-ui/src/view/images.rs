use super::*;

/// GPUI 0.3.8 RenderImage's device contract is BGRA, matching its built-in
/// image loader in gpui-pre/src/elements/img.rs. Decoded official PNG pixels remain RGBA.
pub(super) fn render_image(mut pixels: image::RgbaImage) -> Arc<RenderImage> {
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    Arc::new(RenderImage::new(vec![image::Frame::new(pixels)]))
}

#[cfg(test)]
mod tests {
    #[test]
    fn native_adapter_retains_source_channels_and_alpha() {
        let source = image::RgbaImage::from_pixel(1, 1, image::Rgba([10, 130, 250, 77]));
        let device = super::render_image(source);
        assert_eq!(device.as_bytes(0), Some([250u8, 130, 10, 77].as_slice()));
    }
}
