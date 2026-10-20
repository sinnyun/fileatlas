use super::PANEL;
use ::gpui::{AnyView, App, Context as ViewContext, RenderImage, Task, Window, px};
use cordis::Service;
use fileatlas_plugin_workcrate_boards::{WorkCrateBoard, WorkCrateBoardsService};
use fileatlas_plugin_workcrate_export::WorkCrateExportService;
use fileatlas_plugin_workcrate_projects::WorkCrateProjectsService;
use fileatlas_ui_bridge::{components::WorkbenchStyle as _, theme};
use gpui_kit::{
    base::{Disableable as _, StyledExt as _},
    component::button::{Button, ButtonVariants as _},
    prelude::FluentBuilder as _,
    *,
};
use std::{path::PathBuf, sync::Arc};

pub(super) fn panel(
    projects: Arc<WorkCrateProjectsService>,
    boards: Arc<WorkCrateBoardsService>,
    exports: Arc<WorkCrateExportService>,
    surfaces: Arc<fileatlas_ui_bridge::UiSurfaceState>,
    ui_state: Arc<WorkCrateUiState>,
    thumbnails: Arc<fileatlas_plugin_thumbnails::ThumbnailService>,
    file_operations: Arc<fileatlas_plugin_file_operations::FileOperationsService>,
    window: &mut Window,
    app: &mut App,
) -> AnyView {
    let initial = projects.projects(true).unwrap_or_default();
    let edit_input = app.new(|cx| gpui_kit::component::input::InputState::new(window, cx));
    let search_input = app
        .new(|cx| gpui_kit::component::input::InputState::new(window, cx).placeholder("搜索项目"));
    let initial_selection = ui_state.selected_board();
    let service = Arc::clone(&projects);
    let wake = projects.wake();
    let selection_wake = ui_state.wake();
    app.new(|cx| {
        let task =
            fileatlas_ui_bridge::refresh::listen(cx, wake, |view: &mut WorkCratePanel, cx| {
                let next = view.projects.projects(true).unwrap_or_default();
                if next == view.items {
                    return false;
                }
                view.items = next;
                if view
                    .selected_project
                    .as_ref()
                    .is_some_and(|id| !view.items.iter().any(|item| &item.id == id))
                {
                    view.selected_project = None;
                }
                view.reload(cx);
                true
            });
        let selection_task = fileatlas_ui_bridge::refresh::listen(
            cx,
            selection_wake,
            |view: &mut WorkCratePanel, cx| {
                let Some((project_id, board_id)) = view.ui_state.selected_board() else {
                    return false;
                };
                let project = (!project_id.is_empty()).then_some(project_id);
                let board = (!board_id.is_empty()).then_some(board_id);
                view.selected_project = project;
                view.selected_board = board;
                view.reload(cx);
                true
            },
        );
        let selected_project = initial_selection
            .as_ref()
            .map(|(project_id, _)| project_id.clone())
            .filter(|project_id| initial.iter().any(|project| &project.id == project_id));
        let mut view = WorkCratePanel {
            projects: service,
            boards,
            exports,
            surfaces,
            ui_state,
            thumbnails,
            file_operations,
            items: initial,
            selected_project,
            boards_view: Vec::new(),
            project_covers: Default::default(),
            selected_pages: Default::default(),
            show_archived: false,
            sort_by_name: false,
            image_export_jpeg: false,
            manage_id: None,
            editing: None,
            edit_input,
            search_input,
            query: String::new(),
            selected_image: None,
            gesture: None,
            crop_mode: false,
            edit_saving: false,
            state_gate: Default::default(),
            selected_board: initial_selection
                .as_ref()
                .map(|(_, board_id)| board_id.clone())
                .filter(|id| !id.is_empty()),
            canvas: None,
            zoom: 1.0,
            fit_scale: 0.75,
            device_scale: 1.0,
            rendered_scale: 0.0,
            busy: false,
            message: None,
            _wake_task: task,
            _selection_wake_task: selection_task,
        };
        view.reload(cx);
        view
    })
    .into()
}

#[derive(Clone)]
struct BoardCard {
    error: Option<String>,
    board: WorkCrateBoard,
    image: Option<Arc<RenderImage>>,
    sources: std::collections::BTreeMap<String, Arc<RenderImage>>,
    originals: std::collections::BTreeMap<String, Arc<RenderImage>>,
}

