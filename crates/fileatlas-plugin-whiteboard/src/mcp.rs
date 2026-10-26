use crate::{OWNER, PANEL, UPSTREAM, WhiteboardService};
use cordis::Context;
use fileatlas_contracts::mcp::{
    McpToolDefinition, McpToolFuture, McpToolRegistration, McpToolRegistry,
};
use fileatlas_ui_bridge::panel_catalog::PanelCatalog;
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};

pub fn register(
    context: &Context,
    service: Arc<WhiteboardService>,
    panels: Arc<PanelCatalog>,
) -> Result<(), String> {
    let Ok(registry) = context.try_service::<McpToolRegistry>() else {
        return Ok(());
    };
    let mut tools = vec![];
    let state = Arc::clone(&service);
    tools.push(tool("whiteboard.state", "Read the managed whiteboards and unchanged upstream scene data", false, false, json!({}), move |_| {
        let s = Arc::clone(&state); Box::pin(async move { Ok(json!({"upstream":UPSTREAM,"library":s.snapshot(),"save":s.status(),"path":s.root.join("library.json")})) })
    }));
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
