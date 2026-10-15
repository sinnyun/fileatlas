//! Searchable tag assignment controls for the current gallery selection.

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::input::Input;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use photag_plugin_history::UndoEntry;
use photag_ui_bridge::theme;

use super::GalleryView;

pub(super) fn render(
    view: &mut GalleryView,
    width: f32,
    height: f32,
    cx: &mut Context<GalleryView>,
) -> Div {
    let query = view
        .tag_picker_search
        .read(cx)
        .value()
        .trim()
        .to_lowercase();
    let tags = view
        .snapshot
        .as_ref()
        .ok()
        .map(|snapshot| snapshot.tag_tree.clone())
        .unwrap_or_default();
    let recent_ids = view
        .tagging
        .as_ref()
        .map(|service| service.recent_tags())
        .unwrap_or_default();
    let mut recent = Vec::new();
    collect_recent_labels(&tags, &recent_ids, &mut recent);
    let selected_count = view.selection.selected_media_ids.len();
    let card_height = height.max(360.);
    let body_height = (card_height - 104.).max(240.);
    let body_width = (width - 28.).max(1.);
    let picker_width = (body_width * (2. / 3.)).min(430.).max(1.);
    let preview_width = (body_width - picker_width).max(1.);
    let picker = div()
        .v_flex()
        .gap_2()
        .w(px(picker_width))
        .h(px(body_height))
        .min_w_0()
        .min_h_0()
        .p_3()
        .child(Input::new(&view.tag_picker_search).w_full());
    let mut picker = picker;
    if list_empty(&tags, &query) {
        picker = picker.child(
            div()
                .text_xs()
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child(photag_ui_bridge::i18n::tr("No matching tags.")),
        );
    } else {
        picker = picker.child(
            div()
                .v_flex()
                .w_full()
                .flex_1()
                .min_h_0()
                .rounded_sm()
                .bg(rgb(theme::SURFACE_PANEL))
                .p_1()
                .overflow_y_scrollbar()
                .children(tag_tree_rows(
                    &tags,
                    0,
                    &query,
                    &view.tag_picker_selected,
                    cx,
                )),
        );
    }
    if !recent.is_empty() {
        let mut recent_row = div().h_flex().flex_wrap().gap_1();
        for (id, label) in recent {
            let selected = view.tag_picker_selected.contains(&id);
            recent_row = recent_row.child(
                Button::new(format!("gallery-tag-recent-{id}"))
                    .when(selected, |button| {
                        button.icon(theme::small_icon(gpui_kit::assets::IconName::Check))
                    })
                    .label(photag_ui_bridge::i18n::tr(&label))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.tag_picker_selected.remove(&id) {
                            this.tag_picker_selected.insert(id);
                        }
                        cx.notify();
                    })),
            );
        }
        picker = picker.child(
            div()
                .v_flex()
                .gap_1()
                .max_h(px(64.))
                .flex_shrink_0()
                .child(
                    div()
                        .text_xs()
                        .font_semibold()
                        .child(photag_ui_bridge::i18n::tr("Recently used tags")),
                )
                .child(recent_row),
        );
    }
    let preview = selected_media_preview(view, preview_width);
    div()
        .v_flex()
        .w(px(width))
        .h(px(card_height))
        .min_h_0()
        .overflow_hidden()
        .rounded_md()
        .bg(rgb(theme::SURFACE_MODAL))
        .child(
            div()
                .h_flex()
                .w_full()
                .h(px(48.))
                .items_center()
                .justify_between()
                .border_b_1()
                .border_color(rgba(theme::BORDER.to_rgba()))
                .px_4()
                .child(
                    div()
                        .text_sm()
                        .font_semibold()
                        .child(photag_ui_bridge::i18n::tr_args(
                            "Assign tags to {count} selected media",
                            &[("count", selected_count.to_string())],
                        )),
                )
                .child(
                    Button::new("gallery-tag-picker-close")
                        .ghost()
                        .compact()
                        .icon(theme::small_icon(gpui_kit::assets::IconName::X))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.show_tag_picker = false;
                            cx.notify();
                        })),
                ),
        )
        .child(
            div()
                .h_flex()
                .w_full()
                .h(px(body_height))
                .min_h_0()
                .child(picker)
                .child(
                    div()
                        .v_flex()
                        .w(px(preview_width))
                        .h(px(body_height))
                        .min_w_0()
                        .min_h_0()
                        .border_l_1()
                        .border_color(rgba(theme::BORDER.to_rgba()))
                        .child(preview),
                ),
        )
        .child(
            div()
                .h_flex()
                .w_full()
                .h(px(56.))
                .flex_shrink_0()
                .items_center()
                .justify_end()
                .gap_2()
                .border_t_1()
                .border_color(rgba(theme::BORDER.to_rgba()))
                .px_4()
                .child(
                    Button::new("gallery-tag-picker-clear")
                        .ghost()
                        .compact()
                        .label(photag_ui_bridge::i18n::tr("Clear choices"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.tag_picker_selected.clear();
                            cx.notify();
                        })),
                )
                .child(div().flex_1())
                .child(
                    Button::new("gallery-tag-picker-cancel")
                        .ghost()
                        .compact()
                        .label(photag_ui_bridge::i18n::tr("Cancel"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.show_tag_picker = false;
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("gallery-tag-assign")
                        .compact()
                        .label(photag_ui_bridge::i18n::tr("Assign"))
                        .on_click(cx.listener(|this, _, _, cx| this.apply_picker_tags(true, cx))),
                )
                .child(
                    Button::new("gallery-tag-remove")
                        .ghost()
                        .compact()
                        .label(photag_ui_bridge::i18n::tr("Remove"))
                        .on_click(cx.listener(|this, _, _, cx| this.apply_picker_tags(false, cx))),
                )
                .child(
                    Button::new("gallery-tag-clear")
                        .ghost()
                        .compact()
                        .label(photag_ui_bridge::i18n::tr("Clear all tags"))
                        .on_click(cx.listener(|this, _, _, cx| this.clear_picker_tags(cx))),
                ),
        )
}

