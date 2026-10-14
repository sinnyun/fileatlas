//! Source tag-assignment popup for the active preview item.
//!
//! Port of `vendor/photag-source/src/ui/preview.rs:2499-2604` (popup body),
//! `preview.rs:2184-2238` (`show_tag_assign_menu` rows), `preview.rs:2167-2171`
//! (subtree filter) and `preview.rs:2174-2181` (recent id lookup).
//! Row geometry follows the source: 14 indent per depth, a 20-wide caret slot,
//! the tag icon in the tag color, and a check icon beside selected labels.
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::Input;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use photag_plugin_database::db::queries;
use photag_plugin_history::UndoEntry;
use photag_ui_bridge::i18n::tr;
use photag_ui_bridge::theme;

use super::PreviewView;

/// `preview.rs:2202`: indent per tree depth.
const INDENT: f32 = 14.0;
/// `preview.rs:2218-2220`: the caret slot kept for leaf rows.
const CARET_SLOT: f32 = 20.0;
/// `preview.rs:2181`: `Color32::GRAY` for the empty-library hint.
const GRAY: u32 = 0x7f7f7f;
/// `preview.rs:2530`: filter field width.
const SEARCH_WIDTH: f32 = 180.0;

pub(super) fn popup(
    view: &PreviewView,
    min_width: Option<f32>,
    height: Pixels,
    cx: &mut Context<PreviewView>,
) -> AnyElement {
    let query = view
        .tag_picker_search
        .read(cx)
        .value()
        .trim()
        .to_lowercase();
    let media_id = view.service.snapshot().media_id;
    let assigned: std::collections::HashSet<i64> = media_id
        .and_then(|id| view.tagging.for_media(id).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|tag| tag.id)
        .collect();
    let tags = view.tagging.tag_tree().unwrap_or_default();
    let mut body = div()
        .v_flex()
        .gap(px(theme::SOURCE_ITEM_SPACING_Y))
        .when_some(min_width, |panel, width| panel.min_w(px(width)))
        .h(height)
        .overflow_hidden();
    body = body.child(
        div()
            .h_flex()
            .items_center()
            .gap(px(theme::SOURCE_ITEM_SPACING_X))
            .child(theme::small_icon(gpui_kit::assets::IconName::Search))
            .child(Input::new(&view.tag_picker_search).w(px(SEARCH_WIDTH))),
    );
    if tags.is_empty() {
        return body
            .child(
                div()
                    .text_size(px(theme::FONT_SIZE_BUTTON))
                    .text_color(rgb(GRAY))
                    .child(tr("No tags yet")),
            )
            .into_any_element();
    }
    if query.is_empty() {
        // `preview.rs:2536-2559`: the recent section only shows unfiltered.
        let mut recent = div().v_flex();
        let mut has_recent = false;
        for id in view.tagging.recent_tags().into_iter().take(6) {
            if let Some(tag) = find_tag(&tags, id) {
                has_recent = true;
                recent = recent.child(tag_row(tag, assigned.contains(&id), "recent", cx));
            }
        }
        if has_recent {
            body = body
                .child(
                    div()
                        .text_size(px(theme::FONT_SIZE_SMALL))
                        .text_color(rgb(theme::TEXT_MUTED))
                        .child(tr("Recent")),
                )
                .child(recent)
                .child(div().w_full().h(px(1.)).bg(rgba(theme::HAIRLINE.to_rgba())));
        }
    }
    let mut rows = Vec::new();
    assign_rows(
        &tags,
        0,
        &query,
        &view.collapsed_dropdown_tag_ids,
        &assigned,
        cx,
        &mut rows,
    );
    body.child(
        div()
            .v_flex()
            .flex_1()
            .min_h_0()
            .overflow_y_scrollbar()
            .children(rows),
    )
    .into_any_element()
}

/// `preview.rs:2224-2231`: the colored icon plus its checkable name.
/// Tag names are user data, so they bypass the catalog.
fn tag_row(
    tag: &queries::Tag,
    is_assigned: bool,
    slot: &str,
    cx: &mut Context<PreviewView>,
) -> AnyElement {
    let id = tag.id;
    let icon = theme::tag_icon(tag.icon.as_deref());
    div()
        .h_flex()
        .items_center()
        .gap(px(theme::SOURCE_ITEM_SPACING_X))
        .child(
            div()
                .text_color(rgb(theme::tag_tint(tag.color.as_deref())))
                .child(theme::small_icon(icon)),
        )
        .child(
            Button::new(format!("preview-{slot}-tag-{id}"))
                .ghost()
                .compact()
                .label(tag.name.clone())
                .when(is_assigned, |button| {
                    button.icon(theme::small_icon(gpui_kit::assets::IconName::Check))
                })
                .text_size(px(theme::FONT_SIZE_BUTTON))
                .toggled(is_assigned)
                .on_click(
                    cx.listener(move |this, _, _, cx| this.set_preview_tag(id, !is_assigned, cx)),
                ),
        )
        .into_any_element()
}

