//! File operation dialogs, feedback, and post-operation file_manager repair.

use std::future::Future;
use std::sync::Arc;

use cordis::Routing;
use fileatlas_plugin_file_operations::{BulkCopyOutcome, BulkDeleteOutcome, BulkMoveOutcome};
use gpui_kit::component::WindowExt as _;
use gpui_kit::*;

use super::FileManagerView;

impl FileManagerView {
    pub(super) fn copy_ids_to_directory(
        &mut self,
        ids: Vec<i64>,
        destination: String,
        cx: &mut Context<Self>,
    ) {
        if ids.is_empty() || !std::path::Path::new(&destination).is_dir() {
            return;
        }
        let operations = Arc::clone(&self.file_operations);
        let requested = ids.len();
        self.start_file_operation(
            async move {
                match operations
                    .bulk_copy_to_folder(ids, std::path::PathBuf::from(destination))
                    .await
                {
                    Err(error) => (format!("Copy failed: {error}"), Vec::new()),
                    Ok(outcomes) => (format_copy_outcome(requested, &outcomes), Vec::new()),
                }
            },
            false,
            false,
            cx,
        );
    }

    pub(super) fn move_staged_paths_to_directory(
        &mut self,
        paths: Vec<String>,
        destination: String,
        cx: &mut Context<Self>,
    ) {
        let destination_path = std::path::PathBuf::from(&destination);
        if self.grouping_mode != fileatlas_plugin_grouping::GroupingMode::Subfolder
            || !destination_path.is_dir()
            || paths.is_empty()
        {
            return;
        }
        let operations = Arc::clone(&self.file_operations);
        let cordis = self.cordis.clone();
        let requested = paths.len();
        let destination_label = destination.clone();
        self.start_file_operation(
            async move {
                match operations
                    .move_paths_to_folder(
                        paths.iter().map(std::path::PathBuf::from).collect(),
                        destination_path,
                    )
                    .await
                {
                    Err(error) => (format!("Staging move failed: {error}"), Vec::new()),
                    Ok(outcomes) => {
                        let completed = outcomes
                            .iter()
                            .filter(|outcome| outcome.completed)
                            .map(|outcome| outcome.source.to_string_lossy().into_owned())
                            .collect::<Vec<_>>();
                        if !completed.is_empty()
                            && let Err(error) = cordis
                                .emit::<fileatlas_contracts::FileStagingPathsMoved>(
                                    Routing::Unscoped,
                                    completed,
                                )
                                .await
                        {
                            tracing::warn!(%error, "could not update staging basket after drop move");
                        }
                        let moved = outcomes.iter().filter(|outcome| outcome.completed).count();
                        let failed = outcomes.iter().filter(|outcome| !outcome.completed).count();
                        let errors = outcomes
                            .iter()
                            .filter(|outcome| !outcome.completed)
                            .take(3)
                            .map(|outcome| {
                                format!(
                                    "{}: {}",
                                    outcome.source.display(),
                                    outcome.error.as_deref().unwrap_or("not moved")
                                )
                            })
                            .collect::<Vec<_>>();
                        let mut status = format!(
                            "Moved {moved} of {requested} staged files to {destination_label}."
                        );
                        if failed > 0 {
                            status.push_str(&format!(" {failed} failed or skipped."));
                            if !errors.is_empty() {
                                status.push(' ');
                                status.push_str(&errors.join("; "));
                            }
                        }
                        (status, Vec::new())
                    }
                }
            },
            true,
            false,
            cx,
        );
    }

