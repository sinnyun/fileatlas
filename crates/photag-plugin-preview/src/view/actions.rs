//! Source-compatible preview rename and recycle-bin delete actions.

use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::component::WindowExt as _;
use gpui_kit::*;

use super::PreviewView;

impl PreviewView {
    pub(super) fn begin_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.operation_busy {
            return;
        }
        let snapshot = self.service.snapshot();
        let (Some(media_id), Some(path)) = (snapshot.media_id, snapshot.source) else {
            self.operation_status = Some("Rename requires an indexed media item.".into());
            cx.notify();
            return;
        };
        let Some(filename) = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            self.operation_status = Some("The selected file has no filename.".into());
            cx.notify();
            return;
        };
        self.rename_input
            .update(cx, |input, cx| input.set_value(&filename, window, cx));
        let focus = self.rename_input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        self.rename_target = Some(media_id);
        self.operation_status = None;
        cx.notify();
    }

    pub(super) fn commit_rename(&mut self, cx: &mut Context<Self>) {
        if self.operation_busy {
            return;
        }
        let Some(media_id) = self.rename_target else {
            return;
        };
        let filename = self.rename_input.read(cx).value().trim().to_owned();
        if filename.is_empty() || filename.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|'])
        {
            self.operation_status = Some("Enter a valid filename without path separators.".into());
            cx.notify();
            return;
        }
        let operations = Arc::clone(&self.file_operations);
        let verification = Arc::clone(&self.file_operations);
        let runtime = self.runtime.clone();
        let service = Arc::clone(&self.service);
        let old_path = self.service.snapshot().source;
        let task = runtime.spawn(async move {
            let result = operations
                .rename_with_outcome(media_id, filename.clone())
                .await;
            let current = verification.media_by_id(media_id);
            (result, current, filename)
        });
        self.operation_busy = true;
        self.operation_status = Some("Renaming…".into());
        cx.spawn(async move |this, cx| {
            let (result, current, filename) = task.await.unwrap_or_else(|error| {
                (Err(photag_plugin_file_operations::FileOperationError::Worker(error)), Ok(None), String::new())
            });
            let _ = this.update(cx, |this: &mut PreviewView, cx| {
                this.operation_busy = false;
                this.rename_target = None;
                match result {
                    Ok(true) => {
                        if let Some(path) = old_path {
                            if let Some(parent) = path.parent() {
                                let renamed = parent.join(&filename);
                                service.replace_media_path(&path, renamed, media_id);
                                this.operation_status = Some(format!("Renamed to {filename}."));
                            }
                        }
                    }
                    Ok(false) => this.operation_status = Some("Rename was not applied; the name may be unchanged or already used.".into()),
                    Err(error) => {
                        let path_updated = current.as_ref().ok().and_then(|item| item.as_ref()).is_some_and(|item| {
                            old_path.as_ref().is_some_and(|old| old.parent().is_some_and(|parent| {
                                item.full_path == parent.join(&filename).to_string_lossy().into_owned()
                            }))
                        });
                        if path_updated {
                            if let Some(path) = old_path.as_ref() {
                                if let Some(parent) = path.parent() {
                                    service.replace_media_path(path, parent.join(&filename), media_id);
                                }
                            }
                            this.operation_status = Some(format!("Renamed to {filename}, but the gallery refresh failed: {error}"));
                        } else {
                            let target = old_path.as_ref().and_then(|old| old.parent()).map(|parent| parent.join(&filename));
                            if let (Some(old), Some(target)) = (old_path.as_ref(), target.filter(|target| target.is_file())) {
                                service.replace_media_path(old, target, media_id);
                                this.operation_status = Some(format!("File renamed to {filename}, but the database/index update failed: {error}"));
                            } else {
                                this.operation_status = Some(match current {
                                    Err(read_error) => format!("Rename failed: {error}; could not verify the indexed row: {read_error}"),
                                    _ => format!("Rename failed: {error}"),
                                });
                            }
                        }
                    }
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }

    pub(super) fn request_delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.operation_busy || self.delete_confirmation_open {
            return;
        }
        let snapshot = self.service.snapshot();
        let (Some(media_id), Some(path)) = (snapshot.media_id, snapshot.source) else {
            self.operation_status = Some("Delete requires an indexed media item.".into());
            cx.notify();
            return;
        };
        let filename = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let ordered = self.service.filmstrip_items();
        let index = ordered.iter().position(|(candidate, _)| candidate == &path);
        let next = index.and_then(|index| {
            ordered
                .get(index + 1)
                .or_else(|| {
                    index
                        .checked_sub(1)
                        .and_then(|previous| ordered.get(previous))
                })
                .cloned()
        });
        let view = cx.entity().downgrade();
        let operations = Arc::clone(&self.file_operations);
        let verification = Arc::clone(&self.file_operations);
        let service = Arc::clone(&self.service);
        let runtime = self.runtime.clone();
        self.delete_confirmation_open = true;
        window.open_alert_dialog(cx, move |dialog, _, _| {
            let ok_view = view.clone();
            let cancel_view = view.clone();
            let operations = Arc::clone(&operations);
            let verification = Arc::clone(&verification);
            let service = Arc::clone(&service);
            let runtime = runtime.clone();
            let next = next.clone();
            let path = path.clone();
            dialog.confirm()
                .title("Delete file?")
                .description(format!("Send {filename} to the Recycle Bin? It can be restored later."))
                .ok_text("Delete")
                .cancel_text("Cancel")
                .on_ok(move |_, _, cx| {
                    let operations = Arc::clone(&operations);
                    let verification = Arc::clone(&verification);
                    let service = Arc::clone(&service);
                    let next = next.clone();
                    let path = path.clone();
                    let task = runtime.spawn(async move {
                        let result = operations.delete_to_recycle_bin(media_id).await;
                        let current = verification.media_by_id(media_id);
                        (result, current)
                    });
                    let _ = ok_view.update(cx, |this, cx| {
                        this.delete_confirmation_open = false;
                        this.operation_busy = true;
                        this.operation_status = Some("Deleting…".into());
                        cx.spawn(async move |this, cx| {
                            let task_result = task.await;
                            let _ = this.update(cx, |this: &mut PreviewView, cx| {
                                this.operation_busy = false;
                                match task_result {
                                    Err(error) => this.operation_status = Some(format!("Delete worker failed: {error}")),
                                    Ok((result, current)) => match result {
                                        Ok(outcome) if outcome.removed_from_database => this.finish_delete(
                                            &service, &path, next, outcome.recycle_error, None,
                                        ),
                                        Ok(_) => this.operation_status = Some("The item is no longer indexed.".into()),
                                        Err(error) if matches!(&current, Ok(None)) => this.finish_delete(
                                            &service, &path, next, None, Some(error.to_string()),
                                        ),
                                        Err(error) => this.operation_status = Some(match current {
                                            Err(read_error) => format!("Delete failed: {error}; could not verify the indexed row: {read_error}"),
                                            _ => format!("Delete failed: {error}"),
                                        }),
                                    },
                                }
                                cx.notify();
                            });
                        }).detach();
                    });
                    true
                })
                .on_cancel(move |_, _, cx| {
                    let _ = cancel_view.update(cx, |this, cx| { this.delete_confirmation_open = false; this.operation_status = Some("Delete cancelled.".into()); cx.notify(); });
                    true
                })
        });
    }

    fn finish_delete(
        &mut self,
        service: &crate::PreviewService,
        deleted_path: &std::path::Path,
        next: Option<(PathBuf, Option<i64>)>,
        recycle_error: Option<String>,
        mutation_error: Option<String>,
    ) {
        let remaining = service
            .filmstrip_items()
            .into_iter()
            .filter(|(candidate, _)| candidate != deleted_path)
            .collect::<Vec<_>>();
        // `set_filmstrip` keeps the ID map for paths that remain and leaves
        // path-only entries without an ID, matching the caller's input.
        service.set_filmstrip(remaining.into_iter().map(|(path, _)| path));
        let mut advanced = false;
        let mut closed_current = false;
        if service.snapshot().source.as_deref() == Some(deleted_path) {
            if let Some((next_path, next_id)) = next.filter(|(candidate, _)| candidate.is_file()) {
                if let Err(error) = service.open_media(next_path, next_id) {
                    service.close();
                    self.operation_status = Some(format!(
                        "Deleted, but could not open the next item: {error}"
                    ));
                    return;
                }
                advanced = true;
            } else {
                service.close();
                closed_current = true;
            }
        }
        self.operation_status = Some(
            match (
                recycle_error,
                mutation_error,
                service.snapshot().source.is_some(),
            ) {
                (Some(error), _, _) => {
                    format!("Removed from the library; Recycle Bin reported: {error}")
                }
                (_, Some(error), _) => {
                    format!("Delete completed, but Gallery refresh failed: {error}")
                }
                (None, None, _) if advanced => "Deleted and advanced to the next item.".into(),
                (None, None, _) if closed_current => {
                    "Deleted the last item; preview closed.".into()
                }
                (None, None, _) => "Deleted the item from the library.".into(),
            },
        );
    }
}
