use super::operations::{required, tool};
use super::*;
use fileatlas_plugin_workcrate_boards::{BoardsError, CanvasGeometry};

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
    let mut tools = vec![presets, configure, refresh, attach, editor];
    tools.extend(canvas_tools(boards, state));
    tools
}

/// Tools that drive the official canvas session. They share the document with the panel,
/// so a call made here reaches the same `.vectorcraft` file and the same running program.
fn canvas_tools(
    boards: &Arc<WorkCrateBoardsService>,
    state: &Arc<WorkCrateUiState>,
) -> Vec<McpToolDefinition> {
    let b = boards.clone();
    let open = tool(
        "canvas_open",
        "Open the official canvas session for one artboard page",
        json!({"board_id":{"type":"string"}}),
        vec!["board_id"],
        false,
        move |v| {
            let snapshot = b
                .canvas_open(required(&v, "board_id")?)
                .map_err(|e| e.to_string())?;
            Ok(json!({"canvas": snapshot}))
        },
    );
    let b = boards.clone();
    let state_read = tool(
        "canvas_state",
        "Read the official canvas snapshot: revision, selection, bounds and preview",
        json!({}),
        vec![],
        false,
        move |_| Ok(json!({"canvas": b.canvas_state().map_err(|e| e.to_string())?})),
    );
    let b = boards.clone();
    let gesture = tool(
        "canvas_gesture",
        "Forward official pointer events (down, drag, up, move, doubleclick) in document points",
        json!({"events":{"type":"array","items":{"type":"object"}}}),
        vec!["events"],
        true,
        move |v| {
            let events = v.get("events").cloned().ok_or("events is required")?;
            Ok(json!({"canvas": b.canvas_gesture(events).map_err(|e| e.to_string())?}))
        },
    );
    let b = boards.clone();
    let bounds = tool(
        "canvas_set_bounds",
        "Move or resize selected objects with official geometry in document points",
        json!({"ids":{"type":"array","items":{"type":"integer"}},"x":{"type":"number"},"y":{"type":"number"},"width":{"type":"number"},"height":{"type":"number","exclusiveMinimum":0}}),
        vec!["ids", "x", "y", "width", "height"],
        true,
        move |v| {
            let geometry = CanvasGeometry {
                x: number(&v, "x")?,
                y: number(&v, "y")?,
                width: number(&v, "width")?,
                height: number(&v, "height")?,
            };
            Ok(
                json!({"canvas": b.canvas_set_bounds(&identifiers(&v)?, &geometry).map_err(|e| e.to_string())?}),
            )
        },
    );
    let b = boards.clone();
    let text = tool(
        "canvas_set_text",
        "Replace one text object's content through official VectorCraft",
        json!({"id":{"type":"integer"},"text":{"type":"string"}}),
        vec!["id", "text"],
        true,
        move |v| {
            let id = number(&v, "id")? as u64;
            let text = v["text"]
                .as_str()
                .ok_or("text must be a string")?
                .to_owned();
            Ok(json!({"canvas": b.canvas_set_text(id, &text).map_err(|e| e.to_string())?}))
        },
    );
    let b = boards.clone();
    let create = tool(
        "canvas_create_text",
        "Create a text object at one document point through official VectorCraft",
        json!({"x":{"type":"number"},"y":{"type":"number"},"text":{"type":"string"},"size":{"type":"number","minimum":1}}),
        vec!["x", "y", "text"],
        true,
        move |v| {
            let text = v["text"]
                .as_str()
                .ok_or("text must be a string")?
                .to_owned();
            let size = v["size"].as_f64().unwrap_or(18.);
            Ok(
                json!({"canvas": b.canvas_create_text(number(&v,"x")?, number(&v,"y")?, &text, size).map_err(|e| e.to_string())?}),
            )
        },
    );
    let b = boards.clone();
    let place = tool(
        "canvas_place",
        "Place an image file on the canvas, embedded or linked to its source file",
        json!({"path":{"type":"string"},"at":{"type":"array","items":{"type":"number"},"minItems":2,"maxItems":2},"link":{"type":"boolean"}}),
        vec!["path", "at"],
        true,
        move |v| {
            let at = pair(&v, "at")?;
            let link = v["link"].as_bool().unwrap_or(true);
            let path = std::path::Path::new(required(&v, "path")?);
            Ok(json!({"canvas": b.canvas_place(path, at, link).map_err(|e| e.to_string())?}))
        },
    );
    let b = boards.clone();
    let links = tool(
        "canvas_links",
        "Update linked images from their files or relink them to another file",
        json!({"action":{"enum":["update","relink"]},"ids":{"type":"array","items":{"type":"integer"}},"path":{"type":"string"}}),
        vec!["action", "ids"],
        true,
        move |v| {
            let ids = identifiers(&v)?;
            let snapshot = match required(&v, "action")? {
                "update" => b.canvas_links_update(&ids),
                "relink" => {
                    b.canvas_links_relink(&ids, std::path::Path::new(required(&v, "path")?))
                }
                other => Err(BoardsError::Invalid(format!("unknown link action {other}"))),
            };
            Ok(json!({"canvas": snapshot.map_err(|e| e.to_string())?}))
        },
    );
    let b = boards.clone();
    let step = tool(
        "canvas_step",
        "Undo or redo one official canvas operation",
        json!({"action":{"enum":["undo","redo"]}}),
        vec!["action"],
        true,
        move |v| {
            let snapshot = match required(&v, "action")? {
                "undo" => b.canvas_undo(),
                _ => b.canvas_redo(),
            };
            Ok(json!({"canvas": snapshot.map_err(|e| e.to_string())?}))
        },
    );
    let b = boards.clone();
    let close_ui = state.clone();
    let close = tool(
        "canvas_close",
        "Save and close the official canvas session, then refresh the page catalogue",
        json!({"project_id":{"type":"string"}}),
        vec!["project_id"],
        false,
        move |v| {
            let _ = b.canvas_commit();
            b.canvas_close().map_err(|e| e.to_string())?;
            let catalog = b
                .refresh_project(required(&v, "project_id")?)
                .map_err(|e| e.to_string())?;
            close_ui.request_catalog_refresh();
            Ok(json!({"closed": true, "boards": catalog}))
        },
    );
    vec![
        open, state_read, gesture, bounds, text, create, place, links, step, close,
    ]
}

fn number(value: &serde_json::Value, key: &str) -> Result<f64, String> {
    value
        .get(key)
        .and_then(|v| v.as_f64())
        .ok_or_else(|| format!("{key} must be a number"))
}

fn pair(value: &serde_json::Value, key: &str) -> Result<[f64; 2], String> {
    let items = value
        .get(key)
        .and_then(|v| v.as_array())
        .filter(|items| items.len() == 2)
        .ok_or_else(|| format!("{key} must be two numbers"))?;
    Ok([
        items[0]
            .as_f64()
            .ok_or_else(|| format!("{key} must be numbers"))?,
        items[1]
            .as_f64()
            .ok_or_else(|| format!("{key} must be numbers"))?,
    ])
}

fn identifiers(value: &serde_json::Value) -> Result<Vec<u64>, String> {
    let ids = value
        .get("ids")
        .and_then(|v| v.as_array())
        .filter(|ids| !ids.is_empty())
        .ok_or_else(|| "ids must be a non-empty array of object ids".to_owned())?;
    ids.iter()
        .map(|id| {
            id.as_u64()
                .ok_or_else(|| "ids must contain object ids".to_owned())
        })
        .collect()
}
