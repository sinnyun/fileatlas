use super::*;

impl Render for GalleryView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(request) = self.service.take_ui_request() {
            match request {
                crate::GalleryUiRequest::Statistics => self.show_statistics = true,
                crate::GalleryUiRequest::BulkActions => {
                    bulk_actions::reset_inputs(self, window, cx);
                    self.bulk_action = Some(BulkActionKind::Delete);
                    self.bulk_source = BulkActionSource::WholeLibrary;
                    self.confirm_bulk_action = false;
                    self.confirm_bulk_action_ids = None;
                }
                crate::GalleryUiRequest::CloseDialogs => {
                    self.show_statistics = false;
                    self.bulk_action = None;
                    self.show_selection_details = false;
                    self.show_tag_picker = false;
                    self.confirm_bulk_action = false;
                    self.confirm_bulk_action_ids = None;
                }
                crate::GalleryUiRequest::Select(id) => {
                    if self.service.media_by_id(id).ok().flatten().is_some() {
                        self.selection.selected_media_ids = vec![id];
                        self.selection.anchor_id = Some(id);
                        self.persist_filter_session();
                    }
                }
                crate::GalleryUiRequest::Deselect => {
                    self.selection = GallerySelection::default();
                    self.persist_filter_session();
                }
            }
            cx.notify();
        }
        self.refresh_locale(window, cx);
        // Match gallery.rs: the source computes columns from the Gallery
        // viewport and the square thumb + fixed 12 pt gap.
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
        let previous_viewport_width =
            f32::from(self.scroll.0.borrow().base_handle.bounds().size.width);
        let available_width = if previous_viewport_width > 0.0 {
            previous_viewport_width
        } else {
            f32::from(window.bounds().size.width) - sidebar_width - 12.0 - 24.0
        };
        let columns = ((available_width + 12.0) / (self.thumb_size + 12.0))
            .floor()
            .max(1.0) as usize;
        self.columns = columns;
        let group_by_month = self.group_by_month
            && matches!(
                self.filter.sort,
                SortOrder::NewestFirst | SortOrder::OldestFirst
            );
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
                        build_gallery_rows(
                            &snapshot.skeletons,
                            columns,
                            group_by_month,
                            &self.collapsed_months,
                        )
                    })
                    .unwrap_or_default(),
            );
            self.row_layout_key = Some(row_layout_key);
        }
        let row_layout = Arc::clone(&self.row_layout);
        let row_count = row_layout.len();
        match &self.snapshot {
            Ok(snapshot) => {
                let list_scroll = self.scroll.clone();
                let card_size = self.thumb_size;
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
                    .w_full();
                let rubberband = self.rubberband.as_ref().map(|rubberband| {
                    let viewport = self.scroll.0.borrow().base_handle.bounds();
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
                            uniform_list(
                                "gallery-media-rows",
                                row_count,
                                cx.processor(
                                    move |this, range: std::ops::Range<usize>, _window, cx| {
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
                                        let window_range = requested_start
                                            .saturating_sub(columns * 2)
                                            ..requested_end.saturating_add(columns * 2);
                                        let shifted_window = match &this.snapshot {
                                            Ok(snapshot) => {
                                                this.service.ensure_window(snapshot, window_range)
                                            }
                                            Err(_) => Ok(None),
                                        };
                                        match shifted_window {
                                            Ok(Some(window)) => {
                                                if let Ok(snapshot) = &mut this.snapshot {
                                                    snapshot.window_offset = window.offset;
                                                    snapshot.media = window.media;
                                                    snapshot.media_tags = window.media_tags;
                                                }
                                            }
                                            Ok(None) => {}
                                            Err(error) => this.snapshot = Err(error.to_string()),
                                        }
                                        if let Ok(snapshot) = &this.snapshot {
                                            let card_tag_tree = Arc::new(snapshot.tag_tree.clone());
                                            let selected_drag_paths =
                                                if this.selection.selected_media_ids.is_empty() {
                                                    Vec::new()
                                                } else {
                                                    this.service
                                                        .media_paths_by_ids(
                                                            &this.selection.selected_media_ids,
                                                        )
                                                        .unwrap_or_default()
                                                };
                                            for row_index in range {
                                                let Some(layout_row) = row_layout.get(row_index)
                                                else {
                                                    continue;
                                                };
                                                let first = layout_row.media_range.start;
                                                let end = layout_row.media_range.end;
                                                let month_label = layout_row.month_label.clone();
                                                let has_month_label = month_label.is_some();
                                                let mut cards = Vec::with_capacity(end - first);
                                                if !layout_row.collapsed {
                                                    for skeleton in &snapshot.skeletons[first..end]
                                                    {
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
                                                            cards.push(
                                                                render_media_card(
                                                                    media,
                                                                    selected,
                                                                    Arc::clone(&this.drag_drop),
                                                                    drag_paths,
                                                                    Arc::clone(&this.drag_allowed),
                                                                    card_size,
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
                                                                    .h(px(card_size))
                                                                    .rounded_md()
                                                                    .bg(rgba(theme::panel_rgba(
                                                                        theme::SURFACE_PANEL,
                                                                    )))
                                                                    .into_any_element(),
                                                            );
                                                        }
                                                    }
                                                }
                                                let header = month_label
                                                    .clone()
                                                    .map(|label| {
                                                        let collapsed = layout_row.collapsed;
                                                        let display_label = label.rsplit_once(' ').map(|(month, year)| {
                                                            photag_ui_bridge::i18n::tr_args("{month} {year}", &[("month", photag_ui_bridge::i18n::tr(month)), ("year", year.to_owned())])
                                                        }).unwrap_or_else(|| label.clone());
                                                        div().h_flex().h(px(40.)).child(
                                                            Button::new(format!(
                                                                "gallery-month-{label}"
                                                            ))
                                                            .label(photag_ui_bridge::i18n::tr(
                                                                &(format!(
                                                                    "{} {display_label}",
                                                                    if collapsed {
                                                                        "▸"
                                                                    } else {
                                                                        "▾"
                                                                    }
                                                                ))
                                                                .to_string(),
                                                            ))
                                                            .on_click(cx.listener(
                                                                move |this, _, _, cx| {
                                                                    this.toggle_month_collapsed(
                                                                        label.clone(),
                                                                        cx,
                                                                    );
                                                                },
                                                            )),
                                                        )
                                                    })
                                                    .unwrap_or_else(|| div());
                                                let month_top_gap =
                                                    if row_index > 0 && has_month_label {
                                                        14.0
                                                    } else {
                                                        0.0
                                                    };
                                                rows.push(
                                                    div()
                                                        .id(format!("gallery-row-{row_index}"))
                                                        .v_flex()
                                                        .w_full()
                                                        .h(px(if layout_row.collapsed {
                                                            40. + month_top_gap
                                                        } else {
                                                            card_size
                                                                + 12.
                                                                + if has_month_label {
                                                                    40.
                                                                } else {
                                                                    0.
                                                                }
                                                                + month_top_gap
                                                        }))
                                                        .pt(px(month_top_gap))
                                                        .gap_0()
                                                        .child(header)
                                                        .child(
                                                            div().h_flex().gap_3().children(cards),
                                                        ),
                                                );
                                            }
                                        }
                                        rows
                                    },
                                ),
                            )
                            .track_scroll(&list_scroll)
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
                if snapshot.media.is_empty() {
                    grid = grid.child(
                        div()
                            .flex_1()
                            .w_full()
                            .items_center()
                            .justify_center()
                            .text_sm()
                            .text_color(rgb(theme::TEXT_SECONDARY))
                            .child(photag_ui_bridge::i18n::tr("No indexed media in this view.")),
                    );
                }
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