fn tag_tree_rows(
    tags: &[photag_plugin_database::db::queries::Tag],
    depth: usize,
    query: &str,
    selected: &std::collections::HashSet<i64>,
    cx: &mut Context<GalleryView>,
) -> Vec<Div> {
    let mut rows = Vec::new();
    for tag in tags {
        let matches = query.is_empty()
            || tag.name.to_lowercase().contains(query)
            || tag.full_path.to_lowercase().contains(query);
        if matches {
            let id = tag.id;
            let checked = selected.contains(&id);
            rows.push(
                div()
                    .h_flex()
                    .items_center()
                    .gap_1()
                    .pl(px((depth as f32) * 18.))
                    .child(
                        Button::new(format!("gallery-tag-choice-{id}"))
                            .icon(theme::small_icon(if checked {
                                gpui_kit::assets::IconName::Check
                            } else {
                                gpui_kit::assets::IconName::Square
                            }))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if !this.tag_picker_selected.remove(&id) {
                                    this.tag_picker_selected.insert(id);
                                }
                                cx.notify();
                            })),
                    )
                    .child(div().text_sm().child(tag.name.clone())),
            );
        }
        rows.extend(tag_tree_rows(&tag.children, depth + 1, query, selected, cx));
    }
    rows
}

fn collect_recent_labels(
    tags: &[photag_plugin_database::db::queries::Tag],
    recent_ids: &[i64],
    out: &mut Vec<(i64, String)>,
) {
    for id in recent_ids {
        if out.iter().any(|(existing, _)| existing == id) {
            continue;
        }
        if let Some(tag) = find_tag(tags, *id) {
            out.push((*id, tag.name.clone()));
        }
    }
}

fn find_tag(
    tags: &[photag_plugin_database::db::queries::Tag],
    id: i64,
) -> Option<&photag_plugin_database::db::queries::Tag> {
    tags.iter().find_map(|tag| {
        (tag.id == id)
            .then_some(tag)
            .or_else(|| find_tag(&tag.children, id))
    })
}