    pub(super) fn move_ids_to_directory(
        &mut self,
        ids: Vec<i64>,
        destination: String,
        cx: &mut Context<Self>,
    ) {
        let destination_path = std::path::PathBuf::from(&destination);
        if self.grouping_mode != fileatlas_plugin_grouping::GroupingMode::Subfolder
            || !destination_path.is_dir()
            || ids.is_empty()
        {
            return;
        }
        let operations = Arc::clone(&self.file_operations);
        let file_manager = Arc::clone(&self.service);
        let requested = ids.len();
        self.start_file_operation(
            async move {
                match operations
                    .bulk_move_into_folder(ids.clone(), destination_path)
                    .await
                {
                    Err(error) => {
                        let removed = missing_rows(&file_manager, &ids);
                        (format!("Move failed or refresh failed: {error}"), removed)
                    }
                    Ok(outcomes) => {
                        let removed = outcomes
                            .iter()
                            .filter(|outcome| outcome.moved)
                            .filter_map(|outcome| {
                                file_manager
                                    .media_by_id(outcome.media_id)
                                    .ok()
                                    .is_some_and(|item| item.is_none())
                                    .then_some(outcome.media_id)
                            })
                            .collect::<Vec<_>>();
                        (format_move_outcome(requested, &outcomes), removed)
                    }
                }
            },
            true,
            true,
            cx,
        );
    }

    /// Open the source-compatible folder picker for a multi-item copy.
    pub(super) fn copy_selected_with_dialog(&mut self, cx: &mut Context<Self>) {
        let ids = self.selection.selected_media_ids.clone();
        if ids.is_empty() {
            return;
        }
        if ids.len() == 1 {
            self.copy_selected_to_file(cx);
            return;
        }
        let operations = Arc::clone(&self.file_operations);
        let requested = ids.len();
        cx.spawn(async move |this, cx| {
            let destination = rfd::AsyncFileDialog::new().pick_folder().await;
            if let Some(destination) = destination {
                let destination = destination.path().to_path_buf();
                let operations = Arc::clone(&operations);
                let _ = this.update(cx, |this, cx| {
                    this.start_file_operation(
                        async move {
                            match operations.bulk_copy_to_folder(ids, destination).await {
                                Err(error) => (format!("Copy failed: {error}"), Vec::new()),
                                Ok(outcomes) => {
                                    (format_copy_outcome(requested, &outcomes), Vec::new())
                                }
                            }
                        },
                        false,
                        false,
                        cx,
                    );
                });
            }
        })
        .detach();
    }

    /// Open the source-compatible folder picker for a multi-item move.
    pub(super) fn move_selected_with_dialog(&mut self, cx: &mut Context<Self>) {
        let ids = self.selection.selected_media_ids.clone();
        if ids.is_empty() {
            return;
        }
        if ids.len() == 1 {
            self.move_selected_to_file(cx);
            return;
        }
        let Some(destination) = rfd::FileDialog::new().pick_folder() else {
            return;
        };
        let operations = Arc::clone(&self.file_operations);
        let file_manager = Arc::clone(&self.service);
        let requested = ids.len();
        self.start_file_operation(
            async move {
                match operations
                    .bulk_move_into_folder(ids.clone(), destination)
                    .await
                {
                    Err(error) => {
                        let removed = missing_rows(&file_manager, &ids);
                        (format!("Move failed or refresh failed: {error}"), removed)
                    }
                    Ok(outcomes) => {
                        let moved = outcomes
                            .iter()
                            .filter(|outcome| outcome.moved)
                            .filter_map(|outcome| {
                                file_manager
                                    .media_by_id(outcome.media_id)
                                    .ok()
                                    .is_some_and(|item| item.is_none())
                                    .then_some(outcome.media_id)
                            })
                            .collect::<Vec<_>>();
                        (format_move_outcome(requested, &outcomes), moved)
                    }
                }
            },
            true,
            true,
            cx,
        );
    }

