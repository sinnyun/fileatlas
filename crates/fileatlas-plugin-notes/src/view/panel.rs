use std::sync::Arc;

use fileatlas_ui_bridge::{
    components::{self, WorkbenchTypography as _},
    theme,
};
use gpui_kit::{
    base::{Selectable as _, StyledExt as _},
    component::button::ButtonVariants as _,
    component::scroll::ScrollableElement as _,
    *,
};

use crate::{
    NoteSummary, NoteTrashEntry, NotesService,
    editor::commands::{prefix_lines, replace_all, toggle_task_lines, wrap_selection},
};
pub fn panel(service: Arc<NotesService>, window: &mut Window, app: &mut App) -> AnyView {
    let notes = service.list().unwrap_or_default();
    let trash_mode = service.selected().as_deref() == Some(".trash");
    let selected = if trash_mode {
        None
    } else {
        service
            .selected()
            .or_else(|| notes.first().map(|n| n.path.clone()))
    };
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
    let move_input = app.new(|cx| {
        gpui_kit::component::input::InputState::new(window, cx)
            .placeholder("目标目录路径；留空放到根目录")
    });
    let find_input =
        app.new(|cx| gpui_kit::component::input::InputState::new(window, cx).placeholder("查找"));
    let replace_input =
        app.new(|cx| gpui_kit::component::input::InputState::new(window, cx).placeholder("替换为"));
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
    let trash_entries = service.list_trash().unwrap_or_default();
    app.new(|cx| NotesPanel {
        service,
        notes,
        selected,
        editor,
        title_input,
        category_input,
        status_input,
        tags_input,
        move_input,
        find_input,
        replace_input,
        trash_mode,
        trash_entries,
        confirm_delete: None,
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
    move_input: Entity<gpui_kit::component::input::InputState>,
    find_input: Entity<gpui_kit::component::input::InputState>,
    replace_input: Entity<gpui_kit::component::input::InputState>,
    trash_mode: bool,
    trash_entries: Vec<NoteTrashEntry>,
    confirm_delete: Option<String>,
    preview: bool,
    preview_markdown: String,
    backlinks: Vec<NoteSummary>,
    message: Option<String>,
    _focus: FocusHandle,
}
impl NotesPanel {
    fn select(&mut self, path: String, window: &mut Window, cx: &mut Context<Self>) {
        self.trash_mode = false;
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

    fn format_inline(
        &mut self,
        prefix: &'static str,
        suffix: &'static str,
        placeholder: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (text, selection) = {
            let editor = self.editor.read(cx);
            (editor.value().to_string(), editor.selected_range())
        };
        let (updated, next_selection) =
            wrap_selection(&text, selection, prefix, suffix, placeholder);
        self.editor.update(cx, |editor, cx| {
            editor.replace_all(updated, window, cx);
            editor.set_selected_range(next_selection, cx);
            editor.focus(window, cx);
        });
        self.message = Some("格式已插入，保存后写入笔记".into());
        cx.notify();
    }

    fn format_lines(&mut self, prefix: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        let (text, selection) = {
            let editor = self.editor.read(cx);
            (editor.value().to_string(), editor.selected_range())
        };
        let (updated, next_selection) = prefix_lines(&text, selection, prefix);
        self.editor.update(cx, |editor, cx| {
            editor.replace_all(updated, window, cx);
            editor.set_selected_range(next_selection, cx);
            editor.focus(window, cx);
        });
        self.message = Some("列表格式已切换，保存后写入笔记".into());
        cx.notify();
    }

    fn toggle_tasks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (text, selection) = {
            let editor = self.editor.read(cx);
            (editor.value().to_string(), editor.selected_range())
        };
        let (updated, next_selection) = toggle_task_lines(&text, selection);
        self.editor.update(cx, |editor, cx| {
            editor.replace_all(updated, window, cx);
            editor.set_selected_range(next_selection, cx);
            editor.focus(window, cx);
        });
        self.message = Some("任务状态已切换，保存后写入笔记".into());
        cx.notify();
    }

    fn replace_matches(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let query = self.find_input.read(cx).value().to_string();
        let replacement = self.replace_input.read(cx).value().to_string();
        let text = self.editor.read(cx).value().to_string();
        let (updated, count) = replace_all(&text, &query, &replacement);
        if count > 0 {
            let cursor = updated.len();
            self.editor.update(cx, |editor, cx| {
                editor.replace_all(updated, window, cx);
                editor.set_selected_range(cursor..cursor, cx);
                editor.focus(window, cx);
            });
            self.message = Some(format!("已替换 {count} 处，保存后写入笔记"));
        } else {
            self.message = Some(if query.is_empty() {
                "请输入要查找的文本".into()
            } else {
                "没有找到匹配文本".into()
            });
        }
        cx.notify();
    }
    fn create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let title = format!("新笔记 {}", self.notes.len() + 1);
        match self.service.create(
            &title,
            self.service.selected_folder().as_deref(),
            Some("# 新笔记\n\n"),
        ) {
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

    fn move_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self.selected.clone() else {
            return;
        };
        let destination = self.move_input.read(cx).value().trim().to_owned();
        match self.service.move_note(
            &path,
            (!destination.is_empty()).then_some(destination.as_str()),
        ) {
            Ok(note) => {
                self.notes = self.service.list().unwrap_or_default();
                self.select(note.path.clone(), window, cx);
                self.message = Some("笔记已移动".into());
            }
            Err(error) => self.message = Some(format!("移动失败：{error}")),
        }
        cx.notify();
    }

    fn trash_selected(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.selected.clone() else {
            return;
        };
        match self.service.trash(&path) {
            Ok(_) => {
                self.notes = self.service.list().unwrap_or_default();
                self.selected = None;
                self.message = Some("笔记已移入回收站".into());
            }
            Err(error) => self.message = Some(format!("移入回收站失败：{error}")),
        }
        cx.notify();
    }

    fn refresh_trash(&mut self) {
        self.trash_entries = self.service.list_trash().unwrap_or_default();
    }

    fn show_trash(&mut self, cx: &mut Context<Self>) {
        self.trash_mode = true;
        self.service.select(".trash");
        self.refresh_trash();
        cx.notify();
    }

    fn show_notes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.trash_mode = false;
        if let Some(note) = self.notes.first().cloned() {
            self.select(note.path, window, cx);
        } else {
            self.service.select("");
            self.selected = None;
            cx.notify();
        }
    }

    fn restore_trash(
        &mut self,
        entry: NoteTrashEntry,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let result = if entry.is_folder {
            self.service.restore_folder(&entry.path, None).map(|_| ())
        } else {
            self.service.restore(&entry.path, None).map(|_| ())
        };
        match result {
            Ok(()) => self.message = Some(format!("已恢复：{}", entry.name)),
            Err(error) => self.message = Some(format!("恢复失败：{error}")),
        }
        self.refresh_trash();
        self.notes = self.service.list().unwrap_or_default();
        cx.notify();
        let _ = window;
    }

    fn delete_trash(&mut self, path: String, cx: &mut Context<Self>) {
        if self.confirm_delete.as_deref() != Some(path.as_str()) {
            self.confirm_delete = Some(path);
            self.message = Some("再次点击永久删除以确认".into());
            cx.notify();
            return;
        }
        match self.service.delete_trash(&path) {
            Ok(()) => self.message = Some("已永久删除".into()),
            Err(error) => self.message = Some(format!("删除失败：{error}")),
        }
        self.confirm_delete = None;
        self.refresh_trash();
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
        if self.trash_mode {
            let mut rows = div().v_flex().gap(px(theme::space_compact()));
            for entry in self.trash_entries.clone() {
                let restore = entry.clone();
                let path = entry.path.clone();
                let confirmed = self.confirm_delete.as_deref() == Some(path.as_str());
                rows = rows.child(
                    div()
                        .h_flex()
                        .items_center()
                        .gap(px(theme::space_compact()))
                        .child(
                            div()
                                .flex_1()
                                .child(format!("{} · {} 篇", entry.name, entry.note_count)),
                        )
                        .child(
                            components::button(format!("notes-restore-{}", entry.name))
                                .ghost()
                                .label("恢复")
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.restore_trash(restore.clone(), window, cx)
                                })),
                        )
                        .child(
                            components::button(format!("notes-delete-{}", entry.name))
                                .ghost()
                                .label(if confirmed {
                                    "确认永久删除"
                                } else {
                                    "永久删除"
                                })
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.delete_trash(path.clone(), cx)
                                })),
                        ),
                );
            }
            return div()
                .id("notes-trash-panel")
                .v_flex()
                .size_full()
                .min_h_0()
                .gap(px(theme::space_standard()))
                .p(px(theme::space_expanded()))
                .child(
                    div()
                        .h_flex()
                        .items_center()
                        .child(div().workbench_title().child("笔记回收站"))
                        .child(div().flex_1())
                        .child(
                            components::button("notes-trash-back")
                                .ghost()
                                .label("返回笔记")
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.show_notes(window, cx)),
                                ),
                        )
                        .child(
                            components::button("notes-trash-refresh")
                                .ghost()
                                .label("刷新")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.refresh_trash();
                                    cx.notify();
                                })),
                        ),
                )
                .child(div().workbench_assistant().child(
                    "恢复会将笔记或整个目录放回根目录；永久删除会同时清理该笔记的图片资源。",
                ))
                .child(rows.overflow_y_scrollbar())
                .children(
                    self.message
                        .clone()
                        .map(|message| div().workbench_assistant().child(message)),
                );
        }
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
                    .children((!self.preview).then(|| {
                        div()
                            .h_flex()
                            .gap(px(theme::space_compact()))
                            .child(
                                components::button("notes-format-heading")
                                    .ghost()
                                    .label("标题")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.format_lines("# ", window, cx)
                                    })),
                            )
                            .child(
                                components::button("notes-format-bold")
                                    .ghost()
                                    .label("粗体")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.format_inline("**", "**", "粗体文字", window, cx)
                                    })),
                            )
                            .child(
                                components::button("notes-format-italic")
                                    .ghost()
                                    .label("斜体")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.format_inline("*", "*", "斜体文字", window, cx)
                                    })),
                            )
                            .child(
                                components::button("notes-format-code")
                                    .ghost()
                                    .label("代码")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.format_inline("`", "`", "代码", window, cx)
                                    })),
                            )
                            .child(
                                components::button("notes-format-link")
                                    .ghost()
                                    .label("链接")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.format_inline(
                                            "[",
                                            "](https://)",
                                            "链接文字",
                                            window,
                                            cx,
                                        )
                                    })),
                            )
                            .child(
                                components::button("notes-format-list")
                                    .ghost()
                                    .label("无序列表")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.format_lines("- ", window, cx)
                                    })),
                            )
                            .child(
                                components::button("notes-format-task")
                                    .ghost()
                                    .label("任务项")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.format_lines("- [ ] ", window, cx)
                                    })),
                            )
                            .child(
                                components::button("notes-toggle-task")
                                    .ghost()
                                    .label("切换完成")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.toggle_tasks(window, cx)
                                    })),
                            )
                    }))
                    .children((!self.preview).then(|| {
                        div()
                            .h_flex()
                            .gap(px(theme::space_compact()))
                            .child(components::text_input(&self.find_input).flex_1())
                            .child(components::text_input(&self.replace_input).flex_1())
                            .child(
                                components::button("notes-replace-all")
                                    .ghost()
                                    .label("全部替换")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.replace_matches(window, cx)
                                    })),
                            )
                    }))
                    .child(div().flex_1().min_h_0().child(editor))
                    .child(
                        div()
                            .v_flex()
                            .gap(px(theme::space_compact()))
                            .child(
                                div()
                                    .h_flex()
                                    .gap(px(theme::space_compact()))
                                    .child(
                                        components::button("notes-create")
                                            .label("新建笔记")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.create(window, cx)
                                            })),
                                    )
                                    .child(
                                        components::button("notes-show-trash")
                                            .ghost()
                                            .label("回收站")
                                            .on_click(
                                                cx.listener(|this, _, _, cx| this.show_trash(cx)),
                                            ),
                                    )
                                    .child(div().flex_1())
                                    .child(
                                        components::button("notes-image")
                                            .ghost()
                                            .label("插入图片")
                                            .on_click(
                                                cx.listener(|this, _, _, cx| this.insert_image(cx)),
                                            ),
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
                            )
                            .child(
                                div()
                                    .h_flex()
                                    .gap(px(theme::space_compact()))
                                    .child(components::text_input(&self.move_input).flex_1())
                                    .child(
                                        components::button("notes-move")
                                            .ghost()
                                            .label("移动到")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.move_selected(window, cx)
                                            })),
                                    )
                                    .child(
                                        components::button("notes-trash")
                                            .ghost()
                                            .label("移入回收站")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.trash_selected(cx)
                                            })),
                                    ),
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
