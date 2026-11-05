use super::*;
impl IndexingService {
    /// Run on an owning service worker for the current viewport only.
    pub fn visible_thumbnail(
        &self,
        path: &str,
        _kind: &str,
    ) -> Option<(String, Option<u32>, Option<u32>, Option<Vec<u8>>)> {
        let source = std::path::Path::new(path);
        if source.starts_with(self.thumbnails.directory()) {
            return None;
        }
        self.thumbnails
            .system_thumbnail(source)
            .map(|(path, width, height)| (path, Some(width), Some(height), None))
    }
}
