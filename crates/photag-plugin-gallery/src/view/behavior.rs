use super::*;

impl GalleryView {
    #[tracing::instrument(skip_all, fields(list=?list))]
    pub(super) fn set_layout(&mut self, list: bool, cx: &mut Context<Self>) {
        if self.list_layout != list {
            self.list_layout = list;
            self.row_layout_key = None;
            self.rubberband = None;
            self.pending_scroll_restore = None;
            self.persist_filter_session();
            cx.notify();
        }
    }

    #[tracing::instrument(skip_all, fields(media_id=?media_id))]
    pub(super) fn select_media(
        &mut self,
        media_id: i64,
        modifiers: Modifiers,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.selected_directory = None;
        window.focus(&self.focus_handle, cx);
        let Ok(snapshot) = &self.snapshot else {
            return;
        };
        self.selection.select_click(
            &snapshot.skeletons,
            media_id,
            SelectionModifiers {
                ctrl: modifiers.control,
                alt: modifiers.alt,
                shift: modifiers.shift,
            },
        );
        self.show_exif = false;
        self.exif_cache = None;
        self.confirm_delete = false;
        self.persist_filter_session();
        cx.notify();
    }

    pub(super) fn start_rubberband(&mut self, event: &MouseDownEvent, cx: &mut Context<Self>) {
        self.selected_directory = None;
        let viewport = self.scroll.viewport_bounds();
        if !viewport.contains(&event.position) {
            return;
        }
        let add = event.modifiers.control;
        self.rubberband = Some(GalleryRubberband {
            start: event.position,
            end: event.position,
            initial_selection: if add {
                self.selection.selected_media_ids.clone()
            } else {
                Vec::new()
            },
            initial_anchor: self.selection.anchor_id,
            add,
            clear_on_click: !event.modifiers.control && !event.modifiers.shift,
            dragged: false,
        });
        self.selection.anchor_id = None;
        self.show_exif = false;
        self.exif_cache = None;
        cx.notify();
    }

    pub(super) fn update_rubberband(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if !event.dragging() {
            return;
        }
        let Some(rubberband) = self.rubberband.as_mut() else {
            return;
        };
        rubberband.end = event.position;
        rubberband.dragged = true;
        let viewport = self.scroll.viewport_bounds();
        let band = Bounds::from_corners(
            point(
                rubberband.start.x.min(rubberband.end.x),
                rubberband.start.y.min(rubberband.end.y),
            ),
            point(
                rubberband.start.x.max(rubberband.end.x),
                rubberband.start.y.max(rubberband.end.y),
            ),
        );
        let visible_band = band.intersect(&viewport);
        let mut selected = if rubberband.add {
            rubberband.initial_selection.clone()
        } else {
            Vec::new()
        };
        if visible_band.size.width > px(0.)
            && visible_band.size.height > px(0.)
            && let Ok(snapshot) = &self.snapshot
        {
            let card_width = px(if self.list_layout {
                f32::from(viewport.size.width) - 32.
            } else {
                self.thumb_size
            });
            let card_height = px(if self.list_layout {
                56.
            } else {
                self.thumb_size
            });
            let card_step = card_width + px(16.);
            let start = self.scroll.logical_scroll_top().item_ix;
            let visible_count = (f32::from(viewport.size.height) / 40.).ceil() as usize + 2;
            for (row_index, row) in self
                .row_layout
                .iter()
                .enumerate()
                .skip(start)
                .take(visible_count)
            {
                if row.collapsed {
                    continue;
                }
                let Some(bounds) = self.scroll.bounds_for_item(row_index) else {
                    continue;
                };
                let row_y = bounds.origin.y;
                let card_y = row_y + px(if row.month_label.is_some() { 40. } else { 0. });
                if card_y + card_height < visible_band.top() || card_y > visible_band.bottom() {
                    continue;
                }
                for (column, media_index) in row.media_range.clone().enumerate() {
                    let card_bounds = Bounds::new(
                        point(
                            viewport.origin.x
                                + px(16.)
                                + card_step * (column + row.directory_range.len()) as f32,
                            card_y,
                        ),
                        size(card_width, card_height),
                    );
                    if card_bounds.intersects(&visible_band)
                        && let Some(media) = snapshot.skeletons.get(media_index)
                        && !selected.contains(&media.id)
                    {
                        selected.push(media.id);
                    }
                }
            }
        }
        self.selection.selected_media_ids = selected;
        self.persist_filter_session();
        cx.notify();
    }