struct WorkCratePanel {
    projects: Arc<WorkCrateProjectsService>,
    boards: Arc<WorkCrateBoardsService>,
    exports: Arc<WorkCrateExportService>,
    surfaces: Arc<fileatlas_ui_bridge::UiSurfaceState>,
    ui_state: Arc<WorkCrateUiState>,
    thumbnails: Arc<fileatlas_plugin_thumbnails::ThumbnailService>,
    file_operations: Arc<fileatlas_plugin_file_operations::FileOperationsService>,
    items: Vec<fileatlas_workcrate_contracts::WorkCrateProject>,
    selected_project: Option<String>,
    boards_view: Vec<BoardCard>,
    project_covers: std::collections::BTreeMap<String, Arc<RenderImage>>,
    selected_pages: std::collections::BTreeSet<String>,
    show_archived: bool,
    sort_by_name: bool,
    image_export_jpeg: bool,
    manage_id: Option<String>,
    editing: Option<(String, String)>,
    edit_input: gpui::Entity<gpui_kit::component::input::InputState>,
    search_input: gpui::Entity<gpui_kit::component::input::InputState>,
    query: String,
    selected_image: Option<String>,
    gesture: Option<image_edit::ImageGesture>,
    crop_mode: bool,
    edit_saving: bool,
    state_gate: fileatlas_ui_bridge::refresh::ChangeGate<String>,
    selected_board: Option<String>,
    canvas: Option<Arc<RenderImage>>,
    zoom: f64,
    fit_scale: f64,
    device_scale: f64,
    rendered_scale: f64,
    busy: bool,
    message: Option<String>,
    _wake_task: Task<()>,
    _selection_wake_task: Task<()>,
}

#[derive(Default)]
pub(super) struct WorkCrateUiState {
    selected_board: std::sync::RwLock<Option<(String, String)>>,
    wake: fileatlas_ui_bridge::refresh::Wake,
}

impl Service for WorkCrateUiState {
    const NAME: &'static str = "fileatlas/workcrate-ui-state";
}

impl WorkCrateUiState {
    pub fn selected_board(&self) -> Option<(String, String)> {
        self.selected_board
            .read()
            .ok()
            .and_then(|value| value.clone())
    }

    pub fn select_board(&self, project_id: String, board_id: String) -> Result<(), String> {
        *self
            .selected_board
            .write()
            .map_err(|_| "WorkCrate UI selection is unavailable")? = Some((project_id, board_id));
        self.wake.trigger();
        Ok(())
    }

    pub fn wake(&self) -> fileatlas_ui_bridge::refresh::Wake {
        self.wake.clone()
    }

    pub fn navigate(&self, project: Option<String>, board: Option<String>) -> Result<(), String> {
        self.select_board(project.unwrap_or_default(), board.unwrap_or_default())
    }
}