    /// Copy one selected item through a native Save File dialog.
    pub(super) fn copy_selected_to_file(&mut self, cx: &mut Context<Self>) {
        let Some(&media_id) = self.selection.selected_media_ids.first() else {
            return;
        };
        let Some(item) = self.service.media_by_id(media_id).ok().flatten() else {
            self.operation_status = Some("Selected media is no longer indexed.".into());
            cx.notify();
            return;
        };
        let operations = Arc::clone(&self.file_operations);
        let filename = item.filename;
        cx.spawn(async move |this, cx| {
            let destination = rfd::AsyncFileDialog::new()
                .set_file_name(&filename)
                .save_file()
                .await;
            if let Some(destination) = destination {
                let destination = destination.path().to_path_buf();
                let operations = Arc::clone(&operations);
                let _ = this.update(cx, |this, cx| {
                    this.start_file_operation(
                        async move {
                            match operations.copy_to(media_id, destination).await {
                                Ok(_) => ("Copied selected file.".into(), Vec::new()),
                                Err(error) => (format!("Copy failed: {error}"), Vec::new()),
                            }
                        },
                        false,
                        false,
                        cx,
                    );
                });
            }
        })
        .detach();
    }

    /// Move one selected item through a native Save File dialog.
    pub(super) fn move_selected_to_file(&mut self, cx: &mut Context<Self>) {
        let Some(&media_id) = self.selection.selected_media_ids.first() else {
            return;
        };
        let Some(item) = self.service.media_by_id(media_id).ok().flatten() else {
            self.operation_status = Some("Selected media is no longer indexed.".into());
            cx.notify();
            return;
        };
        let Some(destination) = rfd::FileDialog::new()
            .set_file_name(&item.filename)
            .save_file()
        else {
            return;
        };
        let operations = Arc::clone(&self.file_operations);
        let file_manager = Arc::clone(&self.service);
        self.start_file_operation(
            async move {
                let result = operations
                    .move_to_with_outcome(media_id, destination)
                    .await;
                let removed_from_library = file_manager
                    .media_by_id(media_id)
                    .ok()
                    .is_some_and(|item| item.is_none());
                let removed = removed_from_library
                    .then_some(media_id)
                    .into_iter()
                    .collect();
                match result {
                    Ok(true) => ("Moved selected file.".into(), removed),
                    Ok(false) => (
                        "Move was skipped because the destination conflicts or the file could not be moved.".into(),
                        Vec::new(),
                    ),
                    Err(error) => (format!("Move failed or refresh failed: {error}"), removed),
                }
            },

            true,
            true,
            cx,
        );
    }

