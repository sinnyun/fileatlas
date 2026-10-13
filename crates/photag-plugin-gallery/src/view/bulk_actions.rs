//! Source-compatible filter-driven bulk file actions.
use super::{GalleryView, format_bulk_outcome};
use chrono::{NaiveDate, Utc};
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::Input;
use gpui_kit::*;
use photag_plugin_database::db::queries::MediaQuery;
use photag_ui_bridge::theme;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum BulkActionKind {
    Delete,
    CopyTo,
    MoveTo,
}
impl BulkActionKind {
    fn label(self) -> &'static str {
        match self {
            Self::Delete => "Delete (Recycle Bin)",
            Self::CopyTo => "Copy to…",
            Self::MoveTo => "Move to…",
        }
    }
    fn verb(self) -> &'static str {
        match self {
            Self::Delete => "Delete",
            Self::CopyTo => "Copy",
            Self::MoveTo => "Move",
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum BulkActionSource {
    WholeLibrary,
    SelectedFolder,
    SelectedTag,
    CurrentSelection,
}
impl BulkActionSource {
    fn label(self) -> &'static str {
        match self {
            Self::WholeLibrary => "Whole library",
            Self::SelectedFolder => "Selected folder",
            Self::SelectedTag => "Selected tag",
            Self::CurrentSelection => "Current selection",
        }
    }
    const ALL: [Self; 4] = [
        Self::WholeLibrary,
        Self::SelectedFolder,
        Self::SelectedTag,
        Self::CurrentSelection,
    ];
}
fn read(view: &GalleryView, index: usize, cx: &Context<GalleryView>) -> String {
    view.bulk_inputs[index].read(cx).value().to_string()
}

pub(super) fn reset_inputs(view: &GalleryView, window: &mut Window, cx: &mut Context<GalleryView>) {
    for input in &view.bulk_inputs {
        input.update(cx, |state, cx| state.set_value("", window, cx));
    }
}

fn matching_ids(view: &GalleryView, cx: &Context<GalleryView>) -> Result<Vec<i64>, String> {
    if view.bulk_source == BulkActionSource::CurrentSelection {
        return Ok(view.selection.selected_media_ids.clone());
    }
    let mut query = MediaQuery::default();
    match view.bulk_source {
        BulkActionSource::WholeLibrary => {}
        BulkActionSource::SelectedFolder => {
            query.folder_id = view.filter.folder_id;
            query.path_prefix = view.filter.path_prefix.clone();
        }
        BulkActionSource::SelectedTag => {
            let ids = view.filter.selected_tag_ids.clone();
            if !ids.is_empty() {
                query.tag_ids = Some(ids);
            }
        }
        BulkActionSource::CurrentSelection => unreachable!(),
    }
    let fields = (0..10).map(|i| read(view, i, cx)).collect::<Vec<_>>();
    let date = |index: usize| -> Result<Option<i64>, String> {
        if fields[index].trim().is_empty() {
            return Ok(None);
        }
        let day = NaiveDate::parse_from_str(fields[index].trim(), "%Y-%m-%d")
            .map_err(|_| format!("Invalid date: {}", fields[index]))?;
        Ok(day
            .and_hms_opt(
                if index == 0 { 0 } else { 23 },
                if index == 0 { 0 } else { 59 },
                if index == 0 { 0 } else { 59 },
            )
            .map(|dt| dt.and_utc().timestamp()))
    };
    query.date_from = date(0)?;
    query.date_to = date(1)?;
    let now = Utc::now().timestamp();
    if !fields[2].trim().is_empty() {
        let days: i64 = fields[2]
            .trim()
            .parse()
            .map_err(|_| "Older-than days must be a non-negative integer".to_string())?;
        if days < 0 {
            return Err("Older-than days must be non-negative".into());
        }
        let cutoff = now.saturating_sub(days.saturating_mul(86_400));
        query.date_to = Some(query.date_to.map_or(cutoff, |bound| bound.min(cutoff)));
    }
    if !fields[3].trim().is_empty() {
        let days: i64 = fields[3]
            .trim()
            .parse()
            .map_err(|_| "Newer-than days must be a non-negative integer".to_string())?;
        if days < 0 {
            return Err("Newer-than days must be non-negative".into());
        }
        let cutoff = now.saturating_sub(days.saturating_mul(86_400));
        query.date_from = Some(query.date_from.map_or(cutoff, |bound| bound.max(cutoff)));
    }
    let min_rating = if fields[4].trim().is_empty() {
        0
    } else {
        fields[4]
            .trim()
            .parse::<i32>()
            .map_err(|_| "Rating bounds must be from 0 to 5".to_string())?
    };
    let max_rating = if fields[5].trim().is_empty() {
        5
    } else {
        fields[5]
            .trim()
            .parse::<i32>()
            .map_err(|_| "Rating bounds must be from 0 to 5".to_string())?
    };
    if !(0..=5).contains(&min_rating) || !(0..=5).contains(&max_rating) || min_rating > max_rating {
        return Err("Rating range must be ordered from 0 to 5".into());
    }
    query.rating_min = (min_rating > 0).then_some(min_rating);
    query.rating_max = (max_rating < 5).then_some(max_rating);
    if !fields[6].trim().is_empty() {
        query.search = Some(fields[6].trim().to_string());
        query.search_in_filenames = true;
        query.search_in_paths = false;
        query.search_in_tags = false;
    }
    if !fields[7].trim().is_empty() {
        query.extensions = Some(vec![
            fields[7].trim().trim_start_matches('.').to_lowercase(),
        ]);
    }
    let size = |index: usize| -> Result<Option<i64>, String> {
        if fields[index].trim().is_empty() {
            return Ok(None);
        }
        let mb: f64 = fields[index]
            .trim()
            .parse()
            .map_err(|_| "File size bounds must be non-negative MB values".to_string())?;
        if !mb.is_finite() || mb < 0.0 || mb > (i64::MAX as f64 / 1_048_576.0) {
            return Err("File size bounds must be non-negative MB values".into());
        }
        Ok(Some((mb * 1_048_576.0) as i64))
    };
    query.size_min = size(8)?;
    query.size_max = size(9)?;
    if matches!((query.size_min, query.size_max), (Some(min), Some(max)) if min > max) {
        return Err("Minimum file size cannot exceed maximum file size".into());
    }
    let snapshot = view
        .service
        .load(query)
        .map_err(|error| error.to_string())?;
    Ok(snapshot.skeletons.into_iter().map(|item| item.id).collect())
}

