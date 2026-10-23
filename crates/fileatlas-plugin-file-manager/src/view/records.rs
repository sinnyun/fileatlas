//! Reviewable missing-file record maintenance. All filesystem work is off-thread.
use super::*;
use fileatlas_ui_bridge::components::WorkbenchPaint as _;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
use gpui_kit::component::button::ButtonVariants;

impl FileManagerView {
    pub(super) fn poll_records(&mut self) -> bool {
        if !self.show_records {
            return false;
        }
        let Some(tagging) = self.tagging.clone() else {
            return false;
        };
        self.records_read.update(
            (self.service.data_revision(), self.record_read_revision),
            move || tagging.missing_records(),
        )
    }

    pub(super) fn choose_record_target(&mut self, id: Option<i64>, cx: &mut Context<Self>) {
        let items = self
            .records_read
            .value()
            .and_then(|r| r.as_ref().ok())
            .cloned()
            .unwrap_or_default();
        self.start_record_picker(id, items, cx);
    }

    fn start_record_picker(
        &mut self,
        id: Option<i64>,
        items: Vec<fileatlas_plugin_database::MediaItem>,
        cx: &mut Context<Self>,
    ) {
        if self.location_picker_pending {
            return;
        }
        self.location_picker_pending = true;
        let task = self.runtime.spawn(async move {
            let picked = if id.is_some() {
                rfd::AsyncFileDialog::new().pick_file().await
            } else {
                rfd::AsyncFileDialog::new().pick_folder().await
            };
            let Some(picked) = picked else {
                return Vec::new();
            };
            let target = picked.path().to_path_buf();
            tokio::task::spawn_blocking(move || {
                items
                    .into_iter()
                    .filter_map(|item| {
                        if id.is_some_and(|id| id != item.id) {
                            return None;
                        }
                        let path = if id.is_some() {
                            target.clone()
                        } else {
                            target.join(&item.filename)
                        };
                        path.is_file()
                            .then_some((item.id, path.to_string_lossy().into_owned()))
                    })
                    .collect::<Vec<_>>()
            })
            .await
            .unwrap_or_default()
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |view, cx| {
                view.location_picker_pending = false;
                if let Ok(targets) = result {
                    view.record_candidates.extend(targets);
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn relink_record(&mut self, id: i64, cx: &mut Context<Self>) {
        let Some(path) = self.record_candidates.get(&id).cloned() else {
            return;
        };
        let Some(tagging) = self.tagging.clone() else {
            return;
        };
        self.record_candidates.remove(&id);
        self.start_operation(
            async move {
                match tagging.relink_record(id, &path) {
                    Ok(()) => "File record relinked".into(),
                    Err(error) => format!("Relink failed: {error}"),
                }
            },
            true,
            cx,
        );
    }

    pub(super) fn delete_missing_records(&mut self, cx: &mut Context<Self>) {
        let Some(tagging) = self.tagging.clone() else {
            return;
        };
        let ids = self
            .records_read
            .value()
            .and_then(|r| r.as_ref().ok())
            .map(|items| items.iter().map(|i| i.id).collect::<Vec<_>>())
            .unwrap_or_default();
        self.confirm_delete_missing = false;
        self.start_operation(
            async move {
                match tagging.delete_missing_records(&ids) {
                    Ok(count) => format!("Deleted {count} missing file records"),
                    Err(error) => format!("Delete records failed: {error}"),
                }
            },
            true,
            cx,
        );
    }
}

pub(super) fn render(view: &FileManagerView, cx: &mut Context<FileManagerView>) -> AnyElement {
    let items = view
        .records_read
        .value()
        .and_then(|r| r.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    let count = items.len();
    let pages = count.div_ceil(50).max(1);
    let page = view.records_page.min(pages - 1);
    let mut content = div()
        .v_flex()
        .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
        .w_full()
        .min_h_0()
        .child(
            div()
                .h_flex()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                .child(
                    fileatlas_ui_bridge::components::button("records-choose-folder")
                        .label(fileatlas_ui_bridge::i18n::tr("Choose relink folder"))
                        .on_click(
                            cx.listener(|view, _, _, cx| view.choose_record_target(None, cx)),
                        ),
                )
                .child(
                    fileatlas_ui_bridge::components::button("records-delete-missing")
                        .label(fileatlas_ui_bridge::i18n::tr(
                            if view.confirm_delete_missing {
                                "Confirm delete missing records"
                            } else {
                                "Delete missing records"
                            },
                        ))
                        .on_click(cx.listener(|view, _, _, cx| {
                            if view.confirm_delete_missing {
                                view.delete_missing_records(cx)
                            } else {
                                view.confirm_delete_missing = true;
                                cx.notify();
                            }
                        })),
                )
                .child(
                    fileatlas_ui_bridge::components::button("records-refresh")
                        .ghost()
                        .label(fileatlas_ui_bridge::i18n::tr("Refresh folder"))
                        .on_click(cx.listener(|view, _, _, cx| {
                            view.record_read_revision += 1;
                            cx.notify();
                        })),
                ),
        )
        .child(div().workbench_assistant().child(format!(
            "{}: {count}",
            fileatlas_ui_bridge::i18n::tr("Missing files")
        )));
    if let Some(Err(error)) = view.records_read.value() {
        content = content.child(div().workbench_assistant().child(error.clone()));
    }
    if view.records_read.is_pending() {
        content = content.child(gpui_kit::component::spinner::Spinner::new());
    }
    content = content.child(
        div()
            .h_flex()
            .gap(px(fileatlas_ui_bridge::theme::space_standard()))
            .child(
                fileatlas_ui_bridge::components::button("records-prev")
                    .ghost()
                    .label("‹")
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.records_page = page.saturating_sub(1);
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .workbench_assistant()
                    .child(format!("{} / {pages}", page + 1)),
            )
            .child(
                fileatlas_ui_bridge::components::button("records-next")
                    .ghost()
                    .label("›")
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.records_page = (page + 1).min(pages - 1);
                        cx.notify();
                    })),
            ),
    );
    let rows = items.into_iter().skip(page * 50).take(50).map(|item| {
        let id = item.id;
        let candidate = view.record_candidates.get(&id).cloned();
        let mut row = div()
            .v_flex()
            .gap(px(fileatlas_ui_bridge::theme::space_standard()))
            .p(px(fileatlas_ui_bridge::theme::space_expanded()))
            .workbench_corner(fileatlas_ui_bridge::components::CornerRole::Medium)
            .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Faint)
            .child(div().workbench_body().child(item.filename))
            .child(div().workbench_assistant().child(format!(
                        "{} · {}",
                        card::format_bytes(item.file_size),
                        chrono::DateTime::from_timestamp(item.modified_at, 0)
                            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                            .unwrap_or_default()
                    )))
            .child(div().workbench_assistant().child(item.full_path))
            .child(
                fileatlas_ui_bridge::components::button(format!("relink-choose-{id}"))
                    .ghost()
                    .label(fileatlas_ui_bridge::i18n::tr("Choose replacement file"))
                    .on_click(
                        cx.listener(move |view, _, _, cx| view.choose_record_target(Some(id), cx)),
                    ),
            );
        if let Some(path) = item.thumb_path {
            row = row.child(
                div()
                    .h_flex()
                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                    .child(
                        img(PathBuf::from(path))
                            .size(px(64.))
                            .object_fit(ObjectFit::Contain),
                    )
                    .child(
                        div()
                            .workbench_body()
                            .child(fileatlas_ui_bridge::i18n::tr("Saved preview")),
                    ),
            );
        }
        if let Some(candidate) = candidate {
            row = row
                .child(div().workbench_assistant().child(candidate))
                .child(
                    div()
                        .h_flex()
                        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                        .child(
                            fileatlas_ui_bridge::components::button(format!("relink-apply-{id}"))
                                .label(fileatlas_ui_bridge::i18n::tr("Apply relink"))
                                .on_click(
                                    cx.listener(move |view, _, _, cx| view.relink_record(id, cx)),
                                ),
                        )
                        .child(
                            fileatlas_ui_bridge::components::button(format!("relink-skip-{id}"))
                                .ghost()
                                .label(fileatlas_ui_bridge::i18n::tr("Skip"))
                                .on_click(cx.listener(move |view, _, _, cx| {
                                    view.record_candidates.remove(&id);
                                    cx.notify();
                                })),
                        ),
                );
        }
        row
    });
    content
        .child(
            div()
                .v_flex()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                .h(px(350.))
                .overflow_y_scrollbar()
                .children(rows),
        )
        .into_any_element()
}
