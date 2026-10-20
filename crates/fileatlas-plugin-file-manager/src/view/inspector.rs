//! Right-hand media inspector with fixed preview, independent information,
//! history and extension pages, plus the existing file actions. Reference:
//! docs/reference-ui/concepts/sidebar-pages-preview-2026-10-14-v2.png.

use gpui_kit::assets::IconName;
use gpui_kit::base::Disableable;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::component::popover::Popover;

mod contexts;
mod folder;
mod helpers;
pub(super) mod history;
mod history_tree;
mod pages;
mod preview;
mod program;
mod tag;
pub(super) mod tag_editor;
use super::card;
use super::*;
use gpui_kit::component::button::ButtonVariants;
use helpers::*;

impl FileManagerView {
    pub(super) fn current_inspector_media(&self) -> Option<fileatlas_plugin_database::MediaItem> {
        self.inspector_media()
    }

    pub(super) fn inspector_context_key(&self) -> &'static str {
        self.inspector_context().key()
    }
    pub(super) fn render_inspector(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.ensure_inspector_fallback(cx);
        let context = self.inspector_context();
        let tab = self.active_inspector_tab();
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
                    .text_color(rgb(theme::text_primary()))
                    .child(fileatlas_ui_bridge::i18n::tr(context.title())),
            )
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap(px(4.))
                    .children(
                        matches!(&context, contexts::InspectorContext::File).then(|| {
                            Button::new("file_manager-inspector-open-preview")
                                .ghost()
                                .compact()
                                .icon(theme::icon(IconName::ExternalLink))
                                .tooltip(fileatlas_ui_bridge::i18n::tr("Open preview"))
                                .on_click(cx.listener(|this, _, _, cx| this.preview_selected(cx)))
                        }),
                    )
                    .child(
                        Button::new("file_manager-inspector-close")
                            .ghost()
                            .compact()
                            .icon(theme::icon(IconName::X))
                            .tooltip(fileatlas_ui_bridge::i18n::tr("Close inspector"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Err(error) = this.surfaces.set_open("inspector", false) {
                                    tracing::error!("could not close inspector surface: {error}");
                                }
                                cx.notify();
                            })),
                    ),
            );
        let (preview, body) = if matches!(&context, contexts::InspectorContext::File) {
            let body = match tab {
                "history" => self.inspector_history(cx),
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
            let preview = self.inspector_preview(cx);
            (preview, body)
        } else {
            (
                context.hero(),
                div()
                    .size_full()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(context.body(self, window, cx))
                    .into_any_element(),
            )
        };
        let mut panel = div()
            .v_flex()
            .size_full()
            .min_h_0()
            .child(header)
            .child(preview);
        if matches!(&context, contexts::InspectorContext::File) {
            panel = panel.child(self.inspector_tabs(tab, cx));
        }
        panel
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
            body = body.child(self.tag_capsules(media_id, tags, &snapshot.tag_tree, cx));
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
                    .text_color(rgb(theme::text_secondary()))
                    .child(display_path(&media.full_path)),
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
        tags: &[fileatlas_plugin_database::Tag],
        tag_tree: &[fileatlas_plugin_database::Tag],
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
                    .gap(px(3.))
                    .child(
                        fileatlas_ui_bridge::tag_capsule::TagCapsule::new(tag.name.clone())
                            .color(tag.color.as_deref())
                            .icon(tag.icon.as_deref())
                            .render(),
                    )
                    .child(
                        div()
                            .id(format!("file_manager-inspector-tag-remove-{tag_id}"))
                            .cursor_pointer()
                            .child(
                                theme::icon(IconName::X)
                                    .size(px(11.))
                                    .text_color(rgb(theme::text_muted())),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let media_ids = vec![media_id];
                                this.remove_context_tag(&media_ids, tag_id, cx);
                            })),
                    ),
            );
        }
        let query = self
            .tag_picker_search
            .read(cx)
            .value()
            .trim()
            .to_lowercase();
        let mut selected_tags = Vec::new();
        let mut all_tags = Vec::new();
        let selected_ids = tags
            .iter()
            .map(|tag| tag.id)
            .collect::<std::collections::HashSet<_>>();
        flatten_inspector_tag_choices(tag_tree, 0, &mut all_tags);
        for (tag, depth) in &all_tags {
            if selected_ids.contains(&tag.id)
                && (query.is_empty() || tag.name.to_lowercase().contains(&query))
            {
                selected_tags.push((tag.clone(), *depth));
            }
        }
        let available = all_tags
            .into_iter()
            .filter(|(tag, _)| {
                !selected_ids.contains(&tag.id)
                    && (query.is_empty() || tag.name.to_lowercase().contains(&query))
            })
            .collect::<Vec<_>>();
        let search_input = self.tag_picker_search.clone();
        let picker_open = self.inspector_tag_picker_open;
        let weak = cx.entity().downgrade();
        let open_view = weak.clone();
        let picker = Popover::new("file_manager-inspector-tag-picker")
            .open(picker_open)
            .on_open_change(move |open, window, cx| {
                let _ = open_view.update(cx, |this, cx| {
                    this.inspector_tag_picker_open = *open;
                    if *open {
                        this.tag_picker_search.update(cx, |input, cx| {
                            input.set_value("", window, cx);
                        });
                    }
                    cx.notify();
                });
            })
            .trigger(
                Button::new("file_manager-inspector-tag-add")
                    .ghost()
                    .compact()
                    .icon(theme::icon(IconName::Plus))
                    .label(fileatlas_ui_bridge::i18n::tr("Add tags")),
            )
            .content(move |_, _, cx| {
                let mut input = div()
                    .h_flex()
                    .w_full()
                    .h(px(38. + 27. * selected_tags.len().saturating_sub(1) as f32))
                    .flex_wrap()
                    .items_center()
                    .gap(px(5.))
                    .px(px(8.))
                    .py(px(5.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(rgb(theme::accent()))
                    .bg(rgb(theme::surface_elevated()));
                for (tag, _) in &selected_tags {
                    let tag_id = tag.id;
                    let chip_view = weak.clone();
                    input = input.child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap(px(4.))
                            .child(
                                fileatlas_ui_bridge::tag_capsule::TagCapsule::new(tag.name.clone())
                                    .color(tag.color.as_deref())
                                    .icon(tag.icon.as_deref())
                                    .render(),
                            )
                            .child(
                                div()
                                    .id(format!("file_manager-inspector-tag-remove-{tag_id}"))
                                    .cursor_pointer()
                                    .child(theme::small_icon(IconName::X))
                                    .on_click(cx.listener(move |_, _, _, cx| {
                                        let _ = chip_view.update(cx, |this, cx| {
                                            this.remove_context_tag(&[media_id], tag_id, cx);
                                        });
                                    })),
                            ),
                    );
                }
                input = input.child(
                    fileatlas_ui_bridge::components::text_input(&search_input)
                        .flex_1()
                        .min_w(px(70.)),
                );
                let selected_rows = inspector_tag_choice_rows(
                    media_id,
                    &selected_tags,
                    &selected_ids,
                    weak.clone(),
                    cx,
                );
                let available_rows = inspector_tag_choice_rows(
                    media_id,
                    &available,
                    &selected_ids,
                    weak.clone(),
                    cx,
                );
                let mut panel = div()
                    .v_flex()
                    .w(px(320.))
                    .max_h(px(620.))
                    .gap(px(8.))
                    .child(input);
                if !selected_rows.is_empty() {
                    panel = panel.child(inspector_picker_section("已选中", selected_rows, None));
                }
                panel = panel.child(inspector_picker_section(
                    "所有值",
                    available_rows,
                    Some(px(380.)),
                ));
                panel
            });
        div()
            .v_flex()
            .w_full()
            .gap(px(8.))
            .child(capsules.child(picker))
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
                    display_exif_fields(fields)
                        .into_iter()
                        .map(|(label, value)| info_row(&label, value)),
                )
                .into_any_element(),
            Some(Err(message)) => inspector_message(message),
            None => inspector_message("Loading file info…"),
        }
    }

    fn inspector_bottom(&self, media_id: i64, cx: &mut Context<Self>) -> impl IntoElement {
        let menu_view = cx.entity().downgrade();
        let more = Button::new("file_manager-inspector-more")
            .ghost()
            .compact()
            .icon(theme::icon(IconName::Ellipsis))
            .tooltip(fileatlas_ui_bridge::i18n::tr("More"))
            .dropdown_menu(move |mut menu, _, _| {
                let open_view = menu_view.clone();
                menu = menu.item(
                    PopupMenuItem::new(fileatlas_ui_bridge::i18n::tr("Open in default app"))
                        .on_click(move |_, _, cx| {
                            let _ = open_view
                                .update(cx, |this, cx| this.open_media_action(media_id, false, cx));
                        }),
                );
                let reveal_view = menu_view.clone();
                menu = menu.item(
                    PopupMenuItem::new(fileatlas_ui_bridge::i18n::tr("Show in Explorer")).on_click(
                        move |_, _, cx| {
                            let _ = reveal_view
                                .update(cx, |this, cx| this.open_media_action(media_id, true, cx));
                        },
                    ),
                );
                menu = menu.item(PopupMenuItem::separator());
                let copy_view = menu_view.clone();
                menu = menu.item(
                    PopupMenuItem::new(fileatlas_ui_bridge::i18n::tr("Copy to…")).on_click(
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
                    PopupMenuItem::new(fileatlas_ui_bridge::i18n::tr("Move to…")).on_click(
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
                    PopupMenuItem::new(fileatlas_ui_bridge::i18n::tr("Delete…")).on_click(
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
            .border_color(rgb(theme::separator()))
            .child(
                div().flex_1().child(
                    Button::new("file_manager-inspector-reveal")
                        .outline()
                        .w_full()
                        .label(fileatlas_ui_bridge::i18n::tr("Show in Explorer"))
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
                    .text_color(rgb(theme::text_secondary()))
                    .child(fileatlas_ui_bridge::i18n::tr_args(
                        "{count} selected",
                        &[("count", count.to_string())],
                    )),
            )
            .child(
                Button::new("file_manager-inspector-assign-tags")
                    .primary()
                    .label(fileatlas_ui_bridge::i18n::tr("Assign tags"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_tag_picker = true;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("file_manager-inspector-bulk-actions")
                    .outline()
                    .label(fileatlas_ui_bridge::i18n::tr("Bulk actions…"))
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

fn flatten_inspector_tag_choices(
    tags: &[fileatlas_plugin_database::Tag],
    depth: usize,
    choices: &mut Vec<(fileatlas_plugin_database::Tag, usize)>,
) {
    for tag in tags {
        choices.push((tag.clone(), depth));
        flatten_inspector_tag_choices(&tag.children, depth + 1, choices);
    }
}

fn inspector_picker_section(title: &str, rows: Vec<Div>, max_height: Option<Pixels>) -> AnyElement {
    let heading = div()
        .w_full()
        .px(px(7.))
        .py(px(3.))
        .text_size(px(theme::FONT_SIZE_SMALL))
        .text_color(rgb(theme::text_secondary()))
        .child(fileatlas_ui_bridge::i18n::tr(title));
    let section = div().v_flex().w_full().child(heading);
    if rows.is_empty() {
        if let Some(max_height) = max_height {
            section
                .max_h(max_height)
                .overflow_y_scrollbar()
                .child(inspector_message("No matching tags."))
                .into_any_element()
        } else {
            section
                .child(inspector_message("No matching tags."))
                .into_any_element()
        }
    } else {
        if let Some(max_height) = max_height {
            section
                .max_h(max_height)
                .overflow_y_scrollbar()
                .children(rows)
                .into_any_element()
        } else {
            section.children(rows).into_any_element()
        }
    }
}

fn inspector_tag_choice_rows(
    media_id: i64,
    tags: &[(fileatlas_plugin_database::Tag, usize)],
    selected: &std::collections::HashSet<i64>,
    weak: WeakEntity<FileManagerView>,
    cx: &mut Context<gpui_kit::component::popover::PopoverState>,
) -> Vec<Div> {
    tags.iter()
        .map(|(tag, depth)| {
            let id = tag.id;
            let checked = selected.contains(&id);
            let row_view = weak.clone();
            div()
                .h_flex()
                .w_full()
                .min_h(px(32.))
                .items_center()
                .gap(px(6.))
                .pl(px((*depth as f32) * 14. + 7.))
                .child(
                    Button::new(format!("file_manager-inspector-tag-choice-{id}"))
                        .compact()
                        .icon(theme::small_icon(if checked {
                            IconName::Check
                        } else {
                            IconName::Square
                        }))
                        .on_click(cx.listener(move |_, _, _, cx| {
                            let _ = row_view.update(cx, |this, cx| {
                                if checked {
                                    this.remove_context_tag(&[media_id], id, cx);
                                } else {
                                    this.assign_context_tag(&[media_id], id, cx);
                                }
                            });
                        })),
                )
                .child(
                    fileatlas_ui_bridge::tag_capsule::TagCapsule::new(tag.name.clone())
                        .color(tag.color.as_deref())
                        .icon(tag.icon.as_deref())
                        .render()
                        .flex_1()
                        .min_w_0(),
                )
        })
        .collect()
}
