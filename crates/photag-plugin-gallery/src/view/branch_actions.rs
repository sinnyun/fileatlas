use super::*;
use gpui_kit::base::Disableable;
use gpui_kit::component::button::ButtonVariants;

type BranchActionFuture =
    std::pin::Pin<Box<dyn Future<Output = Result<(String, bool), String>> + Send + 'static>>;

pub(super) fn render_history_branches(view: &GalleryView, cx: &mut Context<GalleryView>) -> Div {
    let mut panel = div()
        .v_flex()
        .w_full()
        .min_w_0()
        .gap(px(8.))
        .child(branch_section_title("Branches"));
    let graph = view
        .history_branches
        .value()
        .and_then(|result| result.as_ref().ok());
    let pending = graph.and_then(|graph| graph.pending_merge.as_ref());
    panel = panel.child(
        div()
            .h_flex()
            .gap(px(6.))
            .child(
                photag_ui_bridge::components::text_input(&view.history_branch_name)
                    .flex_1()
                    .min_w_0(),
            )
            .child(
                Button::new("inspector-history-branch-create")
                    .outline()
                    .label("Create")
                    .disabled(
                        view.history_busy
                            || view.history_branch_name.read(cx).value().trim().is_empty(),
                    )
                    .on_click(cx.listener(|view, _, _, cx| view.create_history_branch(cx))),
            ),
    );
    match view.history_branches.value() {
        Some(Err(error)) => panel = panel.child(branch_note(error)),
        None => {
            panel = panel.child(branch_note("Loading branches…"));
        }
        Some(Ok(graph)) if graph.branches.is_empty() => {
            panel = panel.child(branch_note("Create a file version to start a branch."));
        }
        Some(Ok(graph)) => {
            for (index, branch) in graph.branches.iter().enumerate() {
                let name = branch.name.clone();
                let switch_name = name.clone();
                let archive_name = name.clone();
                let merge_name = name.clone();
                let is_current = branch.is_current;
                panel = panel.child(
                    div()
                        .v_flex()
                        .w_full()
                        .gap(px(5.))
                        .p(px(8.))
                        .rounded(px(7.))
                        .border_1()
                        .border_color(rgba(if is_current {
                            (theme::accent() << 8) | 0x80
                        } else {
                            theme::SEPARATOR_LIGHT
                        }))
                        .child(
                            div()
                                .h_flex()
                                .justify_between()
                                .child(
                                    div()
                                        .text_color(rgb(theme::TEXT_PRIMARY))
                                        .child(name.clone()),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(theme::TEXT_SECONDARY))
                                        .child(if is_current { "Current" } else { "Branch" }),
                                ),
                        )
                        .child(
                            div()
                                .h_flex()
                                .gap(px(5.))
                                .child(
                                    Button::new(format!("inspector-history-branch-switch-{index}"))
                                        .outline()
                                        .compact()
                                        .label("Switch")
                                        .disabled(
                                            view.history_busy || is_current || pending.is_some(),
                                        )
                                        .on_click(cx.listener(move |view, _, _, cx| {
                                            view.switch_history_branch(&switch_name, cx)
                                        })),
                                )
                                .child(
                                    Button::new(format!("inspector-history-branch-merge-{index}"))
                                        .outline()
                                        .compact()
                                        .label("Merge into current")
                                        .disabled(
                                            view.history_busy || is_current || pending.is_some(),
                                        )
                                        .on_click(cx.listener(move |view, _, _, cx| {
                                            view.merge_history_branch(&merge_name, cx)
                                        })),
                                )
                                .child(
                                    Button::new(format!(
                                        "inspector-history-branch-archive-{index}"
                                    ))
                                    .ghost()
                                    .compact()
                                    .label("Archive")
                                    .disabled(view.history_busy || is_current || pending.is_some())
                                    .on_click(cx.listener(
                                        move |view, _, _, cx| {
                                            view.archive_history_branch(&archive_name, cx)
                                        },
                                    )),
                                ),
                        ),
                );
            }
            if let Some(pending) = pending {
                panel = panel.child(
                    div()
                        .v_flex()
                        .gap(px(6.))
                        .p(px(8.))
                        .rounded(px(7.))
                        .bg(rgba((theme::WARNING << 8) | 0x20))
                        .child(branch_section_title("Merge conflicts"))
                        .child(format!(
                            "{} into {}: {}",
                            pending.source_branch,
                            pending.target_branch,
                            pending.conflict_files.join(", ")
                        ))
                        .child(
                            div()
                                .h_flex()
                                .gap(px(5.))
                                .child(
                                    Button::new("inspector-history-merge-ours")
                                        .outline()
                                        .compact()
                                        .label("Keep current")
                                        .disabled(view.history_busy)
                                        .on_click(cx.listener(|view, _, _, cx| {
                                            view.resolve_history_merge("ours", cx)
                                        })),
                                )
                                .child(
                                    Button::new("inspector-history-merge-theirs")
                                        .outline()
                                        .compact()
                                        .label("Use incoming")
                                        .disabled(view.history_busy)
                                        .on_click(cx.listener(|view, _, _, cx| {
                                            view.resolve_history_merge("theirs", cx)
                                        })),
                                )
                                .child(
                                    Button::new("inspector-history-merge-abort")
                                        .ghost()
                                        .compact()
                                        .label("Abort")
                                        .disabled(view.history_busy)
                                        .on_click(cx.listener(|view, _, _, cx| {
                                            view.abort_history_merge(cx)
                                        })),
                                ),
                        ),
                );
            }
        }
    }
    panel
}