pub(super) fn render(view: &GalleryView, cx: &mut Context<GalleryView>) -> AnyElement {
    let mut panel = div()
        .v_flex()
        .gap_2()
        .rounded_md()
        .bg(rgb(theme::SURFACE_PANEL))
        .p_3()
        .child(
            Button::new("gallery-bulk-toggle")
                .label(if view.bulk_action.is_some() {
                    "Close bulk actions"
                } else {
                    "Bulk actions… (Ctrl+Shift+D)"
                })
                .on_click(cx.listener(|this, _, window, cx| {
                    let closing = this.bulk_action.is_some();
                    if !closing {
                        reset_inputs(this, window, cx);
                        this.bulk_source = BulkActionSource::WholeLibrary;
                    }
                    this.bulk_action = (!closing).then_some(BulkActionKind::Delete);
                    this.confirm_bulk_action = false;
                    this.confirm_bulk_action_ids = None;
                    cx.notify();
                })),
        );
    let Some(action) = view.bulk_action else {
        return panel.into_any_element();
    };
    panel = panel.child(div().text_sm().font_semibold().child("Action"));
    panel = panel.child(
        div().h_flex().gap_2().children(
            [
                BulkActionKind::Delete,
                BulkActionKind::CopyTo,
                BulkActionKind::MoveTo,
            ]
            .into_iter()
            .map(|choice| {
                Button::new(format!("gallery-bulk-action-{}", choice.verb()))
                    .label(choice.label())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.bulk_action = Some(choice);
                        this.confirm_bulk_action = false;
                        this.confirm_bulk_action_ids = None;
                        cx.notify();
                    }))
            }),
        ),
    );
    panel = panel.child(div().text_sm().font_semibold().child("Source"));
    panel = panel.child(div().h_flex().flex_wrap().gap_2().children(
        BulkActionSource::ALL.into_iter().map(|choice| {
            Button::new(format!("gallery-bulk-source-{}", choice.label()))
                .label(choice.label())
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.bulk_source = choice;
                    this.confirm_bulk_action = false;
                    this.confirm_bulk_action_ids = None;
                    cx.notify();
                }))
        }),
    ));
    if view.bulk_source != BulkActionSource::CurrentSelection {
        panel = panel.child(div().text_sm().font_semibold().child("Filters (AND)"));
        panel = panel.child(
            div().h_flex().flex_wrap().gap_2().children(
                view.bulk_inputs
                    .iter()
                    .enumerate()
                    .map(|(i, input)| Input::new(input).w(px(if i > 5 { 180. } else { 150. }))),
            ),
        );
    } else {
        panel = panel.child(
            div()
                .text_xs()
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child("Current selection uses its selected items directly; filters are skipped."),
        );
    }
    let matching = matching_ids(view, cx);
    let (count, error) = match matching {
        Ok(ids) => (ids.len(), None),
        Err(error) => (0, Some(error)),
    };
    let has_filter_error = error.is_some();
    if let Some(error) = error {
        panel = panel.child(div().text_sm().text_color(rgb(theme::DANGER)).child(error));
    }
    if let Some(status) = &view.operation_status {
        panel = panel.child(
            div()
                .text_sm()
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child(status.clone()),
        );
    }
    panel = panel.child(
        div()
            .h_flex()
            .items_center()
            .justify_between()
            .child(div().text_sm().child(format!(
                "Matches: {count} item{}",
                if count == 1 { "" } else { "s" }
            )))
            .child(
                div()
                    .h_flex()
                    .gap_2()
                    .child(Button::new("gallery-bulk-cancel").label("Cancel").on_click(
                        cx.listener(|this, _, _, cx| {
                            this.bulk_action = None;
                            this.confirm_bulk_action = false;
                            this.confirm_bulk_action_ids = None;
                            cx.notify();
                        }),
                    ))
                    .child(
                        Button::new("gallery-bulk-run")
                            .label(
                                if action == BulkActionKind::Delete && view.confirm_bulk_action {
                                    format!("Confirm delete {count}")
                                } else {
                                    format!("{} {count}", action.verb())
                                },
                            )
                            .disabled(count == 0 || has_filter_error)
                            .on_click(cx.listener(|this, _, window, cx| execute(this, window, cx))),
                    ),
            ),
    );
    panel.into_any_element()
}

