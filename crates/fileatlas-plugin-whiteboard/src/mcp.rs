use crate::{OWNER, PANEL, UPSTREAM, WhiteboardService, images::ImageCacheStats};
use cordis::Context;
use fileatlas_contracts::mcp::{
    McpToolDefinition, McpToolFuture, McpToolRegistration, McpToolRegistry,
};
use fileatlas_ui_bridge::panel_catalog::PanelCatalog;
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub fn register(
    context: &Context,
    service: Arc<WhiteboardService>,
    panels: Arc<PanelCatalog>,
    cache_stats: Arc<Mutex<ImageCacheStats>>,
) -> Result<(), String> {
    let Ok(registry) = context.try_service::<McpToolRegistry>() else {
        return Ok(());
    };
    let mut tools = vec![];
    let state = Arc::clone(&service);
    tools.push(tool("whiteboard.state", "Read the managed whiteboards and unchanged upstream scene data", false, false, json!({}), move |_| {
        let s = Arc::clone(&state); Box::pin(async move { Ok(json!({"upstream":UPSTREAM,"library":s.snapshot(),"save":s.status(),"path":s.root.join("library.json")})) })
    }));
    let stats = Arc::clone(&cache_stats);
    tools.push(tool(
        "whiteboard.cache_stats",
        "Read image cache entries, decoded memory, quality tiers, and original cache lifecycle counters",
        false,
        false,
        json!({}),
        move |_| {
            let snapshot = stats
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone();
            Box::pin(async move {
                Ok(json!({
                    "cache": snapshot,
                    "memory_note": "Decoded RGBA bitmap estimate; excludes GPUI/GPU and allocator overhead",
                }))
            })
        },
    ));
    let panel_actions = Arc::clone(&service);
    let panel_catalog = Arc::clone(&panels);
    tools.push(tool(
        "whiteboard.panel",
        "Open or close one of the four canvas drawers through the live canvas host",
        true,
        false,
        json!({"side":{"type":"string","enum":["left","right"]},"panel":{"type":"string","enum":["boards","resources","layers","object","close"]}}),
        move |v| {
            let service = Arc::clone(&panel_actions);
            let panel_catalog = Arc::clone(&panel_catalog);
            Box::pin(async move {
                let side = v["side"].as_str().ok_or("side is required")?;
                let panel = v["panel"].as_str().ok_or("panel is required")?;
                let action = match (side, panel) {
                    ("left", "boards") => crate::store::CanvasPanelAction::LeftBoards,
                    ("left", "resources") => crate::store::CanvasPanelAction::LeftResources,
                    ("left", "close") => crate::store::CanvasPanelAction::LeftClose,
                    ("right", "layers") => crate::store::CanvasPanelAction::RightLayers,
                    ("right", "object") => crate::store::CanvasPanelAction::RightObject,
                    ("right", "close") => crate::store::CanvasPanelAction::RightClose,
                    _ => return Err("panel does not belong to the requested side".into()),
                };
                service.queue_panel_action(action)?;
                panel_catalog.open(PANEL)?;
                Ok(json!({"queued":true,"side":side,"panel":panel}))
            })
        },
    ));
    let tool_groups = Arc::clone(&service);
    let panel_catalog = Arc::clone(&panels);
    tools.push(tool(
        "whiteboard.tool_group",
        "Open a canvas tool-group flyout in the fixed panel above the bottom toolbar",
        true,
        false,
        json!({"group":{"type":"string","enum":["shapes","lines","pages_images"]}}),
        move |v| {
            let service = Arc::clone(&tool_groups);
            let panels = Arc::clone(&panel_catalog);
            Box::pin(async move {
                let group = match v["group"].as_str().ok_or("group is required")? {
                    "shapes" => fileatlas_canvas_engine::ToolGroup::Shapes,
                    "lines" => fileatlas_canvas_engine::ToolGroup::Lines,
                    "pages_images" => fileatlas_canvas_engine::ToolGroup::PagesImages,
                    _ => return Err("unsupported tool group".into()),
                };
                let action = crate::store::CanvasToolGroupAction { group };
                service.queue_tool_group_action(action)?;
                panels.open(PANEL)?;
                Ok(json!({"queued":true,"group":v["group"]}))
            })
        },
    ));
    let toolbar_menus = Arc::clone(&service);
    let panel_catalog = Arc::clone(&panels);
    tools.push(tool(
        "whiteboard.toolbar_menu",
        "Open a color, font, or template control through the live canvas engine",
        true,
        false,
        json!({"menu":{"type":"string","enum":["color","font","templates"]}}),
        move |v| {
            let service = Arc::clone(&toolbar_menus);
            let panels = Arc::clone(&panel_catalog);
            Box::pin(async move {
                let name = v["menu"].as_str().ok_or("menu is required")?;
                let action = match name {
                    "color" => crate::store::CanvasToolbarMenuAction::Color,
                    "font" => crate::store::CanvasToolbarMenuAction::Font,
                    "templates" => crate::store::CanvasToolbarMenuAction::Templates,
                    _ => return Err("unsupported toolbar menu".into()),
                };
                service.queue_toolbar_menu_action(action)?;
                panels.open(PANEL)?;
                Ok(json!({"queued":true,"menu":name}))
            })
        },
    ));
    let create = Arc::clone(&service);
    let open = Arc::clone(&panels);
    tools.push(tool(
        "whiteboard.create",
        "Create and open a whiteboard",
        true,
        false,
        json!({"title":{"type":"string"}}),
        move |v| {
            let s = Arc::clone(&create);
            let p = Arc::clone(&open);
            Box::pin(async move {
                let id = s.create(v["title"].as_str().unwrap_or(""))?;
                p.open(PANEL)?;
                Ok(json!({"id":id}))
            })
        },
    ));
    let activate = Arc::clone(&service);
    let open = Arc::clone(&panels);
    tools.push(tool(
        "whiteboard.open",
        "Open a managed whiteboard",
        true,
        false,
        json!({"id":{"type":"integer"}}),
        move |v| {
            let s = Arc::clone(&activate);
            let p = Arc::clone(&open);
            Box::pin(async move {
                s.activate(id(&v)?)?;
                p.open(PANEL)?;
                Ok(json!({"opened":true}))
            })
        },
    ));
    let scroll = Arc::clone(&service);
    let open = Arc::clone(&panels);
    tools.push(tool(
        "whiteboard.scroll",
        "Apply a canvas wheel gesture through the live canvas input handler",
        true,
        false,
        json!({
            "id":{"type":"integer"},
            "delta_x":{"type":"number","minimum":-4096,"maximum":4096},
            "delta_y":{"type":"number","minimum":-4096,"maximum":4096},
            "control":{"type":"boolean"},
            "at":{"type":"array","items":{"type":"number","minimum":0,"maximum":4096},"minItems":2,"maxItems":2}
        }),
        move |v| {
            let service = Arc::clone(&scroll);
            let panels = Arc::clone(&open);
            Box::pin(async move {
                let coordinates = v["at"].as_array().ok_or("at must contain two canvas coordinates")?;
                if coordinates.len() != 2 {
                    return Err("at must contain two canvas coordinates".into());
                }
                let at = [
                    coordinates[0].as_f64().ok_or("invalid at.x")? as f32,
                    coordinates[1].as_f64().ok_or("invalid at.y")? as f32,
                ];
                let scroll = crate::store::CanvasScroll {
                    board_id: id(&v)?,
                    dx: v["delta_x"].as_f64().ok_or("delta_x is required")? as f32,
                    dy: v["delta_y"].as_f64().ok_or("delta_y is required")? as f32,
                    control: v["control"].as_bool().ok_or("control is required")?,
                    at,
                };
                service.queue_canvas_scroll(scroll)?;
                panels.open(PANEL)?;
                Ok(json!({"queued":true}))
            })
        },
    ));
    let rename = Arc::clone(&service);
    tools.push(tool(
        "whiteboard.rename",
        "Rename a managed whiteboard",
        true,
        false,
        json!({"id":{"type":"integer"},"title":{"type":"string","minLength":1}}),
        move |v| {
            let s = Arc::clone(&rename);
            Box::pin(async move {
                s.rename(id(&v)?, v["title"].as_str().ok_or("title is required")?)?;
                Ok(json!({"renamed":true}))
            })
        },
    ));
    let import = Arc::clone(&service);
    let open = Arc::clone(&panels);
    tools.push(tool(
        "whiteboard.import",
        "Import native Zorite Scene JSON; original file and media remain untouched",
        true,
        false,
        json!({"path":{"type":"string","minLength":1}}),
        move |v| {
            let s = Arc::clone(&import);
            let p = Arc::clone(&open);
            Box::pin(async move {
                let path = path(&v)?;
                let id = tokio::task::spawn_blocking(move || s.import(&path))
                    .await
                    .map_err(|e| e.to_string())??;
                p.open(PANEL)?;
                Ok(json!({"id":id}))
            })
        },
    ));
    let export = Arc::clone(&service);
    tools.push(tool(
        "whiteboard.export",
        "Write native Zorite Scene JSON to the requested file",
        true,
        true,
        json!({"id":{"type":"integer"},"path":{"type":"string","minLength":1}}),
        move |v| {
            let s = Arc::clone(&export);
            Box::pin(async move {
                let id = id(&v)?;
                let path = path(&v)?;
                tokio::task::spawn_blocking(move || s.export(id, &path))
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(json!({"exported":true}))
            })
        },
    ));
    let images = Arc::clone(&service);
    let open = Arc::clone(&panels);
    tools.push(tool("whiteboard.add_images", "Link images through the same upstream add_image_at host callback", true, false, json!({"id":{"type":"integer"},"paths":{"type":"array","items":{"type":"string"},"minItems":1,"maxItems":100},"at":{"type":"array","items":{"type":"number"},"minItems":2,"maxItems":2}}), move |v| {
        let s = Arc::clone(&images); let p = Arc::clone(&open); Box::pin(async move {
            let paths = v["paths"].as_array().ok_or("paths is required")?.iter().map(|p| p.as_str().map(PathBuf::from).ok_or("paths must contain strings")).collect::<Result<Vec<_>, _>>()?;
            if paths.is_empty() || paths.len() > 100 { return Err("provide 1 to 100 image paths".into()); }
            let at = if let Some(a) = v.get("at") {
                let a = a.as_array().ok_or("at must contain two finite coordinates")?;
                if a.len() != 2 { return Err("at must contain two finite coordinates".into()); }
                let x = a[0].as_f64().ok_or("invalid x")? as f32;
                let y = a[1].as_f64().ok_or("invalid y")? as f32;
                if !x.is_finite() || !y.is_finite() { return Err("at must contain finite coordinates".into()); }
                Some([x,y])
            } else { None };
            s.queue_images(id(&v)?, paths, at)?; p.open(PANEL)?; Ok(json!({"queued":true}))
        })
    }));
    tools.push(tool(
        "whiteboard.flush",
        "Wait for the coalesced whiteboard file save",
        true,
        false,
        json!({}),
        move |_| {
            let s = Arc::clone(&service);
            Box::pin(async move {
                tokio::task::spawn_blocking(move || s.flush())
                    .await
                    .map_err(|e| e.to_string())??;
                Ok(json!({"saved":true}))
            })
        },
    ));
    let registration = registry.register(OWNER, tools).map_err(|e| e.to_string())?;
    let _publication = context
        .provide(Arc::new(Registration {
            _registration: registration,
        }))
        .map_err(|e| e.to_string())?;
    Ok(())
}
fn id(value: &Value) -> Result<i64, String> {
    value["id"].as_i64().ok_or_else(|| "id is required".into())
}
fn path(value: &Value) -> Result<PathBuf, String> {
    value["path"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| "path is required".into())
}
fn tool<F>(
    name: &str,
    description: &str,
    mutating: bool,
    confirmation_required: bool,
    properties: Value,
    handler: F,
) -> McpToolDefinition
where
    F: Fn(Value) -> McpToolFuture + Send + Sync + 'static,
{
    let required = properties
        .as_object()
        .map(|p| {
            p.keys()
                .filter(|key| {
                    !(name == "whiteboard.create" && key.as_str() == "title")
                        && key.as_str() != "at"
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    McpToolDefinition { name: name.into(), title: name.into(), description: description.into(), mutating, confirmation_required,
        input_schema: json!({"type":"object","properties":properties,"required":required,"additionalProperties":false}).as_object().cloned().unwrap_or_default(), handler: Arc::new(handler) }
}
struct Registration {
    _registration: Arc<McpToolRegistration>,
}
impl cordis::Service for Registration {
    const NAME: &'static str = "fileatlas/whiteboard-mcp";
}
