//! Source-compatible filter-driven bulk file actions.
use super::{GalleryView, format_bulk_outcome};
use chrono::Utc;
mod filters;
mod read_model;
use crate::filter::MediaTypeFilter;
use filters::{filter_controls, match_results_list, section_title};
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::calendar::Date;
use gpui_kit::component::date_picker::DatePicker;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::*;
use photag_plugin_database::db::queries::{MediaItem, MediaQuery};
use photag_ui_bridge::theme;
use read_model::MatchRequest;
pub(super) use read_model::{MatchResults, poll_matches};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulkActionKind {
    Delete,
    CopyTo,
    MoveTo,
}
impl BulkActionKind {
    pub(crate) fn label(self) -> &'static str {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulkActionSource {
    WholeLibrary,
    SelectedFolder,
    SelectedTag,
    CurrentSelection,
}
impl BulkActionSource {
    pub(crate) fn label(self) -> &'static str {
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

pub(super) fn reset_inputs(
    view: &mut GalleryView,
    window: &mut Window,
    cx: &mut Context<GalleryView>,
) {
    for input in &view.bulk_inputs {
        input.update(cx, |state, cx| state.set_value("", window, cx));
    }
    view.bulk_media_type = MediaTypeFilter::All;
    view.bulk_tag_ids.clear();
    view.bulk_extension_exclude = false;
    view.bulk_date_from_value = None;
    view.bulk_date_to_value = None;
    view.bulk_date_from.update(cx, |picker, cx| {
        picker.set_date(Date::Single(None), window, cx)
    });
    view.bulk_date_to.update(cx, |picker, cx| {
        picker.set_date(Date::Single(None), window, cx)
    });
}

fn matching_results(
    view: &GalleryView,
    cx: &Context<GalleryView>,
) -> Result<(Vec<i64>, Vec<MediaItem>), String> {
    matching_request(view, cx)?.load(&view.service)
}

fn matching_request(view: &GalleryView, cx: &Context<GalleryView>) -> Result<MatchRequest, String> {
    if view.bulk_source == BulkActionSource::CurrentSelection {
        let ids = view.selection.selected_media_ids.clone();
        return Ok(MatchRequest::Selection(ids));
    }
    let mut query = MediaQuery::default();
    match view.bulk_source {
        BulkActionSource::WholeLibrary => {}
        BulkActionSource::SelectedFolder => {
            query.folder_id = view.filter.folder_id;
            query.path_prefix = view.filter.path_prefix.clone();
            if let Ok(snapshot) = &view.snapshot {
                if snapshot.browser.active {
                    if snapshot.browser.directory.is_none() {
                        return Err("Select a folder first.".into());
                    }
                    query.direct_directory = snapshot.browser.directory.clone();
                    query.path_prefix = None;
                }
            }
        }
        BulkActionSource::SelectedTag => {
            let ids = view.filter.selected_tag_ids.clone();
            if !ids.is_empty() {
                query.tag_ids = Some(ids);
            }
        }
        BulkActionSource::CurrentSelection => unreachable!(),
    }
    let fields = (0..8).map(|i| read(view, i, cx)).collect::<Vec<_>>();
    query.media_type = match view.bulk_media_type {
        MediaTypeFilter::All => None,
        MediaTypeFilter::Photos => Some("image".into()),
        MediaTypeFilter::Videos => Some("video".into()),
        MediaTypeFilter::Audio => Some("audio".into()),
    };
    query.date_from = view
        .bulk_date_from_value
        .and_then(|day| day.and_hms_opt(0, 0, 0).map(|dt| dt.and_utc().timestamp()));
    query.date_to = view.bulk_date_to_value.and_then(|day| {
        day.and_hms_opt(23, 59, 59)
            .map(|dt| dt.and_utc().timestamp())
    });
    let now = Utc::now().timestamp();
    if !fields[0].trim().is_empty() {
        let days: i64 = fields[0]
            .trim()
            .parse()
            .map_err(|_| "Older-than days must be a non-negative integer".to_string())?;
        if days < 0 {
            return Err("Older-than days must be non-negative".into());
        }
        let cutoff = now.saturating_sub(days.saturating_mul(86_400));
        query.date_to = Some(query.date_to.map_or(cutoff, |bound| bound.min(cutoff)));
    }
    if !fields[1].trim().is_empty() {
        let days: i64 = fields[1]
            .trim()
            .parse()
            .map_err(|_| "Newer-than days must be a non-negative integer".to_string())?;
        if days < 0 {
            return Err("Newer-than days must be non-negative".into());
        }
        let cutoff = now.saturating_sub(days.saturating_mul(86_400));
        query.date_from = Some(query.date_from.map_or(cutoff, |bound| bound.max(cutoff)));
    }
    let min_rating = if fields[2].trim().is_empty() {
        0
    } else {
        fields[2]
            .trim()
            .parse::<i32>()
            .map_err(|_| "Rating bounds must be from 0 to 5".to_string())?
    };
    let max_rating = if fields[3].trim().is_empty() {
        5
    } else {
        fields[3]
            .trim()
            .parse::<i32>()
            .map_err(|_| "Rating bounds must be from 0 to 5".to_string())?
    };
    if !(0..=5).contains(&min_rating) || !(0..=5).contains(&max_rating) || min_rating > max_rating {
        return Err("Rating range must be ordered from 0 to 5".into());
    }
    query.rating_min = (min_rating > 0).then_some(min_rating);
    query.rating_max = (max_rating < 5).then_some(max_rating);
    if !fields[4].trim().is_empty() {
        query.search = Some(fields[4].trim().to_string());
        query.search_in_filenames = true;
        query.search_in_paths = false;
        query.search_in_tags = false;
    }
    let extensions = fields[5]
        .split([',', ';', '|', ' '])
        .map(|extension| extension.trim().trim_start_matches('.').to_lowercase())
        .filter(|extension| !extension.is_empty())
        .collect::<Vec<_>>();
    if !extensions.is_empty() {
        if view.bulk_extension_exclude {
            query.excluded_extensions = Some(extensions);
        } else {
            query.extensions = Some(extensions);
        }
    }
    if !view.bulk_tag_ids.is_empty() && view.bulk_source != BulkActionSource::SelectedTag {
        query.tag_ids = Some(view.bulk_tag_ids.clone());
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
    query.size_min = size(6)?;
    query.size_max = size(7)?;
    if matches!((query.size_min, query.size_max), (Some(min), Some(max)) if min > max) {
        return Err("Minimum file size cannot exceed maximum file size".into());
    }
    Ok(MatchRequest::Query(
        query,
        if view.bulk_source == BulkActionSource::SelectedTag {
            view.bulk_tag_ids.clone()
        } else {
            Vec::new()
        },
    ))
}

fn matching_ids(view: &GalleryView, cx: &Context<GalleryView>) -> Result<Vec<i64>, String> {
    matching_results(view, cx).map(|(ids, _)| ids)
}

pub(super) fn render(
    view: &GalleryView,
    width: f32,
    height: f32,
    cx: &mut Context<GalleryView>,
) -> AnyElement {
    let mut panel = div()
        .v_flex()
        .w_full()
        .gap_3()
        .h(px(height))
        .min_h_0()
        .overflow_hidden()
        .rounded_md()
        .bg(rgb(theme::SURFACE_MODAL))
        .p_3();
    let Some(action) = view.bulk_action else {
        return panel.into_any_element();
    };
    let loading = view.bulk_read.value().is_none();
    let matching = view
        .bulk_read
        .value()
        .cloned()
        .unwrap_or_else(|| Ok((Vec::new(), Vec::new())));
    let (ids, matched_media, error) = match matching {
        Ok((ids, media)) => (ids, media, None),
        Err(error) => (Vec::new(), Vec::new(), Some(error)),
    };
    let count = ids.len();
    let has_filter_error = error.is_some();
    let available_width = (width - 280.).max(320.);
    let column_width = ((available_width - 28.) / 2.).max(120.);
    let action_source = div()
        .h_flex()
        .w_full()
        .items_start()
        .gap_3()
        .child(
            div()
                .v_flex()
                .w(px(column_width))
                .flex_shrink_0()
                .gap_1()
                .child(section_title("1. Choose action"))
                .children(
                    [
                        BulkActionKind::Delete,
                        BulkActionKind::CopyTo,
                        BulkActionKind::MoveTo,
                    ]
                    .into_iter()
                    .map(|choice| {
                        let selected = view.bulk_action == Some(choice);
                        Button::new(format!("gallery-bulk-action-{}", choice.verb()))
                            .ghost()
                            .compact()
                            .w_full()
                            .bg(if view.bulk_action == Some(choice) {
                                rgba((theme::accent() << 8) | 48)
                            } else {
                                rgba(0)
                            })
                            .text_color(rgb(if view.bulk_action == Some(choice) {
                                theme::accent()
                            } else {
                                theme::TEXT_PRIMARY
                            }))
                            .child(choice_row(choice.label(), selected))
                            .toggled(selected)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.bulk_action = Some(choice);
                                this.confirm_bulk_action = false;
                                this.confirm_bulk_action_ids = None;
                                cx.notify();
                            }))
                    }),
                ),
        )
        .child(
            div()
                .v_flex()
                .w(px(column_width))
                .flex_shrink_0()
                .gap_1()
                .child(section_title("2. Choose source"))
                .children(BulkActionSource::ALL.into_iter().map(|choice| {
                    let selected = view.bulk_source == choice;
                    Button::new(format!("gallery-bulk-source-{}", choice.label()))
                        .ghost()
                        .compact()
                        .w_full()
                        .bg(if view.bulk_source == choice {
                            rgba((theme::accent() << 8) | 48)
                        } else {
                            rgba(0)
                        })
                        .text_color(rgb(if view.bulk_source == choice {
                            theme::accent()
                        } else {
                            theme::TEXT_PRIMARY
                        }))
                        .child(choice_row(choice.label(), selected))
                        .toggled(selected)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.bulk_source = choice;
                            this.confirm_bulk_action = false;
                            this.confirm_bulk_action_ids = None;
                            cx.notify();
                        }))
                })),
        );
    let mut controls = div()
        .v_flex()
        .gap_2()
        .overflow_y_scrollbar()
        .w(px(available_width))
        .h_full()
        .min_w_0()
        .min_h_0()
        .flex_shrink_0()
        .child(
            div()
                .h_flex()
                .gap_3()
                .p_2()
                .rounded_md()
                .bg(rgb(theme::SURFACE_PANEL))
                .child(action_source),
        );
    if let Some(error) = error {
        controls = controls.child(div().text_sm().text_color(rgb(theme::DANGER)).child(error));
    }
    if loading {
        controls = controls.child(photag_ui_bridge::i18n::tr("Loading file info…"));
    }
    if let Some(status) = &view.operation_status {
        controls = controls.child(
            div()
                .text_sm()
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child(status.clone()),
        );
    }
    if view.bulk_source == BulkActionSource::CurrentSelection {
        controls = controls.child(
            div()
                .text_xs()
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child(photag_ui_bridge::i18n::tr(
                    "Current selection uses its selected items directly; filters are skipped.",
                )),
        );
    } else {
        controls = controls.child(filter_controls(view, cx));
    }
    let result_list = match_results_list(&matched_media, count, (height - 110.).max(120.));
    let layout = div()
        .h_flex()
        .flex_1()
        .min_h_0()
        .items_start()
        .gap_4()
        .w_full()
        .child(controls)
        .child(result_list);
    panel = panel.child(layout);
    panel = panel.child(
        div()
            .h_flex()
            .flex_shrink_0()
            .items_center()
            .justify_between()
            .mt(px(3.))
            .pt(px(10.))
            .border_t_1()
            .border_color(rgba(theme::SEPARATOR_LIGHT))
            .child(div().text_sm().child(photag_ui_bridge::i18n::tr_args(
                "Matches: {count} items",
                &[("count", count.to_string())],
            )))
            .child(
                div()
                    .h_flex()
                    .gap_2()
                    .child(
                        Button::new("gallery-bulk-cancel")
                            .ghost()
                            .compact()
                            .label(photag_ui_bridge::i18n::tr("Cancel"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.bulk_action = None;
                                this.confirm_bulk_action = false;
                                this.confirm_bulk_action_ids = None;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("gallery-bulk-run")
                            .primary()
                            .compact()
                            .label(photag_ui_bridge::i18n::tr(
                                &(if action == BulkActionKind::Delete && view.confirm_bulk_action {
                                    photag_ui_bridge::i18n::tr_args(
                                        "Confirm delete {count}",
                                        &[("count", count.to_string())],
                                    )
                                } else {
                                    photag_ui_bridge::i18n::tr_args(
                                        "Continue · {action} {count}",
                                        &[
                                            ("action", photag_ui_bridge::i18n::tr(action.verb())),
                                            ("count", count.to_string()),
                                        ],
                                    )
                                })
                                .to_string(),
                            ))
                            .disabled(count == 0 || has_filter_error)
                            .on_click(cx.listener(|this, _, window, cx| execute(this, window, cx))),
                    ),
            ),
    );
    panel.into_any_element()
}

fn choice_row(label: &str, selected: bool) -> Div {
    let accent = theme::accent();
    div()
        .h_flex()
        .w_full()
        .items_center()
        .gap_2()
        .child(
            div()
                .size(px(14.))
                .rounded_full()
                .border_1()
                .border_color(if selected {
                    rgb(accent)
                } else {
                    rgba(theme::BORDER.to_rgba())
                })
                .flex()
                .items_center()
                .justify_center()
                .child(if selected {
                    div().size(px(6.)).rounded_full().bg(rgb(accent))
                } else {
                    div().size(px(6.))
                }),
        )
        .child(
            div()
                .flex_1()
                .text_sm()
                .child(photag_ui_bridge::i18n::tr(label)),
        )
}

fn date_row(view: &GalleryView, from: bool, cx: &mut Context<GalleryView>) -> Div {
    let (label, picker, has_value) = if from {
        (
            "Date from:",
            &view.bulk_date_from,
            view.bulk_date_from_value.is_some(),
        )
    } else {
        (
            "Date to:",
            &view.bulk_date_to,
            view.bulk_date_to_value.is_some(),
        )
    };
    let mut row = div()
        .h_flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .w(px(110.))
                .flex_shrink_0()
                .text_sm()
                .child(photag_ui_bridge::i18n::tr(label)),
        )
        .child(
            DatePicker::new(picker)
                .w(px(220.))
                .placeholder(photag_ui_bridge::i18n::tr("Select date")),
        );
    if has_value {
        row = row.child(
            Button::new(if from {
                "gallery-bulk-clear-date-from"
            } else {
                "gallery-bulk-clear-date-to"
            })
            .icon(photag_ui_bridge::theme::small_icon(
                gpui_kit::assets::IconName::X,
            ))
            .on_click(cx.listener(move |this, _, window, cx| {
                if from {
                    this.bulk_date_from_value = None;
                    this.bulk_date_from.update(cx, |picker, cx| {
                        picker.set_date(Date::Single(None), window, cx)
                    });
                } else {
                    this.bulk_date_to_value = None;
                    this.bulk_date_to.update(cx, |picker, cx| {
                        picker.set_date(Date::Single(None), window, cx)
                    });
                }
                this.confirm_bulk_action = false;
                this.confirm_bulk_action_ids = None;
                cx.notify();
            })),
        );
    }
    row
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
