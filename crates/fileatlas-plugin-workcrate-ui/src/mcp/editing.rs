use super::operations::{required, tool};
use super::*;

pub(super) fn tools(
    projects: &Arc<WorkCrateProjectsService>,
    boards: &Arc<WorkCrateBoardsService>,
    state: &Arc<WorkCrateUiState>,
) -> Vec<McpToolDefinition> {
    let p = projects.clone();
    let b = boards.clone();
    let ui = state.clone();
    let text = tool(
        "text",
        "Add/update/delete native multiline text frames",
        json!({"action":{"enum":["add","update","delete","read"]},"project_id":{"type":"string"},"board_id":{"type":"string"},"text_id":{"type":"string"},"text":{"type":"string"},"size_pt":{"type":"number"},"color":{"type":"string"},"frame":{"type":"array","items":{"type":"number"},"minItems":4,"maxItems":4}}),
        vec!["action", "project_id", "board_id"],
        false,
        move |v| {
            let project = required(&v, "project_id")?;
            let id = required(&v, "board_id")?;
            let catalog = b.board_catalog(project).map_err(|e| e.to_string())?;
            if !catalog.iter().any(|board| board.id == id) {
                return Err("页面不属于该项目".into());
            }
            let board = b.board(id).map_err(|e| e.to_string())?;
            let result = match required(&v, "action")? {
                "add" => b.add_text(id),
                "delete" => b.delete_text(id, required(&v, "text_id")?),
                "update" => {
                    let key = required(&v, "text_id")?;
                    let old = board.texts.get(key).ok_or("文本不存在")?;
                    let frame = if v.get("frame").is_some() {
                        serde_json::from_value::<[f64; 4]>(v["frame"].clone())
                            .map_err(|e| e.to_string())?
                    } else {
                        board.text_frame(key).map_err(|e| e.to_string())?
                    };
                    b.update_text(
                        id,
                        key,
                        v["text"].as_str().unwrap_or(&old.text),
                        v["size_pt"].as_f64().unwrap_or(old.size_pt),
                        v["color"].as_str().unwrap_or(&old.color),
                        frame,
                    )
                }
                _ => Ok(board),
            }
            .map_err(|e| e.to_string())?;
            let fonts = b.text_fonts(id).map_err(|e| e.to_string())?;
            let frames = result
                .texts
                .keys()
                .filter_map(|key| result.text_frame(key).ok().map(|f| (key.clone(), f)))
                .collect::<std::collections::BTreeMap<_, _>>();
            if required(&v, "action")? != "read" {
                ui.select_board(project.into(), id.into())?;
                ui.request_board_refresh();
            }
            Ok(json!({"texts":result.texts,"frames":frames,"font":"Microsoft YaHei","fonts":fonts}))
        },
    );
    let b = boards.clone();
    let ui = state.clone();
    let image = tool(
        "replace_image",
        "Replace or update a linked image, preserving its crop frame",
        json!({"project_id":{"type":"string"},"board_id":{"type":"string"},"asset":{"type":"string"},"path":{"type":"string"}}),
        vec!["project_id", "board_id", "asset"],
        false,
        move |v| {
            let project = required(&v, "project_id")?;
            let id = required(&v, "board_id")?;
            if !b
                .board_catalog(project)
                .map_err(|e| e.to_string())?
                .iter()
                .any(|board| board.id == id)
            {
                return Err("页面不属于该项目".into());
            }
            let replacement = v["path"].as_str().map(std::path::Path::new);
            let board = b
                .refresh_image(id, required(&v, "asset")?, replacement)
                .map_err(|e| e.to_string())?;
            ui.select_board(project.into(), id.into())?;
            ui.request_board_refresh();
            Ok(
                json!({"updated":true,"layout":board.image_layout(required(&v,"asset")?).map_err(|e|e.to_string())?}),
            )
        },
    );
    let presets = tool(
        "field_presets",
        "Save WorkCrate field preset settings",
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
    let ui_options = state.clone();
    let options = tool(
        "text_options",
        "Read or update pinned native character/paragraph/frame/OpenType settings",
        json!({"action":{"enum":["read","update"]},"project_id":{"type":"string"},"board_id":{"type":"string"},"text_id":{"type":"string"},"range":{"type":"array","items":{"type":"integer"},"minItems":2,"maxItems":2},"scope":{"enum":["chars","para","frame","openType","action"]},"attrs":{"type":"object"}}),
        vec!["action", "project_id", "board_id", "text_id"],
        false,
        move |v| {
            let project = required(&v, "project_id")?;
            let board = required(&v, "board_id")?;
            let text = required(&v, "text_id")?;
            if !b
                .board_catalog(project)
                .map_err(|e| e.to_string())?
                .iter()
                .any(|p| p.id == board)
            {
                return Err("页面不属于该项目".into());
            }
            let range = v
                .get("range")
                .map(|r| serde_json::from_value::<[usize; 2]>(r.clone()).map_err(|e| e.to_string()))
                .transpose()?;
            if required(&v, "action")? == "update" {
                b.set_text_options(
                    board,
                    text,
                    range,
                    required(&v, "scope")?,
                    v["attrs"].clone(),
                )
                .map_err(|e| e.to_string())?;
                ui_options.select_board(project.into(), board.into())?;
                ui_options.request_board_refresh();
            }
            b.text_options(board, text, range)
                .map_err(|e| e.to_string())
        },
    );
    vec![text, image, presets, options]
}