    #[tracing::instrument(skip_all)]
    pub(super) fn finish_rubberband(&mut self, _: &MouseUpEvent, cx: &mut Context<Self>) {
        if let Some(rubberband) = self.rubberband.take() {
            if !rubberband.dragged && rubberband.clear_on_click {
                self.selection.selected_media_ids.clear();
                self.selection.anchor_id = None;
                self.show_exif = false;
                self.exif_cache = None;
            } else if !rubberband.dragged {
                self.selection.anchor_id = rubberband.initial_anchor;
            }
            self.persist_filter_session();
            cx.notify();
        }
    }

    pub(super) fn toggle_exif(&mut self, cx: &mut Context<Self>) {
        self.show_exif = !self.show_exif;
        if self.show_exif {
            self.ensure_exif_loaded(cx);
        }
        cx.notify();
    }

    pub(super) fn ensure_exif_loaded(&mut self, cx: &mut Context<Self>) {
        let selected_path = self
            .selection
            .selected_media_ids
            .first()
            .and_then(|&media_id| {
                self.service
                    .media_by_id(media_id)
                    .ok()
                    .flatten()
                    .map(|media| (media_id, media.full_path))
            });
        if let Some((media_id, path)) = selected_path {
            if self
                .exif_cache
                .as_ref()
                .is_some_and(|(id, _)| *id == media_id)
            {
                return;
            }
            self.exif_cache = Some((
                media_id,
                card::read_exif_fields(&path).map_err(str::to_string),
            ));
            cx.notify();
        }
    }

