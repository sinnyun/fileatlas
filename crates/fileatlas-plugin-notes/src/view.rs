use std::sync::Arc;

use fileatlas_ui_bridge::{
    components::{self, WorkbenchTypography as _},
    panel_catalog::PanelCatalog,
    theme,
};
use gpui_kit::{
    base::{Selectable as _, StyledExt as _},
    component::button::ButtonVariants as _,
    component::scroll::ScrollableElement as _,
    *,
};

use crate::{NoteFolder, NoteSummary, NotesService, PANEL};

#[derive(Clone, Default)]
struct NotesIndex {
    notes: Vec<NoteSummary>,
    folders: Vec<NoteFolder>,
}

fn indented(depth: usize, child: impl IntoElement) -> Div {
    div()
        .h_flex()
        .children((0..depth).map(|_| div().w(px(theme::space_expanded()))))
        .child(child)
}

pub(super) fn sidebar(
    service: Arc<NotesService>,
    panels: Arc<PanelCatalog>,
) -> fileatlas_ui_bridge::UiViewContribution {
    fileatlas_ui_bridge::UiViewContribution::new(
        "notes/sidebar-tree",
        "navigation.sidebar-notes",
        0,
        move |_, app| {
            let service = Arc::clone(&service);
            let panels = Arc::clone(&panels);
            app.new(|cx| {
                let wake = fileatlas_ui_bridge::refresh::Wake::default();
                let mut index =
                    fileatlas_ui_bridge::read_model::ReadModel::<u64, NotesIndex>::default();
                index.set_wake(wake.clone());
                let initial_service = Arc::clone(&service);
                index.request(service.revision(), move || {
                    Ok(NotesIndex {
                        notes: initial_service.list().map_err(|e| e.to_string())?,
                        folders: initial_service.list_folders().map_err(|e| e.to_string())?,
                    })
                });
                let task =
                    fileatlas_ui_bridge::refresh::listen(cx, wake, |view: &mut NotesSidebar, _| {
                        view.index.poll()
                    });
                NotesSidebar {
                    service,
                    panels,
                    index,
                    _task: task,
                }
            })
            .into()
        },
    )
}

struct NotesSidebar {
    service: Arc<NotesService>,
    panels: Arc<PanelCatalog>,
    index: fileatlas_ui_bridge::read_model::ReadModel<u64, NotesIndex>,
    _task: Task<()>,
}
impl Render for NotesSidebar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let revision = self.service.revision();
        let service = Arc::clone(&self.service);
        self.index.request(revision, move || {
            Ok(NotesIndex {
                notes: service.list().map_err(|e| e.to_string())?,
                folders: service.list_folders().map_err(|e| e.to_string())?,
            })
        });
        let index = self
            .index
            .value()
            .and_then(|v| v.as_ref().ok())
            .cloned()
            .unwrap_or_default();
        let mut rows = div().v_flex().gap(px(theme::space_compact()));
        for folder in &index.folders {
            rows = rows.child(indented(
                folder.depth,
                div().workbench_assistant().child(folder.name.clone()),
            ));
            for note in index.notes.iter().filter(|note| {
                note.path.starts_with(&format!("{}/", folder.path))
                    && note.path[folder.path.len() + 1..].find('/').is_none()
            }) {
                let path = note.path.clone();
                let title = note.title.clone();
                rows = rows.child(indented(
                    folder.depth + 1,
                    components::button(format!("note-row-{}", note.id))
                        .ghost()
                        .label(title)
                        .on_click(cx.listener(move |this, _, _, _| {
                            this.service.select(&path);
                            if let Err(error) = this.panels.open(PANEL) {
                                tracing::error!(%error, "could not open notes panel");
                            }
                        })),
                ));
            }
        }
        for note in index.notes.iter().filter(|note| !note.path.contains('/')) {
            let path = note.path.clone();
            let title = note.title.clone();
            rows = rows.child(indented(
                0,
                components::button(format!("note-row-{}", note.id))
                    .ghost()
                    .label(title.clone())
                    .on_click(cx.listener(move |this, _, _, _| {
                        this.service.select(&path);
                        if let Err(error) = this.panels.open(PANEL) {
                            tracing::error!(%error, "could not open notes panel");
                        }
                    })),
            ));
        }
        div()
            .v_flex()
            .size_full()
            .min_w_0()
            .gap(px(theme::space_standard()))
            .px(px(theme::space_expanded()))
            .py(px(theme::space_expanded()))
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .child(div().workbench_title().child("笔记")),
            )
            .child(
                components::button("notes-open-panel")
                    .label("打开笔记面板")
                    .on_click(cx.listener(|this, _, _, _| {
                        if let Err(error) = this.panels.open(PANEL) {
                            tracing::error!(%error, "could not open notes panel");
                        }
                    })),
            )
            .child(rows.overflow_y_scrollbar())
    }
}

