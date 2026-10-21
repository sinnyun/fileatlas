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
        json!({"executable":{"type":"string"},"port":{"type":"integer","minimum":1,"maximum":65535}}),
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
    let connect_ui = state.clone();
    let connect = tool(
        "connect_vectorcraft",
        "Connect to or launch official VectorCraft",
        json!({}),
        vec![],
        false,
        move |_| {
            let result = b.connect().map_err(|e| e.to_string());
            connect_ui.set_connection(result.is_ok());
            connect_ui.request_catalog_refresh();
            result
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
    let b = boards.clone();
    let save_ui = state.clone();
    let save = tool(
        "save_project",
        "Save the associated native VectorCraft document",
        json!({"project_id":{"type":"string"}}),
        vec!["project_id"],
        true,
        move |v| {
            b.save_project(required(&v, "project_id")?)
                .map_err(|e| e.to_string())?;
            save_ui.request_board_refresh();
            Ok(json!({"saved":true}))
        },
    );
    vec![presets, configure, connect, attach, editor, save]
}