    pub(super) fn start_operation<F>(&mut self, operation: F, refresh: bool, cx: &mut Context<Self>)
    where
        F: Future<Output = String> + Send + 'static,
    {
        self.operation_status = Some("Working…".to_string());
        self.confirm_delete = false;
        let started = std::time::Instant::now();
        tracing::info!("gallery operation started");
        let operation = self.runtime.spawn(operation);
        cx.spawn(async move |this, cx| {
            let status = operation
                .await
                .unwrap_or_else(|error| format!("Operation task failed: {error}"));
            tracing::info!(duration_ms=started.elapsed().as_secs_f64()*1000.,%status,"gallery operation finished");
            let _ = this.update(cx, |this, cx| {
                if refresh {
                    this.reload();
                }
                this.operation_status = Some(status);
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn open_selected(&mut self, reveal: bool, cx: &mut Context<Self>) {
        let Some(&media_id) = self.selection.selected_media_ids.first() else {
            return;
        };
        self.open_media_action(media_id, reveal, cx);
    }

    pub(super) fn open_media_action(
        &mut self,
        media_id: i64,
        reveal: bool,
        cx: &mut Context<Self>,
    ) {
        let operations = Arc::clone(&self.file_operations);
        self.start_operation(
            async move {
                let result = if reveal {
                    operations.reveal_in_file_manager(media_id).await
                } else {
                    operations.open_external(media_id).await
                };
                result.map_or_else(
                    |error| format!("File action failed: {error}"),
                    |_| {
                        if reveal {
                            "Opened containing folder.".to_string()
                        } else {
                            "Opened with the system default application.".to_string()
                        }
                    },
                )
            },
            false,
            cx,
        );
    }

    pub(super) fn preview_selected(&mut self, cx: &mut Context<Self>) {
        let Some(&media_id) = self.selection.selected_media_ids.first() else {
            return;
        };
        self.preview_media(media_id, cx);
    }

    #[tracing::instrument(skip_all, fields(media_id=?media_id))]
    pub(super) fn preview_media(&mut self, media_id: i64, cx: &mut Context<Self>) -> bool {
        let opened = match self.service.media_by_id(media_id) {
            Ok(Some(media)) if media.media_type == "image" => {
                self.set_preview_filmstrip();
                match self.preview.open_media(media.full_path, Some(media_id)) {
                    Ok(()) => {
                        self.operation_status = Some("Image preview opened.".into());
                        true
                    }
                    Err(error) => {
                        self.operation_status = Some(error.to_string());
                        false
                    }
                }
            }
            Ok(Some(media)) if media.media_type == "video" => {
                self.set_preview_filmstrip();
                match self.preview.open_media(media.full_path, Some(media_id)) {
                    Ok(()) => {
                        self.video_preview_open = true;
                        self.operation_status = Some("Video preview opened.".into());
                    }
                    Err(error) => self.operation_status = Some(error.to_string()),
                }
                false
            }
            Ok(Some(media)) if media.media_type == "audio" => {
                self.set_preview_filmstrip();
                match self.preview.open_media(media.full_path, Some(media_id)) {
                    Ok(()) => self.operation_status = Some("Audio preview opened.".into()),
                    Err(error) => self.operation_status = Some(error.to_string()),
                }
                false
            }
            Ok(Some(_)) => {
                self.operation_status = Some("This file type cannot be previewed.".into());
                false
            }
            Ok(None) => {
                self.operation_status = Some("Selected media is no longer indexed.".into());
                false
            }
            Err(error) => {
                self.operation_status = Some(error.to_string());
                false
            }
        };
        cx.notify();
        opened
    }

    fn set_preview_filmstrip(&self) {
        let items = self
            .snapshot
            .as_ref()
            .map(|snapshot| {
                snapshot
                    .skeletons
                    .iter()
                    .map(|item| item.id)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let filmstrip = self
            .service
            .media_path_pairs_by_ids(&items)
            .unwrap_or_default();
        self.preview.set_filmstrip_media(filmstrip);
    }

    pub(super) fn suggest_tags(&mut self, media_id: i64, cx: &mut Context<Self>) {
        match self.service.media_by_id(media_id) {
            Ok(Some(media)) if media.media_type == "image" => {
                if self.preview_media(media_id, cx) {
                    self.preview.request_ai_suggestions(media_id);
                    self.operation_status = Some("AI tag suggestions requested.".into());
                    cx.notify();
                }
            }
            Ok(Some(_)) => {
                self.operation_status = Some("AI tag suggestions require an image.".into());
                cx.notify();
            }
            Ok(None) => {
                self.operation_status = Some("Selected media is no longer indexed.".into());
                cx.notify();
            }
            Err(error) => {
                self.operation_status = Some(error.to_string());
                cx.notify();
            }
        }
    }

    pub(super) fn close_video_preview(&mut self, cx: &mut Context<Self>) {
        self.playback.close();
        self.preview.close();
        self.video_preview_open = false;
        self.operation_status = Some("Returned to the gallery.".into());
        cx.notify();
    }

    pub(super) fn apply_rating(&mut self, rating: u8, cx: &mut Context<Self>) {
        let media_ids = self.selection.selected_media_ids.clone();
        self.apply_rating_to(&media_ids, rating);
        cx.notify();
    }

    #[tracing::instrument(skip_all, fields(rating=?rating))]
    pub(super) fn apply_rating_to(&mut self, media_ids: &[i64], rating: u8) {
        if media_ids.is_empty() {
            return;
        }
        let old_rating = media_ids
            .first()
            .and_then(|id| self.service.media_by_id(*id).ok().flatten())
            .map(|media| media.rating);
        match self.service.apply_rating(media_ids, rating) {
            Ok(()) => {
                let new_rating = i32::from(rating.min(5));
                if let (Some(history), Some(old_rating)) = (&self.history, old_rating) {
                    if old_rating != new_rating {
                        history.push(UndoEntry::RatingChanged {
                            media_ids: media_ids.to_vec(),
                            old_rating,
                            new_rating,
                        });
                    }
                }
                self.reload();
            }
            Err(error) => self.snapshot = Err(error.to_string()),
        }
    }

    #[tracing::instrument(skip_all, fields(media_type=?media_type))]
    pub(super) fn set_media_type(&mut self, media_type: MediaTypeFilter, cx: &mut Context<Self>) {
        if self.filter.media_type != media_type {
            self.filter.media_type = media_type;
            self.reload();
            cx.notify();
        }
    }

    pub(super) fn toggle_search_scope(&mut self, scope: SearchScope, cx: &mut Context<Self>) {
        self.filter.toggle_search_scope(scope);
        if let Some(settings) = &self.settings {
            let scopes = self.filter.search_scopes;
            if let Err(error) = settings.update(|preferences| {
                preferences.search_scopes = scopes.into();
            }) {
                self.operation_status = Some(format!("Couldn't save search scope: {error}"));
            }
        }
        self.reload();
        cx.notify();
    }

    #[tracing::instrument(skip_all, fields(sort=?sort))]
    pub(super) fn set_sort(&mut self, sort: SortOrder, cx: &mut Context<Self>) {
        if self.filter.sort != sort {
            self.filter.sort = sort;
            self.reload();
            cx.notify();
        }
    }

    #[tracing::instrument(skip_all, fields(minimum=?minimum,value=?value))]
    pub(super) fn set_rating_bound(&mut self, minimum: bool, value: i32, cx: &mut Context<Self>) {
        let old_range = (self.filter.rating_min, self.filter.rating_max);
        self.filter.set_rating_bound(minimum, value);
        if old_range == (self.filter.rating_min, self.filter.rating_max) {
            return;
        }
        self.reload();
        cx.notify();
    }

    #[tracing::instrument(skip_all, fields(extension=?extension))]
    pub(super) fn cycle_extension(&mut self, extension: String, cx: &mut Context<Self>) {
        let (active, opposite) = if self.extension_exclude_mode {
            (
                &mut self.filter.excluded_extensions,
                &mut self.filter.extensions,
            )
        } else {
            (
                &mut self.filter.extensions,
                &mut self.filter.excluded_extensions,
            )
        };
        if active.contains(&extension) {
            active.retain(|value| value != &extension);
        } else {
            opposite.retain(|value| value != &extension);
            active.push(extension);
        }
        self.reload();
        cx.notify();
    }

    #[tracing::instrument(skip_all)]
    pub(super) fn set_extension_click_mode(&mut self, exclude: bool, cx: &mut Context<Self>) {
        self.extension_exclude_mode = exclude;
        if let Ok(mut session) = self.session.snapshot() {
            session.ext_click_mode = Some(if exclude {
                photag_plugin_session::TagClickMode::Exclude
            } else {
                photag_plugin_session::TagClickMode::Select
            });
            if let Err(error) = self.session.replace(session) {
                self.operation_status =
                    Some(format!("Could not save extension filter mode: {error}"));
            }
        }
        cx.notify();
    }

    #[tracing::instrument(skip_all)]
    pub(super) fn clear_extensions(&mut self, cx: &mut Context<Self>) {
        self.filter.clear_extensions();
        self.reload();
        cx.notify();
    }

    pub(super) fn scroll_to_gallery_item(&self, index: usize) {
        let row_index = self
            .row_layout
            .iter()
            .position(|row| row.media_range.contains(&index))
            .unwrap_or(index / self.columns.max(1));
        if self.scroll.bounds_for_item(row_index).is_some() {
            self.scroll.scroll_to_reveal_item(row_index);
        } else {
            // Unmeasured virtual rows have no accumulated height yet. Jump
            // by logical row so the list measures the requested page first.
            self.scroll.scroll_to(ListOffset {
                item_ix: row_index,
                offset_in_item: px(0.),
            });
        }
    }
}