impl GalleryView {
    fn run_branch_action<F>(&mut self, cx: &mut Context<Self>, action: F)
    where
        F: FnOnce(Arc<photag_plugin_versions::VersionsService>, String) -> BranchActionFuture
            + Send
            + 'static,
    {
        let Some(media) = self.current_inspector_media() else {
            return;
        };
        let path = media.full_path.clone();
        let media_id = media.id;
        let service = self.versions.clone();
        let gallery = self.service.clone();
        let operation = self.runtime.spawn(action(service, path));
        self.history_busy = true;
        self.history_error = None;
        cx.spawn(async move |this, cx| {
            let result = match operation.await {
                Ok(result) => result,
                Err(error) => Err(format!("Branch operation task failed: {error}")),
            };
            let result = match result {
                Ok((message, refresh)) if refresh => {
                    match tokio::task::spawn_blocking(move || {
                        gallery.refresh_restored_file(media_id)
                    })
                    .await
                    {
                        Ok(Ok(())) => Ok(message),
                        Ok(Err(error)) => Err(error.to_string()),
                        Err(error) => Err(format!("Index refresh task failed: {error}")),
                    }
                }
                Ok((message, _)) => Ok(message),
                Err(error) => Err(error),
            };
            let _ = this.update(cx, |view, cx| {
                view.history_busy = false;
                match result {
                    Ok(message) => {
                        view.history_generation = view.history_generation.wrapping_add(1);
                        view.refresh_index();
                        view.operation_status = Some(message);
                    }
                    Err(error) => view.history_error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn create_history_branch(&mut self, cx: &mut Context<Self>) {
        let name = self.history_branch_name.read(cx).value().trim().to_owned();
        if name.is_empty() {
            return;
        }
        self.run_branch_action(cx, move |service, path| {
            Box::pin(async move {
                service
                    .create_branch(&path, &name)
                    .await
                    .map(|branch| (format!("Created branch {}.", branch.name), false))
                    .map_err(|error| error.to_string())
            })
        });
    }

    pub(super) fn switch_history_branch(&mut self, name: &str, cx: &mut Context<Self>) {
        let name = name.to_owned();
        self.run_branch_action(cx, move |service, path| {
            Box::pin(async move {
                service
                    .switch_branch(&path, &name, false)
                    .await
                    .map(|name| (format!("Switched to branch {name}.",), true))
                    .map_err(|error| error.to_string())
            })
        });
    }

    pub(super) fn archive_history_branch(&mut self, name: &str, cx: &mut Context<Self>) {
        let name = name.to_owned();
        self.run_branch_action(cx, move |service, path| {
            Box::pin(async move {
                service
                    .archive_branch(&path, &name)
                    .await
                    .map(|_| (format!("Archived branch {name}."), false))
                    .map_err(|error| error.to_string())
            })
        });
    }

    pub(super) fn merge_history_branch(&mut self, name: &str, cx: &mut Context<Self>) {
        let name = name.to_owned();
        self.run_branch_action(cx, move |service, path| {
            Box::pin(async move {
                service
                    .merge_branch(&path, &name)
                    .await
                    .map(|result| {
                        let summary = if result.has_conflicts {
                            format!(
                                "Merge needs resolution: {}",
                                result.conflict_files.join(", ")
                            )
                        } else {
                            format!(
                                "Merged {} into {}.",
                                result.source_branch, result.target_branch
                            )
                        };
                        (summary, result.completed)
                    })
                    .map_err(|error| error.to_string())
            })
        });
    }

    pub(super) fn resolve_history_merge(&mut self, strategy: &str, cx: &mut Context<Self>) {
        let strategy = strategy.to_owned();
        self.run_branch_action(cx, move |service, path| {
            Box::pin(async move {
                service
                    .resolve_merge(&path, &strategy)
                    .await
                    .map(|result| {
                        let summary = if result.completed {
                            "Merge completed.".into()
                        } else {
                            "Some merge conflicts remain.".into()
                        };
                        (summary, result.completed)
                    })
                    .map_err(|error| error.to_string())
            })
        });
    }

    pub(super) fn abort_history_merge(&mut self, cx: &mut Context<Self>) {
        self.run_branch_action(cx, move |service, path| {
            Box::pin(async move {
                service
                    .abort_merge(&path)
                    .await
                    .map(|_| ("Merge aborted.".into(), false))
                    .map_err(|error| error.to_string())
            })
        });
    }
}

fn branch_section_title(title: &str) -> Div {
    div()
        .h_flex()
        .w_full()
        .items_center()
        .justify_between()
        .py(px(6.))
        .font_family(theme::SEMIBOLD_FONT_FAMILY)
        .text_size(px(14.))
        .text_color(rgb(theme::TEXT_PRIMARY))
        .child(photag_ui_bridge::i18n::tr(title))
}

fn branch_note(message: &str) -> Div {
    div()
        .w_full()
        .py(px(6.))
        .text_size(px(theme::FONT_SIZE_SMALL))
        .text_color(rgb(theme::TEXT_SECONDARY))
        .child(photag_ui_bridge::i18n::tr(message))
}
