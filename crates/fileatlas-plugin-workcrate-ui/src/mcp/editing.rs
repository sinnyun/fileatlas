use super::operations::{required, tool};
use super::*;

pub(super) fn tools(
    projects: &Arc<WorkCrateProjectsService>,
    boards: &Arc<WorkCrateBoardsService>,
    state: &Arc<WorkCrateUiState>,
) -> Vec<McpToolDefinition> {
    let p = projects.clone();
    let presets = tool(
        "field_presets",
        "Save material field presets",
        json!({"presets":{"type":"object"}}),
        vec!["presets"],
        false,
        move |v| {
            let presets =
                serde_json::from_value(v["presets"].clone()).map_err(|e| e.to_string())?;
            p.save_field_presets(presets).map_err(|e| e.to_string())?;
            Ok(json!({"saved":true}))
        },
    );
    let b = boards.clone();
    let config_ui = state.clone();
    let configure = tool(
        "configure_vectorcraft",
        "Configure official VectorCraft executable and local control port",
        json!({"executable":{"type":"string"},"cli_executable":{"type":"string"},"port":{"type":"integer","minimum":1,"maximum":65535}}),
        vec!["executable", "port"],
        false,
        move |v| {
            let config = serde_json::from_value(v).map_err(|e| e.to_string())?;
            b.configure(config).map_err(|e| e.to_string())?;
            config_ui.set_connection(false);
            Ok(json!({"configured":true}))
        },
    );
    let b = boards.clone();
    let refresh_ui = state.clone();
    let refresh = tool(
        "refresh_project",
        "Synchronize saved native document and local previews without opening the editor",
        json!({"project_id":{"type":"string"}}),
        vec!["project_id"],
        false,
        move |v| {
            let catalog = b
                .refresh_project(required(&v, "project_id")?)
                .map_err(|e| e.to_string())?;
            refresh_ui.request_catalog_refresh();
            Ok(json!({"refreshed":true,"boards":catalog}))
        },
    );
    let b = boards.clone();
    let ui = state.clone();
    let attach = tool(
        "attach_document",
        "Associate a native VectorCraft document with a project",
        json!({"project_id":{"type":"string"},"path":{"type":"string"}}),
        vec!["project_id", "path"],
        false,
        move |v| {
            let project = required(&v, "project_id")?;
            b.attach_document(project, std::path::Path::new(required(&v, "path")?))
                .map_err(|e| e.to_string())?;
            ui.request_catalog_refresh();
            Ok(json!({"attached":true}))
        },
    );
    let b = boards.clone();
    let ui = state.clone();
    let editor = tool(
        "open_editor",
        "Edit a native artboard in official VectorCraft",
        json!({"project_id":{"type":"string"},"board_id":{"type":"string"}}),
        vec!["project_id", "board_id"],
        false,
        move |v| {
            let board = b
                .load_board(required(&v, "project_id")?, required(&v, "board_id")?)
                .map_err(|e| e.to_string())?;
            b.open_editor(&board.id).map_err(|e| e.to_string())?;
            ui.request_board_refresh();
            Ok(json!({"opened":true}))
        },
    );
    vec![presets, configure, refresh, attach, editor]
}
