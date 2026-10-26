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

pub fn sidebar(
    service: Arc<NotesService>,
    panels: Arc<PanelCatalog>,
) -> fileatlas_ui_bridge::UiViewContribution {
    fileatlas_ui_bridge::UiViewContribution::new(
        "notes/sidebar-tree",
        "navigation.sidebar-notes",
        0,
        move |window, app| {
            let service = Arc::clone(&service);
            let panels = Arc::clone(&panels);
            let search = app.new(|cx| {
                gpui_kit::component::input::InputState::new(window, cx).placeholder("搜索笔记")
            });
            let new_folder = app.new(|cx| {
                gpui_kit::component::input::InputState::new(window, cx).placeholder("新文件夹名称")
            });
            let rename_folder = app.new(|cx| {
                gpui_kit::component::input::InputState::new(window, cx).placeholder("目录名称")
            });
            let move_folder = app.new(|cx| {
                gpui_kit::component::input::InputState::new(window, cx)
                    .placeholder("移动到目录路径，留空为根目录")
            });
            app.new(|cx| {
                let wake = fileatlas_ui_bridge::refresh::Wake::default();
                let mut index =
                    fileatlas_ui_bridge::read_model::ReadModel::<u64, NotesIndex>::default();
                index.set_wake(wake.clone());
                let initial_service = Arc::clone(&service);
                index.request(service.revision(), move || {
                    Ok(read_index(&initial_service)?)
                });
                let task =
                    fileatlas_ui_bridge::refresh::listen(cx, wake, |view: &mut NotesSidebar, _| {
                        view.index.poll()
                    });
                NotesSidebar {
                    service,
                    panels,
                    index,
                    search,
                    new_folder,
                    rename_folder,
                    move_folder,
                    selected_folder: None,
                    error: None,
                    _task: task,
                }
            })
            .into()
        },
    )
}

fn read_index(service: &NotesService) -> Result<NotesIndex, String> {
    Ok(NotesIndex {
        notes: service.list().map_err(|error| error.to_string())?,
        folders: service.list_folders().map_err(|error| error.to_string())?,
    })
}

struct NotesSidebar {
    service: Arc<NotesService>,
    panels: Arc<PanelCatalog>,
    index: fileatlas_ui_bridge::read_model::ReadModel<u64, NotesIndex>,
    search: Entity<gpui_kit::component::input::InputState>,
    new_folder: Entity<gpui_kit::component::input::InputState>,
    rename_folder: Entity<gpui_kit::component::input::InputState>,
    move_folder: Entity<gpui_kit::component::input::InputState>,
    selected_folder: Option<NoteFolder>,
    error: Option<String>,
    _task: Task<()>,
}

impl NotesSidebar {
    fn open_panel(&self) {
        // A single panel instance otherwise keeps its previous view model when
        // navigation changes the selected note or switches into the trash.
        let _ = self.panels.close_definition(PANEL);
        if let Err(error) = self.panels.open(PANEL) {
            tracing::error!(%error, "could not open notes panel");
        }
    }