fn execute(view: &mut GalleryView, _window: &mut Window, cx: &mut Context<GalleryView>) {
    let Some(action) = view.bulk_action else {
        return;
    };
    let ids = match matching_ids(view, cx) {
        Ok(ids) => ids,
        Err(error) => {
            view.operation_status = Some(error);
            cx.notify();
            return;
        }
    };
    if ids.is_empty() {
        view.confirm_bulk_action = false;
        view.confirm_bulk_action_ids = None;
        view.operation_status = Some("No media match these bulk action filters.".into());
        cx.notify();
        return;
    }
    if action == BulkActionKind::Delete
        && (!view.confirm_bulk_action || view.confirm_bulk_action_ids.is_none())
    {
        view.confirm_bulk_action = true;
        view.confirm_bulk_action_ids = Some(ids.clone());
        view.operation_status = Some(format!(
            "Confirm deleting {} matching files to the Recycle Bin.",
            ids.len()
        ));
        cx.notify();
        return;
    }
    if action == BulkActionKind::Delete
        && view.confirm_bulk_action_ids.as_deref() != Some(ids.as_slice())
    {
        view.confirm_bulk_action = true;
        view.confirm_bulk_action_ids = Some(ids.clone());
        view.operation_status = Some(format!(
            "Bulk delete targets changed. Review the new set of {} items and confirm again.",
            ids.len()
        ));
        cx.notify();
        return;
    }
    let destination = if action == BulkActionKind::Delete {
        None
    } else {
        rfd::FileDialog::new().pick_folder()
    };
    if action != BulkActionKind::Delete && destination.is_none() {
        view.bulk_action = None;
        view.confirm_bulk_action = false;
        view.confirm_bulk_action_ids = None;
        view.operation_status = Some(format!(
            "{} cancelled; no destination was selected.",
            action.verb()
        ));
        cx.notify();
        return;
    }
    view.bulk_action = None;
    view.confirm_bulk_action = false;
    view.confirm_bulk_action_ids = None;
    let operations = Arc::clone(&view.file_operations);
    let total = ids.len();
    let refresh = action != BulkActionKind::CopyTo;
    view.start_operation(
        async move {
            match action {
                BulkActionKind::CopyTo => {
                    let Some(destination) = destination else {
                        return "Copy cancelled; no destination was selected.".into();
                    };
                    match operations
                        .bulk_copy_to_folder(ids.clone(), destination)
                        .await
                    {
                        Err(error) => format!("Copy failed: {error}"),
                        Ok(outcomes) => format_bulk_outcome(
                            "Copied",
                            total,
                            outcomes.iter().filter(|item| item.copied).count(),
                            outcomes.iter().filter_map(|item| {
                                item.error.as_ref().map(|error| (item.media_id, error))
                            }),
                        ),
                    }
                }
                BulkActionKind::MoveTo => {
                    let Some(destination) = destination else {
                        return "Move cancelled; no destination was selected.".into();
                    };
                    match operations
                        .bulk_move_into_folder(ids.clone(), destination)
                        .await
                    {
                        Err(error) => format!("Move failed: {error}"),
                        Ok(outcomes) => format_bulk_outcome(
                            "Moved",
                            total,
                            outcomes.iter().filter(|item| item.moved).count(),
                            outcomes.iter().filter_map(|item| {
                                item.error.as_ref().map(|error| (item.media_id, error))
                            }),
                        ),
                    }
                }
                BulkActionKind::Delete => {
                    match operations.bulk_delete_to_recycle_bin(ids.clone()).await {
                        Err(error) => format!("Delete failed: {error}"),
                        Ok(outcomes) => format_bulk_outcome(
                            "Removed",
                            total,
                            outcomes
                                .iter()
                                .filter(|item| item.removed_from_database)
                                .count(),
                            outcomes.iter().filter_map(|item| {
                                item.error.as_ref().map(|error| (item.media_id, error))
                            }),
                        ),
                    }
                }
            }
        },
        refresh,
        cx,
    );
}
