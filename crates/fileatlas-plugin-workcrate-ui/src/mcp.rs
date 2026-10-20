use crate::{PANEL, view::WorkCrateUiState};
use cordis::{Context, Service};
use fileatlas_contracts::mcp::{McpToolDefinition, McpToolRegistration, McpToolRegistry};
use fileatlas_plugin_workcrate_boards::WorkCrateBoardsService;
use fileatlas_plugin_workcrate_export::WorkCrateExportService;
use fileatlas_plugin_workcrate_projects::WorkCrateProjectsService;
use fileatlas_ui_bridge::panel_catalog::PanelCatalog;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
mod operations;

pub(super) fn register(
    context: &Context,
    projects: &Arc<WorkCrateProjectsService>,
    boards: &Arc<WorkCrateBoardsService>,
    exports: &Arc<WorkCrateExportService>,
    panels: &Arc<PanelCatalog>,
    ui_state: &Arc<WorkCrateUiState>,
) -> Result<(), String> {
    let Ok(registry) = context.try_service::<McpToolRegistry>() else {
        return Ok(());
    };
    let state_projects = Arc::clone(projects);
    let state_boards = Arc::clone(boards);
    let state_surfaces = Arc::clone(
        &context
            .try_service::<fileatlas_ui_bridge::UiBridgeService>()
            .map_err(|e| e.to_string())?
            .surfaces,
    );
    let state = McpToolDefinition {
        name: "workcrate-ui.state".into(),
        title: "Read WorkCrate project and board catalog".into(),
        description: "Read project names, board counts and board metadata from WorkCrate. Does not include image bytes or file contents.".into(),
        input_schema: json!({"type":"object","properties":{},"additionalProperties":false}).as_object().cloned().unwrap_or_default(),
        mutating: false,
        confirmation_required: false,
        handler: Arc::new(move |_| {
            let projects = Arc::clone(&state_projects);
            let boards = Arc::clone(&state_boards);
            let surfaces=Arc::clone(&state_surfaces);
            Box::pin(async move {
                let mut values = Vec::new();
                for project in projects.projects(true).map_err(|error| error.to_string())? {
                    let references = projects.references(&project.id).map_err(|error| error.to_string())?;
                    let project_boards = boards.board_catalog(&project.id).map_err(|error| error.to_string())?;
                    values.push(json!({
                        "id": project.id,
                        "name": project.name,
                        "archived":project.archived,"main_image":project.main_image,
                        "reference_count": references.len(),
                        "boards": project_boards.iter().map(|board| json!({
                            "id": board.id,
                            "title": board.title,
                            "width_pt": board.source.settings.page_width,
                            "height_pt": board.source.settings.page_height,
                            "image_count": board.placements.len(),
                            "info":board.info,
                        })).collect::<Vec<_>>(),
                    }));
                }
                let ui=surfaces.value("workcrate.ui").and_then(|v|serde_json::from_str::<serde_json::Value>(&v).ok());
                let controls=["artboard","canvas-crop-mode","canvas-restore-crop","canvas-refresh-project","canvas-add-image","canvas-confirm-images"].into_iter().filter_map(|id| surfaces.value(&format!("workcrate.control.{id}")).map(|v|(id,v))).collect::<std::collections::BTreeMap<_,_>>();
                Ok(json!({"projects": values,"ui":ui,"controls":controls}))
            })
        }),
    };
    let open_panels = Arc::clone(panels);
    let open_boards = Arc::clone(boards);
    let open_state = Arc::clone(ui_state);
    let open_board = McpToolDefinition {
        name: "workcrate-ui.open_board".into(),
        title: "Open a WorkCrate board".into(),
        description: "Open an existing project board in the WorkCrate editor in the current FileAtlas process.".into(),
        input_schema: json!({
            "type":"object","properties":{
                "project_id":{"type":"string","minLength":1},
                "board_id":{"type":"string","minLength":1}
            },"required":["project_id","board_id"],"additionalProperties":false
        }).as_object().cloned().unwrap_or_default(),
        mutating: true,
        confirmation_required: false,
        handler: Arc::new(move |arguments| {
            let request = serde_json::from_value::<OpenBoardRequest>(arguments)
                .map_err(|error| format!("invalid parameters: {error}"));
            let boards = Arc::clone(&open_boards);
            let panels = Arc::clone(&open_panels);
            let ui_state = Arc::clone(&open_state);
            Box::pin(async move {
                let request = request?;
                let board = boards.load_board(&request.project_id, &request.board_id).map_err(|error| error.to_string())?;
                ui_state.select_board(request.project_id, board.id.clone())?;
                let panel_instance = panels.open(PANEL).map_err(|error| error.to_string())?;
                Ok(json!({"opened":true,"board_id":board.id,"panel_instance":panel_instance}))
            })
        }),
    };
    let place_projects = Arc::clone(projects);
    let place_boards = Arc::clone(boards);
    let place_panels = Arc::clone(panels);
    let place_state = Arc::clone(ui_state);
    let export_service = Arc::clone(exports);
    let export_boards = Arc::clone(boards);
    let export = McpToolDefinition {
        name: "workcrate-ui.export_pdf".into(),
        title: "Export a WorkCrate board to PDF".into(),
        description: "Render one board from original source images and write a PDF to the requested path. File writes require Full MCP access and confirm=true.".into(),
        input_schema: json!({
            "type":"object","properties":{
                "project_id":{"type":"string","minLength":1},
                "board_id":{"type":"string","minLength":1},
                "path":{"type":"string","minLength":1},
                "page_index":{"type":"integer","minimum":0,"default":0}
            },"required":["project_id","board_id","path"],"additionalProperties":false
        }).as_object().cloned().unwrap_or_default(),
        mutating: true,
        confirmation_required: true,
        handler: Arc::new(move |arguments| {
            let request = serde_json::from_value::<ExportRequest>(arguments)
                .map_err(|error| format!("invalid parameters: {error}"));
            let exports = Arc::clone(&export_service);
            let boards = Arc::clone(&export_boards);
            Box::pin(async move {
                let request = request?;
                boards.load_board(&request.project_id, &request.board_id).map_err(|error| error.to_string())?;
                let path = std::path::PathBuf::from(&request.path);
                if path.extension().is_none_or(|extension| !extension.eq_ignore_ascii_case("pdf")) {
                    return Err("output path must use the .pdf extension".into());
                }
                let result = exports.export_page_pdf(&request.board_id, request.page_index, &path).map_err(|error| error.to_string())?;
                Ok(json!({"exported":true,"path":result.path,"dpi":result.dpi,"width_px":result.width_px,"height_px":result.height_px}))
            })
        }),
    };
    let place_image = McpToolDefinition {
        name: "workcrate-ui.place_image".into(),
        title: "Place a source image on a WorkCrate board".into(),
        description: "Add a JPEG/JPG/PNG path as a project reference, place its original source pixels on the board, and show that board in the editor. This changes WorkCrate data only and never writes to the source image.".into(),
        input_schema: json!({
            "type":"object","properties":{
                "project_id":{"type":"string","minLength":1},
                "board_id":{"type":"string","minLength":1},
                "path":{"type":"string","minLength":1},
                "x_pt":{"type":"number","default":24},
                "y_pt":{"type":"number","default":24},
                "width_pt":{"type":"number","default":420}
            },"required":["project_id","board_id","path"],"additionalProperties":false
        }).as_object().cloned().unwrap_or_default(),
        mutating: true,
        confirmation_required: false,
        handler: Arc::new(move |arguments| {
            let request = serde_json::from_value::<PlaceImageRequest>(arguments)
                .map_err(|error| format!("invalid parameters: {error}"));
            let projects = Arc::clone(&place_projects);
            let boards = Arc::clone(&place_boards);
            let panels = Arc::clone(&place_panels);
            let ui_state = Arc::clone(&place_state);
            Box::pin(async move {
                let request = request?;
                let path = std::path::PathBuf::from(&request.path);
                if !path.is_file() { return Err("source image does not exist".into()); }
                let existing = boards.load_board(&request.project_id, &request.board_id).map_err(|error| error.to_string())?;
                let references = projects.add_references(&request.project_id, std::slice::from_ref(&path)).map_err(|error| error.to_string())?;
                let canonical = std::fs::canonicalize(&path).unwrap_or(path.clone());
                let reference = references.into_iter().find(|reference| {
                    std::fs::canonicalize(&reference.path).unwrap_or_else(|_| std::path::PathBuf::from(&reference.path)) == canonical
                }).ok_or_else(|| "could not resolve the source image reference".to_owned())?;
                let board = boards.place_reference(
                    &request.board_id,
                    &reference.id,
                    request.x_pt.unwrap_or(24.0),
                    request.y_pt.unwrap_or(24.0),
                    request.width_pt.unwrap_or(420.0),
                    false,
                ).map_err(|error| error.to_string())?;
                ui_state.select_board(request.project_id, board.id.clone())?;
                let _ = panels.open(PANEL).map_err(|error| error.to_string())?;
                Ok(json!({"placed":true,"board_id":board.id,"project_id":existing.project_id,"reference_id":reference.id,"image_count":board.placements.len()}))
            })
        }),
    };
    let mut definitions = vec![state, open_board, place_image, export];
    definitions.extend(operations::tools(
        context, projects, boards, exports, panels, ui_state,
    )?);
    let registration = registry
        .register("fileatlas.workcrate-ui", definitions)
        .map_err(|error| error.to_string())?;
    let _publication = context
        .provide(Arc::new(WorkCrateUiMcpRegistration {
            _registration: registration,
        }))
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OpenBoardRequest {
    project_id: String,
    board_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlaceImageRequest {
    project_id: String,
    board_id: String,
    path: String,
    x_pt: Option<f64>,
    y_pt: Option<f64>,
    width_pt: Option<f64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportRequest {
    project_id: String,
    board_id: String,
    path: String,
    #[serde(default)]
    page_index: usize,
}

struct WorkCrateUiMcpRegistration {
    _registration: Arc<McpToolRegistration>,
}

impl Service for WorkCrateUiMcpRegistration {
    const NAME: &'static str = "fileatlas/workcrate-ui-mcp-registration";
}
