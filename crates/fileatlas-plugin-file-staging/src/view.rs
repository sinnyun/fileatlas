use crate::{FileStagingService, PANEL, StagedFile};
use ::gpui::AnyElement;
use fileatlas_contracts::FileStagingDragPayload;
use fileatlas_plugin_file_operations::FileOperationsService;
use fileatlas_ui_bridge::{components::WorkbenchStyle as _, panel_catalog::PanelCatalog, theme};
use gpui_kit::{
    base::{Disableable as _, StyledExt as _},
    component::button::{Button, ButtonVariants as _},
    prelude::FluentBuilder as _,
    *,
};
use std::{collections::HashSet, sync::Arc};

pub(super) fn sidebar_button(panels: Arc<PanelCatalog>) -> fileatlas_ui_bridge::UiViewContribution {
    fileatlas_ui_bridge::UiViewContribution::new(
        "file-staging/sidebar-view",
        "navigation.launcher-shortcuts",
        8,
        move |_, app| {
            let panels = Arc::clone(&panels);
            app.new(|_| StagingButton { panels }).into()
        },
    )
}
struct StagingButton {
    panels: Arc<PanelCatalog>,
}
impl Render for StagingButton {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("file-staging-open")
            .v_flex()
            .w(px(42.))
            .h(px(54.))
            .items_center()
            .justify_center()
            .gap(px(4.))
            .rounded(px(6.))
            .text_xs()
            .text_color(rgb(theme::text_secondary()))
            .hover(|s| s.bg(rgba(theme::fill_hover())))
            .on_click(cx.listener(|this, _, _, _| {
                if let Err(error) = this.panels.open(PANEL) {
                    tracing::error!(%error, "could not open file staging basket");
                }
            }))
            .child(theme::icon(gpui_kit::assets::IconName::Briefcase))
            .child("暂存篮")
            .into_any_element()
    }
}

