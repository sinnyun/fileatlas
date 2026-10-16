use super::*;

pub(super) const GRID_GAP: f32 = 16.;
pub(super) const GRID_PADDING: f32 = 16.;

impl Render for GalleryView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(request) = self.service.take_ui_request() {
            match request {
                crate::GalleryUiRequest::InspectorTagName(name) => {
                    self.inspector_tag_name
                        .update(cx, |input, cx| input.set_value(name, window, cx));
                }
                crate::GalleryUiRequest::LocateFile(id) => self.locate_file(id, cx),
                crate::GalleryUiRequest::OpenDirectory(path) => self.open_directory(path, cx),
                crate::GalleryUiRequest::DirectoryUp => self.directory_up(cx),
                crate::GalleryUiRequest::Refresh => {
                    self.service.invalidate_directory();
                    self.refresh_index();
                }
                crate::GalleryUiRequest::CancelDirectory => self.service.cancel_directory(),
                crate::GalleryUiRequest::Thumbnails(value) => {
                    self.service.set_thumbnails_enabled(value);
                    self.refresh_index();
                    self.persist_filter_session();
                }
                crate::GalleryUiRequest::ShowDescendants(enabled) => {
                    self.set_descendants(enabled, cx)
                }
                crate::GalleryUiRequest::Layout(list) => self.set_layout(list, cx),
                crate::GalleryUiRequest::RecordCandidate(id, path) => {
                    self.record_candidates.insert(id, path);
                }
                crate::GalleryUiRequest::RelinkRecord(id) => self.relink_record(id, cx),
                crate::GalleryUiRequest::DeleteMissingRecords => {
                    if self.confirm_delete_missing {
                        self.delete_missing_records(cx)
                    } else {
                        self.confirm_delete_missing = true;
                    }
                }
                crate::GalleryUiRequest::MissingRecords => {
                    self.show_records = true;
                    self.record_read_revision += 1;
                    self.records_page = 0;
                }
                crate::GalleryUiRequest::Statistics => self.show_statistics = true,
                crate::GalleryUiRequest::BulkActions => {
                    bulk_actions::reset_inputs(self, window, cx);
                    self.bulk_action = Some(BulkActionKind::Delete);
                    self.bulk_source = BulkActionSource::WholeLibrary;
                    self.confirm_bulk_action = false;
                    self.confirm_bulk_action_ids = None;
                }
                crate::GalleryUiRequest::ConfigureBulk(action, source) => {
                    bulk_actions::reset_inputs(self, window, cx);
                    self.bulk_action = Some(action);
                    self.bulk_source = source;
                    self.confirm_bulk_action = false;
                    self.confirm_bulk_action_ids = None;
                }
                crate::GalleryUiRequest::BulkField(index, value) => {
                    if let Some(input) = self.bulk_inputs.get(index) {
                        input.update(cx, |input, cx| input.set_value(&value, window, cx));
                        self.confirm_bulk_action = false;
                        self.confirm_bulk_action_ids = None;
                    }
                }
                crate::GalleryUiRequest::OpenMediaActions(id) => {
                    self.selected_directory = None;
                    if let Some(media) = self.service.media_by_id(id).ok().flatten() {
                        self.selection.selected_media_ids = vec![id];
                        self.selection.anchor_id = Some(id);
                        self.media_actions_id = Some(id);
                        self.media_actions_item = Some((id, media));
                        self.persist_filter_session();
                    }
                }
                crate::GalleryUiRequest::CloseMediaActions => {
                    self.media_actions_id = None;
                    self.media_actions_item = None;
                }
                crate::GalleryUiRequest::CloseDialogs => {
                    self.show_records = false;
                    self.show_statistics = false;
                    self.bulk_action = None;
                    self.show_selection_details = false;
                    self.show_tag_picker = false;
                    self.media_actions_id = None;
                    self.media_actions_item = None;
                    self.confirm_bulk_action = false;
                    self.confirm_bulk_action_ids = None;
                }
                crate::GalleryUiRequest::Select(id) => {
                    self.selected_directory = None;
                    if self.service.media_by_id(id).ok().flatten().is_some() {
                        self.selection.selected_media_ids = vec![id];
                        self.selection.anchor_id = Some(id);
                        self.persist_filter_session();
                    }
                }
                crate::GalleryUiRequest::Preview(id) => {
                    self.preview_media(id, cx);
                }
                crate::GalleryUiRequest::Deselect => {
                    self.inspector_deselected = true;
                    self.selected_directory = None;
                    self.selection = GallerySelection::default();
                    self.persist_filter_session();
                }
                crate::GalleryUiRequest::SelectMany(ids) => {
                    self.selected_directory = None;
                    self.selection.selected_media_ids = ids;
                    self.selection.anchor_id = self.selection.selected_media_ids.first().copied();
                    self.persist_filter_session();
                }
                crate::GalleryUiRequest::ScrollTo(index) => {
                    if self
                        .snapshot
                        .as_ref()
                        .is_ok_and(|snapshot| index < snapshot.skeletons.len())
                    {
                        self.scroll_to_gallery_item(index);
                    }
                }
                crate::GalleryUiRequest::Search(query) => {
                    self.search
                        .update(cx, |input, cx| input.set_value(&query, window, cx));
                    self.filter.search = query;
                    self.reload();
                }
                crate::GalleryUiRequest::MediaType(value) => self.set_media_type(value, cx),
                crate::GalleryUiRequest::Sort(value) => self.set_sort(value, cx),
                crate::GalleryUiRequest::RatingRange(min, max) => {
                    self.filter.rating_min = min.clamp(0, 5);
                    self.filter.rating_max = max.clamp(min, 5);
                    self.reload();
                }
                crate::GalleryUiRequest::Extension(value, exclude) => {
                    self.set_extension_click_mode(exclude, cx);
                    self.cycle_extension(value, cx);
                }
                crate::GalleryUiRequest::TagFilter(id, exclude) => {
                    self.toggle_tag_filter(id, exclude, cx)
                }
                crate::GalleryUiRequest::TagFilterMode(exclude) => {
                    self.tag_filter_exclude_mode = exclude
                }
                crate::GalleryUiRequest::ExtensionMode(exclude) => {
                    self.set_extension_click_mode(exclude, cx)
                }
                crate::GalleryUiRequest::DateBound(from, date) => {
                    let picker = if from { &self.date_from } else { &self.date_to };
                    picker.update(cx, |picker, cx| {
                        picker.set_date(
                            gpui_kit::component::calendar::Date::Single(date),
                            window,
                            cx,
                        )
                    });
                    if from {
                        self.filter.date_from = date;
                    } else {
                        self.filter.date_to = date;
                    }
                    self.date_calendar.update(cx, |calendar, cx| {
                        calendar.set_date(
                            gpui_kit::component::calendar::Date::Range(
                                self.filter.date_from,
                                self.filter.date_to,
                            ),
                            window,
                            cx,
                        )
                    });
                    self.reload();
                }
                crate::GalleryUiRequest::ClearFilters => self.clear_filters(window, cx),
                crate::GalleryUiRequest::InspectorSection(section) => {
                    self.toggle_inspector_section(section, cx)
                }
                crate::GalleryUiRequest::TagPicker => self.show_tag_picker = !self.show_tag_picker,
                crate::GalleryUiRequest::Grouping(mode) => self.set_grouping(mode, cx),
                crate::GalleryUiRequest::ToggleGroup(label) => {
                    self.toggle_month_collapsed(label, cx)
                }
                crate::GalleryUiRequest::MonthGrouping(enabled) => {
                    self.set_month_grouping(enabled, cx)
                }
                crate::GalleryUiRequest::Hover(id) => {
                    self.hovered_cards.clear();
                    if let Some(id) = id {
                        self.set_card_hover(id, true, cx);
                    }
                }
            }
            cx.notify();
        }
        self.service.publish_ui_snapshot(crate::GalleryUiSnapshot {
            snapshot_revision: self.snapshot_generation,
            records_open: self.show_records,
            records_count: self
                .records_read
                .value()
                .and_then(|r| r.as_ref().ok())
                .map(Vec::len),
            record_candidates: self
                .record_candidates
                .iter()
                .map(|(id, path)| (*id, path.clone()))
                .collect(),
            browsing: self.snapshot.as_ref().is_ok_and(|s| s.browser.active),
            directories: self
                .snapshot
                .as_ref()
                .map(|s| s.browser.directories.clone())
                .unwrap_or_default(),
            current_directory: self
                .snapshot
                .as_ref()
                .ok()
                .and_then(|s| s.browser.directory.clone()),
            parent_directory: self
                .snapshot
                .as_ref()
                .ok()
                .and_then(|s| s.browser.parent.clone()),
            directory_error: self
                .snapshot
                .as_ref()
                .ok()
                .and_then(|s| s.browser.error.clone()),
            selected_directory: self.selected_directory.clone(),
            filter: self.filter.clone(),
            selected_media_ids: self.selection.selected_media_ids.clone(),
            matched_count: self
                .snapshot
                .as_ref()
                .map(|s| s.skeletons.len())
                .unwrap_or(0),
            group_by_month: self.grouping_mode == photag_plugin_grouping::GroupingMode::Month,
            grouping_mode: self.grouping_mode,
            groups: self
                .snapshot
                .as_ref()
                .map(|s| s.groups.clone())
                .unwrap_or_default(),
            collapsed_groups: self.collapsed_months.iter().cloned().collect(),
            list_layout: self.list_layout,
            window_offset: self
                .snapshot
                .as_ref()
                .map(|snapshot| snapshot.window_offset)
                .unwrap_or(0),
            loaded_count: self
                .snapshot
                .as_ref()
                .map(|snapshot| snapshot.media.len())
                .unwrap_or(0),
            inspector_sections: self.inspector_expanded.iter().copied().collect(),
            inspector_context: self.inspector_context_key(),
            bulk_action: self.bulk_action.map(|action| action.label()),
            bulk_source: self.bulk_source.label(),
            statistics_open: self.show_statistics,
            dialog_reads: self.dialog_read_status(),
            tag_picker_open: self.show_tag_picker,
            media_actions_open: self.media_actions_id.is_some(),
            media_actions_id: self.media_actions_id,
        });
        self.refresh_locale(window, cx);
        // v5 grid: columns come from the Gallery viewport strip (rows own the
        // 16 pt horizontal padding) using the square thumb + 16 pt gap.
        let sidebar_width = self
            .session
            .snapshot()
            .ok()
            .map(|session| session.sidebar_width)
            .filter(|width| {
                width.is_finite()
                    && (theme::SIDEBAR_MIN_WIDTH..=theme::SIDEBAR_MAX_WIDTH).contains(width)
            })
            .unwrap_or(theme::SIDEBAR_WIDTH);
        let previous_viewport_width = f32::from(self.scroll.viewport_bounds().size.width);
        let available_width = if previous_viewport_width > 0.0 {
            previous_viewport_width
        } else {
            f32::from(window.bounds().size.width) - sidebar_width - GRID_PADDING * 2.0
        };
        let cards_width = (available_width - GRID_PADDING * 2.0).max(0.0);
        let columns = if self.list_layout {
            1
        } else {
            ((cards_width + GRID_GAP) / (self.thumb_size + GRID_GAP))
                .floor()
                .max(1.0) as usize
        };
        self.columns = columns;
        let group_by_month = self.grouping_mode != photag_plugin_grouping::GroupingMode::None;
        let row_layout_key = (
            columns,
            group_by_month,
            self.filter.sort,
            self.collapsed_months.clone(),
        );
        if self.row_layout_key.as_ref() != Some(&row_layout_key) {
            self.row_layout = Arc::new(
                self.snapshot
                    .as_ref()
                    .map(|snapshot| {
                        if snapshot.browser.active {
                            if self.grouping_mode == photag_plugin_grouping::GroupingMode::None {
                                super::rows::build_browser_rows(
                                    snapshot.browser.directories.len(),
                                    snapshot.skeletons.len(),
                                    columns,
                                )
                            } else {
                                let mut rows = super::rows::build_browser_rows(
                                    snapshot.browser.directories.len(),
                                    0,
                                    columns,
                                );
                                rows.extend(build_gallery_rows(
                                    &snapshot.group_labels,
                                    columns,
                                    &self.collapsed_months,
                                ));
                                rows
                            }
                        } else {
                            build_gallery_rows(
                                &snapshot.group_labels,
                                columns,
                                &self.collapsed_months,
                            )
                        }
                    })
                    .unwrap_or_default(),
            );
            self.row_layout_key = Some(row_layout_key);
            self.scroll.reset(self.row_layout.len());
            if let Some(mut offset) = self.pending_scroll_restore.take() {
                let mut item_ix = 0;
                while item_ix + 1 < self.row_layout.len() {
                    let height = gallery_row_height(
                        &self.row_layout[item_ix],
                        item_ix,
                        self.thumb_size,
                        self.list_layout,
                    );
                    if offset < height {
                        break;
                    }
                    offset -= height;
                    item_ix += 1;
                }
                self.scroll.scroll_to(ListOffset {
                    item_ix,
                    offset_in_item: px(offset),
                });
            }
        }
        let row_layout = Arc::clone(&self.row_layout);

        match &self.snapshot {
            Ok(snapshot) => {
                if snapshot.skeletons.is_empty() && snapshot.browser.directories.is_empty() {
                    if snapshot.browser.active {
                        return directory_browser::empty(
                            self,
                            snapshot
                                .browser
                                .error
                                .as_deref()
                                .or_else(|| self.filters_active().then_some("No matching items")),
                            cx,
                        );
                    }
                    return empty_state::render(self.filters_active() || self.scope_active(), cx);
                }
                let list_scroll = self.scroll.clone();
                let list_layout = self.list_layout;
                let card_size = if list_layout { 56. } else { self.thumb_size };
                let list_width = list_layout.then_some(cards_width.max(100.));
                let selected_ids = self.selection.selected_media_ids.clone();
                let loaded_selected_paths = self
                    .snapshot
                    .as_ref()
                    .ok()
                    .map(|snapshot| {
                        selected_ids
                            .iter()
                            .filter_map(|id| {
                                snapshot
                                    .media
                                    .iter()
                                    .find(|media| media.id == *id)
                                    .map(|media| PathBuf::from(media.full_path.as_str()))
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let selected_drag_paths =
                    self.selected_drag_paths(&selected_ids, loaded_selected_paths, cx);
                let row_layout = Arc::clone(&row_layout);
                let mut grid = div()
                    .id("gallery-grid-root")
                    .track_focus(&self.focus_handle)
                    .on_key_down(
                        cx.listener(|this, event, window, cx| this.handle_key(event, window, cx)),
                    )
                    .v_flex()
                    // The shell parent owns the viewport height; growing here avoids
                    // size_full resolving against the window and overlapping the header.
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .pt(px(GRID_PADDING));
                let rubberband = self.rubberband.as_ref().map(|rubberband| {
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
                    )
                    .intersect(&viewport);
                    let left = f32::from(band.origin.x - viewport.origin.x);
                    let top = f32::from(band.origin.y - viewport.origin.y);
                    (
                        left,
                        top,
                        f32::from(band.size.width),
                        f32::from(band.size.height),
                    )
                });
                grid = grid.child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .w_full()
                        .relative()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, event: &MouseDownEvent, _, cx| {
                                this.start_rubberband(event, cx)
                            }),
                        )
                        .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                            this.update_rubberband(event, cx)
                        }))
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(|this, event: &MouseUpEvent, _, cx| {
                                this.finish_rubberband(event, cx)
                            }),
                        )
                        .child(
                            list(
                                list_scroll.clone(),
                                cx.processor(move |this, row_index: usize, window, cx| {
                                    let range = row_index..row_index + 1;
                                    let mut rows = Vec::with_capacity(range.len());
                                    let requested_start = row_layout
                                        .get(range.start)
                                        .map(|row| row.media_range.start)
                                        .unwrap_or(0);
                                    let requested_end = range
                                        .end
                                        .checked_sub(1)
                                        .and_then(|row| row_layout.get(row))
                                        .map(|row| row.media_range.end)
                                        .unwrap_or(requested_start);
                                    let window_range = requested_start.saturating_sub(columns * 2)
                                        ..requested_end.saturating_add(columns * 2);
                                    this.request_visible_window(window_range);
                                    let mut states = Vec::new();
                                    let view_focused = this.focus_handle.is_focused(window);
                                    let loaded: HashSet<_> = this
                                        .snapshot
                                        .as_ref()
                                        .map(|snapshot| {
                                            snapshot.media.iter().map(|media| media.id).collect()
                                        })
                                        .unwrap_or_default();
                                    if let Ok(snapshot) = &this.snapshot {
                                        this.card_motion.retain(|id, _| loaded.contains(id));
                                        this.hovered_cards.retain(|id| loaded.contains(id));
                                        let card_tag_tree = Arc::new(snapshot.tag_tree.clone());
                                        let selected_drag_paths = selected_drag_paths.clone();
                                        for row_index in range {
                                            let Some(layout_row) = row_layout.get(row_index) else {
                                                continue;
                                            };
                                            let first = layout_row.media_range.start;
                                            let end = layout_row.media_range.end;
                                            let month_label = layout_row.month_label.clone();
                                            let has_month_label = month_label.is_some();
                                            let mut cards = Vec::with_capacity(
                                                end - first + layout_row.directory_range.len(),
                                            );
                                            for folder in &snapshot.browser.directories
                                                [layout_row.directory_range.clone()]
                                            {
                                                cards.push(directory_browser::tile(
                                                    folder,
                                                    this.selected_directory.as_deref()
                                                        == Some(folder.path.as_str()),
                                                    card_size,
                                                    list_width,
                                                    cx,
                                                ));
                                            }
                                            if !layout_row.collapsed {
                                                for skeleton in &snapshot.skeletons[first..end] {
                                                    if let Some(media) = snapshot
                                                        .media
                                                        .iter()
                                                        .find(|item| item.id == skeleton.id)
                                                    {
                                                        let selected = this
                                                            .selection
                                                            .selected_media_ids
                                                            .contains(&media.id);
                                                        let rating_target_ids = if selected {
                                                            this.selection
                                                                .selected_media_ids
                                                                .clone()
                                                        } else {
                                                            vec![media.id]
                                                        };
                                                        let drag_paths = if selected
                                                            && !selected_drag_paths.is_empty()
                                                        {
                                                            selected_drag_paths.clone()
                                                        } else {
                                                            vec![PathBuf::from(
                                                                media.full_path.as_str(),
                                                            )]
                                                        };
                                                        let now = std::time::Instant::now();
                                                        let motion = this
                                                            .card_motion
                                                            .entry(media.id)
                                                            .or_insert_with(|| {
                                                                super::card_motion::CardMotion::new(
                                                                    selected, now,
                                                                )
                                                            });
                                                        let (hover, selection, moving) = motion
                                                            .sample(
                                                                this.hovered_cards
                                                                    .contains(&media.id),
                                                                selected,
                                                                now,
                                                                cx.reduce_motion(),
                                                            );
                                                        if moving && !cx.reduce_motion() {
                                                            window.request_animation_frame();
                                                        }
                                                        states.push(crate::GalleryCardState {
                                                            media_id: media.id,
                                                            hover,
                                                            selection,
                                                            timestamp_ms:
                                                                std::time::SystemTime::now()
                                                                    .duration_since(
                                                                        std::time::UNIX_EPOCH,
                                                                    )
                                                                    .unwrap_or_default()
                                                                    .as_millis()
                                                                    as u64,
                                                        });
                                                        cards.push(
                                                            render_media_card(
                                                                media,
                                                                hover,
                                                                selection,
                                                                view_focused,
                                                                Arc::clone(&this.drag_drop),
                                                                drag_paths,
                                                                Arc::clone(&this.drag_allowed),
                                                                card_size,
                                                                list_width,
                                                                media.micro_thumb.as_deref(),
                                                                rating_target_ids,
                                                                Arc::clone(&card_tag_tree),
                                                                cx,
                                                            )
                                                            .into_any_element(),
                                                        );
                                                    } else {
                                                        cards.push(
                                                            div()
                                                                .w(px(card_size))
                                                                .h(px(card_size
                                                                    + card::CARD_CAPTION_HEIGHT))
                                                                .rounded(px(theme::THUMB_ROUND))
                                                                .bg(rgb(theme::SURFACE_FAINT))
                                                                .into_any_element(),
                                                        );
                                                    }
                                                }
                                            }
                                            let header = month_label
                                                .clone()
                                                .map(|label| {
                                                    let collapsed = layout_row.collapsed;
                                                    let now = std::time::Instant::now();
                                                    let motion = this
                                                        .month_motion
                                                        .entry(label.clone())
                                                        .or_insert_with(|| {
                                                            super::card_motion::CardMotion::new(
                                                                !collapsed, now,
                                                            )
                                                            .with_selection_seconds(0.15)
                                                        });
                                                    let (_, expanded, moving) = motion.sample(
                                                        false,
                                                        !collapsed,
                                                        now,
                                                        cx.reduce_motion(),
                                                    );
                                                    if moving {
                                                        window.request_animation_frame();
                                                    }
                                                    super::month_header::render(
                                                        label,
                                                        layout_row.month_count,
                                                        expanded,
                                                        snapshot.browser.directory.as_deref(),
                                                        cx,
                                                    )
                                                    .into_any_element()
                                                })
                                                .unwrap_or_else(|| div().into_any_element());
                                            let month_top_gap = if row_index > 0 && has_month_label
                                            {
                                                theme::GROUP_TOP_GAP
                                            } else {
                                                0.0
                                            };
                                            let header_gap = if has_month_label {
                                                theme::GROUP_HEADER_GAP
                                            } else {
                                                0.0
                                            };
                                            rows.push(
                                                div()
                                                    .id(format!("gallery-row-{row_index}"))
                                                    .v_flex()
                                                    .w_full()
                                                    .px(px(GRID_PADDING))
                                                    .h(px(gallery_row_height(
                                                        layout_row,
                                                        row_index,
                                                        card_size,
                                                        list_layout,
                                                    )))
                                                    .pt(px(month_top_gap))
                                                    .gap_0()
                                                    .child(header)
                                                    .child(
                                                        div()
                                                            .h_flex()
                                                            .mt(px(header_gap))
                                                            .gap(px(GRID_GAP))
                                                            .children(cards),
                                                    ),
                                            );
                                        }
                                    }
                                    this.service.publish_card_states(states, &loaded);
                                    rows.into_iter()
                                        .next()
                                        .map(IntoElement::into_any_element)
                                        .unwrap_or_else(|| div().into_any_element())
                                }),
                            )
                            .size_full(),
                        )
                        .vertical_scrollbar(&list_scroll)
                        .children(rubberband.map(|(left, top, width, height)| {
                            div()
                                .absolute()
                                .left(px(left))
                                .top(px(top))
                                .w(px(width))
                                .h(px(height))
                                .border_1()
                                .border_color(rgb(theme::accent()))
                                .bg(rgb(theme::accent_alpha(theme::accent(), 0.2)))
                        })),
                );
                grid.into_any_element()
            }
            Err(error) => div()
                .size_full()
                .bg(rgba(theme::base_rgba(theme::SURFACE_BASE)))
                .p_4()
                .text_color(rgb(theme::DANGER))
                .child(format!("Could not load media: {error}"))
                .into_any_element(),
        }
    }
}

