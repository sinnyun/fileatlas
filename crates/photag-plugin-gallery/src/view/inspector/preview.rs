//! Fixed Inspector preview and tab controls.

use super::*;
use gpui_kit::component::button::ButtonVariants;

impl GalleryView {
    pub(super) fn active_inspector_tab(&self) -> &'static str {
        match self.surfaces.value("gallery.inspector.tab").as_deref() {
            Some("history") => "history",
            Some("extensions") => "extensions",
            _ => "information",
        }
    }

    pub(super) fn inspector_tabs(&self, active: &str, cx: &mut Context<Self>) -> Div {
        let mut tabs = div()
            .h_flex()
            .w_full()
            .flex_shrink_0()
            .h(px(38.))
            .px(px(8.))
            .border_t_1()
            .border_b_1()
            .border_color(rgb(theme::SEPARATOR));
        for (key, label) in [
            ("information", "Information"),
            ("history", "History"),
            ("extensions", "Add-ons"),
        ] {
            let selected = active == key;
            tabs = tabs.child(
                Button::new(format!("gallery-inspector-tab-{key}"))
                    .ghost()
                    .compact()
                    .flex_1()
                    .h_full()
                    .text_color(rgb(if selected {
                        theme::accent()
                    } else {
                        theme::TEXT_SECONDARY
                    }))
                    .border_b_2()
                    .border_color(rgb(if selected {
                        theme::accent()
                    } else {
                        theme::SURFACE_MODAL
                    }))
                    .child(
                        div()
                            .w_full()
                            .h_flex()
                            .justify_center()
                            .child(photag_ui_bridge::i18n::tr(label)),
                    )
                    .toggled(selected)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let _ = this.surfaces.set_value("gallery.inspector.tab", key);
                        cx.notify();
                    })),
            );
        }
        tabs
    }

    pub(super) fn inspector_preview(&self, cx: &mut Context<Self>) -> AnyElement {
        let selected_count = self.selection.selected_media_ids.len();
        let media = self.inspector_media();
        let mut preview = div()
            .v_flex()
            .w_full()
            .flex_shrink_0()
            .gap(px(6.))
            .px(px(12.))
            .pt(px(8.))
            .pb(px(10.))
            .border_b_1()
            .border_color(rgb(theme::SEPARATOR));
        if let Some(media) = media {
            let media_id = media.id;
            let media_type = media.media_type.clone();
            let mut thumbnail = div()
                .id("gallery-inspector-preview-image")
                .relative()
                .w_full()
                // The reference keeps a near-square canvas in a roughly 300px
                // inspector. Width remains fluid; the details below scroll
                // independently when the window is short.
                .h(px(220.))
                .rounded(px(theme::ROUND_SM))
                .overflow_hidden()
                .bg(rgb(theme::SURFACE_PANEL))
                .flex()
                .items_center()
                .justify_center();
            if let Some(path) = &media.thumb_path {
                let fallback_type = media_type.clone();
                thumbnail = thumbnail.child(
                    img(std::path::PathBuf::from(path))
                        .w_full()
                        .h_full()
                        .object_fit(ObjectFit::Contain)
                        .with_fallback(move || preview_placeholder(&fallback_type)),
                );
            } else {
                thumbnail = thumbnail.child(preview_placeholder(&media_type));
            }
            preview = preview.child(
                thumbnail
                    .cursor_pointer()
                    .hover(|style| style.bg(rgba(theme::FILL_HOVER)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.preview_media(media_id, cx);
                    })),
            );
            preview = preview.child(
                div()
                    .w_full()
                    .overflow_hidden()
                    .text_ellipsis()
                    .text_size(px(theme::FONT_SIZE_BODY))
                    .text_color(rgb(theme::TEXT_PRIMARY))
                    .child(media.filename),
            );
            if selected_count == 1 {
                preview =
                    preview.child(self.rating_stars(media.id, media.rating.clamp(0, 5) as u8, cx));
            } else {
                preview = preview.child(
                    div()
                        .text_xs()
                        .text_color(rgb(theme::TEXT_SECONDARY))
                        .child(photag_ui_bridge::i18n::tr_args(
                            "{count} selected",
                            &[("count", selected_count.to_string())],
                        )),
                );
            }
        } else {
            preview = preview.child(
                div()
                    .w_full()
                    .h(px(220.))
                    .rounded(px(theme::ROUND_SM))
                    .bg(rgb(theme::SURFACE_PANEL))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(theme::FONT_SIZE_SMALL))
                    .text_color(rgb(theme::TEXT_MUTED))
                    .child(photag_ui_bridge::i18n::tr("No file selected")),
            );
        }
        preview.into_any_element()
    }

    fn inspector_media(&self) -> Option<photag_plugin_database::db::queries::MediaItem> {
        let media_id = *self.selection.selected_media_ids.first()?;
        self.snapshot
            .as_ref()
            .ok()
            .and_then(|snapshot| snapshot.media.iter().find(|media| media.id == media_id))
            .cloned()
            .or_else(|| self.service.media_by_id(media_id).ok().flatten())
    }

    fn rating_stars(&self, media_id: i64, rating: u8, cx: &mut Context<Self>) -> Div {
        let mut stars = div().h_flex().items_center().gap(px(3.));
        for star in 1..=5u8 {
            stars = stars.child(
                div()
                    .id(format!("gallery-inspector-preview-star-{media_id}-{star}"))
                    .cursor_pointer()
                    .text_size(px(17.))
                    .text_color(rgb(if star <= rating {
                        theme::STAR
                    } else {
                        theme::TEXT_MUTED
                    }))
                    .hover(|style| style.text_color(rgb(theme::STAR)))
                    .child(if star <= rating { "★" } else { "☆" })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let next = if star == rating { 0 } else { star };
                        let selected = this.selection.selected_media_ids.clone();
                        this.apply_rating_to(&[media_id], next);
                        this.restore_selection(selected);
                        cx.notify();
                    })),
            );
        }
        stars
    }
}

fn preview_placeholder(media_type: &str) -> AnyElement {
    let icon = match media_type {
        "video" => theme::icons::VIDEO,
        "audio" => theme::icons::AUDIO,
        _ => theme::icons::PICTURE,
    };
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .font_family(theme::ICON_FONT_FAMILY)
        .text_size(px(32.))
        .text_color(rgb(theme::TEXT_MUTED))
        .child(icon.to_string())
        .into_any_element()
}
