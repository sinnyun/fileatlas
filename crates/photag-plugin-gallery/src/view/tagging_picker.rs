//! Searchable tag assignment controls for the current gallery selection.

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::Input;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::*;
use photag_plugin_history::UndoEntry;
use photag_ui_bridge::theme;

use super::GalleryView;

pub(super) fn render(view: &mut GalleryView, cx: &mut Context<GalleryView>) -> Div {
    let query = view
        .tag_picker_search
        .read(cx)
        .value()
        .trim()
        .to_lowercase();
    let tags = view
        .tagging
        .as_ref()
        .and_then(|service| service.tag_tree().ok())
        .unwrap_or_default();
    let recent_ids = view
        .tagging
        .as_ref()
        .map(|service| service.recent_tags())
        .unwrap_or_default();
    let mut recent = Vec::new();
    collect_recent_labels(&tags, &recent_ids, &mut recent);
    let selected_count = view.selection.selected_media_ids.len();
    let mut picker = div()
        .v_flex()
        .gap_2()
        .w(px(430.))
        .flex_shrink_0()
        .rounded_md()
        .bg(rgba(theme::base_rgba(theme::SURFACE_BASE)))
        .p_3()
        .child(
            div()
                .text_sm()
                .font_semibold()
                .child(photag_ui_bridge::i18n::tr_args(
                    "Assign tags to {count} selected media",
                    &[("count", selected_count.to_string())],
                )),
        )
        .child(Input::new(&view.tag_picker_search).w_full());
    picker = picker.child(
        div()
            .h_flex()
            .gap_2()
            .child(
                Button::new("gallery-tag-assign")
                    .label(photag_ui_bridge::i18n::tr("Assign"))
                    .on_click(cx.listener(|this, _, _, cx| this.apply_picker_tags(true, cx))),
            )
            .child(
                Button::new("gallery-tag-remove")
                    .label(photag_ui_bridge::i18n::tr("Remove"))
                    .on_click(cx.listener(|this, _, _, cx| this.apply_picker_tags(false, cx))),
            )
            .child(
                Button::new("gallery-tag-clear")
                    .label(photag_ui_bridge::i18n::tr("Clear all tags"))
                    .on_click(cx.listener(|this, _, _, cx| this.clear_picker_tags(cx))),
            )
            .child(
                Button::new("gallery-tag-picker-clear")
                    .label(photag_ui_bridge::i18n::tr("Clear choices"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.tag_picker_selected.clear();
                        cx.notify();
                    })),
            ),
    );
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
                .gap_1()
                .max_h(px(250.))
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
                    .label(photag_ui_bridge::i18n::tr(
                        &(if selected {
                            format!("✓ {label}")
                        } else {
                            label
                        })
                        .to_string(),
                    ))
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
                .child(
                    div()
                        .text_xs()
                        .font_semibold()
                        .child(photag_ui_bridge::i18n::tr("Recently used tags")),
                )
                .child(recent_row),
        );
    }
    let preview = selected_media_preview(view);
    div()
        .h_flex()
        .items_start()
        .gap_3()
        .child(picker)
        .child(preview)
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
                            .label(if checked { "☑" } else { "□" })
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

fn selected_media_preview(view: &GalleryView) -> Div {
    let selected = &view.selection.selected_media_ids;
    let media = selected
        .first()
        .and_then(|id| view.service.media_by_id(*id).ok().flatten());
    let mut preview = div()
        .v_flex()
        .items_center()
        .gap_2()
        .w(px(220.))
        .rounded_md()
        .bg(rgb(theme::SURFACE_PANEL))
        .p_3()
        .child(
            div()
                .text_xs()
                .font_semibold()
                .child(photag_ui_bridge::i18n::tr("Selected media")),
        );
    if let Some(media) = media {
        if let Some(path) = &media.thumb_path {
            preview = preview.child(
                img(std::path::PathBuf::from(path))
                    .w(px(190.))
                    .h(px(150.))
                    .object_fit(ObjectFit::Cover)
                    .rounded_md(),
            );
        }
        preview = preview.child(div().text_sm().child(media.filename.clone()));
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
