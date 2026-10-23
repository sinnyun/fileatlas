//! Fixed Inspector preview and tab controls.

use super::*;
use fileatlas_ui_bridge::components::WorkbenchPaint as _;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;

impl FileManagerView {
    pub(super) fn active_inspector_tab(&self) -> &'static str {
        match self.surfaces.value("file_manager.inspector.tab").as_deref() {
            Some("history") => "history",
            Some("extensions") => "extensions",
            _ => "information",
        }
    }

    pub(super) fn inspector_tabs(&self, active: &str, cx: &mut Context<Self>) -> Div {
        let mut tabs = div()
            .h_flex()
            .items_center()
            .w_full()
            .flex_shrink_0()
            .h(px(theme::ui_height(theme::CONTROL_HEIGHT_LARGE)))
            .px(px(fileatlas_ui_bridge::theme::space_standard()))
            .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Top)
            .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Bottom);
        for (key, label) in [
            ("information", "Information"),
            ("history", "History"),
            ("extensions", "Add-ons"),
        ] {
            let selected = active == key;
            tabs = tabs.child(
                fileatlas_ui_bridge::components::tab_button(
                    format!("file_manager-inspector-tab-{key}"),
                    fileatlas_ui_bridge::i18n::tr(label),
                    selected,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    let _ = this.surfaces.set_value("file_manager.inspector.tab", key);
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
            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
            .px(px(fileatlas_ui_bridge::theme::space_expanded()))
            .pt(px(fileatlas_ui_bridge::theme::space_standard()))
            .pb(px(fileatlas_ui_bridge::theme::space_standard()))
            .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Bottom);
        if let Some(media) = media {
            let media_id = media.id;
            let media_type = media.media_type.clone();
            let mut thumbnail = div()
                .id("file_manager-inspector-preview-image")
                .relative()
                .w_full()
                // The reference keeps a near-square canvas in a roughly 300px
                // inspector. Width remains fluid; the details below scroll
                // independently when the window is short.
                .h(px(180.))
                .workbench_corner(fileatlas_ui_bridge::components::CornerRole::Small)
                .overflow_hidden()
                .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Panel)
                .flex()
                .items_center()
                .justify_center();
            if let Some(path) = &media.thumb_path {
                let fallback_type = media_type.clone();
                let filename = media.filename.clone();
                thumbnail = thumbnail.child(
                    img(std::path::PathBuf::from(path))
                        .w_full()
                        .h_full()
                        .object_fit(ObjectFit::Contain)
                        .with_fallback(move || {
                            super::super::file_format::placeholder(&filename, &fallback_type, 40.)
                                .into_any_element()
                        }),
                );
            } else {
                thumbnail = thumbnail.child(super::super::file_format::placeholder(
                    &media.filename,
                    &media_type,
                    40.,
                ));
            }
            preview = preview.child(
                thumbnail
                    .cursor_pointer()
                    .workbench_hover_surface()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.preview_media(media_id, cx);
                    })),
            );
            preview = preview.child(
                div()
                    .w_full()
                    .overflow_hidden()
                    .text_ellipsis()
                    .workbench_body()
                    .text_center()
                    .workbench_foreground(fileatlas_ui_bridge::components::ForegroundRole::Primary)
                    .child(media.filename),
            );
            if selected_count == 1 {
                preview =
                    preview.child(self.rating_stars(media.id, media.rating.clamp(0, 5) as u8, cx));
            } else {
                preview = preview.child(div().workbench_assistant().child(
                    fileatlas_ui_bridge::i18n::tr_args(
                        "{count} selected",
                        &[("count", selected_count.to_string())],
                    ),
                ));
            }
        } else {
            let message = if selected_count > 0
                && self.selection.selected_media_ids.first().is_some_and(|id| {
                    self.inspector_read.key() == Some(id) && self.inspector_read.is_pending()
                }) {
                "Loading file info…"
            } else {
                "No file selected"
            };
            preview = preview.child(
                div()
                    .w_full()
                    .h(px(220.))
                    .workbench_corner(fileatlas_ui_bridge::components::CornerRole::Small)
                    .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Panel)
                    .flex()
                    .items_center()
                    .justify_center()
                    .workbench_assistant()
                    .child(fileatlas_ui_bridge::i18n::tr(message)),
            );
        }
        preview.into_any_element()
    }

    pub(super) fn inspector_media(&self) -> Option<fileatlas_plugin_database::MediaItem> {
        let media_id = *self.selection.selected_media_ids.first()?;
        self.snapshot
            .as_ref()
            .ok()
            .and_then(|snapshot| snapshot.media.iter().find(|media| media.id == media_id))
            .cloned()
            .or_else(|| {
                self.inspector_fallback(media_id)
                    .and_then(|cached| cached.media.clone())
            })
    }

    fn rating_stars(&self, media_id: i64, rating: u8, cx: &mut Context<Self>) -> Div {
        let mut stars = div()
            .h_flex()
            .w_full()
            .justify_center()
            .items_center()
            .gap(px(fileatlas_ui_bridge::theme::space_compact()));
        for star in 1..=5u8 {
            stars = stars.child(
                div()
                    .id(format!(
                        "file_manager-inspector-preview-star-{media_id}-{star}"
                    ))
                    .cursor_pointer()
                    .workbench_foreground(if star <= rating {
                        fileatlas_ui_bridge::components::ForegroundRole::Accent
                    } else {
                        fileatlas_ui_bridge::components::ForegroundRole::Muted
                    })
                    .workbench_hover_foreground(theme::accent_text())
                    .child(
                        theme::icon(gpui_kit::assets::IconName::Star).workbench_foreground(
                            if star <= rating {
                                fileatlas_ui_bridge::components::ForegroundRole::Accent
                            } else {
                                fileatlas_ui_bridge::components::ForegroundRole::Muted
                            },
                        ),
                    )
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
