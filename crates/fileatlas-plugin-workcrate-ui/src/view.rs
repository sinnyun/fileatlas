use super::PANEL;
use ::gpui::{AnyView, App, Context as ViewContext, RenderImage, Task, Window, px};
use cordis::Service;
use fileatlas_plugin_workcrate_boards::{
    AutoSync, CanvasGeometry, CanvasObject, CanvasSnapshot, PreviewState, WorkCrateBoard,
    WorkCrateBoardsService,
};
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
mod autosync;
mod canvas;
mod canvas_surface;
mod state;
pub(super) use state::WorkCrateUiState;
mod images;
use images::render_image;
mod actions;
mod fields;
mod inspector;
mod management;
mod object;
mod page_order;
mod pages;
mod portable;
mod preview;
mod projects;
mod render;
pub(crate) mod tracking;
#[derive(Clone)]
struct BoardCard {
    board: WorkCrateBoard,
    image: Option<Arc<RenderImage>>,
    error: Option<String>,
    preview_checked: bool,
    canvas_revision: Option<u64>,
}
/// Card-level page facts read from each project's local boards cache; the
/// projects screen shows them without opening any official process.
#[derive(Clone, Default)]
pub(crate) struct PageSummary {
    title: String,
    material: String,
    size: String,
    quantity: String,
}
#[derive(Clone, Default)]
pub(crate) struct ProjectSummary {
    pages: usize,
    first: Option<PageSummary>,
    document: Option<String>,
}
struct Snapshot {
    items: Vec<fileatlas_workcrate_contracts::WorkCrateProject>,
    presets: fileatlas_plugin_workcrate_projects::FieldPresets,
    boards: Vec<WorkCrateBoard>,
    covers: BTreeMap<String, Arc<RenderImage>>,
    summaries: BTreeMap<String, ProjectSummary>,
    config: fileatlas_plugin_workcrate_boards::VectorCraftConfig,
    document: Option<PathBuf>,
}
struct WorkCratePanel {
    panels: Arc<fileatlas_ui_bridge::panel_catalog::PanelCatalog>,
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
    summaries: BTreeMap<String, ProjectSummary>,
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
    query: String,
    canvas_zoom: f32,
    canvas: Option<CanvasSnapshot>,
    canvas_image: Option<Arc<RenderImage>>,
    canvas_object: Option<CanvasObject>,
    canvas_text_input: Entity<gpui_kit::component::input::TextareaState>,
    canvas_drag: Option<canvas::CanvasDrag>,
    canvas_busy: bool,
    canvas_read: ReadModel<(u64, u64, u64), Option<CanvasSnapshot>>,
    canvas_probe_tick: u64,
    canvas_poll: u64,
    config: fileatlas_plugin_workcrate_boards::VectorCraftConfig,
    document: Option<PathBuf>,
    busy: bool,
    field_saves: BTreeSet<(String, String)>,
    message: Option<String>,
    preview_state_read: ReadModel<u64, PreviewState>,
    preview_state_tick: u64,
    auto_sync: bool,
    auto_syncs: u64,
    /// Whole-catalogue reloads. Localized updates must never move this counter.
    reloads: u64,
    /// Whole-view renders. Used to prove an interaction did not repaint the panel.
    renders: u64,
    /// Drop requests accepted by the canvas, whatever surface they came from.
    drops: u64,
    auto_sync_error: Option<String>,
    auto_sync_retry_at: Option<std::time::Instant>,
    load_generation: u64,
    order_generation: u64,
    board_refresh_revision: u64,
    catalog_refresh_revision: u64,
    model: ReadModel<u64, Snapshot>,
    preview_model: ReadModel<(u64, String), Arc<RenderImage>>,
    preview_queue: std::collections::VecDeque<String>,
    preview_current: Option<String>,
    state_gate: refresh::ChangeGate<String>,
    _tasks: Vec<Task<()>>,
    _subscriptions: Vec<::gpui::Subscription>,
}
pub(super) fn panel(
    projects: Arc<WorkCrateProjectsService>,
    boards: Arc<WorkCrateBoardsService>,
    exports: Arc<WorkCrateExportService>,
    surfaces: Arc<fileatlas_ui_bridge::UiSurfaceState>,
    ui_state: Arc<WorkCrateUiState>,
    thumbnails: Arc<fileatlas_plugin_thumbnails::ThumbnailService>,
    _file_operations: Arc<fileatlas_plugin_file_operations::FileOperationsService>,
    panels: Arc<fileatlas_ui_bridge::panel_catalog::PanelCatalog>,
    preferences: Arc<fileatlas_plugin_settings::SettingsService>,
    window: &mut Window,
    app: &mut App,
) -> AnyView {
    let edit_input = app.new(|cx| gpui_kit::component::input::InputState::new(window, cx));
    let canvas_text_input = app.new(|cx| {
        gpui_kit::component::input::TextareaState::new(window, cx)
            .placeholder("输入文字内容，支持换行")
    });
    let search_input = app
        .new(|cx| gpui_kit::component::input::InputState::new(window, cx).placeholder("搜索项目"));
    let defaults = preferences
        .snapshot()
        .map(|s| s.workcrate)
        .unwrap_or_default();
    app.new(|cx| {
        let search_subscription =
            cx.subscribe(&search_input, |v: &mut WorkCratePanel, input, event, cx| {
                if matches!(event, gpui_kit::component::input::InputEvent::Change) {
                    let query = input.read(cx).value().trim().to_lowercase();
                    if v.query != query {
                        v.query = query;
                        cx.notify();
                    }
                }
            });
        // Enter commits the field draft without moving focus to the save button.
        let edit_subscription =
            cx.subscribe(&edit_input, |v: &mut WorkCratePanel, _, event, cx| {
                if matches!(
                    event,
                    gpui_kit::component::input::InputEvent::PressEnter { .. }
                ) && v.editing.is_some()
                    && v.field_saves.is_empty()
                {
                    v.save_edit(cx);
                }
            });
        let wake = refresh::Wake::default();
        let preview_wake = refresh::Wake::default();
        let canvas_wake = refresh::Wake::default();
        let preview_state_wake = refresh::Wake::default();
        let mut model = ReadModel::default();
        model.set_wake(wake.clone());
        let mut preview_model = ReadModel::default();
        preview_model.set_wake(preview_wake.clone());
        let mut canvas_read = ReadModel::default();
        canvas_read.set_wake(canvas_wake.clone());
        let mut preview_state_read = ReadModel::default();
        preview_state_read.set_wake(preview_state_wake.clone());
        let tasks = vec![
            refresh::listen(cx, wake, |v: &mut WorkCratePanel, cx| {
                let delivered = v.deliver();
                // The editor is one interface with the official session always open; a
                // restored or remotely picked page opens its session once its snapshot lands.
                if delivered {
                    v.ensure_canvas_session(cx);
                }
                delivered
            }),
            refresh::listen(cx, canvas_wake, |v: &mut WorkCratePanel, cx| {
                v.deliver_canvas_probe(cx)
            }),
            refresh::listen(cx, preview_wake, |v: &mut WorkCratePanel, _| {
                v.deliver_preview()
            }),
            refresh::listen(cx, preview_state_wake, |v: &mut WorkCratePanel, cx| {
                v.deliver_preview_state(cx)
            }),
            refresh::listen(cx, ui_state.wake(), |v: &mut WorkCratePanel, cx| {
                if let Some(save) = v.ui_state.take_order_refresh() {
                    v.receive_order_save(save, cx);
                    return true;
                }
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
                    let selection_changed =
                        selection != (v.selected_project.clone(), v.selected_board.clone());
                    let catalog_changed = revision.1 != v.catalog_refresh_revision;
                    // One official session holds the whole document, so a page change
                    // inside the editor only moves its artboard: the connection stays
                    // and no catalogue reload runs.
                    let page_switch = v.canvas.is_some()
                        && selection.0 == v.selected_project
                        && selection.0.is_some()
                        && selection.1.is_some()
                        && selection.1 != v.selected_board;
                    if page_switch {
                        v.selected_project = selection.0;
                        v.selected_board = selection.1;
                        v.editing = None;
                        v.board_refresh_revision = revision.0;
                        v.catalog_refresh_revision = revision.1;
                        v.switch_canvas_page(cx);
                        return true;
                    }
                    if v.canvas.is_some() && selection.1 != v.selected_board {
                        // Leaving the page editor for the page list or another project:
                        // only that exit closes the session.
                        v.leave_canvas(cx);
                    } else if v.canvas.is_some() && catalog_changed {
                        // Native catalogue operations close the editing session before
                        // opening their transaction. Do not reuse that stale frame.
                        v.clear_canvas_state();
                    }
                    v.selected_project = selection.0;
                    v.selected_board = selection.1;
                    v.board_refresh_revision = revision.0;
                    v.catalog_refresh_revision = revision.1;
                    if !selection_changed && !catalog_changed && v.canvas.is_some() {
                        // Another client re-read one page. The catalogue and every other
                        // card are unchanged, so only the visible page moves; no reload.
                        v.refresh_visible_board(cx);
                        return true;
                    }
                    v.reload(cx);
                    return true;
                }
                false
            }),
            refresh::listen(cx, projects.wake(), |v: &mut WorkCratePanel, cx| {
                v.deliver_project_change(cx)
            }),
        ];
        let selection = ui_state.selected_board().unwrap_or_default();
        let mut v = WorkCratePanel {
            panels,
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
            summaries: Default::default(),
            selected_project: (!selection.0.is_empty()).then_some(selection.0),
            selected_board: (!selection.1.is_empty()).then_some(selection.1),
            selected_pages: Default::default(),
            multi_select: false,
            show_archived: false,
            sort_by_name: defaults.sort_by_name,
            image_export_jpeg: defaults.image_export_jpeg,
            manage_id: None,
            editing: None,
            edit_input,
            search_input,
            query: String::new(),
            canvas_zoom: 1.,
            canvas: None,
            canvas_image: None,
            canvas_object: None,
            canvas_text_input,
            canvas_drag: None,
            canvas_busy: false,
            canvas_read,
            canvas_probe_tick: 0,
            canvas_poll: 0,
            config: Default::default(),
            document: None,
            busy: false,
            field_saves: Default::default(),
            message: None,
            preview_state_read,
            preview_state_tick: 0,
            auto_sync: false,
            auto_syncs: 0,
            reloads: 0,
            renders: 0,
            drops: 0,
            auto_sync_error: None,
            auto_sync_retry_at: None,
            load_generation: 0,
            order_generation: 0,
            board_refresh_revision: 0,
            catalog_refresh_revision: 0,
            model,
            preview_model,
            preview_queue: Default::default(),
            preview_current: None,
            state_gate: Default::default(),
            _tasks: tasks,
            _subscriptions: vec![search_subscription, edit_subscription],
        };
        v.reload(cx);
        v.start_preview_watcher(cx);
        v
    })
    .into()
}
impl WorkCratePanel {
    fn reload(&mut self, cx: &mut ViewContext<Self>) {
        self.load_generation += 1;
        self.reloads += 1;
        self.boards_view
            .retain(|c| Some(&c.board.project_id) == self.selected_project.as_ref());
        self.preview_queue.clear();
        self.preview_current = None;
        let projects = self.projects.clone();
        let boards = self.boards.clone();
        let selected = self.selected_project.clone();
        let existing_covers = self.project_covers.clone();
        let existing_summaries = self.summaries.clone();
        self.busy = true;
        self.model.request(self.load_generation, move || {
            let items = projects.projects(true).map_err(|e| e.to_string())?;
            let presets = match &selected {
                Some(id) => projects.effective_field_presets(id),
                None => projects.field_presets(),
            }
            .map_err(|e| e.to_string())?;
            let config = boards.configuration().map_err(|e| e.to_string())?;
            let mut covers = existing_covers;
            let mut summaries = existing_summaries;
            if selected.is_none() {
                covers.clear();
                summaries.clear();
                for p in &items {
                    let catalog = boards.board_catalog(&p.id).unwrap_or_default();
                    summaries.insert(
                        p.id.clone(),
                        ProjectSummary {
                            pages: catalog.len(),
                            document: boards.document_path(&p.id).ok().flatten().map(|path| {
                                path.to_string_lossy()
                                    .trim_start_matches(r"\\?\")
                                    .to_owned()
                            }),
                            first: catalog.iter().min_by_key(|b| b.index).map(|b| PageSummary {
                                title: b.title.clone(),
                                material: b.info.material.clone(),
                                size: b.info.size.clone(),
                                quantity: b.info.quantity.clone(),
                            }),
                        },
                    );
                    let path = p
                        .main_image
                        .as_ref()
                        .and_then(|cover| {
                            projects
                                .project_directory(&p.id)
                                .ok()
                                .map(|dir| dir.join(&cover.path))
                        })
                        .or_else(|| {
                            boards
                                .board_catalog(&p.id)
                                .ok()?
                                .first()
                                .and_then(|board| boards.render_preview(&board.id, 1600).ok())
                        });
                    if let Some(path) = path {
                        if let Ok(image) = image::open(path) {
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
                summaries,
                config,
                document,
            })
        });
        cx.notify();
    }
    /// Project-index changes (rename, cover, reference, archive) never alter the page
    /// catalogue. Patch the index in place so an open editor keeps its official session,
    /// cards and decoded previews; only the project screen reloads its covers.
    fn deliver_project_change(&mut self, cx: &mut ViewContext<Self>) -> bool {
        if self.selected_project.is_none() {
            self.reload(cx);
            return true;
        }
        let Ok(items) = self.projects.projects(true) else {
            return false;
        };
        if self
            .selected_project
            .as_ref()
            .is_some_and(|id| !items.iter().any(|p| &p.id == id))
        {
            // The open project disappeared; leaving it is a real screen change.
            self.selected_project = None;
            self.selected_board = None;
            self.reload(cx);
            return true;
        }
        let Ok(presets) = self.selected_project.as_ref().map_or_else(
            || self.projects.field_presets(),
            |id| self.projects.effective_field_presets(id),
        ) else {
            return false;
        };
        if items == self.items && presets == self.preset_fields {
            return false;
        }
        self.items = items;
        self.preset_fields = presets;
        true
    }
    fn deliver(&mut self) -> bool {
        if !self.model.poll() {
            return false;
        }
        if self.model.key() != Some(&self.load_generation) {
            return false;
        }
        if let Some(result) = self.model.value() {
            self.busy = false;
            match result {
                Ok(s) => {
                    self.items = s.items.clone();
                    self.preset_fields = s.presets.clone();
                    self.project_covers = s.covers.clone();
                    self.summaries = s.summaries.clone();
                    self.config = s.config.clone();
                    self.document = s.document.clone();
                    let previous = std::mem::take(&mut self.boards_view);
                    self.boards_view = s
                        .boards
                        .iter()
                        .cloned()
                        .map(|board| BoardCard {
                            image: previous
                                .iter()
                                .find(|c| c.board.id == board.id)
                                .and_then(|c| c.image.clone()),
                            board,
                            error: None,
                            preview_checked: false,
                            canvas_revision: None,
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
                    if let Some(selected) = &self.selected_board {
                        self.preview_queue.retain(|id| id != selected);
                        self.preview_queue.push_front(selected.clone());
                    }
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
                    if c.canvas_revision.is_some() {
                        self.start_preview();
                        return true;
                    }
                    match self.preview_model.value() {
                        Some(Ok(i)) => {
                            c.image = Some(i.clone());
                            c.preview_checked = true;
                        }
                        Some(Err(e)) => {
                            c.error = Some(e.clone());
                            c.preview_checked = true;
                        }
                        _ => {}
                    }
                }
            }
            self.start_preview();
        }
        true
    }
    fn open_page(&mut self, id: &str, cx: &mut ViewContext<Self>) {
        if self.busy
            || self.canvas_busy
            || !self.field_saves.is_empty()
            || self.selected_board.as_deref() == Some(id)
        {
            return;
        }
        self.selected_board = Some(id.into());
        self.editing = None;
        let _ = self
            .ui_state
            .navigate(self.selected_project.clone(), self.selected_board.clone());
        // A page change inside the page editor keeps the official session; only its
        // artboard moves. The first page shown opens the session itself.
        if self.canvas.is_some() {
            self.switch_canvas_page(cx);
        } else {
            self.ensure_canvas_session(cx);
        }
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
        if self.canvas.is_some() {
            let boards = self.boards.clone();
            std::thread::spawn(move || {
                let _ = boards.canvas_commit();
                let _ = boards.canvas_close();
            });
        }
    }
}