    fn create_folder(&mut self, cx: &mut Context<Self>) {
        let name = self.new_folder.read(cx).value().trim().to_owned();
        match self.service.create_folder(
            self.selected_folder
                .as_ref()
                .map(|folder| folder.path.as_str()),
            &name,
        ) {
            Ok(folder) => {
                self.service.select_folder(Some(&folder.path));
                self.selected_folder = Some(folder);
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn rename_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(folder) = self.selected_folder.as_ref() else {
            return;
        };
        let name = self.rename_folder.read(cx).value().trim().to_owned();
        match self.service.rename_folder(&folder.path, &name) {
            Ok(folder) => {
                self.service.select_folder(Some(&folder.path));
                self.rename_folder.update(cx, |input, cx| {
                    input.set_value(folder.name.clone(), window, cx)
                });
                self.selected_folder = Some(folder);
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn move_selected(&mut self, cx: &mut Context<Self>) {
        let Some(folder) = self.selected_folder.as_ref() else {
            return;
        };
        let parent = self.move_folder.read(cx).value().trim().to_owned();
        match self.service.move_folder(&folder.path, Some(&parent)) {
            Ok(folder) => {
                self.service.select_folder(Some(&folder.path));
                self.selected_folder = Some(folder);
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn trash_selected(&mut self, cx: &mut Context<Self>) {
        let Some(folder) = self.selected_folder.take() else {
            return;
        };
        match self.service.trash_folder(&folder.path) {
            Ok(_) => {
                self.service.select_folder(None);
                self.error = Some("目录已移入回收站".into());
            }
            Err(error) => {
                self.selected_folder = Some(folder);
                self.error = Some(error.to_string());
            }
        }
        cx.notify();
    }

    fn select_folder(&mut self, folder: NoteFolder, window: &mut Window, cx: &mut Context<Self>) {
        self.rename_folder.update(cx, |input, cx| {
            input.set_value(folder.name.clone(), window, cx)
        });
        self.move_folder.update(cx, |input, cx| {
            input.set_value(parent_path(&folder.path), window, cx)
        });
        self.service.select_folder(Some(&folder.path));
        self.selected_folder = Some(folder);
        self.error = None;
        cx.notify();
    }

    fn open_new_note(&self) {
        self.service.select("");
        self.service.select_folder(
            self.selected_folder
                .as_ref()
                .map(|folder| folder.path.as_str()),
        );
        self.open_panel();
    }

    fn open_trash(&self) {
        self.service.select(".trash");
        self.open_panel();
    }
}

fn parent_path(path: &str) -> String {
    path.rsplit_once('/')
        .map_or_else(String::new, |(parent, _)| parent.to_owned())
}

impl Render for NotesSidebar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let revision = self.service.revision();
        let service = Arc::clone(&self.service);
        self.index.request(revision, move || read_index(&service));
        let index = self
            .index
            .value()
            .and_then(|value| value.as_ref().ok())
            .cloned()
            .unwrap_or_default();
        let query = self.search.read(cx).value().trim().to_lowercase();
        let matching = |note: &&NoteSummary| {
            query.is_empty()
                || note.title.to_lowercase().contains(&query)
                || note.path.to_lowercase().contains(&query)
                || note.category.to_lowercase().contains(&query)
                || note.status.to_lowercase().contains(&query)
                || note
                    .tags
                    .iter()
                    .any(|tag| tag.to_lowercase().contains(&query))
        };
        let mut rows = div().v_flex().gap(px(theme::space_compact()));
        for folder in &index.folders {
            let folder_matches = query.is_empty()
                || folder.path.to_lowercase().contains(&query)
                || index
                    .notes
                    .iter()
                    .filter(matching)
                    .any(|note| note.path.starts_with(&format!("{}/", folder.path)));
            if !folder_matches {
                continue;
            }
            let selected = self
                .selected_folder
                .as_ref()
                .is_some_and(|selected| selected.path == folder.path);
            let folder_for_click = folder.clone();
            rows = rows.child(indented(
                folder.depth,
                components::button(format!("notes-folder-{}", folder.path))
                    .ghost()
                    .selected(selected)
                    .label(folder.name.clone())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.select_folder(folder_for_click.clone(), window, cx)
                    })),
            ));
            for note in index.notes.iter().filter(matching).filter(|note| {
                note.path.starts_with(&format!("{}/", folder.path))
                    && note.path[folder.path.len() + 1..].find('/').is_none()
            }) {
                let path = note.path.clone();
                rows = rows.child(indented(
                    folder.depth + 1,
                    components::button(format!("note-row-{}", note.id))
                        .ghost()
                        .label(note.title.clone())
                        .on_click(cx.listener(move |this, _, _, _| {
                            this.service.select(&path);
                            this.open_panel();
                        })),
                ));
            }
        }
        for note in index
            .notes
            .iter()
            .filter(matching)
            .filter(|note| !note.path.contains('/'))
        {
            let path = note.path.clone();
            rows = rows.child(
                components::button(format!("note-row-{}", note.id))
                    .ghost()
                    .label(note.title.clone())
                    .on_click(cx.listener(move |this, _, _, _| {
                        this.service.select(&path);
                        this.open_panel();
                    })),
            );
        }
        let mut content = div()
            .v_flex()
            .size_full()
            .min_w_0()
            .gap(px(theme::space_standard()))
            .px(px(theme::space_expanded()))
            .py(px(theme::space_expanded()))
            .child(div().workbench_title().child("笔记"))
            .child(components::search_input(&self.search))
            .child(
                div()
                    .h_flex()
                    .gap(px(theme::space_compact()))
                    .child(
                        components::button("notes-open-panel")
                            .label("新建笔记")
                            .on_click(cx.listener(|this, _, _, _| this.open_new_note())),
                    )
                    .child(
                        components::button("notes-open-trash")
                            .ghost()
                            .label("回收站")
                            .on_click(cx.listener(|this, _, _, _| this.open_trash())),
                    ),
            )
            .child(
                div()
                    .h_flex()
                    .gap(px(theme::space_compact()))
                    .child(components::text_input(&self.new_folder).flex_1())
                    .child(
                        components::button("notes-create-folder")
                            .label("新建目录")
                            .on_click(cx.listener(|this, _, _, cx| this.create_folder(cx))),
                    ),
            )
            .child(rows.overflow_y_scrollbar());
        if let Some(folder) = self.selected_folder.clone() {
            content = content
                .child(fileatlas_ui_bridge::components::divider())
                .child(
                    div()
                        .workbench_assistant()
                        .child(format!("目录操作 · {}", folder.path)),
                )
                .child(components::text_input(&self.rename_folder))
                .child(
                    components::button("notes-rename-folder")
                        .ghost()
                        .label("重命名目录")
                        .on_click(
                            cx.listener(|this, _, window, cx| this.rename_selected(window, cx)),
                        ),
                )
                .child(components::text_input(&self.move_folder))
                .child(
                    components::button("notes-move-folder")
                        .ghost()
                        .label("移动目录")
                        .on_click(cx.listener(|this, _, _, cx| this.move_selected(cx))),
                )
                .child(
                    components::button("notes-trash-folder")
                        .ghost()
                        .label("移入回收站")
                        .on_click(cx.listener(|this, _, _, cx| this.trash_selected(cx))),
                );
        }
        if let Some(error) = self.error.clone() {
            content = content.child(div().workbench_assistant().child(error));
        }
        content
    }
}
