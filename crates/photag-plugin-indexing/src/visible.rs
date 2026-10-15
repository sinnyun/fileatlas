use super::*;
impl IndexingService {
    /// Run on an owning service worker for the current viewport only.
    pub fn visible_thumbnail(
        &self,
        path: &str,
        kind: &str,
    ) -> Option<(String, Option<u32>, Option<u32>, Option<Vec<u8>>)> {
        let source = std::path::Path::new(path);
        if source.starts_with(self.thumbnails.directory()) {
            return None;
        }
        match kind {
            "image" => self
                .thumbnails
                .generate_image(source)
                .map(|(p, w, h, m)| (p, (w > 0).then_some(w), (h > 0).then_some(h), m)),
            "video" => {
                let target = photag_plugin_thumbnails::thumbs::thumb_path_for(
                    source,
                    self.thumbnails.directory(),
                );
                if !target.exists()
                    && !ffmpeg::extract_video_thumbnail(path, &target.to_string_lossy())
                {
                    return None;
                }
                self.thumbnails
                    .maintain_cache(Some(&target.to_string_lossy()));
                Some((target.to_string_lossy().into_owned(), None, None, None))
            }
            _ => None,
        }
    }
}
