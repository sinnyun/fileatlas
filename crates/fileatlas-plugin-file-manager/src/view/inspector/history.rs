//! Lore-backed file history and version switching.
use super::*;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;

#[derive(Clone)]
pub(super) struct Version {
    pub version_id: String,
    pub label: String,
    pub branch: bool,
    pub modified: i64,
    pub size: i64,
    pub dimensions: String,
    pub thumbnail: Option<PathBuf>,
}

fn to_version(
    entry: &fileatlas_plugin_versions::VersionEntry,
    media: &fileatlas_plugin_database::MediaItem,
) -> Version {
    Version {
        version_id: entry.version_id.clone(),
        label: entry.title.clone(),
        branch: false,
        modified: chrono::DateTime::parse_from_rfc3339(&entry.created_at)
            .map(|time| time.timestamp())
            .unwrap_or(media.modified_at),
        size: entry.size_bytes.min(i64::MAX as u64) as i64,
        dimensions: dimensions(media),
        thumbnail: entry.thumbnail_path.as_ref().map(PathBuf::from),
    }
}

impl FileManagerView {
    fn request_history(&mut self, media: &fileatlas_plugin_database::MediaItem) {
        let key = (media.id, media.full_path.clone(), self.history_generation);
        let path = media.full_path.clone();
        let versions = self.versions.clone();
        let runtime = self.runtime.clone();
        self.history_read.request(key, move || {
            runtime
                .block_on(versions.list(&path))
                .map_err(|error| error.to_string())
        });
        let key = (media.id, media.full_path.clone(), self.history_generation);
        let path = media.full_path.clone();
        let versions = self.versions.clone();
        let runtime = self.runtime.clone();
        self.history_branches.request(key, move || {
            runtime
                .block_on(versions.branch_graph(&path, false))
                .map_err(|error| error.to_string())
        });
    }