    /// Show the native GPUI confirmation dialog for the current selection.
    pub(super) fn request_delete_confirmation(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selection.selected_media_ids.is_empty() {
            return;
        }
        let ids = self.selection.selected_media_ids.clone();
        let count = ids.len();
        self.confirm_delete = true;
        self.operation_status = Some(format!(
            "Confirming deletion of {count} item{}…",
            if count == 1 { "" } else { "s" }
        ));
        let view = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |dialog, _, _| {
            let ok_view = view.clone();
            let ok_ids = ids.clone();
            let cancel_view = view.clone();
            dialog
                .confirm()
                .title("Delete files?")
                .description(format!(
                    "Send {count} file{} to the Recycle Bin? They can be restored later.",
                    if count == 1 { "" } else { "s" }
                ))
                .ok_text(format!("Delete {count}"))
                .cancel_text("Cancel")
                .on_ok(move |_, _, cx| {
                    let _ = ok_view.update(cx, |this, cx| {
                        this.confirm_delete = false;
                        this.selection.selected_media_ids = ok_ids.clone();
                        this.selection.anchor_id = ok_ids.first().copied();
                        this.delete_selected_after_confirmation(cx);
                    });
                    true
                })
                .on_cancel(move |_, _, cx| {
                    let _ = cancel_view.update(cx, |this, cx| {
                        this.confirm_delete = false;
                        this.operation_status = Some("Delete cancelled.".into());
                        cx.notify();
                    });
                    true
                })
        });
        cx.notify();
    }

    pub(super) fn cancel_delete_confirmation(&mut self, cx: &mut Context<Self>) {
        if self.confirm_delete {
            self.confirm_delete = false;
            self.operation_status = Some("Delete cancelled.".into());
            cx.notify();
        }
    }

    pub(super) fn delete_selected_after_confirmation(&mut self, cx: &mut Context<Self>) {
        let ids = self.selection.selected_media_ids.clone();
        if ids.is_empty() {
            self.confirm_delete = false;
            return;
        }
        let operations = Arc::clone(&self.file_operations);
        let file_manager = Arc::clone(&self.service);
        let requested = ids.len();
        self.start_file_operation(
            async move {
                match operations.bulk_delete_to_recycle_bin(ids.clone()).await {
                    Err(error) => {
                        let removed = missing_rows(&file_manager, &ids);
                        (format!("Delete failed or refresh failed: {error}"), removed)
                    }
                    Ok(outcomes) => {
                        let removed = ids.clone();
                        (format_delete_outcome(requested, &outcomes), removed)
                    }
                }
            },
            true,
            false,
            cx,
        );
    }

    pub(super) fn rename_selected_with_repair(&mut self, cx: &mut Context<Self>) {
        if self.selection.selected_media_ids.len() != 1 {
            return;
        }
        let media_id = self.selection.selected_media_ids[0];
        let filename = self.rename_input.read(cx).value().to_string();
        let old_filename = self
            .service
            .media_by_id(media_id)
            .ok()
            .flatten()
            .map(|media| media.filename);
        let operations = Arc::clone(&self.file_operations);
        let file_manager = Arc::clone(&self.service);
        let history = self.history.clone();
        let expected_filename = filename.trim().to_string();
        self.start_file_operation(
            async move {
                match operations
                    .rename_with_outcome(media_id, expected_filename.clone())
                    .await
                {
                    Err(error) => match file_manager.media_by_id(media_id) {
                        Ok(Some(media)) if media.filename == expected_filename => {
                            if let (Some(history), Some(old_filename)) = (history, old_filename) {
                                if old_filename != expected_filename {
                                    history.push(fileatlas_plugin_history::UndoEntry::Renamed {
                                        media_id,
                                        old_filename,
                                        new_filename: expected_filename.clone(),
                                    });
                                }
                            }
                            (
                                format!(
                                    "Renamed to {}, but file_manager refresh failed: {error}",
                                    media.filename
                                ),
                                Vec::new(),
                            )
                        }
                        _ => (format!("Rename failed: {error}"), Vec::new()),
                    },
                    Ok(false) => (
                        "Rename was not applied; check the filename and collisions.".into(),
                        Vec::new(),
                    ),
                    Ok(true) => match file_manager.media_by_id(media_id) {
                        Ok(Some(media)) if media.filename == expected_filename => {
                            if let (Some(history), Some(old_filename)) = (history, old_filename) {
                                if old_filename != expected_filename {
                                    history.push(fileatlas_plugin_history::UndoEntry::Renamed {
                                        media_id,
                                        old_filename,
                                        new_filename: expected_filename.clone(),
                                    });
                                }
                            }
                            (format!("Renamed to {}.", media.filename), Vec::new())
                        }
                        Ok(_) => (
                            "Rename completed, but the updated row could not be verified.".into(),
                            Vec::new(),
                        ),
                        Err(error) => (
                            format!("Rename completed, but refresh failed: {error}"),
                            Vec::new(),
                        ),
                    },
                }
            },
            true,
            false,
            cx,
        );
    }

    fn start_file_operation<F>(
        &mut self,
        operation: F,
        refresh: bool,
        repair_selection_on_empty: bool,
        cx: &mut Context<Self>,
    ) where
        F: Future<Output = (String, Vec<i64>)> + Send + 'static,
    {
        self.operation_status = Some("Working…".into());
        self.confirm_delete = false;
        let selected_before = self.selection.selected_media_ids.clone();
        let skeleton_ids = self
            .snapshot
            .as_ref()
            .map(|snapshot| {
                snapshot
                    .skeletons
                    .iter()
                    .map(|item| item.id)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let operation = self.runtime.spawn(operation);
        cx.spawn(async move |this, cx| {
            let result = operation
                .await
                .unwrap_or_else(|error| (format!("Operation task failed: {error}"), Vec::new()));
            let _ = this.update(cx, |this, cx| {
                if refresh {
                    // A file mutation stays in the current browsing scope. Keep
                    // rendered rows, selection and scroll while data refreshes.
                    this.refresh_index();
                }
                let removed = result
                    .1
                    .iter()
                    .copied()
                    .collect::<std::collections::HashSet<_>>();
                let visible = this
                    .snapshot
                    .as_ref()
                    .map(|snapshot| {
                        snapshot
                            .skeletons
                            .iter()
                            .map(|item| item.id)
                            .collect::<std::collections::HashSet<_>>()
                    })
                    .unwrap_or_default();
                this.selection.selected_media_ids = selected_before
                    .iter()
                    .copied()
                    .filter(|id| visible.contains(id) && !removed.contains(id))
                    .collect();
                this.selection.anchor_id = this.selection.selected_media_ids.first().copied();

                if repair_selection_on_empty
                    && this.selection.selected_media_ids.is_empty()
                    && !selected_before.is_empty()
                {
                    let next = selected_before
                        .first()
                        .and_then(|id| next_visible(&skeleton_ids, *id, &removed, &visible));
                    if let Some(id) = next {
                        this.selection.selected_media_ids = vec![id];
                        this.selection.anchor_id = Some(id);
                    }
                }
                this.operation_status = Some(result.0);
                this.persist_filter_session();
                cx.notify();
            });
        })
        .detach();
    }
}

fn next_visible(
    order: &[i64],
    id: i64,
    removed: &std::collections::HashSet<i64>,
    visible: &std::collections::HashSet<i64>,
) -> Option<i64> {
    let index = order.iter().position(|candidate| *candidate == id)?;
    order
        .iter()
        .skip(index + 1)
        .find(|candidate| visible.contains(candidate) && !removed.contains(candidate))
        .or_else(|| {
            order[..index]
                .iter()
                .rev()
                .find(|candidate| visible.contains(candidate) && !removed.contains(candidate))
        })
        .copied()
}

fn missing_rows(file_manager: &crate::FileManagerService, ids: &[i64]) -> Vec<i64> {
    ids.iter()
        .copied()
        .filter(|id| {
            file_manager
                .media_by_id(*id)
                .ok()
                .is_some_and(|item| item.is_none())
        })
        .collect()
}

fn format_copy_outcome(requested: usize, outcomes: &[BulkCopyOutcome]) -> String {
    let copied = outcomes.iter().filter(|outcome| outcome.copied).count();
    super::format_bulk_outcome(
        "Copied",
        requested,
        copied,
        outcomes.iter().filter_map(|outcome| {
            outcome
                .error
                .as_ref()
                .map(|error| (outcome.media_id, error))
        }),
    )
}

fn format_move_outcome(requested: usize, outcomes: &[BulkMoveOutcome]) -> String {
    let moved = outcomes.iter().filter(|outcome| outcome.moved).count();
    super::format_bulk_outcome(
        "Moved",
        requested,
        moved,
        outcomes.iter().filter_map(|outcome| {
            outcome
                .error
                .as_ref()
                .map(|error| (outcome.media_id, error))
        }),
    )
}

fn format_delete_outcome(requested: usize, outcomes: &[BulkDeleteOutcome]) -> String {
    let removed = outcomes
        .iter()
        .filter(|outcome| outcome.removed_from_database)
        .count();
    let recycled = outcomes.iter().filter(|outcome| outcome.recycled).count();
    let mut message = format!(
        "Removed {removed} of {requested} selected items from the library; {recycled} files sent to the Recycle Bin."
    );
    let errors = outcomes
        .iter()
        .filter_map(|outcome| {
            outcome
                .error
                .as_ref()
                .map(|error| format!("{}: {error}", outcome.media_id))
        })
        .collect::<Vec<_>>();
    if requested > removed {
        message.push_str(" Some items were missing or their database rows could not be removed.");
    }
    if !errors.is_empty() {
        message.push_str(" Errors: ");
        message.push_str(&errors.join("; "));
    }
    message
}
