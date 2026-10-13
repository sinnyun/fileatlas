//! Searchable single-item tag picker for the active preview media.

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::Input;
use gpui_kit::*;
use photag_plugin_database::db::queries;
use photag_plugin_history::UndoEntry;
use photag_ui_bridge::theme;

use super::PreviewView;

pub(super) fn render(view: &PreviewView, cx: &mut Context<PreviewView>) -> Div {
    let Some(media_id) = view.service.snapshot().media_id else {
        return div().text_sm().child("Open an indexed item to edit tags.");
    };
    let query = view
        .tag_picker_search
        .read(cx)
        .value()
        .trim()
        .to_lowercase();
    let assigned = view.tagging.for_media(media_id).unwrap_or_default();
    let assigned_ids: std::collections::HashSet<_> = assigned.iter().map(|tag| tag.id).collect();
    let tags = view.tagging.tag_tree().unwrap_or_default();
    let mut rows = Vec::new();
    flatten(&tags, 0, &query, &mut rows);
    let mut panel = div()
        .v_flex()
        .gap_1()
        .rounded_md()
        .bg(rgb(theme::SURFACE_MODAL))
        .p_2()
        .child(div().text_sm().font_semibold().child("Tags for this item"))
        .child(Input::new(&view.tag_picker_search).w_full());
    let mut choices = div().h_flex().flex_wrap().gap_1();
    for (id, label) in rows.into_iter().take(160) {
        let is_assigned = assigned_ids.contains(&id);
        choices = choices.child(
            Button::new(format!("preview-tag-{id}"))
                .label(format!("{} {label}", if is_assigned { "✓" } else { "+" }))
                .on_click(cx.listener(move |this, _, _, cx| this.toggle_preview_tag(id, cx))),
        );
    }
    if choices_empty(&tags, &query) {
        panel = panel.child(
            div()
                .text_xs()
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child("No matching tags."),
        );
    } else {
        panel = panel.child(choices);
    }
    let mut current = div().h_flex().flex_wrap().gap_1();
    for tag in assigned {
        let id = tag.id;
        current = current.child(
            Button::new(format!("preview-tag-remove-{id}"))
                .label(format!("Remove {}", tag.full_path))
                .on_click(cx.listener(move |this, _, _, cx| this.set_preview_tag(id, false, cx))),
        );
    }
    panel
        .child(div().text_xs().child("Assigned tags"))
        .child(current)
}

impl PreviewView {
    pub(super) fn handle_tag_hotkey(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        let configured = self.settings.snapshot().ok().and_then(|settings| {
            settings
                .tag_hotkeys
                .into_iter()
                .find(|hotkey| hotkey.key.eq_ignore_ascii_case(key))
        });
        let Some(hotkey) = configured else {
            return false;
        };
        let Some(media_id) = self.service.snapshot().media_id else {
            return true;
        };
        let already_assigned = self
            .tagging
            .for_media(media_id)
            .ok()
            .is_some_and(|tags| tags.iter().any(|tag| tag.id == hotkey.tag_id));
        self.set_preview_tag(hotkey.tag_id, !already_assigned, cx);
        true
    }

    pub(super) fn apply_rating_shortcut(&mut self, rating: u8, cx: &mut Context<Self>) {
        let Some(media_id) = self.service.snapshot().media_id else {
            return;
        };
        let Some(database) = self.service.database() else {
            self.operation_status = Some("Database service is unavailable.".into());
            cx.notify();
            return;
        };
        let old_rating = database
            .with_reader(|connection| queries::get_media_by_id(connection, media_id))
            .ok()
            .flatten()
            .map(|media| media.rating);
        match database.with_writer(|connection| {
            queries::set_rating_bulk(connection, &[media_id], i32::from(rating.min(5)))
        }) {
            Ok(()) => {
                let new_rating = i32::from(rating.min(5));
                if let (Some(history), Some(old_rating)) = (&self.history, old_rating) {
                    if old_rating != new_rating {
                        history.push(UndoEntry::RatingChanged {
                            media_ids: vec![media_id],
                            old_rating,
                            new_rating,
                        });
                    }
                }
                self.operation_status = Some(format!("Rating set to {new_rating} / 5."));
                self.emit_index_changed(cx);
                cx.notify();
            }
            Err(error) => {
                self.operation_status = Some(format!("Rating update failed: {error}"));
                cx.notify();
            }
        }
    }

    fn toggle_preview_tag(&mut self, tag_id: i64, cx: &mut Context<Self>) {
        let media_id = self.service.snapshot().media_id;
        let assigned = media_id
            .and_then(|id| self.tagging.for_media(id).ok())
            .is_some_and(|tags| tags.iter().any(|tag| tag.id == tag_id));
        self.set_preview_tag(tag_id, !assigned, cx);
    }

    fn set_preview_tag(&mut self, tag_id: i64, assign: bool, cx: &mut Context<Self>) {
        let Some(media_id) = self.service.snapshot().media_id else {
            return;
        };
        let result = if assign {
            self.tagging.assign(&[media_id], &[tag_id])
        } else {
            self.tagging.remove(&[media_id], &[tag_id])
        };
        match result {
            Ok(()) => {
                if let Some(history) = &self.history {
                    history.push(if assign {
                        UndoEntry::AssignedTags {
                            media_ids: vec![media_id],
                            tag_ids: vec![tag_id],
                        }
                    } else {
                        UndoEntry::RemovedTags {
                            media_ids: vec![media_id],
                            tag_ids: vec![tag_id],
                        }
                    });
                }
                self.operation_status = Some(if assign {
                    "Tag assigned.".into()
                } else {
                    "Tag removed.".into()
                });
                self.emit_index_changed(cx);
            }
            Err(error) => self.operation_status = Some(format!("Tag update failed: {error}")),
        }
        cx.notify();
    }

    fn emit_index_changed(&self, cx: &mut Context<Self>) {
        let cordis = self.cordis.clone();
        cx.spawn(async move |_, _| {
            let _ = cordis
                .emit::<photag_contracts::MediaIndexChanged>(cordis::Routing::Unscoped, ())
                .await;
        })
        .detach();
    }
}

fn flatten(
    tags: &[photag_plugin_database::db::queries::Tag],
    depth: usize,
    query: &str,
    rows: &mut Vec<(i64, String)>,
) {
    for tag in tags {
        if query.is_empty()
            || tag.name.to_lowercase().contains(query)
            || tag.full_path.to_lowercase().contains(query)
        {
            rows.push((tag.id, format!("{}{}", "  ".repeat(depth), tag.full_path)));
        }
        flatten(&tag.children, depth + 1, query, rows);
    }
}

fn choices_empty(tags: &[photag_plugin_database::db::queries::Tag], query: &str) -> bool {
    !tags.iter().any(|tag| {
        query.is_empty()
            || tag.name.to_lowercase().contains(query)
            || tag.full_path.to_lowercase().contains(query)
            || !choices_empty(&tag.children, query)
    })
}
