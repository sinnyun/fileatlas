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
                if let Some((ids, path, images)) = view.ui_state.take_export() {
                    let exports = view.exports.clone();
                    view.perform_export(
                        move || {
                            if images {
                                let paths = if ids.len() == 1 && path.extension().is_some() {
                                    vec![path]
                                } else {
                                    ids.iter()
                                        .enumerate()
                                        .map(|(i, _)| path.join(format!("page-{:02}.png", i + 1)))
                                        .collect()
                                };
                                exports
                                    .export_pages_images(&ids, &paths)
                                    .map(|_| ())
                                    .map_err(|e| e.to_string())
                            } else {
                                exports
                                    .export_pages_pdf(&ids, &path)
                                    .map(|_| ())
                                    .map_err(|e| e.to_string())
                            }
                        },
                        cx,
                    );
                    return true;
                }
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
        let proxy_wake = fileatlas_ui_bridge::refresh::Wake::default();
        let mut proxy_model = fileatlas_ui_bridge::read_model::ReadModel::default();
        proxy_model.set_wake(proxy_wake.clone());
        let proxy_task = fileatlas_ui_bridge::refresh::listen(
            cx,
            proxy_wake,
            |view: &mut WorkCratePanel, cx| view.deliver_display(cx),
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
            load_generation: 0,
            proxy_request: None,
            proxy_model,
            display_request: None,
            _proxy_wake_task: proxy_task,
            preview_hits: 0,
            export_status: None,
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
    load_generation: u64,
    proxy_request: Option<String>,
    proxy_model:
        fileatlas_ui_bridge::read_model::ReadModel<(u64, String, bool), loading::DisplayImages>,
    display_request: Option<loading::DisplayRequest>,
    _proxy_wake_task: Task<()>,
    preview_hits: usize,
    export_status: Option<String>,
    message: Option<String>,
    _wake_task: Task<()>,
    _selection_wake_task: Task<()>,
}

#[derive(Default)]
pub(super) struct WorkCrateUiState {
    selected_board: std::sync::RwLock<Option<(String, String)>>,
    export_request: std::sync::Mutex<Option<(Vec<String>, PathBuf, bool)>>,
    wake: fileatlas_ui_bridge::refresh::Wake,
}

impl Service for WorkCrateUiState {
    const NAME: &'static str = "fileatlas/workcrate-ui-state";
}

impl WorkCrateUiState {
    pub fn queue_export(
        &self,
        ids: Vec<String>,
        path: PathBuf,
        images: bool,
    ) -> Result<(), String> {
        let mut pending = self.export_request.lock().map_err(|e| e.to_string())?;
        if pending.is_some() {
            return Err("已有等待执行的导出".into());
        }
        *pending = Some((ids, path, images));
        self.wake.trigger();
        Ok(())
    }

    fn take_export(&self) -> Option<(Vec<String>, PathBuf, bool)> {
        self.export_request.lock().ok()?.take()
    }
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
                    image::RgbaImage::from_raw(page.width, page.height, page.to_straight())
                        .map(render_image)
                        .ok_or_else(|| "页面像素无效".to_string())
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
        let Some(id) = self.selected_board.clone() else {
            return;
        };
        let exports = self.exports.clone();
        cx.spawn(async move |this, cx| {
            if let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter("PDF", &["pdf"])
                .set_file_name("page.pdf")
                .save_file()
                .await
            {
                let path = file.path().to_path_buf();
                let _ = this.update(cx, |view, cx| {
                    view.perform_export(
                        move || {
                            exports
                                .export_page_pdf(&id, 0, &path)
                                .map(|_| ())
                                .map_err(|e| e.to_string())
                        },
                        cx,
                    )
                });
            }
        })
        .detach();
    }
}

mod canvas;
mod exporting;
mod images;
mod inspector;
mod loading;
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