fn selected_media_preview(view: &GalleryView, width: f32) -> Div {
    let selected = &view.selection.selected_media_ids;
    let selected_id = selected.first().copied();
    let media = selected_id.and_then(|id| {
        view.snapshot.as_ref().ok().and_then(|snapshot| {
            snapshot
                .media
                .iter()
                .find(|media| media.id == id)
                .cloned()
                .or_else(|| {
                    view.inspector_fallback
                        .as_ref()
                        .filter(|fallback| fallback.media_id == id)
                        .and_then(|fallback| fallback.media.clone())
                })
        })
    });
    let mut preview = div()
        .v_flex()
        .items_start()
        .gap_1()
        .w(px(width))
        .h_full()
        .min_w_0()
        .overflow_hidden()
        .p_3()
        .child(
            div()
                .text_xs()
                .font_semibold()
                .child(photag_ui_bridge::i18n::tr("Selected media")),
        );
    if let Some(media) = media {
        let mut tags_for_media = view
            .snapshot
            .as_ref()
            .ok()
            .and_then(|snapshot| snapshot.media_tags.get(&media.id).cloned())
            .or_else(|| {
                view.inspector_fallback
                    .as_ref()
                    .filter(|fallback| fallback.media_id == media.id)
                    .map(|fallback| fallback.tags.clone())
            })
            .unwrap_or_default();
        if let Some(path) = &media.thumb_path {
            preview = preview.child(
                img(std::path::PathBuf::from(path))
                    .w_full()
                    .h(px(132.))
                    .object_fit(ObjectFit::Cover)
                    .rounded_md(),
            );
        } else {
            preview = preview.child(
                div()
                    .w_full()
                    .h(px(132.))
                    .rounded_md()
                    .bg(rgb(theme::SURFACE_FAINT))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(theme::TEXT_MUTED))
                    .child(theme::large_icon(if media.media_type == "video" {
                        gpui_kit::assets::IconName::FileVideoCamera
                    } else if media.media_type == "audio" {
                        gpui_kit::assets::IconName::FileMusic
                    } else {
                        gpui_kit::assets::IconName::FileImage
                    })),
            );
        }
        preview = preview.child(
            div()
                .w_full()
                .text_sm()
                .font_medium()
                .child(media.filename.clone()),
        );
        if let (Some(width), Some(height)) = (media.width, media.height) {
            preview = preview.child(
                div()
                    .text_xs()
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child(format!("{width} × {height}")),
            );
        }
        tags_for_media.sort_by(|left, right| left.name.cmp(&right.name));
        preview = preview.child(
            div()
                .w_full()
                .pt_1()
                .text_xs()
                .font_semibold()
                .child(photag_ui_bridge::i18n::tr("Tags")),
        );
        if tags_for_media.is_empty() {
            preview = preview.child(
                div()
                    .text_xs()
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child(photag_ui_bridge::i18n::tr("No tags assigned yet.")),
            );
        } else {
            let mut chips = div().h_flex().w_full().flex_wrap().gap_1();
            for tag in tags_for_media.into_iter().take(6) {
                chips = chips.child(
                    div()
                        .rounded_sm()
                        .bg(rgb(theme::SURFACE_PANEL))
                        .px_2()
                        .py(px(2.))
                        .text_xs()
                        .child(tag.name),
                );
            }
            preview = preview.child(chips);
        }
        if selected.len() > 1 {
            preview = preview.child(div().text_xs().child(photag_ui_bridge::i18n::tr_args(
                "and {count} more selected",
                &[("count", (selected.len() - 1).to_string())],
            )));
        }
    } else {
        preview = preview.child(
            div()
                .text_xs()
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child(photag_ui_bridge::i18n::tr("Select media to see a preview.")),
        );
    }
    preview
}

impl GalleryView {
    pub(super) fn assign_context_tag(
        &mut self,
        media_ids: &[i64],
        tag_id: i64,
        cx: &mut Context<Self>,
    ) {
        let Some(tagging) = &self.tagging else {
            return;
        };
        match tagging.assign(media_ids, &[tag_id]) {
            Ok(()) => {
                if let Some(history) = &self.history {
                    history.push(UndoEntry::AssignedTags {
                        media_ids: media_ids.to_vec(),
                        tag_ids: vec![tag_id],
                    });
                }
                self.operation_status = Some("Tag assigned.".into());
                self.reload();
                self.restore_selection(media_ids.to_vec());
            }
            Err(error) => self.operation_status = Some(format!("Tag update failed: {error}")),
        }
        cx.notify();
    }

