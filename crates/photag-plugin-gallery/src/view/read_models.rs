use super::*;
use photag_plugin_database::db::queries::{LibraryStats, MediaQuery};

pub(super) type StatisticsRead = (LibraryStats, Result<i64, String>);

impl GalleryView {
    pub(super) fn dialog_read_status(&self) -> crate::ui_requests::GalleryDialogReads {
        let mut status = crate::ui_requests::GalleryDialogReads {
            statistics_state: "loading",
            bulk_state: "loading",
            ..Default::default()
        };
        match self.statistics_read.value() {
            Some(Ok((stats, audio))) => {
                status.statistics_state = "ready";
                status.photos = Some(stats.total_photos);
                status.rating_counts = Some(stats.rating_counts);
                status.videos = Some(stats.total_videos);
                status.audio = audio.as_ref().ok().copied();
            }
            Some(Err(error)) => {
                status.statistics_state = "failed";
                status.statistics_error = Some(error.clone());
            }
            None => {}
        }
        match self.bulk_read.value() {
            Some(Ok((ids, media))) => {
                status.bulk_state = "ready";
                status.bulk_count = Some(ids.len());
                status.bulk_preview_ids = media.iter().map(|m| m.id).collect();
            }
            Some(Err(error)) => {
                status.bulk_state = "failed";
                status.bulk_error = Some(error.clone());
            }
            None => {}
        }
        status
    }
    pub(super) fn poll_dialog_reads(&mut self, cx: &mut Context<Self>) -> bool {
        let mut changed = self.poll_exif_read();
        if self.inspector_read.poll() {
            if let Some(Ok(info)) = self.inspector_read.value() {
                if self.selection.selected_media_ids.first() == Some(&info.media_id) {
                    self.inspector_fallback = Some(info.clone());
                }
            }
            changed = true;
        }
        if self.selection_details_read.poll() {
            if let (Some(id), Some(Ok((count, metadata)))) = (
                self.selection_details_read.key(),
                self.selection_details_read.value(),
            ) {
                if self.selection.selected_media_ids.as_slice() == [*id] {
                    self.selection_details_cache = Some((*id, *count, metadata.clone()));
                }
            }
            changed = true;
        }
        if self.selected_drag_read.poll() {
            if let (Some(ids), Some(Ok(paths))) = (
                self.selected_drag_read.key(),
                self.selected_drag_read.value(),
            ) {
                if &self.selection.selected_media_ids == ids {
                    self.selected_drag_paths_cache = Some((ids.clone(), paths.clone()));
                }
            }
            changed = true;
        }
        if self.show_statistics {
            let service = Arc::clone(&self.service);
            if self.statistics_read.update(
                (self.observed_index_revision, self.service.data_revision()),
                move || {
                    let stats = service.statistics().map_err(|error| error.to_string())?;
                    let audio = service
                        .load(MediaQuery {
                            media_type: Some("audio".into()),
                            ..Default::default()
                        })
                        .map(|snapshot| {
                            snapshot
                                .skeletons
                                .iter()
                                .filter(|media| media.is_audio)
                                .count() as i64
                        })
                        .map_err(|error| error.to_string());
                    Ok((stats, audio))
                },
            ) {
                changed = true;
            }
        } else {
            self.statistics_read = Default::default();
        }
        if self.bulk_action.is_some() {
            changed |= bulk_actions::poll_matches(self, cx);
        } else {
            self.bulk_read = Default::default();
        }
        changed
    }

    fn poll_exif_read(&mut self) -> bool {
        if self.selection.selected_media_ids.len() != 1
            || !(self.show_exif || self.inspector_expanded.contains("exif"))
        {
            self.exif_read = Default::default();
            self.exif_cache = None;
            return false;
        }
        let media_id = self.selection.selected_media_ids[0];
        let path = self
            .snapshot
            .as_ref()
            .ok()
            .and_then(|snapshot| snapshot.media.iter().find(|media| media.id == media_id))
            .or_else(|| {
                self.inspector_fallback(media_id)
                    .and_then(|fallback| fallback.media.as_ref())
            })
            .map(|media| media.full_path.clone());
        let service = Arc::clone(&self.service);
        let changed = self.exif_read.update(
            (
                media_id,
                self.observed_index_revision,
                self.service.data_revision(),
            ),
            move || {
                let path = match path {
                    Some(path) => path,
                    None => {
                        service
                            .media_by_id(media_id)
                            .map_err(|error| error.to_string())?
                            .ok_or_else(|| {
                                format!("Selected media {media_id} is no longer indexed.")
                            })?
                            .full_path
                    }
                };
                card::read_exif_fields(&path).map_err(str::to_string)
            },
        );
        let result = self.exif_read.value().cloned();
        if changed || (self.exif_cache.is_none() && result.is_some()) {
            self.exif_cache = result.map(|result| (media_id, result));
            return true;
        }
        false
    }
}