impl WorkCratePanel {
    fn reload(&mut self, cx: &mut ViewContext<Self>) {
        let Some(project_id) = self.selected_project.clone() else {
            self.busy = false;
            self.boards_view.clear();
            self.load_project_covers(cx);
            cx.notify();
            return;
        };
        let boards = Arc::clone(&self.boards);
        let requested_project = project_id.clone();
        self.busy = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let loaded = cx
                .background_executor()
                .spawn(async move {
                    let models = boards
                        .board_catalog(&project_id)
                        .map_err(|error| error.to_string())?;
                    let mut cards = Vec::new();
                    for model in models {
                        let (board, error) = match boards.load_board(&project_id, &model.id) {
                            Ok(board) => (board, None),
                            Err(error) => (
                                boards.board(&model.id).unwrap_or(model),
                                Some(error.to_string()),
                            ),
                        };
                        let preview = boards
                            .render_page(&board.id, 0, 0.5)
                            .ok()
                            .and_then(|page| image::load_from_memory(&page.to_png()).ok())
                            .map(|image| render_image(image.into_rgba8()));
                        let decoded: std::collections::BTreeMap<_, _> = board
                            .source
                            .assets
                            .iter()
                            .filter_map(|(id, asset)| {
                                image::load_from_memory(&asset.data)
                                    .ok()
                                    .map(|image| (id.to_string(), image))
                            })
                            .collect();
                        let sources = decoded
                            .iter()
                            .map(|(id, image)| {
                                (
                                    id.clone(),
                                    render_image(image.thumbnail(120, 120).into_rgba8()),
                                )
                            })
                            .collect();
                        let originals = decoded
                            .into_iter()
                            .map(|(id, image)| (id, render_image(image.into_rgba8())))
                            .collect();
                        cards.push(BoardCard {
                            error: error.clone(),
                            sources,
                            originals,
                            board,
                            image: if error.is_some() { None } else { preview },
                        });
                    }
                    Ok::<_, String>(cards)
                })
                .await;
            let _ = this.update(cx, |view, cx| {
                if view.selected_project.as_ref() != Some(&requested_project) {
                    return;
                }
                view.busy = false;
                match loaded {
                    Ok(cards) => {
                        view.boards_view = cards;
                        if let Some(id) = &view.selected_board {
                            if let Some(board) =
                                view.boards_view.iter().find(|card| &card.board.id == id)
                            {
                                view.render_canvas(board.board.clone(), cx);
                            }
                        }
                    }
                    Err(error) => view.message = Some(error),
                };
                cx.notify();
            });
        })
        .detach();
    }

    fn render_canvas(&mut self, board: WorkCrateBoard, cx: &mut ViewContext<Self>) {
        let boards = Arc::clone(&self.boards);
        let zoom = self.fit_scale * self.zoom * self.device_scale;
        let requested_id = board.id.clone();
        self.rendered_scale = zoom;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let page = boards
                        .render_template(&board.id, zoom)
                        .map_err(|error| error.to_string())?;
                    image::load_from_memory(&page.to_png())
                        .map(|image| render_image(image.into_rgba8()))
                        .map_err(|error| error.to_string())
                })
                .await;
            let _ = this.update(cx, |view, cx| {
                if view.selected_board.as_ref() != Some(&requested_id)
                    || (view.rendered_scale - zoom).abs() > 0.005
                {
                    return;
                }
                match result {
                    Ok(image) => view.canvas = Some(image),
                    Err(error) => view.message = Some(error),
                };
                cx.notify();
            });
        })
        .detach();
    }

    fn create_project(&mut self, cx: &mut ViewContext<Self>) {
        let projects = Arc::clone(&self.projects);
        self.busy = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let names = projects
                        .projects(true)
                        .map_err(|error| error.to_string())?
                        .into_iter()
                        .map(|p| p.name)
                        .collect::<std::collections::HashSet<_>>();
                    let mut index = 1;
                    let name = loop {
                        let candidate = format!("新建项目 {index}");
                        if !names.contains(&candidate) {
                            break candidate;
                        }
                        index += 1;
                    };
                    projects
                        .create_project(&name)
                        .map_err(|error| error.to_string())
                })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                match result {
                    Ok(project) => {
                        view.selected_project = Some(project.id);
                        view.items = view.projects.projects(true).unwrap_or_default();
                        view.reload(cx);
                    }
                    Err(error) => view.message = Some(error),
                };
                cx.notify();
            });
        })
        .detach();
    }

    fn create_board(&mut self, cx: &mut ViewContext<Self>) {
        let Some(project_id) = self.selected_project.clone() else {
            self.message = Some("请先创建项目".into());
            cx.notify();
            return;
        };
        let boards = Arc::clone(&self.boards);
        self.busy = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    boards
                        .create_page(&project_id)
                        .map_err(|error| error.to_string())
                })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                match result {
                    Ok(board) => {
                        view.selected_board = Some(board.id);
                        view.reload(cx);
                    }
                    Err(error) => view.message = Some(error),
                };
                cx.notify();
            });
        })
        .detach();
    }

    fn export(&mut self, cx: &mut ViewContext<Self>) {
        let Some(board_id) = self.selected_board.clone() else {
            self.message = Some("请先选择画板".into());
            cx.notify();
            return;
        };
        let exports = Arc::clone(&self.exports);
        self.busy = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let target = rfd::AsyncFileDialog::new()
                .add_filter("PDF", &["pdf"])
                .set_file_name("board.pdf")
                .save_file()
                .await
                .map(|file| file.path().to_path_buf());
            let result = if let Some(path) = target {
                cx.background_executor()
                    .spawn(async move {
                        exports
                            .export_page_pdf(&board_id, 0, &path)
                            .map_err(|error| error.to_string())
                    })
                    .await
                    .map(Some)
            } else {
                Ok(None)
            };
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                match result {
                    Ok(Some(export)) => {
                        view.message = Some(format!(
                            "已导出 PDF：{} DPI，{}×{} px",
                            export.dpi, export.width_px, export.height_px
                        ))
                    }
                    Ok(None) => {}
                    Err(error) => view.message = Some(format!("导出失败：{error}")),
                };
                cx.notify();
            });
        })
        .detach();
    }
}

mod canvas;
mod images;
mod inspector;
use images::render_image;

impl Drop for WorkCratePanel {
    fn drop(&mut self) {
        let _ = self.surfaces.set_value("workcrate.ui", "null");
        let _ = self.surfaces.set_value("workcrate.viewport", "null");
    }
}
mod management;
mod render;

mod input;

mod preview;

mod image_edit;
mod linked_images;
mod tracking;