    pub(super) fn remove_context_tag(
        &mut self,
        media_ids: &[i64],
        tag_id: i64,
        cx: &mut Context<Self>,
    ) {
        let Some(tagging) = &self.tagging else {
            return;
        };
        match tagging.remove(media_ids, &[tag_id]) {
            Ok(()) => {
                if let Some(history) = &self.history {
                    history.push(UndoEntry::RemovedTags {
                        media_ids: media_ids.to_vec(),
                        tag_ids: vec![tag_id],
                    });
                }
                self.operation_status =
                    Some(photag_ui_bridge::i18n::tr("Tag removed.").to_string());
                self.reload();
                self.restore_selection(media_ids.to_vec());
            }
            Err(error) => {
                self.operation_status = Some(
                    photag_ui_bridge::i18n::tr_args(
                        "Tag update failed: {error}",
                        &[("error", error.to_string())],
                    )
                    .to_string(),
                )
            }
        }
        cx.notify();
    }

    fn apply_picker_tags(&mut self, assign: bool, cx: &mut Context<Self>) {
        let ids: Vec<_> = self.tag_picker_selected.iter().copied().collect();
        let media_ids = self.selection.selected_media_ids.clone();
        if ids.is_empty() || media_ids.is_empty() {
            return;
        }
        let Some(tagging) = &self.tagging else {
            return;
        };
        let result = if assign {
            tagging.assign(&media_ids, &ids)
        } else {
            tagging.remove(&media_ids, &ids)
        };
        match result {
            Ok(()) => {
                if let Some(history) = &self.history {
                    history.push(if assign {
                        UndoEntry::AssignedTags {
                            media_ids: media_ids.clone(),
                            tag_ids: ids,
                        }
                    } else {
                        UndoEntry::RemovedTags {
                            media_ids: media_ids.clone(),
                            tag_ids: ids,
                        }
                    });
                }
                self.operation_status = Some(
                    if assign {
                        "Tags assigned."
                    } else {
                        "Tags removed."
                    }
                    .into(),
                );
                self.reload();
                self.restore_selection(media_ids);
            }
            Err(error) => self.operation_status = Some(format!("Tag update failed: {error}")),
        }
        cx.notify();
    }

    pub(super) fn clear_picker_tags(&mut self, cx: &mut Context<Self>) {
        let media_ids = self.selection.selected_media_ids.clone();
        if media_ids.is_empty() {
            return;
        }
        let Some(tagging) = &self.tagging else {
            return;
        };
        let history_entry = self
            .history
            .as_ref()
            .and_then(|history| history.cleared_tags_entry(media_ids.clone()).ok());
        match tagging.clear(&media_ids) {
            Ok(()) => {
                if let (Some(history), Some(entry)) = (&self.history, history_entry) {
                    history.push(entry);
                }
                self.operation_status = Some("All tags cleared from selected items.".into());
                self.reload();
                self.restore_selection(media_ids);
            }
            Err(error) => self.operation_status = Some(format!("Tag update failed: {error}")),
        }
        cx.notify();
    }

    pub(super) fn restore_selection(&mut self, media_ids: Vec<i64>) {
        if let Ok(snapshot) = &self.snapshot {
            self.selection.selected_media_ids = media_ids
                .into_iter()
                .filter(|id| snapshot.skeletons.iter().any(|media| media.id == *id))
                .collect();
            self.selection.anchor_id = self.selection.selected_media_ids.first().copied();
            self.persist_filter_session();
        }
    }
}

fn list_empty(tags: &[photag_plugin_database::db::queries::Tag], query: &str) -> bool {
    fn has(tags: &[photag_plugin_database::db::queries::Tag], query: &str) -> bool {
        tags.iter().any(|tag| {
            query.is_empty()
                || tag.name.to_lowercase().contains(query)
                || tag.full_path.to_lowercase().contains(query)
                || has(&tag.children, query)
        })
    }
    !has(tags, query)
}