/// `preview.rs:2184-2238`: recursive tree rows with carets and indentation.
fn assign_rows(
    tags: &[queries::Tag],
    depth: usize,
    query: &str,
    collapsed: &std::collections::HashSet<i64>,
    assigned: &std::collections::HashSet<i64>,
    cx: &mut Context<PreviewView>,
    rows: &mut Vec<AnyElement>,
) {
    for tag in tags {
        if !query.is_empty() && !subtree_matches(tag, query) {
            continue;
        }
        let has_children = !tag.children.is_empty();
        // `preview.rs:2206`: an active filter force-expands every subtree.
        let expanded = !query.is_empty() || !collapsed.contains(&tag.id);
        let mut row = div()
            .h_flex()
            .items_center()
            .gap(px(theme::SOURCE_ITEM_SPACING_X))
            .pl(px(INDENT * depth as f32));
        if has_children && query.is_empty() {
            let id = tag.id;
            row = row.child(
                Button::new(format!("preview-tag-caret-{id}"))
                    .ghost()
                    .compact()
                    .icon(theme::small_icon(if expanded {
                        gpui_kit::assets::IconName::ChevronDown
                    } else {
                        gpui_kit::assets::IconName::ChevronRight
                    }))
                    .on_click(cx.listener(move |this, _, _, cx| this.toggle_tag_dropdown(id, cx))),
            );
        } else {
            row = row.child(div().w(px(CARET_SLOT)));
        }
        row = row.child(tag_row(tag, assigned.contains(&tag.id), "tree", cx));
        rows.push(row.into_any_element());
        if has_children && expanded {
            assign_rows(
                &tag.children,
                depth + 1,
                query,
                collapsed,
                assigned,
                cx,
                rows,
            );
        }
    }
}

/// `preview.rs:2167-2171`: a parent stays visible through any descendant.
fn subtree_matches(tag: &queries::Tag, query: &str) -> bool {
    tag.name.to_lowercase().contains(query)
        || tag
            .children
            .iter()
            .any(|child| subtree_matches(child, query))
}

/// `preview.rs:2174-2181`: the recent list resolves ids anywhere in the tree.
fn find_tag<'a>(tags: &'a [queries::Tag], id: i64) -> Option<&'a queries::Tag> {
    for tag in tags {
        if tag.id == id {
            return Some(tag);
        }
        if let Some(found) = find_tag(&tag.children, id) {
            return Some(found);
        }
    }
    None
}

impl PreviewView {
    fn toggle_tag_dropdown(&mut self, id: i64, cx: &mut Context<Self>) {
        if !self.collapsed_dropdown_tag_ids.insert(id) {
            self.collapsed_dropdown_tag_ids.remove(&id);
        }
        match self.session.snapshot() {
            Ok(mut session) => {
                session.collapsed_dropdown_tag_ids = self.collapsed_dropdown_tag_ids.clone();
                if let Err(error) = self.session.replace(session) {
                    self.operation_status = Some(format!("Could not save tag tree state: {error}"));
                }
            }
            Err(error) => {
                self.operation_status = Some(format!("Could not read session state: {error}"))
            }
        }
        cx.notify();
    }

    /// `preview.rs:2606-2618`: clear every tag from the current item, keeping
    /// the source undo snapshot of the removed associations.
    pub(super) fn clear_preview_tags(&mut self, cx: &mut Context<Self>) {
        let Some(media_id) = self.service.snapshot().media_id else {
            return;
        };
        let removed: Vec<i64> = self
            .tagging
            .for_media(media_id)
            .ok()
            .unwrap_or_default()
            .into_iter()
            .map(|tag| tag.id)
            .collect();
        if removed.is_empty() {
            return;
        }
        match self.tagging.clear(&[media_id]) {
            Ok(()) => {
                if let Some(history) = &self.history {
                    history.push(UndoEntry::RemovedTags {
                        media_ids: vec![media_id],
                        tag_ids: removed,
                    });
                }
                self.operation_status = Some("Removed every tag from this item.".into());
                self.emit_index_changed(cx);
            }
            Err(error) => self.operation_status = Some(format!("Tag update failed: {error}")),
        }
        cx.notify();
    }

    /// `preview.rs:2651-2667` with `main.rs:2087-2088`: the toggle writes the
    /// settings value, which the next frame reads back.
    pub(super) fn toggle_filmstrip(&mut self, cx: &mut Context<Self>) {
        let show = !self.filmstrip_visible();
        if let Err(error) = self
            .settings
            .update(|settings| settings.show_filmstrip = show)
        {
            self.operation_status = Some(format!("Could not save filmstrip visibility: {error}"));
        }
        cx.notify();
    }

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

    pub(crate) fn apply_rating_shortcut(&mut self, rating: u8, cx: &mut Context<Self>) {
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

    pub(super) fn set_preview_tag(&mut self, tag_id: i64, assign: bool, cx: &mut Context<Self>) {
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

    pub(super) fn emit_index_changed(&self, cx: &mut Context<Self>) {
        let cordis = self.cordis.clone();
        cx.spawn(async move |_, _| {
            let _ = cordis
                .emit::<photag_contracts::MediaIndexChanged>(cordis::Routing::Unscoped, ())
                .await;
        })
        .detach();
    }
}
