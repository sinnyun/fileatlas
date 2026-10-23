//! Searchable tag assignment controls for the current file_manager selection.

use fileatlas_plugin_history::UndoEntry;
use fileatlas_ui_bridge::components::WorkbenchForeground as _;
use fileatlas_ui_bridge::components::WorkbenchPaint as _;
use fileatlas_ui_bridge::components::WorkbenchStyle as _;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
use fileatlas_ui_bridge::theme;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::*;

use super::FileManagerView;

pub(super) fn render(
    view: &mut FileManagerView,
    width: f32,
    height: f32,
    cx: &mut Context<FileManagerView>,
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
        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
        .w(px(picker_width))
        .h(px(body_height))
        .min_w_0()
        .min_h_0()
        .p(px(fileatlas_ui_bridge::theme::space_expanded()))
        .child(fileatlas_ui_bridge::components::text_input(&view.tag_picker_search).w_full());
    let mut picker = picker;
    if list_empty(&tags, &query) {
        picker = picker.child(
            div()
                .workbench_assistant()
                .child(fileatlas_ui_bridge::i18n::tr("No matching tags.")),
        );
    } else {
        picker = picker.child(
            div()
                .v_flex()
                .w_full()
                .flex_1()
                .min_h_0()
                .workbench_corner(fileatlas_ui_bridge::components::CornerRole::Small)
                .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Panel)
                .p(px(fileatlas_ui_bridge::theme::space_compact()))
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
        let mut recent_row = div()
            .h_flex()
            .flex_wrap()
            .gap(px(fileatlas_ui_bridge::theme::space_compact()));
        for tag in recent {
            let id = tag.id;
            let selected = view.tag_picker_selected.contains(&id);
            recent_row = recent_row.child(
                div()
                    .id(format!("file_manager-tag-recent-{id}"))
                    .cursor_pointer()
                    .child(
                        fileatlas_ui_bridge::tag_capsule::TagCapsule::new(tag.name.clone())
                            .color(tag.color.as_deref())
                            .icon(tag.icon.as_deref())
                            .selected(selected)
                            .render(),
                    )
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
                .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                .max_h(px(64.))
                .flex_shrink_0()
                .child(
                    div()
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr("Recently used tags")),
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
        .child(
            div()
                .h_flex()
                .w_full()
                .h(px(theme::ui_height(theme::CONTROL_HEIGHT_FOOTER)))
                .items_center()
                .justify_between()
                .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Bottom)
                .px(px(fileatlas_ui_bridge::theme::space_expanded()))
                .child(
                    div()
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr_args(
                            "Assign tags to {count} selected media",
                            &[("count", selected_count.to_string())],
                        )),
                )
                .child(
                    fileatlas_ui_bridge::components::button("file_manager-tag-picker-close")
                        .ghost()
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
                        .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Left)
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
                .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Top)
                .px(px(fileatlas_ui_bridge::theme::space_expanded()))
                .child(
                    fileatlas_ui_bridge::components::button("file_manager-tag-picker-clear")
                        .ghost()
                        .label(fileatlas_ui_bridge::i18n::tr("Clear choices"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.tag_picker_selected.clear();
                            cx.notify();
                        })),
                )
                .child(div().flex_1())
                .child(
                    fileatlas_ui_bridge::components::button("file_manager-tag-picker-cancel")
                        .ghost()
                        .label(fileatlas_ui_bridge::i18n::tr("Cancel"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.show_tag_picker = false;
                            cx.notify();
                        })),
                )
                .child(
                    fileatlas_ui_bridge::components::button("file_manager-tag-assign")
                        .label(fileatlas_ui_bridge::i18n::tr("Assign"))
                        .on_click(cx.listener(|this, _, _, cx| this.apply_picker_tags(true, cx))),
                )
                .child(
                    fileatlas_ui_bridge::components::button("file_manager-tag-remove")
                        .ghost()
                        .label(fileatlas_ui_bridge::i18n::tr("Remove"))
                        .on_click(cx.listener(|this, _, _, cx| this.apply_picker_tags(false, cx))),
                )
                .child(
                    fileatlas_ui_bridge::components::button("file_manager-tag-clear")
                        .ghost()
                        .label(fileatlas_ui_bridge::i18n::tr("Clear all tags"))
                        .on_click(cx.listener(|this, _, _, cx| this.clear_picker_tags(cx))),
                ),
        )
}

fn tag_tree_rows(
    tags: &[fileatlas_plugin_database::Tag],
    depth: usize,
    query: &str,
    selected: &std::collections::HashSet<i64>,
    cx: &mut Context<FileManagerView>,
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
                    .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                    .pl(px((depth as f32) * 18.))
                    .child(
                        fileatlas_ui_bridge::components::button(format!(
                            "file_manager-tag-choice-{id}"
                        ))
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
                    .child(
                        fileatlas_ui_bridge::tag_capsule::TagCapsule::new(tag.name.clone())
                            .color(tag.color.as_deref())
                            .icon(tag.icon.as_deref())
                            .selected(checked)
                            .render()
                            .flex_1()
                            .min_w_0(),
                    ),
            );
        }
        rows.extend(tag_tree_rows(&tag.children, depth + 1, query, selected, cx));
    }
    rows
}

fn collect_recent_labels(
    tags: &[fileatlas_plugin_database::Tag],
    recent_ids: &[i64],
    out: &mut Vec<fileatlas_plugin_database::Tag>,
) {
    for id in recent_ids {
        if out.iter().any(|existing| existing.id == *id) {
            continue;
        }
        if let Some(tag) = find_tag(tags, *id) {
            out.push(tag.clone());
        }
    }
}

