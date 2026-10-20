use crate::ClipboardService;
use cordis::{Context, Service};
use fileatlas_contracts::mcp::{McpToolDefinition, McpToolRegistration, McpToolRegistry};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

pub(super) fn register(context: &Context, clipboard: &Arc<ClipboardService>) -> Result<(), String> {
    let Ok(registry) = context.try_service::<McpToolRegistry>() else {
        return Ok(());
    };

    let state_service = Arc::clone(clipboard);
    let state = McpToolDefinition {
        name: "clipboard.snapshot".into(),
        title: "Read clipboard workbench state".into(),
        description: "Read clipboard item metadata, categories, counts and capture state. Clipboard text is omitted.".into(),
        input_schema: json!({"type":"object","properties":{},"additionalProperties":false})
            .as_object().cloned().unwrap_or_default(),
        mutating: false,
        confirmation_required: false,
        handler: Arc::new(move |_| {
            let clipboard = Arc::clone(&state_service);
            Box::pin(async move {
                let categories = clipboard.categories()?;
                let items = clipboard.snapshot()?;
                Ok(json!({
                    "count": items.len(),
                    "revision": clipboard.revision(),
                    "favorite_count": items.iter().filter(|item| item.pinned).count(),
                    "capture_paused": clipboard.is_paused(),
                    "categories": categories.iter().map(|category| json!({"id":category.id,"name":category.name})).collect::<Vec<_>>(),
                    "items": items.iter().map(|item| json!({
                        "id":item.id,
                        "length":if item.image_path.is_some(){0}else{item.content.chars().count()},
                        "category_id":item.category_id,
                        "kind":if item.image_path.is_some(){"image"}else{crate::content::classify(&item.content).key()},
                        "image_available":item.image_path.as_deref().is_some_and(|path|std::path::Path::new(path).is_file()),
                        "image_width":item.image_width,
                        "image_height":item.image_height,
                        "favorite":item.pinned,
                        "created_at":item.created_at,
                        "last_used_at":item.last_used_at
                    })).collect::<Vec<_>>()
                }))
            })
        }),
    };

    let add_service = Arc::clone(clipboard);
    let add_text = McpToolDefinition {
        name: "clipboard.add_text".into(),
        title: "Add clipboard text".into(),
        description: "Add a text item to the clipboard workbench without reading or writing the operating system clipboard.".into(),
        input_schema: json!({
            "type":"object","properties":{
                "content":{"type":"string","minLength":1,"maxLength":1000000},
                "category_id":{"type":["integer","null"],"minimum":1},
                "favorite":{"type":"boolean","default":false}
            },"required":["content"],"additionalProperties":false
        }).as_object().cloned().unwrap_or_default(),
        mutating: true,
        confirmation_required: false,
        handler: Arc::new(move |arguments| {
            let clipboard = Arc::clone(&add_service);
            Box::pin(async move {
                let request: AddTextRequest = serde_json::from_value(arguments)
                    .map_err(|error| format!("invalid parameters: {error}"))?;
                let content = request.content.trim();
                if content.is_empty() || content.len() > 1_000_000 {
                    return Err("content must contain 1 to 1,000,000 UTF-8 bytes".into());
                }
                if let Some(category_id) = request.category_id
                    && !clipboard.categories()?.iter().any(|category| category.id == category_id)
                {
                    return Err("category_id does not identify an existing category".into());
                }
                let id = clipboard.add_text(content, request.category_id, request.favorite.unwrap_or(false))?;
                Ok(json!({"added":true,"id":id}))
            })
        }),
    };

    let registration = registry
        .register("fileatlas.clipboard", [state, add_text])
        .map_err(|error| error.to_string())?;
    let _mcp_registration = context
        .provide(Arc::new(ClipboardMcpRegistration {
            _registration: registration,
        }))
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AddTextRequest {
    content: String,
    category_id: Option<i64>,
    favorite: Option<bool>,
}

struct ClipboardMcpRegistration {
    _registration: Arc<McpToolRegistration>,
}

impl Service for ClipboardMcpRegistration {
    const NAME: &'static str = "fileatlas/clipboard-mcp-registration";
}