pub(super) fn panel(
    service: Arc<FileStagingService>,
    operations: Arc<FileOperationsService>,
    _window: &mut Window,
    app: &mut App,
) -> AnyView {
    let files = service.snapshot();
    let revision = service.revision();
    let wake_service = Arc::clone(&service);
    app.new(|cx| {
        let wake_task = fileatlas_ui_bridge::refresh::listen(
            cx,
            wake_service.wake(),
            |view: &mut StagingPanel, _| {
                let revision = view.service.revision();
                if revision == view.revision {
                    return false;
                }
                let files = view.service.snapshot();
                let changed = files != view.files;
                view.revision = revision;
                if changed {
                    view.files = files;
                    view.selected_paths
                        .retain(|path| view.files.iter().any(|file| file.path == *path));
                }
                changed
            },
        );
        StagingPanel {
            service,
            operations,
            files,
            revision,
            busy: false,
            message: None,
            selected_paths: HashSet::new(),
            _wake_task: wake_task,
        }
    })
    .into()
}
struct StagingPanel {
    service: Arc<FileStagingService>,
    operations: Arc<FileOperationsService>,
    files: Vec<StagedFile>,
    revision: u64,
    busy: bool,
    message: Option<String>,
    selected_paths: HashSet<String>,
    _wake_task: gpui::Task<()>,
}
impl StagingPanel {
    fn add_dragged_paths(&mut self, paths: Vec<String>, cx: &mut Context<Self>) {
        if self.busy || paths.is_empty() {
            return;
        }
        self.busy = true;
        self.message = None;
        cx.notify();
        let service = Arc::clone(&self.service);
        let worker = cx.background_executor().spawn(async move {
            service.add_paths(paths.into_iter().map(std::path::PathBuf::from))
        });
        cx.spawn(async move |this, cx| {
            let result = worker.await.map_err(|error| error.to_string());
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                match result {
                    Ok(files) => {
                        let added = files.len().saturating_sub(view.files.len());
                        view.files = files;
                        view.revision = view.service.revision();
                        view.message = Some(format!("已加入 {added} 个文件"));
                    }
                    Err(error) => view.message = Some(format!("拖入失败：{error}")),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn toggle_selection(&mut self, path: String, control: bool, cx: &mut Context<Self>) {
        if control {
            if !self.selected_paths.insert(path.clone()) {
                self.selected_paths.remove(&path);
            }
        } else if self.selected_paths.len() == 1 && self.selected_paths.contains(&path) {
            // Keep a selected card selected on a plain click, matching the
            // primary file browser's click behavior.
        } else {
            self.selected_paths.clear();
            self.selected_paths.insert(path);
        }
        cx.notify();
    }

    fn add_files(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.message = None;
        cx.notify();
        let service = Arc::clone(&self.service);
        cx.spawn(async move |this, cx| {
            let picked = rfd::AsyncFileDialog::new()
                .pick_files()
                .await
                .map(|files| {
                    files
                        .into_iter()
                        .map(|f| f.path().to_path_buf())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let result = if picked.is_empty() {
                None
            } else {
                Some(
                    tokio::task::spawn_blocking(move || service.add_paths(picked))
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|result| result.map_err(|e| e.to_string())),
                )
            };
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                match result {
                    Some(Ok(files)) => {
                        view.files = files;
                        view.selected_paths
                            .retain(|path| view.files.iter().any(|file| file.path == *path));
                        view.message = Some("文件已加入暂存篮".into());
                    }
                    Some(Err(e)) => view.message = Some(format!("添加失败：{e}")),
                    None => {}
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn remove(&mut self, path: String, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        cx.notify();
        let service = Arc::clone(&self.service);
        cx.spawn(async move |this, cx| {
            let result = tokio::task::spawn_blocking(move || service.remove(&path)).await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                match result {
                    Ok(Ok(files)) => view.files = files,
                    Ok(Err(e)) => view.message = Some(format!("移除失败：{e}")),
                    Err(e) => view.message = Some(e.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn transfer(&mut self, move_files: bool, cx: &mut Context<Self>) {
        if self.busy || self.files.is_empty() {
            return;
        }
        self.busy = true;
        self.message = Some("请选择目标文件夹…".into());
        cx.notify();
        let paths = self
            .files
            .iter()
            .filter(|file| {
                (self.selected_paths.is_empty() || self.selected_paths.contains(&file.path))
                    && file.available
            })
            .map(|f| std::path::PathBuf::from(&f.path))
            .collect::<Vec<_>>();
        let operations = Arc::clone(&self.operations);
        let service = Arc::clone(&self.service);
        cx.spawn(async move |this, cx| {
            let folder = rfd::AsyncFileDialog::new()
                .pick_folder()
                .await
                .map(|f| f.path().to_path_buf());
            let result = if let Some(folder) = folder {
                if move_files {
                    operations.move_paths_to_folder(paths, folder).await
                } else {
                    operations.copy_paths_to_folder(paths, folder).await
                }
                .map(Some)
            } else {
                Ok(None)
            };
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                match result {
                    Ok(Some(outcomes)) => {
                        let completed = outcomes
                            .iter()
                            .filter(|o| o.completed)
                            .map(|o| o.source.to_string_lossy().into_owned())
                            .collect::<Vec<_>>();
                        for path in completed {
                            if let Ok(files) = service.remove(&path) {
                                view.files = files;
                                view.selected_paths.remove(&path);
                            }
                        }
                        let failed = outcomes.iter().filter(|o| !o.completed).count();
                        let details = outcomes
                            .iter()
                            .filter(|o| !o.completed)
                            .take(3)
                            .map(|o| {
                                format!(
                                    "{}：{}",
                                    o.source.display(),
                                    o.error.as_deref().unwrap_or("未完成")
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("；");
                        view.message = Some(if details.is_empty() {
                            format!("已完成 {} 项", outcomes.len())
                        } else {
                            format!(
                                "已完成 {} 项，失败或跳过 {} 项：{}",
                                outcomes.len().saturating_sub(failed),
                                failed,
                                details
                            )
                        });
                    }
                    Ok(None) => view.message = None,
                    Err(e) => view.message = Some(format!("操作失败：{e}")),
                }
                cx.notify();
            });
        })
        .detach();
    }
}
impl Render for StagingPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let files = self.files.clone();
        let mut list = div()
            .id("staging-file-list")
            .h_flex()
            .flex_wrap()
            .flex_1()
            .min_h_0()
            .gap(px(10.))
            .overflow_y_scroll()
            .p(px(12.));
        if files.is_empty() {
            let empty = div()
                .workbench_section()
                .w_full()
                .flex_1()
                .min_h(px(180.))
                .v_flex()
                .items_center()
                .justify_center()
                .gap(px(8.))
                .child(
                    theme::icon(gpui_kit::assets::IconName::File)
                        .text_color(rgb(theme::text_muted())),
                )
                .child("暂存篮为空")
                .child("从文件列表添加文件；可拖到文件夹直接移动。");
            list = list.child(empty);
        } else {
            for (index, item) in files.into_iter().enumerate() {
                let path = std::path::PathBuf::from(&item.path);
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| item.path.clone());
                let available = item.available;
                let selected = self.selected_paths.contains(&item.path);
                let source_dir = path
                    .parent()
                    .map(|parent| parent.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let thumbnail_path = path.clone();
                let image_file = is_image_file(&path);
                let drag_paths = if selected {
                    self.files
                        .iter()
                        .filter(|file| file.available && self.selected_paths.contains(&file.path))
                        .map(|file| file.path.clone())
                        .collect::<Vec<_>>()
                } else if available {
                    vec![item.path.clone()]
                } else {
                    Vec::new()
                };
                let drag_payload = FileStagingDragPayload { paths: drag_paths };
                let selection_path = item.path.clone();
                let remove_path = item.path.clone();
                let card = div()
                    .id(format!("staging-card-{index}"))
                    .v_flex()
                    .w(px(220.))
                    .min_w(px(180.))
                    .gap(px(6.))
                    .workbench_card(selected)
                    .p(px(8.))
                    .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                        this.toggle_selection(
                            selection_path.clone(),
                            event.modifiers().control,
                            cx,
                        );
                    }))
                    .on_drag(drag_payload, |payload, _, _, cx| {
                        cx.new(|_| StagingDragGhost {
                            count: payload.paths.len(),
                        })
                    });
                let thumbnail: AnyElement = if available && image_file {
                    img(thumbnail_path)
                        .w_full()
                        .h(px(136.))
                        .object_fit(ObjectFit::Cover)
                        .with_fallback(|| file_thumbnail_placeholder().into_any_element())
                        .into_any_element()
                } else {
                    file_thumbnail_placeholder().into_any_element()
                };
                let card = card
                    .child(
                        div()
                            .relative()
                            .w_full()
                            .h(px(136.))
                            .rounded(px(theme::THUMB_ROUND))
                            .overflow_hidden()
                            .bg(rgb(theme::surface_faint()))
                            .child(thumbnail)
                            .when(!available, |tile| {
                                tile.child(
                                    div()
                                        .absolute()
                                        .inset_0()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .bg(rgba(theme::panel_rgba(theme::surface_card())))
                                        .text_xs()
                                        .text_color(rgb(theme::danger()))
                                        .child("文件不可用"),
                                )
                            }),
                    )
                    .child(div().w_full().truncate().child(name))
                    .child(
                        div()
                            .w_full()
                            .truncate()
                            .text_xs()
                            .text_color(rgb(theme::text_muted()))
                            .child(source_dir),
                    )
                    .child(
                        Button::new(format!("staging-remove-{index}"))
                            .ghost()
                            .compact()
                            .label("移除")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.remove(remove_path.clone(), cx)
                            })),
                    );
                list = list.child(card);
            }
        }
        let invalid = self.files.iter().filter(|f| !f.available).count();
        div()
            .id("file-staging-panel")
            .drag_over::<fileatlas_contracts::FileManagerFileDragPayload>(|style, _, _, _| {
                style.border_color(rgb(theme::accent_hover()))
            })
            .on_drop::<fileatlas_contracts::FileManagerFileDragPayload>(cx.listener(
                |this, payload: &fileatlas_contracts::FileManagerFileDragPayload, _, cx| {
                    if payload.allows_drop() {
                        this.add_dragged_paths(payload.paths.clone(), cx);
                    }
                },
            ))
            .v_flex()
            .size_full()
            .min_w_0()
            .min_h_0()
            .gap(px(10.))
            .p(px(16.))
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap(px(8.))
                    .child(div().text_lg().child("文件暂存篮"))
                    .child(
                        div()
                            .text_color(rgb(theme::text_muted()))
                            .child(format!("{} 个文件", self.files.len())),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("staging-refresh-status")
                            .ghost()
                            .label("刷新状态")
                            .disabled(self.busy || self.files.is_empty())
                            .on_click(cx.listener(|this, _, _, cx| {
                                if this.busy {
                                    return;
                                }
                                this.busy = true;
                                cx.notify();
                                let service = Arc::clone(&this.service);
                                cx.spawn(async move |this, cx| {
                                    let result = tokio::task::spawn_blocking(move || {
                                        service.refresh_status()
                                    })
                                    .await;
                                    let _ = this.update(cx, |view, cx| {
                                        view.busy = false;
                                        match result {
                                            Ok(Ok(files)) => view.files = files,
                                            Ok(Err(e)) => view.message = Some(e.to_string()),
                                            Err(e) => view.message = Some(e.to_string()),
                                        }
                                        cx.notify();
                                    });
                                })
                                .detach();
                            })),
                    )
                    .child(
                        Button::new("staging-add")
                            .outline()
                            .label(if self.busy {
                                "处理中…"
                            } else {
                                "添加文件…"
                            })
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, _, cx| this.add_files(cx))),
                    )
                    .child(
                        Button::new("staging-clear")
                            .ghost()
                            .label("清空")
                            .disabled(self.busy || self.files.is_empty())
                            .on_click(cx.listener(|this, _, _, cx| {
                                if this.busy {
                                    return;
                                }
                                this.busy = true;
                                cx.notify();
                                let service = Arc::clone(&this.service);
                                cx.spawn(async move |this, cx| {
                                    let result =
                                        tokio::task::spawn_blocking(move || service.clear()).await;
                                    let _ = this.update(cx, |view, cx| {
                                        view.busy = false;
                                        match result {
                                            Ok(Ok(files)) => view.files = files,
                                            Ok(Err(e)) => view.message = Some(e.to_string()),
                                            Err(e) => view.message = Some(e.to_string()),
                                        }
                                        cx.notify();
                                    });
                                })
                                .detach();
                            })),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(theme::text_muted()))
                    .child("暂存项独立于当前窗格和工作区；仅保存路径，不复制文件内容。"),
            )
            .child(list)
            .child(
                div()
                    .h_flex()
                    .w_full()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(rgb(theme::text_secondary()))
                            .child(self.message.clone().unwrap_or_else(|| {
                                if self.selected_paths.is_empty() {
                                    format!("{} 个失效项；拖到文件夹即可移动", invalid)
                                } else {
                                    format!(
                                        "已选择 {} 项；拖到文件夹即可移动（{} 个失效项）",
                                        self.selected_paths.len(),
                                        invalid
                                    )
                                }
                            })),
                    )
                    .child(
                        Button::new("staging-prune")
                            .outline()
                            .label(format!("移除失效项 ({invalid})"))
                            .disabled(self.busy || invalid == 0)
                            .on_click(cx.listener(|this, _, _, cx| {
                                if this.busy {
                                    return;
                                }
                                this.busy = true;
                                cx.notify();
                                let service = Arc::clone(&this.service);
                                cx.spawn(async move |this, cx| {
                                    let result = tokio::task::spawn_blocking(move || {
                                        service.remove_missing()
                                    })
                                    .await;
                                    let _ = this.update(cx, |view, cx| {
                                        view.busy = false;
                                        match result {
                                            Ok(Ok(files)) => {
                                                view.files = files;
                                                view.message = Some("失效项已移除".into());
                                            }
                                            Ok(Err(e)) => view.message = Some(e.to_string()),
                                            Err(e) => view.message = Some(e.to_string()),
                                        }
                                        cx.notify();
                                    });
                                })
                                .detach();
                            })),
                    )
                    .child(
                        Button::new("staging-copy")
                            .outline()
                            .label("复制到…")
                            .disabled(self.busy || self.files.is_empty())
                            .on_click(cx.listener(|this, _, _, cx| this.transfer(false, cx))),
                    )
                    .child(
                        Button::new("staging-move")
                            .primary()
                            .label("移动到…")
                            .disabled(self.busy || self.files.is_empty())
                            .on_click(cx.listener(|this, _, _, cx| this.transfer(true, cx))),
                    ),
            )
    }
}

struct StagingDragGhost {
    count: usize,
}

impl Render for StagingDragGhost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .h_flex()
            .items_center()
            .gap(px(8.))
            .workbench_card(true)
            .p(px(10.))
            .child(theme::icon(gpui_kit::assets::IconName::File))
            .child(format!("移动 {} 个文件", self.count))
    }
}

fn is_image_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "avif" | "bmp" | "gif" | "jpeg" | "jpg" | "png" | "tif" | "tiff" | "webp"
            )
        })
}

fn file_thumbnail_placeholder() -> impl IntoElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .text_color(rgb(theme::text_muted()))
        .child(theme::icon(gpui_kit::assets::IconName::File))
}
