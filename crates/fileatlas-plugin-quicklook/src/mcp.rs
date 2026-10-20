use crate::QuickLookService;
use cordis::{Context, Service};
use fileatlas_contracts::mcp::{McpToolDefinition, McpToolRegistration, McpToolRegistry};
use fileatlas_plugin_file_operations::FileOperationsService;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

pub(super) fn register(
    context: &Context,
    quicklook: &Arc<QuickLookService>,
    files: &Arc<FileOperationsService>,
) -> Result<(), String> {
    let Ok(registry) = context.try_service::<McpToolRegistry>() else {
        return Ok(());
    };
    let file_service = Arc::clone(files);
    let quicklook_service = Arc::clone(quicklook);
    let open = McpToolDefinition {
        name: "quicklook.open_media".into(),
        title: "Open media in QuickLook".into(),
        description: "Ask the already-running QuickLook application to view one indexed media file. FileAtlas does not start QuickLook.".into(),
        input_schema: json!({"type":"object","properties":{"media_id":{"type":"integer","minimum":1}},"required":["media_id"],"additionalProperties":false})
            .as_object().cloned().unwrap_or_default(),
        mutating: true,
        confirmation_required: false,
        handler: Arc::new(move |arguments| {
            let files = Arc::clone(&file_service);
            let quicklook = Arc::clone(&quicklook_service);
            Box::pin(async move {
                let request: MediaRequest = serde_json::from_value(arguments)
                    .map_err(|error| format!("invalid parameters: {error}"))?;
                if request.media_id <= 0 { return Err("media_id must be positive".into()); }
                let media = files.media_by_id(request.media_id)
                    .map_err(|error|error.to_string())?
                    .ok_or_else(||"media record was not found".to_owned())?;
                let path = media.full_path;
                tokio::task::spawn_blocking(move || quicklook.open(path))
                    .await.map_err(|error|error.to_string())?
                    .map_err(|error|error.to_string())?;
                Ok(json!({"ok":true,"viewer":"quicklook","media_id":request.media_id}))
            })
        }),
    };
    let state_service = Arc::clone(quicklook);
    let state = McpToolDefinition {
        name: "quicklook.snapshot".into(),
        title: "Read QuickLook dispatch state".into(),
        description: "Read the most recent QuickLook dispatch attempt. This does not prove the external viewer rendered the file.".into(),
        input_schema: json!({"type":"object","properties":{},"additionalProperties":false})
            .as_object().cloned().unwrap_or_default(),
        mutating: false,
        confirmation_required: false,
        handler: Arc::new(move |_| {
            let quicklook=Arc::clone(&state_service);
            Box::pin(async move { serde_json::to_value(quicklook.snapshot()).map_err(|error|error.to_string()) })
        }),
    };
    let inspect_service = Arc::clone(quicklook);
    let inspect = McpToolDefinition {
        name: "quicklook.inspect".into(),
        title: "Inspect QuickLook windows".into(),
        description: "Inspect QuickLook's visible windows and local IPC diagnostics; this does not capture their pixels.".into(),
        input_schema: json!({"type":"object","properties":{},"additionalProperties":false})
            .as_object().cloned().unwrap_or_default(),
        mutating: false,
        confirmation_required: false,
        handler: Arc::new(move |_| {
            let quicklook=Arc::clone(&inspect_service);
            Box::pin(async move {
                #[cfg(windows)]
                { serde_json::to_value(quicklook.inspect()).map_err(|error|error.to_string()) }
                #[cfg(not(windows))]
                { let _=quicklook; Err("QuickLook inspection requires Windows".into()) }
            })
        }),
    };
    let registration = registry
        .register("fileatlas.quicklook", [open, state, inspect])
        .map_err(|error| error.to_string())?;
    let _publication = context
        .provide(Arc::new(QuickLookMcpRegistration {
            _registration: registration,
        }))
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MediaRequest {
    media_id: i64,
}

struct QuickLookMcpRegistration {
    _registration: Arc<McpToolRegistration>,
}
impl Service for QuickLookMcpRegistration {
    const NAME: &'static str = "fileatlas/quicklook-mcp-registration";
}
