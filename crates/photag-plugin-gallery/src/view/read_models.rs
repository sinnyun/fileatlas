use super::*;
use photag_plugin_database::{LibraryStats, MediaQuery};

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
                            annotated_only: service.browser_enabled(),
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

impl GalleryView {
    pub(super) fn request_visible_window(&mut self, requested: std::ops::Range<usize>) {
        self.visible_demand = Some(match self.visible_demand.take() {
            Some(previous) => previous.start.min(requested.start)..previous.end.max(requested.end),
            None => requested,
        });
    }
    pub(super) fn flush_visible_window(&mut self) {
        if self.snapshot_read.is_pending() {
            return;
        }
        let Some(requested) = self.visible_demand.take() else {
            return;
        };
        let Ok(snapshot) = &self.snapshot else { return };
        let total = snapshot.skeletons.len();
        if total == 0 {
            self.request_version_states(&[]);
            return;
        }
        let begin = requested.start.min(total);
        let end = requested.end.min(total).max(begin);
        let size = 200.max(end - begin).min(total);
        let offset = ((begin + end) / 2)
            .saturating_sub(size / 2)
            .min(total - size);
        let ids = snapshot.skeletons[offset..offset + size]
            .iter()
            .map(|s| s.id)
            .collect::<Vec<_>>();
        let visible = snapshot.skeletons[begin..end]
            .iter()
            .map(|s| s.id)
            .collect::<Vec<_>>();
        let status = self.service.directory_status();
        self.request_version_states(&visible);
        let key = (
            status.0,
            self.snapshot_generation,
            ids.clone(),
            visible.clone(),
        );
        if self.viewport_read.key() == Some(&key) {
            return;
        }
        let service = self.service.clone();
        let request = service.viewport_request.fetch_add(1, Ordering::AcqRel) + 1;
        self.viewport_read.set_wake(self.service.wake.clone());
        self.viewport_read.request(key, move || {
            service.prepare_window(ids, visible, offset, status.0, request)
        });
    }
}
