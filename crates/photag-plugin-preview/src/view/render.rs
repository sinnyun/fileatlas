use super::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::Icon;

impl Render for PreviewView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let locale_revision = photag_ui_bridge::i18n::revision();
        if self.locale_revision != locale_revision {
            self.locale_revision = locale_revision;
            self.rename_input.update(cx, |input, cx| {
                input.set_placeholder(photag_ui_bridge::i18n::tr("New filename"), window, cx)
            });
            self.tag_picker_search.update(cx, |input, cx| {
                input.set_placeholder(photag_ui_bridge::i18n::tr("Filter tags…"), window, cx)
            });
        }

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
        if matches!(
            snapshot.media_kind,
            Some(PreviewMediaKind::Video | PreviewMediaKind::Audio)
        ) {
            if let Some(path) = snapshot.source.as_ref() {
                if self.last_playback_source.as_ref() != Some(path) {
                    self.last_playback_source = Some(path.clone());
                    match self.service.playback() {
                        Some(playback) => {
                            if let Err(error) = playback.open_media(path, snapshot.media_id) {
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
        let media_kind = snapshot.media_kind;
        let is_image = media_kind == Some(PreviewMediaKind::Image);
        let display = if matches!(
            media_kind,
            Some(PreviewMediaKind::Audio | PreviewMediaKind::Video)
        ) {
            Ok(None)
        } else {
            self.service.display_path()
        };
        // `preview.rs:1693`: the floating bands key off "can something be shown",
        // not "has the full-resolution decode finished".
        let has_display = matches!(&display, Ok(Some(_)));
        let title = snapshot
            .source
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "No media selected".into());

        let bounds = photag_ui_bridge::panels::content_bounds(&self.surfaces, "preview", window);
        let viewport = (bounds.width, bounds.height);
        // `preview.rs:1750`: the fit box covers 90% of the preview area.
        let fit_width = viewport.0 * 0.9;
        let fit_height = viewport.1 * 0.9;

        let (image, fit_scale) = match display {
            Ok(Some(path)) => {
                // `preview.rs:1763`: only a full-resolution decode caps at 1x;
                // a low-resolution first frame is upscaled to the fit box.
                let progressive = snapshot
                    .source
                    .as_deref()
                    .and_then(|source| self.service.progressive_path(source));
                let is_progressive = progressive.as_ref() == Some(&path);
                let dimensions = image::image_dimensions(&path).ok();
                let fit_scale = dimensions.map_or(1.0, |(width, height)| {
                    let raw = (fit_width / width as f32).min(fit_height / height as f32);
                    if is_progressive { raw } else { raw.min(1.0) }
                });
                let scale = if snapshot.fit_to_window {
                    fit_scale
                } else {
                    snapshot.zoom_scale
                };
                let image = match dimensions {
                    Some((width, height)) => img(path)
                        .w(px(width as f32 * scale))
                        .h(px(height as f32 * scale))
                        .object_fit(ObjectFit::Contain)
                        .relative()
                        .left(px(snapshot.pan_x))
                        .top(px(snapshot.pan_y))
                        .into_any_element(),
                    None => img(path)
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
                        .label(photag_ui_bridge::i18n::tr(
                            &(if rating == 0 {
                                "No rating".into()
                            } else {
                                format!("{rating}★")
                            })
                            .to_string(),
                        ))
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
                    .child(photag_ui_bridge::i18n::tr(
                        "Video is playing in Media Playback.",
                    ))
                    .into_any_element(),
                1.0,
            ),
            Ok(None) => (
                div()
                    .text_sm()
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child(photag_ui_bridge::i18n::tr(
                        "Open an indexed image to preview it.",
                    ))
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
        // `preview.rs:1934-1938`: the header percentage truncates and reports
        // the fitted scale while fit mode is on.
        let service = Arc::clone(&self.service);
        let can_zoom = has_display;
        let cursor_center = (bounds.x + viewport.0 / 2.0, bounds.y + viewport.1 / 2.0);
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
        let can_pan = has_display;
        let down = cx.listener(move |this, event: &MouseDownEvent, window, cx| {
            window.focus(&this.focus_handle, cx);
            this.last_drag_position =
                can_pan.then_some((f32::from(event.position.x), f32::from(event.position.y)));
        });
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
            .absolute()
            .top(px(52.0))
            .bottom(px(if self.filmstrip_visible() && is_image {
                154.0
            } else {
                54.0
            }))
            .left_0()
            .right_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgb(if media_kind == Some(PreviewMediaKind::Audio) {
                theme::SURFACE_AUDIO
            } else {
                theme::SURFACE_EXTREME
            }))
            .overflow_hidden()
            .on_scroll_wheel(wheel)
            .on_mouse_down(MouseButton::Left, down)
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    this.context_menu_open = true;
                    this.context_menu_anchor = Some((
                        f32::from(event.position.x) - bounds.x,
                        f32::from(event.position.y) - bounds.y,
                    ));
                    cx.notify();
                }),
            )
            .on_mouse_move(moved)
            .on_mouse_up(MouseButton::Left, up)
            .child(image);
        if snapshot.show_info {
            if let Some(path) = snapshot.source.as_deref() {
                let indexed = snapshot
                    .media_id
                    .and_then(|media_id| support::indexed_info(&self.service, media_id));
                stage = stage.child(
                    div()
                        .absolute()
                        .right(px(12.0))
                        .top(px(12.0))
                        .w(px(310.0))
                        .h(px((viewport.1
                            - if self.filmstrip_visible() { 210. } else { 130. })
                        .max(100.)))
                        .occlude()
                        .child(crate::info::info_panel(
                            path,
                            indexed,
                            &self.collapsed_info_sections,
                            cx,
                        )),
                );
            }
        }

        let mut root = div()
            .absolute()
            .inset_0()
            .bg(rgb(theme::SURFACE_EXTREME))
            .track_focus(&self.focus_handle)
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                this.service.mark_activity();
                this.move_toolbar_drag(
                    (f32::from(event.position.x), f32::from(event.position.y)),
                    cx,
                );
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _: &MouseUpEvent, _, cx| {
                    this.end_toolbar_drag(cx);
                }),
            )
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.handle_key(event, window, cx)
            }))
            .child(stage);

        // `preview.rs:1695-1700`: the strip paints before the toolbar so the
        // toolbar can stack above it at its lifted anchor.
        if let Some(plate) = filmstrip::plate(self, viewport.0, cx) {
            root = root.child(plate);
        }
        if let Some(toolbar) = self.render_toolbar(has_display, viewport, window, cx) {
            root = root.child(toolbar);
        }

        // The v5 shell keeps identity and primary actions in one fixed top bar.
        if chrome_alpha > 0.01 {
            let info = cx.listener(|this, _: &ClickEvent, _, cx| {
                this.service.toggle_info();
                cx.notify();
            });
            let ai = cx.listener(|this, _: &ClickEvent, _, cx| {
                if let Some(media_id) = this.service.snapshot().media_id {
                    this.service.request_ai_suggestions(media_id);
                    cx.notify();
                }
            });
            let exif = cx.listener(|this, _: &ClickEvent, _, cx| {
                this.service.toggle_info();
                cx.notify();
            });
            let close = cx.listener(|this, _: &ClickEvent, _, cx| {
                this.rename_target = None;
                this.service.close();
                cx.notify();
            });
            root = root.child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(px(52.0))
                    .px_3()
                    .h_flex()
                    .items_center()
                    .gap_3()
                    .bg(rgba(theme::modal_rgba(theme::SURFACE_BASE)))
                    .border_b_1()
                    .border_color(rgba(theme::SEPARATOR_LIGHT))
                    .opacity(chrome_alpha)
                    .child(Icon::new(IconName::Aperture).text_color(rgb(theme::accent())))
                    .child(div().text_sm().font_medium().child("PhoTag"))
                    .child(
                        div()
                            .w(px(1.0))
                            .h(px(24.0))
                            .bg(rgba(theme::SEPARATOR_LIGHT)),
                    )
                    .child(div().flex_1().min_w_0().text_sm().child(title.clone()))
                    .child(button("Info").icon(IconName::Info).on_click(info))
                    .child(
                        button("AI")
                            .icon(IconName::Sparkles)
                            .disabled(snapshot.media_id.is_none())
                            .on_click(ai),
                    )
                    .child(button("EXIF").icon(IconName::Tag).on_click(exif))
                    .child(
                        button("Close")
                            .icon(IconName::Close)
                            .tooltip(photag_ui_bridge::i18n::tr("Close"))
                            .on_click(close),
                    ),
            );
        }

        if is_image && !has_display {
            root = root.child(overlays::loading_label());
        }
        if self.rename_target.is_some() {
            root = root.child(overlays::rename_editor(self, cx));
        }
        if let Some(status) = &self.operation_status {
            root = root.child(
                div()
                    .absolute()
                    .left(px(8.0))
                    .bottom(px(4.0))
                    .text_xs()
                    .child(status.clone()),
            );
        }
        if let Some(error) = &self.toolbar_error {
            root = root.child(
                div()
                    .absolute()
                    .left(px(8.0))
                    .bottom(px(20.0))
                    .text_xs()
                    .text_color(rgb(theme::DANGER))
                    .child(format!("Could not save preview toolbar position: {error}")),
            );
        }
        if self.context_menu_open {
            let anchor = self.context_menu_anchor.unwrap_or((viewport.0 / 2.0, 60.0));
            root = root.child(
                div()
                    .absolute()
                    .left(px(anchor.0))
                    .top(px(anchor.1))
                    .occlude()
                    .child(context_menu::render(self, cx)),
            );
        }
        root
    }
}
