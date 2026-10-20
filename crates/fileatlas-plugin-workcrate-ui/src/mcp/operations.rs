use super::*;

pub(super) fn tools(
    context: &Context,
    projects: &Arc<WorkCrateProjectsService>,
    boards: &Arc<WorkCrateBoardsService>,
    exports: &Arc<WorkCrateExportService>,
    panels: &Arc<PanelCatalog>,
    state: &Arc<WorkCrateUiState>,
) -> Result<Vec<McpToolDefinition>, String> {
    let p = Arc::clone(projects);
    let b = Arc::clone(boards);
    let ui = Arc::clone(state);
    let panels = Arc::clone(panels);
    let navigate = tool(
        "navigate",
        "Navigate WorkCrate project, page list or editor",
        json!({"project_id":{"type":"string"},"board_id":{"type":"string"}}),
        vec![],
        false,
        move |v| {
            let project = v
                .get("project_id")
                .and_then(|v| v.as_str())
                .map(str::to_owned);
            let board = v
                .get("board_id")
                .and_then(|v| v.as_str())
                .map(str::to_owned);
            if let Some(id) = &project {
                p.project_directory(id).map_err(|e| e.to_string())?;
            }
            if let Some(id) = &board {
                b.load_board(project.as_deref().ok_or("project_id is required")?, id)
                    .map_err(|e| e.to_string())?;
            }
            ui.navigate(project, board)?;
            panels.open(PANEL).map_err(|e| e.to_string())?;
            Ok(json!({"navigated":true}))
        },
    );
    let p = Arc::clone(projects);
    let b = Arc::clone(boards);
    let thumbnails = context
        .try_service::<fileatlas_plugin_thumbnails::ThumbnailService>()
        .map_err(|e| e.to_string())?;
    let manage = tool(
        "manage_project",
        "Create, rename, duplicate, archive, restore or choose a project cover",
        json!({"action":{"enum":["create","rename","duplicate","archive","restore","cover"]},"project_id":{"type":"string"},"name":{"type":"string"},"path":{"type":"string"}}),
        vec!["action"],
        false,
        move |v| {
            let action = required(&v, "action")?;
            if action == "create" {
                let project = p
                    .create_project(required(&v, "name")?)
                    .map_err(|e| e.to_string())?;
                return Ok(json!({"project":project}));
            }
            let id = required(&v, "project_id")?;
            match action {
                "rename" => {
                    p.rename_project(id, required(&v, "name")?)
                        .map_err(|e| e.to_string())?;
                }
                "archive" | "restore" => {
                    p.set_archived(id, action == "archive")
                        .map_err(|e| e.to_string())?;
                }
                "cover" => {
                    p.set_main_image(id, std::path::Path::new(required(&v, "path")?), &thumbnails)
                        .map_err(|e| e.to_string())?;
                }
                "duplicate" => {
                    let copy = p
                        .duplicate_project(id, required(&v, "name")?)
                        .map_err(|e| e.to_string())?;
                    b.duplicate_project_pages(id, &copy.id)
                        .map_err(|e| e.to_string())?;
                    return Ok(json!({"project":copy}));
                }
                _ => return Err("unsupported action".into()),
            }
            Ok(json!({"updated":true}))
        },
    );
    let p = Arc::clone(projects);
    let delete = tool(
        "delete_project",
        "Remove project to retained WorkCrate trash; original linked files remain untouched",
        json!({"project_id":{"type":"string"}}),
        vec!["project_id"],
        true,
        move |v| {
            let trash = p
                .delete_project(required(&v, "project_id")?)
                .map_err(|e| e.to_string())?;
            Ok(json!({"deleted":true,"retained_at":trash}))
        },
    );
    let b = Arc::clone(boards);
    let create_page = tool(
        "create_page",
        "Create an A3 landscape page",
        json!({"project_id":{"type":"string"}}),
        vec!["project_id"],
        false,
        move |v| {
            let board = b
                .create_page(required(&v, "project_id")?)
                .map_err(|e| e.to_string())?;
            Ok(
                json!({"board_id":board.id,"info":board.info,"width_pt":board.source.settings.page_width,"height_pt":board.source.settings.page_height}),
            )
        },
    );
    let b = Arc::clone(boards);
    let ui = Arc::clone(state);
    let update = tool(
        "update_page",
        "Edit a page name, physical size, project-name override and content fields",
        json!({"project_id":{"type":"string"},"board_id":{"type":"string"},"title":{"type":"string"},"info":{"type":"object"}}),
        vec!["project_id", "board_id"],
        false,
        move |v| {
            let mut board = b
                .load_board(required(&v, "project_id")?, required(&v, "board_id")?)
                .map_err(|e| e.to_string())?;
            if let Some(title) = v.get("title").and_then(|v| v.as_str()) {
                board.title = title.into();
                board.info.auto_from_first = false;
            }
            if let Some(patch) = v.get("info").and_then(|v| v.as_object()) {
                let mut value = serde_json::to_value(&board.info).map_err(|e| e.to_string())?;
                for (key, val) in patch {
                    value[key] = val.clone();
                }
                board.info = serde_json::from_value(value).map_err(|e| e.to_string())?;
            }
            let saved = b
                .update_info(&board.id, &board.title, board.info)
                .map_err(|e| e.to_string())?;
            ui.select_board(saved.project_id.clone(), saved.id.clone())?;
            Ok(json!({"updated":true,"title":saved.title,"info":saved.info}))
        },
    );
    let b = Arc::clone(boards);
    let e = Arc::clone(exports);
    let batch = tool(
        "export_pages",
        "Export ordered project pages to one PDF or an individual page to a PNG/JPEG",
        json!({"project_id":{"type":"string"},"board_ids":{"type":"array","items":{"type":"string"}},"path":{"type":"string"},"format":{"enum":["pdf","image"]}}),
        vec!["project_id", "board_ids", "path", "format"],
        true,
        move |v| {
            let ids = v["board_ids"]
                .as_array()
                .ok_or("board_ids must be an array")?
                .iter()
                .map(|v| v.as_str().map(str::to_owned).ok_or("invalid board id"))
                .collect::<Result<Vec<_>, _>>()?;
            for id in &ids {
                b.load_board(required(&v, "project_id")?, id)
                    .map_err(|e| e.to_string())?;
            }
            let path = std::path::Path::new(required(&v, "path")?);
            if required(&v, "format")? == "pdf" {
                let count = e.export_pages_pdf(&ids, path).map_err(|e| e.to_string())?;
                Ok(json!({"exported":true,"page_count":count}))
            } else {
                if ids.len() != 1 {
                    return Err("image export needs exactly one board id".into());
                }
                let page = e
                    .export_page_image(&ids[0], path)
                    .map_err(|e| e.to_string())?;
                Ok(json!({"exported":true,"width_px":page.width_px,"height_px":page.height_px}))
            }
        },
    );
    let b = Arc::clone(boards);
    let ui = Arc::clone(state);
    let links = tool(
        "image_links",
        "Set an extended source-file link or refresh all linked images in a project",
        json!({"action":{"enum":["link_source","refresh_project"]},"project_id":{"type":"string"},"board_id":{"type":"string"},"asset":{"type":"string"},"path":{"type":"string"}}),
        vec!["action", "project_id"],
        false,
        move |v| {
            let project = required(&v, "project_id")?;
            let result = if required(&v, "action")? == "refresh_project" {
                serde_json::to_value(
                    b.refresh_project_images(project)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?
            } else {
                let board = required(&v, "board_id")?;
                b.load_board(project, board).map_err(|e| e.to_string())?;
                let path =
                    std::fs::canonicalize(required(&v, "path")?).map_err(|e| e.to_string())?;
                if !path.is_file() {
                    return Err("source link must be a file".into());
                }
                b.link_source(
                    board,
                    required(&v, "asset")?,
                    Some(path.to_string_lossy().into_owned()),
                )
                .map_err(|e| e.to_string())?;
                json!({"linked":true})
            };
            if let Some((selected, board)) = ui.selected_board() {
                if selected == project {
                    ui.navigate(Some(selected), (!board.is_empty()).then_some(board))?;
                }
            }
            Ok(result)
        },
    );
    Ok(vec![
        navigate,
        manage,
        delete,
        create_page,
        update,
        batch,
        links,
    ])
}

fn required<'a>(value: &'a serde_json::Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| format!("{key} is required"))
}

fn tool(
    name: &str,
    title: &str,
    properties: serde_json::Value,
    required: Vec<&str>,
    confirmation: bool,
    handler: impl Fn(serde_json::Value) -> Result<serde_json::Value, String> + Send + Sync + 'static,
) -> McpToolDefinition {
    let handler = Arc::new(handler);
    McpToolDefinition{name:format!("workcrate-ui.{name}"),title:title.into(),description:title.into(),input_schema:json!({"type":"object","properties":properties,"required":required,"additionalProperties":false}).as_object().cloned().unwrap_or_default(),mutating:true,confirmation_required:confirmation,handler:Arc::new(move|v|{let handler=Arc::clone(&handler);Box::pin(async move{handler(v)})})}
}
