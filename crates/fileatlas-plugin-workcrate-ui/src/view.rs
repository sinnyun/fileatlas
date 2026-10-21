use super::PANEL;
use ::gpui::{AnyView, App, Context as ViewContext, RenderImage, Task, Window, px};
use cordis::Service;
use fileatlas_plugin_workcrate_boards::{WorkCrateBoard, WorkCrateBoardsService};
use fileatlas_plugin_workcrate_export::WorkCrateExportService;
use fileatlas_plugin_workcrate_projects::WorkCrateProjectsService;
use fileatlas_ui_bridge::{components::WorkbenchStyle as _, read_model::ReadModel, refresh, theme};
use gpui_kit::{
    base::{Disableable as _, StyledExt as _},
    component::button::{Button, ButtonVariants as _},
    prelude::FluentBuilder as _,
    *,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
};
mod state;
pub(super) use state::WorkCrateUiState;
mod images;
use images::render_image;
mod actions;
mod fields;
mod inspector;
mod management;
mod pages;
mod preview;
mod projects;
mod render;
pub(crate) mod tracking;
#[derive(Clone)]
struct BoardCard {
    board: WorkCrateBoard,
    image: Option<Arc<RenderImage>>,
    error: Option<String>,
}
struct Snapshot {
    items: Vec<fileatlas_workcrate_contracts::WorkCrateProject>,
    presets: fileatlas_plugin_workcrate_projects::FieldPresets,
    boards: Vec<WorkCrateBoard>,
    covers: BTreeMap<String, Arc<RenderImage>>,
    config: fileatlas_plugin_workcrate_boards::VectorCraftConfig,
    document: Option<PathBuf>,
}
struct WorkCratePanel {
    projects: Arc<WorkCrateProjectsService>,
    boards: Arc<WorkCrateBoardsService>,
    exports: Arc<WorkCrateExportService>,
    surfaces: Arc<fileatlas_ui_bridge::UiSurfaceState>,
    ui_state: Arc<WorkCrateUiState>,
    thumbnails: Arc<fileatlas_plugin_thumbnails::ThumbnailService>,
    items: Vec<fileatlas_workcrate_contracts::WorkCrateProject>,
    preset_fields: fileatlas_plugin_workcrate_projects::FieldPresets,
    boards_view: Vec<BoardCard>,
    project_covers: BTreeMap<String, Arc<RenderImage>>,
    selected_project: Option<String>,
    selected_board: Option<String>,
    selected_pages: BTreeSet<String>,
    multi_select: bool,
    show_archived: bool,
    sort_by_name: bool,
    image_export_jpeg: bool,
    manage_id: Option<String>,
    editing: Option<(String, String)>,
    edit_input: Entity<gpui_kit::component::input::InputState>,
    search_input: Entity<gpui_kit::component::input::InputState>,
    port_input: Entity<gpui_kit::component::input::InputState>,
    query: String,
    port_synced: Option<u16>,
    config: fileatlas_plugin_workcrate_boards::VectorCraftConfig,
    document: Option<PathBuf>,
    connection_seen: bool,
    busy: bool,
    message: Option<String>,
    load_generation: u64,
    board_refresh_revision: u64,
    catalog_refresh_revision: u64,
    model: ReadModel<u64, Snapshot>,
    preview_model: ReadModel<(u64, String), Arc<RenderImage>>,
    preview_queue: std::collections::VecDeque<String>,
    preview_current: Option<String>,
    state_gate: refresh::ChangeGate<String>,
    _tasks: Vec<Task<()>>,
}
pub(super) fn panel(
    projects: Arc<WorkCrateProjectsService>,
    boards: Arc<WorkCrateBoardsService>,
    exports: Arc<WorkCrateExportService>,
    surfaces: Arc<fileatlas_ui_bridge::UiSurfaceState>,
    ui_state: Arc<WorkCrateUiState>,
    thumbnails: Arc<fileatlas_plugin_thumbnails::ThumbnailService>,
    _file_operations: Arc<fileatlas_plugin_file_operations::FileOperationsService>,
    window: &mut Window,
    app: &mut App,
) -> AnyView {
    let edit_input = app.new(|cx| gpui_kit::component::input::InputState::new(window, cx));
    let search_input = app
        .new(|cx| gpui_kit::component::input::InputState::new(window, cx).placeholder("搜索项目"));
    let port_input = app.new(|cx| {
        let mut v = gpui_kit::component::input::InputState::new(window, cx);
        v.set_value("17979", window, cx);
        v
    });
    app.new(|cx| {
        let wake = refresh::Wake::default();
        let preview_wake = refresh::Wake::default();
        let mut model = ReadModel::default();
        model.set_wake(wake.clone());
        let mut preview_model = ReadModel::default();
        preview_model.set_wake(preview_wake.clone());
        let tasks = vec![
            refresh::listen(cx, wake, |v: &mut WorkCratePanel, _| v.deliver()),
            refresh::listen(cx, preview_wake, |v: &mut WorkCratePanel, _| {
                v.deliver_preview()
            }),
            refresh::listen(cx, ui_state.wake(), |v: &mut WorkCratePanel, cx| {
                let selection = v
                    .ui_state
                    .selected_board()
                    .map(|(p, b)| ((!p.is_empty()).then_some(p), (!b.is_empty()).then_some(b)))
                    .unwrap_or_default();
                let revision = (
                    v.ui_state.board_refresh_revision(),
                    v.ui_state.catalog_refresh_revision(),
                );
                if selection != (v.selected_project.clone(), v.selected_board.clone())
                    || revision != (v.board_refresh_revision, v.catalog_refresh_revision)
                {
                    v.selected_project = selection.0;
                    v.selected_board = selection.1;
                    v.board_refresh_revision = revision.0;
                    v.catalog_refresh_revision = revision.1;
                    v.reload(cx);
                    return true;
                }
                let connected = v.ui_state.connection();
                if connected != v.connection_seen {
                    v.connection_seen = connected;
                    return true;
                }
                false
            }),
            refresh::listen(cx, projects.wake(), |v: &mut WorkCratePanel, cx| {
                v.reload(cx);
                true
            }),
        ];
        let selection = ui_state.selected_board().unwrap_or_default();
        let mut v = WorkCratePanel {
            projects,
            boards,
            exports,
            surfaces,
            ui_state,
            thumbnails,
            items: vec![],
            preset_fields: Default::default(),
            boards_view: vec![],
            project_covers: Default::default(),
            selected_project: (!selection.0.is_empty()).then_some(selection.0),
            selected_board: (!selection.1.is_empty()).then_some(selection.1),
            selected_pages: Default::default(),
            multi_select: false,
            show_archived: false,
            sort_by_name: false,
            image_export_jpeg: false,
            manage_id: None,
            editing: None,
            edit_input,
            search_input,
            port_input,
            query: String::new(),
            port_synced: None,
            config: Default::default(),
            document: None,
            connection_seen: false,
            busy: false,
            message: None,
            load_generation: 0,
            board_refresh_revision: 0,
            catalog_refresh_revision: 0,
            model,
            preview_model,
            preview_queue: Default::default(),
            preview_current: None,
            state_gate: Default::default(),
            _tasks: tasks,
        };
        v.reload(cx);
        v
    })
    .into()
}
impl WorkCratePanel {
    fn reload(&mut self, cx: &mut ViewContext<Self>) {
        self.load_generation += 1;
        self.boards_view.clear();
        self.preview_queue.clear();
        self.preview_current = None;
        let projects = self.projects.clone();
        let boards = self.boards.clone();
        let selected = self.selected_project.clone();
        self.busy = true;
        self.model.request(self.load_generation, move || {
            let items = projects.projects(true).map_err(|e| e.to_string())?;
            let presets = projects.field_presets().map_err(|e| e.to_string())?;
            let config = boards.configuration().map_err(|e| e.to_string())?;
            let mut covers = BTreeMap::new();
            for p in &items {
                if let Some(cover) = &p.main_image {
                    if let Ok(dir) = projects.project_directory(&p.id) {
                        if let Ok(image) = image::open(dir.join(&cover.path)) {
                            covers.insert(
                                p.id.clone(),
                                render_image(image.thumbnail(720, 480).into_rgba8()),
                            );
                        }
                    }
                }
            }
            let document = selected
                .as_ref()
                .map(|id| boards.document_path(id).map_err(|e| e.to_string()))
                .transpose()?
                .flatten();
            let catalog = if let Some(id) = selected {
                if document.is_some() {
                    boards.board_catalog(&id).map_err(|e| e.to_string())?
                } else {
                    vec![]
                }
            } else {
                vec![]
            };
            Ok(Snapshot {
                items,
                presets,
                boards: catalog,
                covers,
                config,
                document,
            })
        });
        cx.notify();
    }
    fn deliver(&mut self) -> bool {
        if !self.model.poll() {
            return false;
        }
        if let Some(result) = self.model.value() {
            self.busy = false;
            match result {
                Ok(s) => {
                    self.items = s.items.clone();
                    self.preset_fields = s.presets.clone();
                    self.project_covers = s.covers.clone();
                    self.config = s.config.clone();
                    self.document = s.document.clone();
                    self.boards_view = s
                        .boards
                        .iter()
                        .cloned()
                        .map(|board| BoardCard {
                            board,
                            image: None,
                            error: None,
                        })
                        .collect();
                    self.selected_pages
                        .retain(|id| self.boards_view.iter().any(|c| &c.board.id == id));
                    if self
                        .selected_project
                        .as_ref()
                        .is_some_and(|id| !self.items.iter().any(|p| &p.id == id))
                    {
                        self.selected_project = None;
                        self.selected_board = None;
                    }
                    if self
                        .selected_board
                        .as_ref()
                        .is_some_and(|id| !self.boards_view.iter().any(|c| &c.board.id == id))
                    {
                        self.selected_board = None;
                    }
                    self.preview_queue = self
                        .boards_view
                        .iter()
                        .map(|c| c.board.id.clone())
                        .collect();
                    self.start_preview();
                }
                Err(e) => {
                    self.message = Some(e.clone());
                }
            }
        }
        true
    }
    fn start_preview(&mut self) {
        if let Some(id) = self.preview_queue.pop_front() {
            let b = self.boards.clone();
            self.preview_current = Some(id.clone());
            self.preview_model
                .request((self.load_generation, id.clone()), move || {
                    let path = b.render_preview(&id, 1600).map_err(|e| e.to_string())?;
                    image::open(path)
                        .map(|i| render_image(i.into_rgba8()))
                        .map_err(|e| e.to_string())
                });
        }
    }
    fn deliver_preview(&mut self) -> bool {
        if !self.preview_model.poll() {
            return false;
        }
        if self
            .preview_model
            .key()
            .is_some_and(|(g, _)| *g == self.load_generation)
        {
            if let Some(id) = self.preview_current.take() {
                if let Some(c) = self.boards_view.iter_mut().find(|c| c.board.id == id) {
                    match self.preview_model.value() {
                        Some(Ok(i)) => c.image = Some(i.clone()),
                        Some(Err(e)) => c.error = Some(e.clone()),
                        _ => {}
                    }
                }
            }
            self.start_preview();
        }
        true
    }
    fn open_page(&mut self, id: &str, cx: &mut ViewContext<Self>) {
        self.selected_board = Some(id.into());
        self.editing = None;
        let _ = self
            .ui_state
            .navigate(self.selected_project.clone(), self.selected_board.clone());
        cx.notify();
    }
    fn panel_content_size(&self) -> (f64, f64) {
        let id = self
            .surfaces
            .open_panel_ids()
            .into_iter()
            .find(|id| id.starts_with(&format!("{PANEL}@")))
            .unwrap_or_else(|| PANEL.into());
        self.surfaces
            .value(&format!("panel.{id}.content_bounds"))
            .and_then(|s| {
                let n = s
                    .split(',')
                    .map(str::parse::<f64>)
                    .collect::<Result<Vec<_>, _>>()
                    .ok()?;
                (n.len() == 4).then_some((n[2], n[3]))
            })
            .unwrap_or((1240., 860.))
    }
}
impl Drop for WorkCratePanel {
    fn drop(&mut self) {
        let _ = self.surfaces.set_value("workcrate.ui", "null");
    }
}
