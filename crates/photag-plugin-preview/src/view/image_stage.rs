//! Select ready native image data: thumbnail -> progressive -> full raster.
use super::*;

impl PreviewView {
    pub(super) fn render_ready_image(
        &mut self,
        path: std::path::PathBuf,
        snapshot: &crate::PreviewSnapshot,
        fit_box: (f32, f32),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (AnyElement, f32) {
        let (generation, thumbnail) = self.service.presentation_source();
        let resource: gpui::Resource = path.into();
        let full = window.use_asset::<gpui::ImgResourceLoader>(&resource, cx);
        let mut failure = None;
        let ready = match full {
            Some(Ok(image)) => Some((
                image,
                if !self.display_path_cache.is_pending() {
                    "full"
                } else {
                    "original"
                },
            )),
            result => {
                if let Some(Err(error)) = result {
                    let message = error.to_string();
                    self.service.image_load_failed(generation, message.clone());
                    failure = Some(message);
                }
                let progressive = snapshot
                    .source
                    .as_deref()
                    .and_then(|source| self.service.progressive_path(source));
                [(progressive, "progressive"), (thumbnail, "thumbnail")]
                    .into_iter()
                    .find_map(|(path, quality)| {
                        let resource: gpui::Resource = path?.into();
                        window
                            .use_asset::<gpui::ImgResourceLoader>(&resource, cx)
                            .and_then(Result::ok)
                            .map(|image| (image, quality))
                    })
            }
        };
        let Some((image, quality)) = ready else {
            return (
                div()
                    .text_sm()
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child(
                        failure.unwrap_or_else(|| photag_ui_bridge::i18n::tr("Loading file info…")),
                    )
                    .into_any_element(),
                1.0,
            );
        };
        let dimensions = image.size(0);
        let (width, height) = (
            dimensions.width.0.max(1) as f32,
            dimensions.height.0.max(1) as f32,
        );
        let raw_fit = (fit_box.0 / width).min(fit_box.1 / height);
        let fit_scale = if matches!(quality, "thumbnail" | "progressive") {
            raw_fit
        } else {
            raw_fit.min(1.0)
        };
        let scale = if snapshot.fit_to_window {
            fit_scale
        } else {
            snapshot.zoom_scale
        };
        let service = Arc::clone(&self.service);
        // The marker runs in paint, after the decoded image's draw command,
        // rather than when the open handler or disk decoder merely returns.
        let painted = gpui::canvas(
            |_, _, _| (),
            move |_, (), _, _| {
                service.image_presented(generation, quality, (width as u32, height as u32));
            },
        )
        .absolute()
        .inset_0();
        (
            div()
                .relative()
                .w(px(width * scale))
                .h(px(height * scale))
                .left(px(snapshot.pan_x))
                .top(px(snapshot.pan_y))
                .child(img(image).size_full().object_fit(ObjectFit::Contain))
                .child(painted)
                .into_any_element(),
            fit_scale,
        )
    }
}