fn find_tag(
    tags: &[fileatlas_plugin_database::Tag],
    id: i64,
) -> Option<&fileatlas_plugin_database::Tag> {
    tags.iter().find_map(|tag| {
        (tag.id == id)
            .then_some(tag)
            .or_else(|| find_tag(&tag.children, id))
    })
}

fn selected_media_preview(view: &FileManagerView, width: f32) -> Div {
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
        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
        .w(px(width))
        .h_full()
        .min_w_0()
        .overflow_hidden()
        .p(px(fileatlas_ui_bridge::theme::space_expanded()))
        .child(
            div()
                .workbench_assistant()
                .child(fileatlas_ui_bridge::i18n::tr("Selected media")),
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
                    .workbench_corner(fileatlas_ui_bridge::components::CornerRole::Medium),
            );
        } else {
            preview = preview.child(
                div()
                    .w_full()
                    .h(px(132.))
                    .workbench_corner(fileatlas_ui_bridge::components::CornerRole::Medium)
                    .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Faint)
                    .flex()
                    .items_center()
                    .justify_center()
                    .workbench_foreground(fileatlas_ui_bridge::components::ForegroundRole::Muted)
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
                .workbench_assistant()
                .child(media.filename.clone()),
        );
        if let (Some(width), Some(height)) = (media.width, media.height) {
            preview = preview.child(
                div()
                    .workbench_assistant()
                    .child(format!("{width} × {height}")),
            );
        }
        tags_for_media.sort_by(|left, right| left.name.cmp(&right.name));
        preview = preview.child(
            div()
                .w_full()
                .pt(px(fileatlas_ui_bridge::theme::space_compact()))
                .workbench_assistant()
                .child(fileatlas_ui_bridge::i18n::tr("Tags")),
        );
        if tags_for_media.is_empty() {
            preview = preview.child(
                div()
                    .workbench_assistant()
                    .child(fileatlas_ui_bridge::i18n::tr("No tags assigned yet.")),
            );
        } else {
            let mut chips = div()
                .h_flex()
                .w_full()
                .flex_wrap()
                .gap(px(fileatlas_ui_bridge::theme::space_compact()));
            for tag in tags_for_media.into_iter().take(6) {
                chips = chips.child(
                    fileatlas_ui_bridge::tag_capsule::TagCapsule::new(tag.name)
                        .color(tag.color.as_deref())
                        .icon(tag.icon.as_deref())
                        .render(),
                );
            }
            preview = preview.child(chips);
        }
        if selected.len() > 1 {
            preview = preview.child(div().workbench_assistant().child(
                fileatlas_ui_bridge::i18n::tr_args(
                    "and {count} more selected",
                    &[("count", (selected.len() - 1).to_string())],
                ),
            ));
        }
    } else {
        preview = preview.child(
            div()
                .workbench_assistant()
                .child(fileatlas_ui_bridge::i18n::tr(
                    "Select media to see a preview.",
                )),
        );
    }
    preview
}

impl FileManagerView {
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
                self.refresh_tag_assignments(media_ids);
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
                    Some(fileatlas_ui_bridge::i18n::tr("Tag removed.").to_string());
                self.refresh_tag_assignments(media_ids);
            }
            Err(error) => {
                self.operation_status = Some(
                    fileatlas_ui_bridge::i18n::tr_args(
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
                self.refresh_tag_assignments(&media_ids);
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
                self.refresh_tag_assignments(&media_ids);
            }
            Err(error) => self.operation_status = Some(format!("Tag update failed: {error}")),
        }
        cx.notify();
    }

    fn refresh_tag_assignments(&mut self, media_ids: &[i64]) {
        // Tag mutations do not change file rows unless the active query itself
        // filters by tags. Patch only the affected tag cache in the common case
        // so the virtual list, scroll position, selection, and loaded window are
        // left untouched.
        if !self.filter.selected_tag_ids.is_empty() || !self.filter.excluded_tag_ids.is_empty() {
            self.refresh_index();
            return;
        }

        let Some(tagging) = &self.tagging else {
            return;
        };
        let updates = media_ids
            .iter()
            .copied()
            .map(|media_id| tagging.for_media(media_id).map(|tags| (media_id, tags)))
            .collect::<Result<Vec<_>, _>>();
        let Ok(updates) = updates else {
            tracing::warn!(
                "could not refresh changed file_manager tag assignments; scheduling a window refresh"
            );
            self.refresh_index();
            return;
        };

        if let Ok(snapshot) = &mut self.snapshot {
            for (media_id, tags) in &updates {
                snapshot.media_tags.insert(*media_id, tags.clone());
            }
        }
        if let Some(fallback) = &mut self.inspector_fallback {
            if let Some((_, tags)) = updates
                .iter()
                .find(|(media_id, _)| *media_id == fallback.media_id)
            {
                fallback.tags.clone_from(tags);
            }
        }
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

fn list_empty(tags: &[fileatlas_plugin_database::Tag], query: &str) -> bool {
    fn has(tags: &[fileatlas_plugin_database::Tag], query: &str) -> bool {
        tags.iter().any(|tag| {
            query.is_empty()
                || tag.name.to_lowercase().contains(query)
                || tag.full_path.to_lowercase().contains(query)
                || has(&tag.children, query)
        })
    }
    !has(tags, query)
}
