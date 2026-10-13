use super::*;

impl Render for GalleryView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let columns = ((f32::from(window.bounds().size.width) - 24.) / (self.thumb_size + 44.))
            .floor()
            .clamp(1., 4.) as usize;
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
        let tag_picker = if self.tagging.is_some() && !self.selection.selected_media_ids.is_empty()
        {
            Some(tagging_picker::render(self, cx))
        } else {
            None
        };
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
                    .size_full()
                    .gap_3()
                    .bg(rgba(theme::base_rgba(theme::SURFACE_BASE)))
                    .p_4()
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .justify_between()
                            .child(div().text_lg().font_semibold().child(
                                if !self.filter.selected_tag_ids.is_empty() {
                                    "Tagged Media"
                                } else if self.filter.folder_id.is_some() {
                                    "Selected Folder"
                                } else {
                                    "All Media"
                                },
                            ))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(theme::TEXT_SECONDARY))
                                    .child(format!("{} items", snapshot.media_count)),
                            ),
                    );
                let mut gallery_options =
                    div()
                        .h_flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(theme::TEXT_SECONDARY))
                                .child("Thumbnail size"),
                        )
                        .child(Button::new("gallery-thumb-smaller").label("−").on_click(
                            cx.listener(|this, _, _, cx| {
                                this.thumb_size = (this.thumb_size - 20.).max(100.);
                                cx.notify();
                            }),
                        ))
                        .child(
                            div()
                                .text_xs()
                                .child(format!("{} px", self.thumb_size as u32)),
                        )
                        .child(Button::new("gallery-thumb-larger").label("+").on_click(
                            cx.listener(|this, _, _, cx| {
                                this.thumb_size = (this.thumb_size + 20.).min(300.);
                                cx.notify();
                            }),
                        ));
                if matches!(
                    self.filter.sort,
                    SortOrder::NewestFirst | SortOrder::OldestFirst
                ) {
                    gallery_options = gallery_options.child(
                        Button::new("gallery-month-groups")
                            .label(if self.group_by_month {
                                "Months: on"
                            } else {
                                "Months: off"
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.set_month_grouping(!this.group_by_month, cx);
                            })),
                    );
                }
                grid = grid.child(gallery_options);
                grid = grid.child(
                    Button::new("gallery-statistics")
                        .label(if self.show_statistics {
                            "Hide statistics"
                        } else {
                            "Statistics"
                        })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.show_statistics = !this.show_statistics;
                            cx.notify();
                        })),
                );
                if self.show_statistics {
                    grid = grid.child(super::statistics::render(self, cx));
                }
                if self.video_preview_open {
                    grid = grid.child(
                        Button::new("gallery-return-from-video")
                            .label("Return to gallery")
                            .on_click(cx.listener(|this, _, _, cx| this.close_video_preview(cx))),
                    );
                }
                grid = grid.child(Input::new(&self.search).w_full());
                grid = grid.child(controls::search_scope_row(&self.filter, cx));
                grid = grid.child(controls::clear_filters_button(cx));
                grid = grid.child(controls::active_filter_row(
                    &self.filter,
                    &snapshot.tag_tree,
                    cx,
                ));
                grid = grid.child(controls::media_type_row(&self.filter, cx));
                grid = grid.child(controls::sort_row(&self.filter, cx));
                grid = grid.child(controls::rating_row(&self.filter, cx));
                grid = grid.child(controls::date_row(&self.date_from, &self.date_to));
                grid = grid.child(controls::extension_row(
                    &self.filter,
                    &snapshot.available_extensions,
                    self.extension_exclude_mode,
                    cx,
                ));
                if let Some(tag_picker) = tag_picker {
                    grid = grid.child(tag_picker);
                }
                grid = grid.child(bulk_actions::render(self, cx));
                if let Some(status) = &self.operation_status {
                    grid = grid.child(
                        div()
                            .text_sm()
                            .text_color(rgb(theme::TEXT_SECONDARY))
                            .child(status.clone()),
                    );
                }
                if self.tagging.is_some() {
                    for failure in photag_plugin_tagging::writeback::failures() {
                        let path = failure.path.clone();
                        let label = failure
                            .path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("media")
                            .to_string();
                        grid = grid.child(
                            div()
                                .h_flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .rounded_md()
                                .bg(rgb(theme::SURFACE_FAINT))
                                .p_2()
                                .child(
                                    div()
                                        .flex_1()
                                        .text_sm()
                                        .text_color(rgb(theme::TEXT_SECONDARY))
                                        .child(format!(
                                            "Tag metadata writeback failed for {label}: {}",
                                            failure.error
                                        )),
                                )
                                .child(
                                    Button::new(format!(
                                        "gallery-clear-writeback-{}",
                                        failure.path.display()
                                    ))
                                    .label("Dismiss")
                                    .on_click(cx.listener(
                                        move |_, _, _, cx| {
                                            photag_plugin_tagging::writeback::clear_failure(&path);
                                            cx.notify();
                                        },
                                    )),
                                ),
                        );
                    }
                }
                grid = grid.child(div().h_1().w_full().bg(rgb(theme::SURFACE_FAINT)));
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
                                                                    .w(px(card_size + 32.))
                                                                    .h(px(card_size + 64.))
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
                                                    .map(|label| {
                                                        let collapsed = layout_row.collapsed;
                                                        div().h_flex().h(px(20.)).child(
                                                            Button::new(format!(
                                                                "gallery-month-{label}"
                                                            ))
                                                            .label(format!(
                                                                "{} {label}",
                                                                if collapsed {
                                                                    "▸"
                                                                } else {
                                                                    "▾"
                                                                }
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
                                                    .unwrap_or_else(|| div().h(px(20.)));
                                                rows.push(
                                                    div()
                                                        .id(format!("gallery-row-{row_index}"))
                                                        .v_flex()
                                                        .w_full()
                                                        .h(px(if layout_row.collapsed {
                                                            48.
                                                        } else {
                                                            card_size + 92.
                                                        }))
                                                        .gap_2()
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
                            .child("No indexed media in this view."),
                    );
                }
                if !self.selection.selected_media_ids.is_empty() {
                    grid = grid.child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap_1()
                            .child(div().text_sm().child(format!(
                                "{} selected · Rate",
                                self.selection.selected_media_ids.len()
                            )))
                            .children((0..=5).map(|rating| {
                                Button::new(format!("gallery-rating-{rating}"))
                                    .label(format!("{rating}★"))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.apply_rating(rating, cx)
                                    }))
                            }))
                            .child(
                                Button::new("gallery-clear-selection")
                                    .label("Clear")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.selection = GallerySelection::default();
                                        this.persist_filter_session();
                                        cx.notify();
                                    })),
                            ),
                    );
                    let mut actions = div()
                        .v_flex()
                        .gap_2()
                        .rounded_md()
                        .bg(rgb(theme::SURFACE_PANEL))
                        .p_3()
                        .child(div().text_sm().font_semibold().child("File actions"));
                    if self.selection.selected_media_ids.len() == 1 {
                        actions = actions.child(
                            div()
                                .h_flex()
                                .gap_2()
                                .child(
                                    Button::new("gallery-preview-selected")
                                        .label("Preview")
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.preview_selected(cx)),
                                        ),
                                )
                                .child(Button::new("gallery-open-selected").label("Open").on_click(
                                    cx.listener(|this, _, _, cx| this.open_selected(false, cx)),
                                ))
                                .child(
                                    Button::new("gallery-reveal-selected")
                                        .label("Reveal")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.open_selected(true, cx)
                                        })),
                                ),
                        );
                        actions = actions.child(
                            div()
                                .h_flex()
                                .gap_2()
                                .child(Input::new(&self.rename_input).w(px(240.)))
                                .child(
                                    Button::new("gallery-rename-selected")
                                        .label("Rename")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.rename_selected_with_repair(cx)
                                        })),
                                ),
                        );
                    }
                    actions = actions.child(
                        div()
                            .h_flex()
                            .gap_2()
                            .child(
                                Button::new("gallery-copy-selected")
                                    .label("Copy…")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.copy_selected_with_dialog(cx)
                                    })),
                            )
                            .child(
                                Button::new("gallery-move-selected")
                                    .label("Move…")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.move_selected_with_dialog(cx)
                                    })),
                            )
                            .child(
                                Button::new("gallery-delete-selected")
                                    .label("Delete selected")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.request_delete_confirmation(window, cx)
                                    })),
                            ),
                    );
                    grid = grid.child(actions);
                }
                if self.selection.selected_media_ids.len() == 1 {
                    let selected_id = self.selection.selected_media_ids[0];
                    if let Some(media) = snapshot.media.iter().find(|media| media.id == selected_id)
                    {
                        let tags = snapshot
                            .media_tags
                            .get(&selected_id)
                            .map(Vec::as_slice)
                            .unwrap_or_default();
                        let open_count = self.service.open_count(selected_id).unwrap_or(0);
                        let exif_fields = self
                            .exif_cache
                            .as_ref()
                            .filter(|(media_id, _)| *media_id == selected_id)
                            .map(|(_, fields)| fields);
                        grid = grid.child(render_media_details(
                            media,
                            tags,
                            open_count,
                            self.show_exif,
                            exif_fields,
                            cx,
                        ));
                    }
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
