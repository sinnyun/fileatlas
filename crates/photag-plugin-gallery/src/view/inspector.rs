//! Right-hand media inspector with fixed preview, independent information,
//! history and extension pages, plus the existing file actions. Reference:
//! docs/reference-ui/concepts/sidebar-pages-preview-2026-10-14-v2.png.

use gpui_kit::assets::IconName;
use gpui_kit::base::Disableable;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};

mod helpers;
mod pages;
mod preview;
use super::card;
use super::*;
use gpui_kit::component::button::ButtonVariants;
use helpers::*;

impl GalleryView {
    pub(super) fn render_inspector(&mut self, cx: &mut Context<Self>) -> AnyElement {
        self.ensure_inspector_fallback(cx);
        let header = div()
            .h_flex()
            .flex_shrink_0()
            .w_full()
            .items_center()
            .justify_between()
            .h(px(44.))
            .px(px(14.))
            .child(
                div()
                    .font_family(theme::SEMIBOLD_FONT_FAMILY)
                    .text_size(px(14.))
                    .text_color(rgb(theme::TEXT_PRIMARY))
                    .child(photag_ui_bridge::i18n::tr("Preview")),
            )
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap(px(4.))
                    .child(
                        Button::new("gallery-inspector-open-preview")
                            .ghost()
                            .compact()
                            .icon(theme::icon(IconName::ExternalLink))
                            .tooltip(photag_ui_bridge::i18n::tr("Open preview"))
                            .disabled(self.selection.selected_media_ids.is_empty())
                            .on_click(cx.listener(|this, _, _, cx| this.preview_selected(cx))),
                    )
                    .child(
                        Button::new("gallery-inspector-close")
                            .ghost()
                            .compact()
                            .icon(theme::icon(IconName::X))
                            .tooltip(photag_ui_bridge::i18n::tr("Close inspector"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Err(error) = this.surfaces.set_open("inspector", false) {
                                    tracing::error!("could not close inspector surface: {error}");
                                }
                                cx.notify();
                            })),
                    ),
            );
        let tab = self.active_inspector_tab();
        let preview = self.inspector_preview(cx);
        let tabs = self.inspector_tabs(tab, cx);
        let body = match tab {
            "history" => self.inspector_history(),
            "extensions" => self.inspector_extensions(),
            _ => match self.selection.selected_media_ids.len() {
                0 => inspector_empty(),
                1 => self.inspector_single(cx),
                count => Self::inspector_multi(count, cx),
            },
        };
        let body = if tab == "information" && self.selection.selected_media_ids.len() == 1 {
            body
        } else {
            div()
                .size_full()
                .min_h_0()
                .overflow_y_scrollbar()
                .child(body)
                .into_any_element()
        };
        div()
            .v_flex()
            .size_full()
            .min_h_0()
            .child(header)
            .child(preview)
            .child(tabs)
            .child(div().v_flex().flex_1().min_h_0().child(body))
            .into_any_element()
    }

    fn ensure_inspector_fallback(&mut self, _: &mut Context<Self>) {
        let Some(media_id) = self.selection.selected_media_ids.first().copied() else {
            return;
        };
        if self
            .snapshot
            .as_ref()
            .is_ok_and(|s| s.media.iter().any(|m| m.id == media_id))
        {
            return;
        }
        let service = self.service.clone();
        let tagging = self.tagging.clone();
        self.inspector_read.request(media_id, move || {
            let media = service.media_by_id(media_id).map_err(|e| e.to_string())?;
            let tags = match tagging {
                Some(tagging) => tagging.for_media(media_id).map_err(|e| e.to_string())?,
                None => Vec::new(),
            };
            Ok(InspectorFallback {
                media_id,
                media,
                tags,
            })
        });
    }

    pub(super) fn inspector_fallback(&self, media_id: i64) -> Option<&InspectorFallback> {
        self.inspector_fallback
            .as_ref()
            .filter(|cached| cached.media_id == media_id)
    }

    fn inspector_single(&self, cx: &mut Context<Self>) -> AnyElement {
        let media_id = self.selection.selected_media_ids[0];
        let Ok(snapshot) = &self.snapshot else {
            return inspector_message("File info is unavailable.");
        };
        let fallback = self
            .inspector_fallback(media_id)
            .and_then(|cached| cached.media.as_ref());
        let media = if let Some(media) = snapshot.media.iter().find(|media| media.id == media_id) {
            media
        } else if let Some(media) = fallback {
            media
        } else if self.inspector_read.key() == Some(&media_id) && self.inspector_read.is_pending() {
            return inspector_message("Loading file info…");
        } else {
            return inspector_message("Selected media is no longer indexed.");
        };
        let tags = if let Some(tags) = snapshot.media_tags.get(&media_id) {
            tags.as_slice()
        } else {
            self.inspector_fallback(media_id)
                .map(|cached| cached.tags.as_slice())
                .unwrap_or_default()
        };
        let mut body = div().v_flex().w_full().gap(px(2.)).px(px(14.)).pb(px(14.));
        let basic_open = self.inspector_expanded.contains("basic");
        body = body.child(inspector_divider()).child(section_header(
            "basic",
            "Basic info",
            basic_open,
            cx,
        ));
        if basic_open {
            body = body.child(basic_info_rows(media));
        }
        let tags_open = self.inspector_expanded.contains("tags");
        body = body
            .child(inspector_divider())
            .child(section_header("tags", "Tags", tags_open, cx));
        if tags_open {
            body = body.child(self.tag_capsules(media_id, tags, cx));
        }
        if media.media_type == "image" {
            let exif_open = self.inspector_expanded.contains("exif");
            body = body
                .child(inspector_divider())
                .child(section_header("exif", "EXIF", exif_open, cx));
            if exif_open {
                body = body.child(self.exif_rows(media_id));
            }
        }
        let path_open = self.inspector_expanded.contains("path");
        body = body.child(inspector_divider()).child(section_header(
            "path",
            "File path",
            path_open,
            cx,
        ));
        if path_open {
            body = body.child(
                div()
                    .w_full()
                    .pb(px(6.))
                    .text_size(px(theme::FONT_SIZE_TINY))
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child(media.full_path.clone()),
            );
        }
        div()
            .v_flex()
            .size_full()
            .min_h_0()
            .child(div().flex_1().min_h_0().overflow_y_scrollbar().child(body))
            .child(self.inspector_bottom(media_id, cx))
            .into_any_element()
    }

    fn tag_capsules(
        &self,
        media_id: i64,
        tags: &[photag_plugin_database::db::queries::Tag],
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut capsules = div()
            .h_flex()
            .flex_wrap()
            .gap(px(6.))
            .pb(px(6.))
            .items_center();
        for tag in tags {
            let tag_id = tag.id;
            capsules = capsules.child(
                div()
                    .h_flex()
                    .items_center()
                    .gap(px(5.))
                    .h(px(24.))
                    .px(px(9.))
                    .rounded(px(12.))
                    .border_1()
                    .border_color(rgba(theme::SEPARATOR_LIGHT))
                    .child(
                        div()
                            .size(px(8.))
                            .rounded_full()
                            .bg(rgb(parse_tag_color(tag.color.as_deref()))),
                    )
                    .child(
                        div()
                            .text_size(px(theme::FONT_SIZE_SMALL))
                            .text_color(rgb(theme::TEXT_PRIMARY))
                            .child(tag.name.clone()),
                    )
                    .child(
                        div()
                            .id(format!("gallery-inspector-tag-remove-{tag_id}"))
                            .cursor_pointer()
                            .child(
                                theme::icon(IconName::X)
                                    .size(px(11.))
                                    .text_color(rgb(theme::TEXT_MUTED)),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let media_ids = vec![media_id];
                                this.remove_context_tag(&media_ids, tag_id, cx);
                            })),
                    ),
            );
        }
        capsules = capsules.child(
            div()
                .id("gallery-inspector-tag-add")
                .size(px(24.))
                .rounded(px(theme::ROUND_SM))
                .border_1()
                .border_color(rgba(theme::SEPARATOR_LIGHT))
                .cursor_pointer()
                .hover(|style| style.bg(rgba(theme::FILL_HOVER)))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    theme::icon(IconName::Plus)
                        .size(px(13.))
                        .text_color(rgb(theme::TEXT_SECONDARY)),
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.show_tag_picker = true;
                    cx.notify();
                })),
        );
        capsules
    }

    fn exif_rows(&self, media_id: i64) -> AnyElement {
        let fields = self
            .exif_cache
            .as_ref()
            .filter(|(id, _)| *id == media_id)
            .map(|(_, fields)| fields);
        match fields {
            Some(Ok(fields)) if fields.is_empty() => inspector_message("No EXIF metadata."),
            Some(Ok(fields)) => div()
                .v_flex()
                .w_full()
                .pb(px(6.))
                .children(
                    fields
                        .iter()
                        .map(|(label, value)| info_row(label, value.clone())),
                )
                .into_any_element(),
            Some(Err(message)) => inspector_message(message),
            None => inspector_message("Loading file info…"),
        }
    }

    fn inspector_bottom(&self, media_id: i64, cx: &mut Context<Self>) -> impl IntoElement {
        let menu_view = cx.entity().downgrade();
        let more = Button::new("gallery-inspector-more")
            .ghost()
            .compact()
            .icon(theme::icon(IconName::Ellipsis))
            .tooltip(photag_ui_bridge::i18n::tr("More"))
            .dropdown_menu(move |mut menu, _, _| {
                let open_view = menu_view.clone();
                menu = menu.item(
                    PopupMenuItem::new(photag_ui_bridge::i18n::tr("Open in default app")).on_click(
                        move |_, _, cx| {
                            let _ = open_view
                                .update(cx, |this, cx| this.open_media_action(media_id, false, cx));
                        },
                    ),
                );
                let reveal_view = menu_view.clone();
                menu = menu.item(
                    PopupMenuItem::new(photag_ui_bridge::i18n::tr("Show in Explorer")).on_click(
                        move |_, _, cx| {
                            let _ = reveal_view
                                .update(cx, |this, cx| this.open_media_action(media_id, true, cx));
                        },
                    ),
                );
                menu = menu.item(PopupMenuItem::separator());
                let copy_view = menu_view.clone();
                menu = menu.item(
                    PopupMenuItem::new(photag_ui_bridge::i18n::tr("Copy to…")).on_click(
                        move |_, _, cx| {
                            let _ = copy_view.update(cx, |this, cx| {
                                this.selection.selected_media_ids = vec![media_id];
                                this.copy_selected_with_dialog(cx);
                            });
                        },
                    ),
                );
                let move_view = menu_view.clone();
                menu = menu.item(
                    PopupMenuItem::new(photag_ui_bridge::i18n::tr("Move to…")).on_click(
                        move |_, _, cx| {
                            let _ = move_view.update(cx, |this, cx| {
                                this.selection.selected_media_ids = vec![media_id];
                                this.move_selected_with_dialog(cx);
                            });
                        },
                    ),
                );
                menu = menu.item(PopupMenuItem::separator());
                let delete_view = menu_view.clone();
                menu.item(
                    PopupMenuItem::new(photag_ui_bridge::i18n::tr("Delete…")).on_click(
                        move |_, window, cx| {
                            let _ = delete_view.update(cx, |this, cx| {
                                this.selection.selected_media_ids = vec![media_id];
                                this.selection.anchor_id = Some(media_id);
                                this.request_delete_confirmation(window, cx);
                            });
                        },
                    ),
                )
            });
        div()
            .h_flex()
            .w_full()
            .flex_shrink_0()
            .items_center()
            .gap(px(8.))
            .p(px(12.))
            .border_t_1()
            .border_color(rgb(theme::SEPARATOR))
            .child(
                div().flex_1().child(
                    Button::new("gallery-inspector-reveal")
                        .outline()
                        .w_full()
                        .label(photag_ui_bridge::i18n::tr("Show in Explorer"))
                        .icon(theme::icon(IconName::FolderOpen))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.open_media_action(media_id, true, cx)
                        })),
                ),
            )
            .child(more)
    }

    fn inspector_multi(count: usize, cx: &mut Context<Self>) -> AnyElement {
        div()
            .v_flex()
            .w_full()
            .gap(px(8.))
            .px(px(14.))
            .py(px(16.))
            .child(
                div()
                    .text_size(px(theme::FONT_SIZE_BODY))
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child(photag_ui_bridge::i18n::tr_args(
                        "{count} selected",
                        &[("count", count.to_string())],
                    )),
            )
            .child(
                Button::new("gallery-inspector-assign-tags")
                    .primary()
                    .label(photag_ui_bridge::i18n::tr("Assign tags"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_tag_picker = true;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("gallery-inspector-bulk-actions")
                    .outline()
                    .label(photag_ui_bridge::i18n::tr("Bulk actions…"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        bulk_actions::reset_inputs(this, window, cx);
                        this.bulk_source = BulkActionSource::CurrentSelection;
                        this.bulk_action = Some(BulkActionKind::Delete);
                        this.confirm_bulk_action = false;
                        this.confirm_bulk_action_ids = None;
                        cx.notify();
                    })),
            )
            .into_any_element()
    }

    pub(super) fn toggle_inspector_section(
        &mut self,
        section: &'static str,
        cx: &mut Context<Self>,
    ) {
        if !self.inspector_expanded.insert(section) {
            self.inspector_expanded.remove(&section);
        }
        cx.notify();
    }
}