pub(super) fn panel(service: Arc<NotesService>, window: &mut Window, app: &mut App) -> AnyView {
    let notes = service.list().unwrap_or_default();
    let selected = service
        .selected()
        .or_else(|| notes.first().map(|n| n.path.clone()));
    let document = selected.as_deref().and_then(|id| service.read(id).ok());
    let body = document
        .as_ref()
        .map(|document| document.body.clone())
        .unwrap_or_default();
    let editor = app.new(|cx| {
        gpui_kit::component::input::TextareaState::new(window, cx)
            .placeholder("用 Markdown 记录想法、任务和链接")
    });
    editor.update(app, |state, cx| state.set_value(body, window, cx));
    let title_input =
        app.new(|cx| gpui_kit::component::input::InputState::new(window, cx).placeholder("标题"));
    let category_input =
        app.new(|cx| gpui_kit::component::input::InputState::new(window, cx).placeholder("分类"));
    let status_input = app.new(|cx| {
        gpui_kit::component::input::InputState::new(window, cx)
            .placeholder("draft / in_progress / completed / archived")
    });
    let tags_input = app.new(|cx| {
        gpui_kit::component::input::InputState::new(window, cx).placeholder("用逗号分隔标签")
    });
    if let Some(document) = document.as_ref() {
        title_input.update(app, |state, cx| {
            state.set_value(document.summary.title.clone(), window, cx)
        });
        category_input.update(app, |state, cx| {
            state.set_value(document.summary.category.clone(), window, cx)
        });
        status_input.update(app, |state, cx| {
            state.set_value(document.summary.status.clone(), window, cx)
        });
        tags_input.update(app, |state, cx| {
            state.set_value(document.summary.tags.join(", "), window, cx)
        });
    }
    let preview_markdown = selected
        .as_deref()
        .and_then(|id| service.rendered_markdown(id).ok())
        .unwrap_or_default();
    app.new(|cx| NotesPanel {
        service,
        notes,
        selected,
        editor,
        title_input,
        category_input,
        status_input,
        tags_input,
        preview: false,
        preview_markdown,
        backlinks: document
            .map(|document| document.backlinks)
            .unwrap_or_default(),
        message: None,
        _focus: cx.focus_handle(),
    })
    .into()
}