    pub(super) fn inspector_history(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let Some(media) = self.inspector_media() else {
            return inspector_message("Select one file to view its version history.");
        };
        self.request_history(&media);
        let loading = self.history_read.is_pending();
        let read = self.history_read.value();
        let error = read.and_then(|result| result.as_ref().err()).cloned();
        let records = read
            .and_then(|result| result.as_ref().ok())
            .cloned()
            .unwrap_or_default();
        let versions = records
            .iter()
            .map(|entry| to_version(entry, &media))
            .collect::<Vec<_>>();
        let active = records.iter().position(|entry| entry.is_current);
        let selected = self
            .history_selected
            .as_ref()
            .filter(|(id, _)| *id == media.id)
            .and_then(|(_, revision)| {
                records
                    .iter()
                    .position(|entry| &entry.version_id == revision)
            })
            .or(active)
            .unwrap_or(0);
        let _ = self
            .surfaces
            .set_value("file_manager.inspector.history-enabled", "true");
        let _ = self.surfaces.set_value(
            "file_manager.inspector.history-count",
            &records.len().to_string(),
        );
        let _ = self.surfaces.set_value(
            "file_manager.inspector.history-active",
            &active
                .and_then(|index| records.get(index))
                .map(|entry| entry.version_id.as_str())
                .unwrap_or(""),
        );
        let _ = self.surfaces.set_value(
            "file_manager.inspector.history-selected",
            &records
                .get(selected)
                .map(|entry| entry.version_id.as_str())
                .unwrap_or(""),
        );
        let selected_version = versions.get(selected);
        let selected_revision = records
            .get(selected)
            .map(|entry| entry.version_id.clone())
            .unwrap_or_default();
        let weak = cx.entity().downgrade();
        let create_open = self.history_create_open;
        let mut body = div()
            .v_flex()
            .w_full()
            .min_w_0()
            .gap(px(theme::space_standard()))
            .p(px(fileatlas_ui_bridge::theme::space_expanded()));
        if create_open {
            body = body
                .child(
                    fileatlas_ui_bridge::components::text_input(&self.history_title)
                        .w_full()
                        .min_w_0(),
                )
                .child(
                    div()
                        .h_flex()
                        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                        .child(
                            fileatlas_ui_bridge::components::button("inspector-history-save")
                                .primary()
                                .flex_1()
                                .label(fileatlas_ui_bridge::i18n::tr("Save version"))
                                .disabled(
                                    self.history_busy
                                        || self.history_title.read(cx).value().trim().is_empty(),
                                )
                                .on_click(
                                    cx.listener(|view, _, _, cx| view.create_history_version(cx)),
                                ),
                        )
                        .child(
                            fileatlas_ui_bridge::components::button("inspector-history-cancel")
                                .outline()
                                .label(fileatlas_ui_bridge::i18n::tr("Cancel"))
                                .disabled(self.history_busy)
                                .on_click(cx.listener(|view, _, _, cx| {
                                    view.history_create_open = false;
                                    cx.notify();
                                })),
                        ),
                );
        } else {
            body = body.child(
                div()
                    .h_flex()
                    .w_full()
                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                    .child(
                        fileatlas_ui_bridge::components::button("inspector-history-create")
                            .primary()
                            .flex_1()
                            .icon(theme::icon(IconName::Plus))
                            .label(fileatlas_ui_bridge::i18n::tr("Create version"))
                            .disabled(self.history_busy)
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.history_error = None;
                                view.history_create_open = true;
                                cx.notify();
                            })),
                    )
                    .child(
                        fileatlas_ui_bridge::components::button("inspector-history-switch")
                            .outline()
                            .flex_1()
                            .label(fileatlas_ui_bridge::i18n::tr("Switch version"))
                            .disabled(
                                self.history_busy
                                    || selected_version.is_none()
                                    || active == Some(selected),
                            )
                            .on_click(cx.listener(move |view, _, _, cx| {
                                view.activate_history_version(&selected_revision, cx)
                            })),
                    ),
            );
        }
        body = body.child(
            div()
                .h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .child(section_title("Version history"))
                .child(
                    div()
                        .workbench_assistant()
                        .child(format!("{} versions · Lore", versions.len())),
                ),
        );
        body = body.child(fileatlas_ui_bridge::components::subtle_divider());
        if let Some(message) = error.or_else(|| self.history_error.clone()) {
            body = body.child(inspector_message(&message));
        } else if loading && versions.is_empty() {
            body = body.child(inspector_message("Loading version history…"));
        } else if versions.is_empty() {
            body = body.child(inspector_message(
                "No saved versions. Create a version to start this file’s history.",
            ));
        } else {
            body = body.child(history_tree::render(
                &versions,
                selected,
                active.unwrap_or(usize::MAX),
                0,
                !self.history_busy,
                cx,
            ));
            if let Some(selected_version) = selected_version {
                body = body
                    .child(fileatlas_ui_bridge::components::subtle_divider())
                    .child(
                        well()
                            .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                            .child(section_title("Selected version"))
                            .child(
                                div()
                                    .h_flex()
                                    .w_full()
                                    .min_w_0()
                                    .items_start()
                                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                                    .child(history_tree::thumbnail(
                                        selected_version.thumbnail.clone(),
                                        72.,
                                        54.,
                                    ))
                                    .child(
                                        div()
                                            .v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .child(info_row(
                                                "Version",
                                                selected_version.label.clone(),
                                            ))
                                            .child(info_row(
                                                "Modified",
                                                card::format_timestamp(selected_version.modified),
                                            ))
                                            .child(info_row(
                                                "File size",
                                                card::format_bytes(selected_version.size),
                                            ))
                                            .child(info_row(
                                                "Image dimensions",
                                                selected_version.dimensions.clone(),
                                            )),
                                    ),
                            ),
                    );
            }
        }
        if let Some(status) = self.version_state(media.id, &media.full_path) {
            if let Some(state) = &status.state
                && state.status != fileatlas_plugin_versions::FileVersionStatus::Unversioned
            {
                body = body.child(
                    div()
                        .workbench_assistant()
                        .child(super::super::version_badges::message(state)),
                );
            }
            if let Some(error) = &status.error {
                body = body.child(inspector_message(&format!(
                    "{} {error}",
                    fileatlas_ui_bridge::i18n::tr("Version status could not be checked.")
                )));
            }
        }
        body = body.child(super::super::branch_actions::render_history_branches(
            self, cx,
        ));
        let _ = weak;
        body.into_any_element()
    }

    pub(super) fn select_history_version(&mut self, version_id: &str, cx: &mut Context<Self>) {
        if self.history_busy {
            return;
        }
        let Some(media) = self.inspector_media() else {
            return;
        };
        let Some(Ok(entries)) = self.history_read.value() else {
            return;
        };
        let Some(entry) = entries.iter().find(|entry| entry.version_id == version_id) else {
            return;
        };
        self.history_selected = Some((media.id, entry.version_id.clone()));
        let _ = self
            .surfaces
            .set_value("file_manager.inspector.history-selected", &entry.version_id);
        cx.notify();
    }

    pub(super) fn create_history_version(&mut self, cx: &mut Context<Self>) {
        if self.history_busy {
            return;
        }
        let Some(media) = self.inspector_media() else {
            return;
        };
        let title = self.history_title.read(cx).value().trim().to_string();
        if title.is_empty() {
            return;
        }
        self.history_busy = true;
        self.history_error = None;
        let path = media.full_path.clone();
        let versions = self.versions.clone();
        let operation = self.runtime.spawn(async move {
            versions
                .create_version(&path, &title)
                .await
                .map_err(|e| e.to_string())
        });
        cx.spawn(async move |this, cx| {
            let result = operation
                .await
                .unwrap_or_else(|e| Err(format!("Version task failed: {e}")));
            let _ = this.update(cx, |view, cx| {
                view.history_busy = false;
                match result {
                    Ok(entry) => {
                        view.history_generation = view.history_generation.wrapping_add(1);
                        view.history_selected = Some((media.id, entry.version_id.clone()));
                        view.history_create_open = false;
                        let _ = view.surfaces.set_value(
                            "file_manager.inspector.history-last-created",
                            &entry.version_id,
                        );
                    }
                    Err(error) => view.history_error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn activate_history_version(&mut self, version_id: &str, cx: &mut Context<Self>) {
        if self.history_busy {
            return;
        }
        let Some(media) = self.inspector_media() else {
            return;
        };
        let Some(Ok(entries)) = self.history_read.value() else {
            return;
        };
        let Some(entry) = entries.iter().find(|entry| entry.version_id == version_id) else {
            return;
        };
        if entry.is_current {
            return;
        }
        let revision = entry.version_id.clone();
        let activation_revision = revision.clone();
        let path = media.full_path.clone();
        let service = self.versions.clone();
        let file_manager = self.service.clone();
        let media_id = media.id;
        let operation = self.runtime.spawn(async move {
            service
                .activate(&path, &activation_revision)
                .await
                .map_err(|error| error.to_string())?;
            tokio::task::spawn_blocking(move || file_manager.refresh_restored_file(media_id))
                .await
                .map_err(|error| format!("Index refresh task failed: {error}"))??;
            Ok::<(), String>(())
        });
        self.history_busy = true;
        self.history_error = None;
        cx.spawn(async move |this, cx| {
            let result = operation
                .await
                .unwrap_or_else(|error| Err(format!("Version task failed: {error}")));
            let _ = this.update(cx, |view, cx| {
                view.history_busy = false;
                match result {
                    Ok(()) => {
                        view.history_generation = view.history_generation.wrapping_add(1);
                        view.history_selected = Some((media.id, revision.clone()));
                        view.refresh_index();
                        view.operation_status =
                            Some("Switched to the selected file version.".into());
                        let _ = view
                            .surfaces
                            .set_value("file_manager.inspector.history-current", &revision);
                    }
                    Err(error) => view.history_error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}

fn dimensions(media: &fileatlas_plugin_database::MediaItem) -> String {
    media
        .width
        .zip(media.height)
        .map(|(w, h)| format!("{w} × {h}"))
        .unwrap_or_else(|| "—".into())
}
