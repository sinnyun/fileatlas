use super::*;

impl GalleryView {
    pub(super) fn selection_details(&self, root: Div, cx: &mut Context<Self>) -> Div {
        if !self.show_selection_details || self.selection.selected_media_ids.is_empty() {
            return root;
        }
        let Ok(snapshot) = &self.snapshot else {
            return root;
        };
        let mut panel = div().v_flex().gap_2();
        if self.show_selection_details && !self.selection.selected_media_ids.is_empty() {
            panel = panel.child(
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
                            .label(photag_ui_bridge::i18n::tr(
                                &(format!("{rating}★")).to_string(),
                            ))
                            .on_click(
                                cx.listener(move |this, _, _, cx| this.apply_rating(rating, cx)),
                            )
                    }))
                    .child(
                        Button::new("gallery-clear-selection")
                            .label(photag_ui_bridge::i18n::tr("Clear"))
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
                .child(
                    div()
                        .text_sm()
                        .font_semibold()
                        .child(photag_ui_bridge::i18n::tr("File actions")),
                );
            actions = actions.child(
                div().h_flex().gap_2().child(
                    Button::new("gallery-preview-selected")
                        .label(photag_ui_bridge::i18n::tr("Preview first selected"))
                        .on_click(cx.listener(|this, _, _, cx| this.preview_selected(cx))),
                ),
            );
            if self.show_selection_details && self.selection.selected_media_ids.len() == 1 {
                actions = actions.child(
                    div()
                        .h_flex()
                        .gap_2()
                        .child(
                            Button::new("gallery-open-selected")
                                .label(photag_ui_bridge::i18n::tr("Open"))
                                .on_click(
                                    cx.listener(|this, _, _, cx| this.open_selected(false, cx)),
                                ),
                        )
                        .child(
                            Button::new("gallery-reveal-selected")
                                .label(photag_ui_bridge::i18n::tr(&("Reveal").to_string()))
                                .on_click(
                                    cx.listener(|this, _, _, cx| this.open_selected(true, cx)),
                                ),
                        ),
                );
                actions = actions.child(
                    div()
                        .h_flex()
                        .gap_2()
                        .child(Input::new(&self.rename_input).w(px(240.)))
                        .child(
                            Button::new("gallery-rename-selected")
                                .label(photag_ui_bridge::i18n::tr("Rename"))
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
                            .label(photag_ui_bridge::i18n::tr("Copy…"))
                            .on_click(
                                cx.listener(|this, _, _, cx| this.copy_selected_with_dialog(cx)),
                            ),
                    )
                    .child(
                        Button::new("gallery-move-selected")
                            .label(photag_ui_bridge::i18n::tr("Move…"))
                            .on_click(
                                cx.listener(|this, _, _, cx| this.move_selected_with_dialog(cx)),
                            ),
                    )
                    .child(
                        Button::new("gallery-delete-selected")
                            .label(photag_ui_bridge::i18n::tr("Delete selected"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.request_delete_confirmation(window, cx)
                            })),
                    ),
            );
            panel = panel.child(actions);
        }
        if self.show_selection_details && self.selection.selected_media_ids.len() == 1 {
            let selected_id = self.selection.selected_media_ids[0];
            if let Some(media) = snapshot.media.iter().find(|media| media.id == selected_id) {
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
                panel = panel.child(render_media_details(
                    media,
                    tags,
                    open_count,
                    self.show_exif,
                    exif_fields,
                    cx,
                ));
            }
        }

        root.child(
            div()
                .absolute()
                .right(px(12.))
                .bottom(px(48.))
                .w(px(420.))
                .max_h(px(600.))
                .overflow_y_scrollbar()
                .bg(rgb(theme::SURFACE_MODAL))
                .rounded_md()
                .p_3()
                .child(panel),
        )
    }
}