impl GalleryView {
    fn selected_drag_paths(
        &mut self,
        selected_ids: &[i64],
        loaded_paths: Vec<PathBuf>,
        _: &mut Context<Self>,
    ) -> Vec<PathBuf> {
        if selected_ids.is_empty() {
            return Vec::new();
        }
        if loaded_paths.len() == selected_ids.len() {
            return loaded_paths;
        }
        if let Some((cached_ids, paths)) = &self.selected_drag_paths_cache
            && cached_ids == selected_ids
        {
            return paths.clone();
        }

        let service = self.service.clone();
        let ids = selected_ids.to_vec();
        self.selected_drag_read.request(ids.clone(), move || {
            service.media_paths_by_ids(&ids).map_err(|e| e.to_string())
        });
        loaded_paths
    }
}

pub(super) fn gallery_row_height(
    row: &GalleryVisualRow,
    index: usize,
    thumb: f32,
    list: bool,
) -> f32 {
    let header = if row.month_label.is_some() {
        theme::GROUP_HEADER_HEIGHT + theme::GROUP_HEADER_GAP
    } else {
        0.
    };
    let month_gap = if index > 0 && row.month_label.is_some() {
        theme::GROUP_TOP_GAP
    } else {
        0.
    };
    let cards = if row.collapsed {
        0.
    } else {
        if list {
            56.
        } else {
            thumb + card::CARD_CAPTION_HEIGHT
        }
    };
    header + month_gap + cards + 16.
}
