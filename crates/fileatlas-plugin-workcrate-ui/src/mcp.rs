use crate::{PANEL, view::WorkCrateUiState};
use cordis::{Context, Service};
use fileatlas_contracts::mcp::{McpToolDefinition, McpToolRegistration, McpToolRegistry};
use fileatlas_plugin_workcrate_boards::WorkCrateBoardsService;
use fileatlas_plugin_workcrate_export::WorkCrateExportService;
use fileatlas_plugin_workcrate_projects::WorkCrateProjectsService;
use fileatlas_ui_bridge::panel_catalog::PanelCatalog;
use serde_json::json;
use std::sync::Arc;
mod editing;
mod operations;

pub(super) fn register(
    context: &Context,
    projects: &Arc<WorkCrateProjectsService>,
    boards: &Arc<WorkCrateBoardsService>,
    exports: &Arc<WorkCrateExportService>,
    panels: &Arc<PanelCatalog>,
    ui_state: &Arc<WorkCrateUiState>,
) -> Result<(), String> {
    use operations::{required, tool};
    let Ok(registry) = context.try_service::<McpToolRegistry>() else {
        return Ok(());
    };
    let p = projects.clone();
    let e = exports.clone();
    let surfaces = context
        .try_service::<fileatlas_ui_bridge::UiBridgeService>()
        .map_err(|e| e.to_string())?
        .surfaces
        .clone();
    // Read the shared UI snapshot: querying state must never switch native documents.
    let state = McpToolDefinition {
        name:"workcrate-ui.state".into(),title:"Read cached WorkCrate state".into(),
        description:"Read projects and the current native-artboard UI snapshot without opening documents.".into(),
        input_schema:json!({"type":"object","properties":{"project_id":{"type":"string"},"control_ids":{"type":"array","items":{"type":"string"}}},"additionalProperties":false}).as_object().cloned().unwrap_or_default(),
        mutating:false, confirmation_required:false,
        handler:Arc::new(move |v| {
            let p=p.clone(); let e=e.clone(); let surfaces=surfaces.clone();
            Box::pin(async move {
                let projects = p.projects(true).map_err(|e|e.to_string())?;
                let ui = surfaces.value("workcrate.ui").and_then(|v|serde_json::from_str::<serde_json::Value>(&v).ok());
                let controls = v["control_ids"].as_array().into_iter().flatten().filter_map(|v|v.as_str()).filter_map(|id|surfaces.value(&format!("workcrate.control.{id}")).map(|value|(id.to_owned(),value))).collect::<std::collections::BTreeMap<_,_>>();
                Ok(json!({"projects":projects,"ui":ui,"defaults_form":surfaces.value("workcrate.settings").and_then(|v|serde_json::from_str::<serde_json::Value>(&v).ok()),"controls":controls,"export_progress":e.progress(),"field_presets":p.field_presets().map_err(|e|e.to_string())?,"project_presets":v["project_id"].as_str().map(|id| p.project_field_presets(id).map_err(|e| e.to_string())).transpose()?,"effective_presets":v["project_id"].as_str().map(|id| p.effective_field_presets(id).map_err(|e| e.to_string())).transpose()?}))
            })
        })
    };
    let b = boards.clone();
    let ui = ui_state.clone();
    let panels_open = panels.clone();
    let open = tool(
        "open_board",
        "Show a native artboard preview in FileAtlas",
        json!({"project_id":{"type":"string"},"board_id":{"type":"string"}}),
        vec!["project_id", "board_id"],
        false,
        move |v| {
            let project = required(&v, "project_id")?;
            let board = b
                .load_board(project, required(&v, "board_id")?)
                .map_err(|e| e.to_string())?;
            ui.select_board(project.into(), board.id.clone())?;
            panels_open.open(PANEL).map_err(|e| e.to_string())?;
            Ok(json!({"opened":true,"board":board}))
        },
    );
    let b = boards.clone();
    let ui = ui_state.clone();
    let place = tool(
        "place_image",
        "Place a source file using official VectorCraft",
        json!({"project_id":{"type":"string"},"board_id":{"type":"string"},"path":{"type":"string"}}),
        vec!["project_id", "board_id", "path"],
        false,
        move |v| {
            let board = b
                .load_board(required(&v, "project_id")?, required(&v, "board_id")?)
                .map_err(|e| e.to_string())?;
            b.place_file(&board.id, std::path::Path::new(required(&v, "path")?))
                .map_err(|e| e.to_string())?;
            ui.request_board_refresh();
            Ok(json!({"placed":true,"board_id":board.id}))
        },
    );
    let b = boards.clone();
    let e = exports.clone();
    let export = tool(
        "export_pdf",
        "Export one native artboard to PDF using official VectorCraft",
        json!({"project_id":{"type":"string"},"board_id":{"type":"string"},"path":{"type":"string"},"page_index":{"type":"integer","minimum":0,"maximum":0}}),
        vec!["project_id", "board_id", "path"],
        true,
        move |v| {
            let board = b
                .load_board(required(&v, "project_id")?, required(&v, "board_id")?)
                .map_err(|e| e.to_string())?;
            let result = e
                .export_page_pdf(&board.id, 0, std::path::Path::new(required(&v, "path")?))
                .map_err(|e| e.to_string())?;
            Ok(json!({"exported":true,"path":result.path}))
        },
    );
    let mut definitions = vec![state, open, place, export];
    definitions.extend(operations::tools(
        context, projects, boards, exports, panels, ui_state,
    )?);
    definitions.extend(editing::tools(projects, boards, ui_state));
    let registration = registry
        .register("fileatlas.workcrate-ui", definitions)
        .map_err(|e| e.to_string())?;
    let _publication = context
        .provide(Arc::new(WorkCrateUiMcpRegistration {
            _registration: registration,
        }))
        .map_err(|e| e.to_string())?;
    Ok(())
}
struct WorkCrateUiMcpRegistration {
    _registration: Arc<McpToolRegistration>,
}
impl Service for WorkCrateUiMcpRegistration {
    const NAME: &'static str = "fileatlas/workcrate-ui-mcp-registration";
}