struct NotesPanel {
    service: Arc<NotesService>,
    notes: Vec<NoteSummary>,
    selected: Option<String>,
    editor: Entity<gpui_kit::component::input::TextareaState>,
    title_input: Entity<gpui_kit::component::input::InputState>,
    category_input: Entity<gpui_kit::component::input::InputState>,
    status_input: Entity<gpui_kit::component::input::InputState>,
    tags_input: Entity<gpui_kit::component::input::InputState>,
    preview: bool,
    preview_markdown: String,
    backlinks: Vec<NoteSummary>,
    message: Option<String>,
    _focus: FocusHandle,
}
impl NotesPanel {
    fn select(&mut self, path: String, window: &mut Window, cx: &mut Context<Self>) {
        self.selected = Some(path.clone());
        self.service.select(&path);
        let document = self.service.read(&path).ok();
        let body = document
            .as_ref()
            .map(|doc| doc.body.clone())
            .unwrap_or_default();
        self.editor
            .update(cx, |state, cx| state.set_value(body, window, cx));
        if let Some(document) = document {
            self.title_input.update(cx, |state, cx| {
                state.set_value(document.summary.title, window, cx)
            });
            self.category_input.update(cx, |state, cx| {
                state.set_value(document.summary.category, window, cx)
            });
            self.status_input.update(cx, |state, cx| {
                state.set_value(document.summary.status, window, cx)
            });
            self.tags_input.update(cx, |state, cx| {
                state.set_value(document.summary.tags.join(", "), window, cx)
            });
            self.backlinks = document.backlinks;
        }
        self.preview_markdown = self.service.rendered_markdown(&path).unwrap_or_default();
        cx.notify();
    }
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self.selected.clone() else {
            return;
        };
        let title = self.title_input.read(cx).value().to_string();
        let content = self.editor.read(cx).value().to_string();
        let category = self.category_input.read(cx).value().to_string();
        let status = self.status_input.read(cx).value().to_string();
        let tags = self
            .tags_input
            .read(cx)
            .value()
            .split(',')
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let current = self.notes.iter().find(|note| note.path == path).cloned();
        let result = (|| {
            let summary = if let Some(note) = current.filter(|note| note.title != title.trim()) {
                self.service.rename(&note.id, &title)?
            } else {
                self.service.read(&path)?.summary
            };
            self.service.update(
                &summary.id,
                &content,
                Some(&category),
                Some(&status),
                Some(tags),
            )
        })();
        match result {
            Ok(note) => {
                self.selected = Some(note.path.clone());
                self.service.select(&note.path);
                self.title_input
                    .update(cx, |state, cx| state.set_value(note.title, window, cx));
                self.preview_markdown =
                    self.service.rendered_markdown(&note.id).unwrap_or_default();
                self.notes = self.service.list().unwrap_or_default();
                self.backlinks = self.service.backlinks(&note.id).unwrap_or_default();
                self.message = Some("已保存".into());
            }
            Err(error) => self.message = Some(format!("保存失败：{error}")),
        }
        cx.notify();
    }
    fn create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let title = format!("新笔记 {}", self.notes.len() + 1);
        match self.service.create(&title, None, Some("# 新笔记\n\n")) {
            Ok(note) => {
                self.notes = self.service.list().unwrap_or_default();
                self.select(note.path, window, cx);
                self.title_input
                    .update(cx, |state, cx| state.set_value(note.title, window, cx));
                self.message = Some("笔记已创建".into());
            }
            Err(error) => self.message = Some(format!("创建失败：{error}")),
        }
        cx.notify();
    }

    fn insert_image(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selected.clone() else {
            return;
        };
        let service = Arc::clone(&self.service);
        cx.spawn(async move |this, cx| {
            let Some(file) = rfd::AsyncFileDialog::new().pick_file().await else {
                return;
            };
            let source = file.path().to_path_buf();
            let result = tokio::task::spawn_blocking(move || service.insert_image(&id, &source))
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result.map_err(|error| error.to_string()));
            let _ = this.update_in(cx, |view: &mut NotesPanel, window, cx| {
                match result {
                    Ok(markdown) => {
                        let current = view.editor.read(cx).value().to_string();
                        view.editor.update(cx, |state, cx| {
                            state.set_value(
                                format!("{}\n{}\n", current.trim_end(), markdown),
                                window,
                                cx,
                            )
                        });
                        view.message = Some("图片已插入，保存后写入笔记".into());
                    }
                    Err(error) => view.message = Some(format!("插入图片失败：{error}")),
                }
                cx.notify();
            });
        })
        .detach();
    }
}
impl Render for NotesPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut list = div()
            .v_flex()
            .w(px(240.))
            .min_w(px(180.))
            .h_full()
            .overflow_y_scrollbar()
            .gap(px(theme::space_compact()));
        for note in &self.notes {
            let path = note.path.clone();
            let selected = self.selected.as_deref() == Some(path.as_str());
            list = list.child(
                components::button(format!("notes-select-{}", note.id))
                    .ghost()
                    .selected(selected)
                    .label(note.title.clone())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.select(path.clone(), window, cx)
                    })),
            );
        }
        let editor: AnyElement = if self.preview {
            gpui_kit::component::text::markdown(self.preview_markdown.clone()).into_any_element()
        } else {
            components::textarea(&self.editor)
                .h_full()
                .into_any_element()
        };
        div()
            .id("notes-panel")
            .h_flex()
            .size_full()
            .min_w_0()
            .min_h_0()
            .gap(px(theme::space_expanded()))
            .p(px(theme::space_expanded()))
            .child(list)
            .child(fileatlas_ui_bridge::components::vertical_divider())
            .child(
                div()
                    .v_flex()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .gap(px(theme::space_standard()))
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap(px(theme::space_compact()))
                            .child(components::text_input(&self.title_input).w(px(280.)))
                            .child(div().flex_1())
                            .child(
                                self.message
                                    .clone()
                                    .unwrap_or_else(|| "Markdown · 本地文件".into()),
                            )
                            .child(components::button("notes-save").label("保存").on_click(
                                cx.listener(|this, _, window, cx| this.save(window, cx)),
                            )),
                    )
                    .child(div().flex_1().min_h_0().child(editor))
                    .child(
                        div()
                            .h_flex()
                            .gap(px(theme::space_compact()))
                            .child(
                                components::button("notes-create")
                                    .label("新建笔记")
                                    .on_click(
                                        cx.listener(|this, _, window, cx| this.create(window, cx)),
                                    ),
                            )
                            .child(
                                components::button("notes-image")
                                    .ghost()
                                    .label("插入图片")
                                    .on_click(cx.listener(|this, _, _, cx| this.insert_image(cx))),
                            )
                            .child(
                                components::button("notes-preview")
                                    .ghost()
                                    .label(if self.preview { "编辑" } else { "预览" })
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        if !this.preview {
                                            this.save(window, cx);
                                        }
                                        this.preview = !this.preview;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(fileatlas_ui_bridge::components::vertical_divider())
            .child(
                div()
                    .v_flex()
                    .w(px(220.))
                    .min_w(px(180.))
                    .h_full()
                    .gap(px(theme::space_compact()))
                    .child(div().workbench_title().child("属性"))
                    .child(components::text_input(&self.category_input))
                    .child(components::text_input(&self.status_input))
                    .child(components::text_input(&self.tags_input))
                    .child(div().workbench_assistant().child("反向链接"))
                    .children(
                        self.backlinks
                            .iter()
                            .map(|note| div().workbench_assistant().child(note.title.clone())),
                    ),
            )
    }
}
