use crate::FileStagingService;
use cordis::{Context, Service};
use fileatlas_contracts::mcp::{
    McpToolDefinition, McpToolFuture, McpToolRegistration, McpToolRegistry,
};
use fileatlas_plugin_file_operations::FileOperationsService;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};

pub(super) fn register(
    context: &Context,
    basket: &Arc<FileStagingService>,
    files: &Arc<FileOperationsService>,
) -> Result<(), crate::StagingError> {
    let Ok(registry) = context.try_service::<McpToolRegistry>() else {
        return Ok(());
    };
    let basket = Arc::clone(basket);
    let mut tools = Vec::new();
    let service = Arc::clone(&basket);
    tools.push(tool(
        "file-staging.list",
        "List staged files",
        false,
        false,
        json!({"type":"object","properties":{},"additionalProperties":false}),
        move |_| {
            let service = Arc::clone(&service);
            Box::pin(async move { Ok(json!({"items":service.snapshot()})) })
        },
    ));
    let service = Arc::clone(&basket);
    tools.push(tool("file-staging.add", "Add files to staging basket", true, false, json!({"type":"object","properties":{"paths":{"type":"array","items":{"type":"string","minLength":1},"minItems":1,"maxItems":100}},"required":["paths"],"additionalProperties":false}), move |args| {
        let service=Arc::clone(&service); Box::pin(async move { let request:PathsRequest=parse(args)?; if !(1..=100).contains(&request.paths.len()) { return Err("paths must contain 1 to 100 entries".into()); } let paths=request.paths.into_iter().map(PathBuf::from).collect::<Vec<_>>(); let items=service.add_paths(paths).map_err(|e|e.to_string())?; Ok(json!({"items":items})) })
    }));
    let service = Arc::clone(&basket);
    tools.push(tool("file-staging.remove", "Remove staged file", true, false, json!({"type":"object","properties":{"path":{"type":"string","minLength":1}},"required":["path"],"additionalProperties":false}), move |args| {
        let service=Arc::clone(&service); Box::pin(async move { let request:PathRequest=parse(args)?; let items=service.remove(&request.path).map_err(|e|e.to_string())?; Ok(json!({"items":items})) })
    }));
    let service = Arc::clone(&basket);
    tools.push(tool(
        "file-staging.clear",
        "Clear staging basket",
        true,
        false,
        json!({"type":"object","properties":{},"additionalProperties":false}),
        move |_| {
            let service = Arc::clone(&service);
            Box::pin(async move {
                let items = service.clear().map_err(|e| e.to_string())?;
                Ok(json!({"items":items}))
            })
        },
    ));
    let service = Arc::clone(&basket);
    tools.push(tool(
        "file-staging.refresh_status",
        "Refresh staged path status",
        true,
        false,
        json!({"type":"object","properties":{},"additionalProperties":false}),
        move |_| {
            let service = Arc::clone(&service);
            Box::pin(async move {
                let service = Arc::clone(&service);
                let items = tokio::task::spawn_blocking(move || service.refresh_status())
                    .await
                    .map_err(|e| e.to_string())?
                    .map_err(|e| e.to_string())?;
                Ok(json!({"items":items}))
            })
        },
    ));
    let service = Arc::clone(&basket);
    tools.push(tool(
        "file-staging.remove_missing",
        "Remove unavailable staged paths",
        true,
        false,
        json!({"type":"object","properties":{},"additionalProperties":false}),
        move |_| {
            let service = Arc::clone(&service);
            Box::pin(async move {
                let items = service.remove_missing().map_err(|e| e.to_string())?;
                Ok(json!({"items":items}))
            })
        },
    ));
    let service = Arc::clone(&basket);
    let operations = Arc::clone(files);
    tools.push(tool("file-staging.copy", "Copy staged files", true, true, json!({"type":"object","properties":{"destination_folder":{"type":"string","minLength":1}},"required":["destination_folder"],"additionalProperties":false}), move |args| {
        let service=Arc::clone(&service); let operations=Arc::clone(&operations); Box::pin(async move { let request:DestinationRequest=parse(args)?; let paths=service.snapshot().into_iter().map(|f|PathBuf::from(f.path)).collect(); let outcomes=operations.copy_paths_to_folder(paths,PathBuf::from(request.destination_folder)).await.map_err(|e|e.to_string())?; remove_completed(&service,&outcomes)?; Ok(outcomes_json(&outcomes)) })
    }));
    let service = Arc::clone(&basket);
    let operations = Arc::clone(files);
    tools.push(tool("file-staging.move", "Move staged files", true, true, json!({"type":"object","properties":{"destination_folder":{"type":"string","minLength":1}},"required":["destination_folder"],"additionalProperties":false}), move |args| {
        let service=Arc::clone(&service); let operations=Arc::clone(&operations); Box::pin(async move { let request:DestinationRequest=parse(args)?; let paths=service.snapshot().into_iter().map(|f|PathBuf::from(f.path)).collect(); let outcomes=operations.move_paths_to_folder(paths,PathBuf::from(request.destination_folder)).await.map_err(|e|e.to_string())?; remove_completed(&service,&outcomes)?; Ok(outcomes_json(&outcomes)) })
    }));
    let registration = registry
        .register("fileatlas.file-staging", tools)
        .map_err(|e| crate::StagingError::Plugin(e.to_string()))?;
    let _publication = context
        .provide(Arc::new(RegistrationOwner {
            _registration: registration,
        }))
        .map_err(|e| crate::StagingError::Plugin(e.to_string()))?;
    Ok(())
}
fn remove_completed(
    service: &FileStagingService,
    outcomes: &[fileatlas_plugin_file_operations::PathOperationOutcome],
) -> Result<(), String> {
    for item in outcomes.iter().filter(|o| o.completed) {
        service
            .remove(&item.source.to_string_lossy())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn outcomes_json(outcomes: &[fileatlas_plugin_file_operations::PathOperationOutcome]) -> Value {
    json!({"items":outcomes.iter().map(|o|json!({"source":o.source,"completed":o.completed,"error":o.error})).collect::<Vec<_>>()})
}
fn tool<F>(
    name: &str,
    title: &str,
    mutating: bool,
    confirmation_required: bool,
    schema: Value,
    handler: F,
) -> McpToolDefinition
where
    F: Fn(Value) -> McpToolFuture + Send + Sync + 'static,
{
    McpToolDefinition {
        name: name.into(),
        title: title.into(),
        description: title.into(),
        input_schema: schema.as_object().cloned().unwrap_or_default(),
        mutating,
        confirmation_required,
        handler: Arc::new(handler),
    }
}
fn parse<T: DeserializeOwned>(v: Value) -> Result<T, String> {
    serde_json::from_value(v).map_err(|e| format!("invalid parameters: {e}"))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PathsRequest {
    paths: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PathRequest {
    path: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DestinationRequest {
    destination_folder: String,
}
struct RegistrationOwner {
    _registration: Arc<McpToolRegistration>,
}
impl Service for RegistrationOwner {
    const NAME: &'static str = "fileatlas/file-staging-mcp-registration";
}
