use super::*;

impl PreviewView {
    pub(super) fn poll_reads(&mut self) -> bool {
        let file_changed = self
            .file_revision_gate
            .changed(self.service.file_revision());
        if file_changed {
            self.file_info_cache = Default::default();
            self.exif_cache = Default::default();
            self.video_info_cache = Default::default();
            self.display_path_cache = Default::default();
            self.image_dimensions_cache = Default::default();
        }
        let index_changed = self
            .index_revision_gate
            .changed(self.service.data_revision());
        if file_changed || index_changed {
            self.indexed_info_cache = Default::default();
            self.tag_picker_read = Default::default();
            self.tag_tree_cache = None;
            self.tag_picker_cache = None;
        }
        let mut changed = file_changed
            | index_changed
            | self.video_info_cache.poll()
            | self.file_info_cache.poll()
            | self.exif_cache.poll()
            | self.display_path_cache.poll()
            | self.image_dimensions_cache.poll()
            | self.indexed_info_cache.poll();
        if self.tag_picker_read.poll() {
            if let (Some(id), Some(Ok((tree, assigned)))) =
                (self.tag_picker_read.key(), self.tag_picker_read.value())
            {
                self.tag_tree_cache = Some(tree.clone());
                self.tag_picker_cache = Some((*id, assigned.clone()));
            }
            changed = true;
        }
        changed
    }

    pub(super) fn request_video_info(
        &mut self,
        path: std::path::PathBuf,
        _: &mut Context<Self>,
    ) -> Option<crate::info::VideoProbe> {
        let source = path.clone();
        self.video_info_cache
            .request(path, move || Ok(crate::info::probe_video(&source)));
        self.video_info_cache
            .value()
            .and_then(|v| v.as_ref().ok())
            .cloned()
            .flatten()
    }

    pub(super) fn request_file_info(
        &mut self,
        path: std::path::PathBuf,
        _: &mut Context<Self>,
    ) -> Option<crate::info::FileInfo> {
        let source = path.clone();
        self.file_info_cache
            .request(path, move || Ok(crate::info::read_file_info(&source)));
        self.file_info_cache
            .value()
            .and_then(|v| v.as_ref().ok())
            .cloned()
            .flatten()
    }

    pub(super) fn request_exif(
        &mut self,
        path: std::path::PathBuf,
        _: &mut Context<Self>,
    ) -> Option<Result<Vec<(String, String)>, String>> {
        let source = path.clone();
        self.exif_cache.request(path, move || {
            crate::info::read_exif(&source).map_err(str::to_owned)
        });
        self.exif_cache.value().cloned()
    }

    pub(super) fn request_display_path(
        &mut self,
        snapshot: &crate::PreviewSnapshot,
        _: &mut Context<Self>,
    ) -> Option<Result<Option<std::path::PathBuf>, String>> {
        let source = snapshot.source.as_ref()?;
        if snapshot.quarter_turns == 0 && !snapshot.flip_horizontal && !snapshot.flip_vertical {
            self.display_path_cache = Default::default();
            return Some(Ok(Some(source.clone())));
        }
        let key = DisplayPathKey {
            source: source.clone(),
            quarter_turns: snapshot.quarter_turns,
            flip_horizontal: snapshot.flip_horizontal,
            flip_vertical: snapshot.flip_vertical,
        };
        let service = Arc::clone(&self.service);
        self.display_path_cache.request(key, move || {
            service.display_path().map_err(|e| e.to_string())
        });
        self.display_path_cache
            .value()
            .cloned()
            .or_else(|| Some(Ok(Some(source.clone()))))
    }

    pub(super) fn request_tag_picker_data(&mut self, media_id: i64, _: &mut Context<Self>) {
        if self
            .tag_picker_cache
            .as_ref()
            .is_some_and(|(id, _)| *id == media_id)
            && self.tag_tree_cache.is_some()
        {
            return;
        }
        let tagging = Arc::clone(&self.tagging);
        self.tag_picker_read.request(media_id, move || {
            let tree = tagging.tag_tree().map_err(|e| e.to_string())?;
            let assigned = tagging
                .for_media(media_id)
                .map_err(|e| e.to_string())?
                .into_iter()
                .map(|tag| tag.id)
                .collect();
            Ok((tree, assigned))
        });
    }

    pub(super) fn request_image_dimensions(
        &mut self,
        path: std::path::PathBuf,
        _: &mut Context<Self>,
    ) -> Option<(u32, u32)> {
        let source = path.clone();
        self.image_dimensions_cache
            .request(path, move || Ok(image::image_dimensions(source).ok()));
        self.image_dimensions_cache
            .value()
            .and_then(|v| v.as_ref().ok())
            .copied()
            .flatten()
    }

    pub(super) fn request_indexed_info(
        &mut self,
        media_id: i64,
        _: &mut Context<Self>,
    ) -> Option<crate::info::IndexedInfo> {
        let service = Arc::clone(&self.service);
        self.indexed_info_cache.request(media_id, move || {
            Ok(support::indexed_info(&service, media_id))
        });
        self.indexed_info_cache
            .value()
            .and_then(|v| v.as_ref().ok())
            .cloned()
            .flatten()
    }
}
