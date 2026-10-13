use super::*;

impl Render for PreviewView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.service.snapshot();
        if snapshot.source.is_none() {
            return div().size_0();
        }
        let chrome_alpha = self.service.chrome_alpha();
        if snapshot.source != self.last_seen_source {
            self.last_seen_source = snapshot.source.clone();
            if snapshot.source.is_some() {
                window.focus(&self.focus_handle, cx);
            }
        }
        if snapshot.media_kind == Some(PreviewMediaKind::Video) {
            if let Some(path) = snapshot.source.as_ref() {
                if self.last_playback_source.as_ref() != Some(path) {
                    self.last_playback_source = Some(path.clone());
                    match self.service.playback() {
                        Some(playback) => {
                            if let Err(error) = playback.open(path) {
                                if matches!(
                                    &error,
                                    photag_plugin_media_playback::PlaybackError::ProbeFailed(_)
                                ) {
                                    // PhoTag launches the media's default app when FFprobe
                                    // cannot identify it, then leaves the in-app preview.
                                    let _ = open::that_detached(path);
                                    self.service.close();
                                } else {
                                    self.operation_status = Some(error.to_string());
                                }
                            }
                        }
                        None => {
                            self.operation_status =
                                Some("Media playback service is unavailable.".into());
                        }
                    }
                }
            }
        } else if self.last_playback_source.take().is_some() {
            if let Some(playback) = self.service.playback() {
                playback.close();
            }
        }
        let display = if matches!(
            snapshot.media_kind,
            Some(PreviewMediaKind::Audio | PreviewMediaKind::Video)
        ) {
            Ok(None)
        } else {
            self.service.display_path()
        };
        let title = snapshot
            .source
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "No media selected".into());
        let service = Arc::clone(&self.service);
        let close = cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.rename_target = None;
            service.close();
            cx.notify();
        });
        let rename = cx.listener(|this, _: &ClickEvent, window, cx| this.begin_rename(window, cx));
        let delete =
            cx.listener(|this, _: &ClickEvent, window, cx| this.request_delete(window, cx));
        let service = Arc::clone(&self.service);
        let rotate_left = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.rotate_counter_clockwise();
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let rotate_right = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.rotate_clockwise();
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let flip_h = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.flip_horizontal();
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let flip_v = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.flip_vertical();
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let fit = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.set_fit();
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let info = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.toggle_info();
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let zoom_25 = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.set_zoom_percent(25);
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let zoom_50 = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.set_zoom_percent(50);
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let zoom_100 = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.set_zoom_percent(100);
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let zoom_125 = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.set_zoom_percent(125);
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let zoom_150 = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.set_zoom_percent(150);
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let zoom_200 = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.set_zoom_percent(200);
            cx.notify();
        });
        let service = Arc::clone(&self.service);
        let zoom_mode = cx.listener(move |_, _: &ClickEvent, _, cx| {
            service.toggle_zoom_at_cursor();
            cx.notify();
        });
        let media_kind = snapshot.media_kind;
        let is_image = media_kind == Some(PreviewMediaKind::Image);
        let toolbar_settings = self
            .settings
            .snapshot()
            .map(|settings| settings.toolbar)
            .unwrap_or_default();
        let controls = div()
            .h_flex()
            .items_center()
            .flex_wrap()
            .max_w_full()
            .gap_2()
            .when(is_image && toolbar_settings.rotate_buttons, |controls| {
                controls
                    .child(button("Rotate left").on_click(rotate_left))
                    .child(button("Rotate right").on_click(rotate_right))
            })
            .when(is_image && toolbar_settings.flip_buttons, |controls| {
                controls
                    .child(button("Flip H").on_click(flip_h))
                    .child(button("Flip V").on_click(flip_v))
            })
            .when(is_image && toolbar_settings.zoom_reset_button, |controls| {
                controls.child(button("Fit").on_click(fit))
            })
            .when(toolbar_settings.tags_button, |controls| {
                controls.child(
                    button(if self.tag_picker_open {
                        "Hide tags"
                    } else {
                        "Tags"
                    })
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.tag_picker_open = !this.tag_picker_open;
                        cx.notify();
                    })),
                )
            })
            .when(is_image && toolbar_settings.zoom_presets, |controls| {
                controls
                    .child(button("25%").on_click(zoom_25))
                    .child(button("50%").on_click(zoom_50))
                    .child(button("100%").on_click(zoom_100))
                    .child(button("125%").on_click(zoom_125))
                    .child(button("150%").on_click(zoom_150))
                    .child(button("200%").on_click(zoom_200))
            })
            .when(is_image && toolbar_settings.zoom_mode_button, |controls| {
                controls.child(
                    button(if snapshot.zoom_at_cursor {
                        "Zoom: cursor"
                    } else {
                        "Zoom: center"
                    })
                    .on_click(zoom_mode),
                )
            })
            .when(toolbar_settings.filmstrip_toggle, |controls| {
                controls.child(
                    button(if self.show_filmstrip {
                        "Hide filmstrip"
                    } else {
                        "Show filmstrip"
                    })
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        let show = !this.show_filmstrip;
                        match this
                            .settings
                            .update(|settings| settings.show_filmstrip = show)
                        {
                            Ok(_) => {
                                this.show_filmstrip = show;
                                this.toolbar_error = None;
                            }
                            Err(error) => {
                                this.operation_status =
                                    Some(format!("Could not save filmstrip visibility: {error}"))
                            }
                        }
                        cx.notify();
                    })),
                )
            })
            .child(
                button(if snapshot.show_info {
                    "Hide info"
                } else {
                    "Info"
                })
                .on_click(info),
            )
            .child(button("Rename (F2)").on_click(rename))
            .when(toolbar_settings.open_external_button, |controls| {
                controls.child(button("Open externally").on_click(
                    cx.listener(|this, _: &ClickEvent, _, cx| this.open_in_default_app(cx)),
                ))
            })
            .when(toolbar_settings.explorer_button, |controls| {
                controls.child(button("Show in folder").on_click(
                    cx.listener(|this, _: &ClickEvent, _, cx| this.reveal_in_file_manager(cx)),
                ))
            })
            .when(toolbar_settings.delete_button, |controls| {
                controls.child(button("Delete").on_click(delete))
            })
            .child(button("Close").on_click(close));
        let bounds = window.bounds().size;
        let viewport_width = (f32::from(bounds.width) - 32.0).max(1.0);
        let viewport_height = (f32::from(bounds.height) - 112.0).max(1.0);
        let has_image = matches!(&display, Ok(Some(_)));
        let (image, fit_scale) = match display {
            Ok(Some(path)) => {
                let dimensions = image::image_dimensions(&path).ok();
                let display_path = if snapshot.fit_to_window {
                    snapshot
                        .source
                        .as_deref()
                        .and_then(|source| self.service.progressive_path(source))
                        .unwrap_or(path)
                } else {
                    path
                };
                let fit_scale = dimensions.map_or(1.0, |(width, height)| {
                    (viewport_width / width as f32)
                        .min(viewport_height / height as f32)
                        .min(1.0)
                });
                let image = match dimensions {
                    Some((width, height)) if !snapshot.fit_to_window => img(display_path)
                        .w(px(width as f32 * snapshot.zoom_scale))
                        .h(px(height as f32 * snapshot.zoom_scale))
                        .object_fit(ObjectFit::Contain)
                        .relative()
                        .left(px(snapshot.pan_x))
                        .top(px(snapshot.pan_y))
                        .into_any_element(),
                    _ => img(display_path)
                        .max_w_full()
                        .max_h_full()
                        .object_fit(ObjectFit::Contain)
                        .relative()
                        .left(px(snapshot.pan_x))
                        .top(px(snapshot.pan_y))
                        .into_any_element(),
                };
                (image, fit_scale)
            }
            Ok(None) if media_kind == Some(PreviewMediaKind::Audio) => {
                let open = cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.open_in_default_app(cx);
                });
                let ratings = (0..=5).map(|rating| {
                    Button::new(format!("preview-audio-rating-{rating}"))
                        .label(if rating == 0 {
                            "No rating".into()
                        } else {
                            format!("{rating}★")
                        })
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.apply_rating_shortcut(rating, cx);
                        }))
                });
                (
                    div()
                        .v_flex()
                        .items_center()
                        .justify_center()
                        .gap_3()
                        .text_color(rgb(theme::TEXT_PRIMARY))
                        .child(div().text_2xl().child("♫"))
                        .child(div().text_lg().font_medium().child(title.clone()))
                        .child(button("▶  Open in default player").on_click(open))
                        .child(div().h_flex().gap_1().children(ratings))
                        .into_any_element(),
                    1.0,
                )
            }
            Ok(None) if media_kind == Some(PreviewMediaKind::Video) => (
                div()
                    .v_flex()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .text_color(rgb(theme::TEXT_PRIMARY))
                    .child(div().text_2xl().child("▶"))
                    .child(div().text_lg().font_medium().child(title.clone()))
                    .child("Video is playing in Media Playback.")
                    .into_any_element(),
                1.0,
            ),
            Ok(None) => (
                div()
                    .text_sm()
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child("Open an indexed image to preview it.")
                    .into_any_element(),
                1.0,
            ),
            Err(error) => (
                div()
                    .text_sm()
                    .text_color(rgb(theme::DANGER))
                    .child(error.to_string())
                    .into_any_element(),
                1.0,
            ),
        };
        let service = Arc::clone(&self.service);
        let can_zoom = has_image;
        let cursor_center = (
            f32::from(bounds.width) / 2.0,
            f32::from(bounds.height) / 2.0,
        );
        let wheel = cx.listener(move |_, event: &ScrollWheelEvent, _, cx| {
            if !can_zoom {
                return;
            }
            let delta = event.delta.pixel_delta(px(40.0));
            let scroll_y = f32::from(delta.y);
            let cursor_delta = (
                f32::from(event.position.x) - cursor_center.0,
                f32::from(event.position.y) - cursor_center.1,
            );
            service.zoom_wheel(scroll_y, fit_scale, cursor_delta);
            cx.notify();
        });
        let can_pan = has_image;
        let down = cx.listener(move |this, event: &MouseDownEvent, window, cx| {
            window.focus(&this.focus_handle, cx);
            this.last_drag_position =
                can_pan.then_some((f32::from(event.position.x), f32::from(event.position.y)));
        });
        let can_pan = has_image;
        let moved = cx.listener(move |this, event: &MouseMoveEvent, _, cx| {
            this.service.mark_activity();
            if !can_pan || !event.dragging() {
                this.last_drag_position = None;
                return;
            }
            let current = (f32::from(event.position.x), f32::from(event.position.y));
            if let Some(previous) = this.last_drag_position.replace(current) {
                this.service
                    .pan_by(current.0 - previous.0, current.1 - previous.1);
                cx.notify();
            }
        });
        let up = cx.listener(|this, _: &MouseUpEvent, _, _| {
            this.last_drag_position = None;
        });
        let mut stage = div()
            .v_flex()
            .relative()
            .flex_1()
            .min_h_0()
            .w_full()
            .bg(rgba(if media_kind == Some(PreviewMediaKind::Audio) {
                (theme::SURFACE_AUDIO << 8) | 0xff
            } else {
                theme::base_rgba(theme::SURFACE_BASE)
            }))
            .overflow_hidden();
        stage = stage.child(
            div()
                .h_flex()
                .flex_1()
                .min_h_0()
                .w_full()
                .items_center()
                .justify_center()
                .overflow_hidden()
                .on_scroll_wheel(wheel)
                .on_mouse_down(MouseButton::Left, down)
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(|this, _: &MouseDownEvent, _, cx| {
                        this.context_menu_open = true;
                        cx.notify();
                    }),
                )
                .on_mouse_move(moved)
                .on_mouse_up(MouseButton::Left, up)
                .child(image),
        );
        if snapshot.show_info {
            if let Some(path) = snapshot.source.as_deref() {
                let indexed = snapshot
                    .media_id
                    .and_then(|media_id| support::indexed_info(&self.service, media_id));
                stage = stage.child(
                    div()
                        .absolute()
                        .left(px(12.0))
                        .top(px(12.0))
                        .max_h_full()
                        .child(crate::info::info_panel(path, indexed)),
                );
            }
        }
        let saved_toolbar_pos = self
            .settings
            .snapshot()
            .map(|settings| settings.toolbar_pos)
            .unwrap_or_default();
        let (toolbar_x, toolbar_y) = self
            .toolbar_drag_offset
            .unwrap_or((saved_toolbar_pos.offset_x, saved_toolbar_pos.offset_y));
        let (toolbar_x, toolbar_y) =
            clamp_toolbar_offset(toolbar_x, toolbar_y, viewport_width, viewport_height);
        let drag_start = cx.listener(move |this, event: &MouseDownEvent, window, cx| {
            window.focus(&this.focus_handle, cx);
            let saved = this
                .settings
                .snapshot()
                .ok()
                .map(|settings| settings.toolbar_pos);
            if saved.is_some_and(|position| !position.locked) {
                this.toolbar_drag_origin = Some((
                    f32::from(event.position.x),
                    f32::from(event.position.y),
                    toolbar_x,
                    toolbar_y,
                ));
                this.toolbar_drag_offset = Some((toolbar_x, toolbar_y));
            }
        });
        let drag_move = cx.listener(move |this, event: &MouseMoveEvent, _, cx| {
            this.service.mark_activity();
            if !event.dragging() {
                return;
            }
            if let Some((start_x, start_y, offset_x, offset_y)) = this.toolbar_drag_origin {
                this.toolbar_drag_offset = Some(clamp_toolbar_offset(
                    offset_x + f32::from(event.position.x) - start_x,
                    offset_y + f32::from(event.position.y) - start_y,
                    viewport_width,
                    viewport_height,
                ));
                cx.notify();
            }
        });
        let drag_end = cx.listener(|this, _: &MouseUpEvent, _, cx| {
            if let Some((offset_x, offset_y)) = this.toolbar_drag_offset.take() {
                this.toolbar_drag_origin = None;
                match this.settings.update(|settings| {
                    settings.toolbar_pos.offset_x = offset_x;
                    settings.toolbar_pos.offset_y = offset_y;
                }) {
                    Ok(_) => this.toolbar_error = None,
                    Err(error) => this.toolbar_error = Some(error.to_string()),
                }
                cx.notify();
            }
        });
        let toolbar = div()
            .absolute()
            .bottom(px(24.0 - toolbar_y))
            .left(px(toolbar_x))
            .w_full()
            .h_flex()
            .items_center()
            .justify_center()
            .gap_2()
            .child(
                div()
                    .id("preview-toolbar-drag-handle")
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .bg(rgba(theme::modal_rgba(theme::SURFACE_MODAL)))
                    .text_color(rgb(if saved_toolbar_pos.locked {
                        theme::TEXT_SECONDARY
                    } else {
                        theme::TEXT_PRIMARY
                    }))
                    .child(if saved_toolbar_pos.locked {
                        "🔒"
                    } else {
                        "⠿"
                    })
                    .on_mouse_down(MouseButton::Left, drag_start),
            )
            .child(controls)
            .on_scroll_wheel(cx.listener(|this, _: &ScrollWheelEvent, _, _| {
                this.service.mark_activity();
            }));
        stage = stage
            .on_mouse_move(drag_move)
            .on_mouse_up(MouseButton::Left, drag_end)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, _| this.service.mark_activity()),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, _: &MouseDownEvent, _, _| this.service.mark_activity()),
            )
            .when(chrome_alpha > 0.01, |stage| {
                stage.child(toolbar.opacity(chrome_alpha))
            });
        if let Some(error) = &self.toolbar_error {
            stage = stage.child(
                div()
                    .absolute()
                    .bottom(px(4.0))
                    .left(px(8.0))
                    .text_xs()
                    .text_color(rgb(theme::DANGER))
                    .child(format!("Could not save preview toolbar position: {error}")),
            );
        }
        let items = self.service.filmstrip_items();
        let paths: Vec<_> = items.iter().map(|(path, _)| path.clone()).collect();
        let current = snapshot
            .source
            .as_ref()
            .and_then(|source| paths.iter().position(|path| path == source));
        let strip = if self.show_filmstrip && paths.len() > 1 && chrome_alpha > 0.01 {
            let center = current.unwrap_or(0);
            let start = center.saturating_sub(3);
            let end = (start + 7).min(paths.len());
            let items = paths[start..end]
                .iter()
                .enumerate()
                .map(|(offset, path)| {
                    let path = path.clone();
                    let media_kind = classify_media_path(&path);
                    let media_id = items[start + offset].1;
                    let service = Arc::clone(&self.service);
                    let selected = start + offset == center;
                    let thumbnail = self.service.filmstrip_thumbnail(&path);
                    div()
                        .id(format!("preview-filmstrip-{offset}"))
                        .w(px(68.0))
                        .h(px(68.0))
                        .p_1()
                        .rounded_md()
                        .bg(rgb(if selected {
                            theme::SURFACE_FAINT
                        } else {
                            theme::modal_rgba(theme::SURFACE_MODAL)
                        }))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                                window.focus(&this.focus_handle, cx);
                                let _ = service.open_media(path.clone(), media_id);
                                cx.notify();
                            }),
                        )
                        .child(match thumbnail {
                            Some(thumbnail) => img(thumbnail)
                                .w_full()
                                .h_full()
                                .object_fit(ObjectFit::Contain)
                                .into_any_element(),
                            None => div()
                                .size_full()
                                .items_center()
                                .justify_center()
                                .text_xs()
                                .child(match media_kind {
                                    PreviewMediaKind::Audio => "♫",
                                    PreviewMediaKind::Video => "▶",
                                    _ => "…",
                                })
                                .into_any_element(),
                        })
                })
                .collect::<Vec<_>>();
            div()
                .h_flex()
                .justify_center()
                .gap_1()
                .children(items)
                .opacity(chrome_alpha)
                .into_any_element()
        } else {
            div().into_any_element()
        };
        let tag_picker_panel = (toolbar_settings.tags_button && self.tag_picker_open)
            .then(|| tag_picker::render(self, cx));
        let context_menu_panel = self
            .context_menu_open
            .then(|| context_menu::render(self, cx));
        div()
            .v_flex()
            .absolute()
            .inset_0()
            .gap_2()
            .p_3()
            .bg(rgba(0x000000dd))
            .track_focus(&self.focus_handle)
            .on_mouse_move(cx.listener(|this, _: &MouseMoveEvent, _, _| {
                this.service.mark_activity();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.handle_key(event, window, cx)
            }))
            .child(support::title(title, chrome_alpha))
            .when_some(context_menu_panel, |root, panel| root.child(panel))
            .when_some(tag_picker_panel, |root, panel| root.child(panel))
            .when_some(self.rename_target, |root, _| {
                root.child(
                    div()
                        .h_flex()
                        .gap_2()
                        .child(Input::new(&self.rename_input).w(px(260.)))
                        .child(
                            Button::new("preview-rename-commit")
                                .label("Rename")
                                .on_click(cx.listener(|this, _, _, cx| this.commit_rename(cx))),
                        )
                        .child(
                            Button::new("preview-rename-cancel")
                                .label("Cancel")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.rename_target = None;
                                    cx.notify();
                                })),
                        ),
                )
            })
            .when_some(self.operation_status.as_ref(), |root, status| {
                root.child(div().text_sm().child(status.clone()))
            })
            .child(stage)
            .child(strip)
    }
}
