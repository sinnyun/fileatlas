//! Searchable tag assignment controls for the current gallery selection.

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::Input;
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
    let mut rows = Vec::new();
    flatten(&tags, 0, &query, &mut rows);
    let selected_count = view.selection.selected_media_ids.len();
    let mut panel = div()
        .v_flex()
        .gap_1()
        .rounded_md()
        .bg(rgb(theme::SURFACE_BASE))
        .p_2()
        .child(
            div()
                .text_sm()
                .font_semibold()
                .child(format!("Assign tags to {selected_count} selected")),
        )
        .child(Input::new(&view.tag_picker_search).w_full());
    panel = panel.child(
        div()
            .h_flex()
            .gap_1()
            .child(
                Button::new("gallery-tag-assign")
                    .label("Assign")
                    .on_click(cx.listener(|this, _, _, cx| this.apply_picker_tags(true, cx))),
            )
            .child(
                Button::new("gallery-tag-remove")
                    .label("Remove")
                    .on_click(cx.listener(|this, _, _, cx| this.apply_picker_tags(false, cx))),
            )
            .child(
                Button::new("gallery-tag-clear")
                    .label("Clear all tags")
                    .on_click(cx.listener(|this, _, _, cx| this.clear_picker_tags(cx))),
            )
            .child(
                Button::new("gallery-tag-picker-clear")
                    .label("Clear choices")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.tag_picker_selected.clear();
                        cx.notify();
                    })),
            ),
    );
    let mut list = div().h_flex().flex_wrap().gap_1();
    for (id, label) in rows.into_iter().take(120) {
        let selected = view.tag_picker_selected.contains(&id);
        list = list.child(
            Button::new(format!("gallery-tag-choice-{id}"))
                .label(if selected {
                    format!("✓ {label}")
                } else {
                    label
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    if !this.tag_picker_selected.remove(&id) {
                        this.tag_picker_selected.insert(id);
                    }
                    cx.notify();
                })),
        );
    }
    if list_empty(&tags, &query) {
        panel = panel.child(
            div()
                .text_xs()
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child("No matching tags."),
        );
    } else {
        panel = panel.child(list);
    }
    panel
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

    fn clear_picker_tags(&mut self, cx: &mut Context<Self>) {
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

    fn restore_selection(&mut self, media_ids: Vec<i64>) {
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

fn flatten(
    tags: &[photag_plugin_database::db::queries::Tag],
    depth: usize,
    query: &str,
    out: &mut Vec<(i64, String)>,
) {
    for tag in tags {
        if query.is_empty()
            || tag.name.to_lowercase().contains(query)
            || tag.full_path.to_lowercase().contains(query)
        {
            out.push((tag.id, format!("{}{}", "  ".repeat(depth), tag.full_path)));
        }
        flatten(&tag.children, depth + 1, query, out);
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
