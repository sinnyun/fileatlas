use crate::FileOperationsService;
use cordis::{Context, Service};
use fileatlas_contracts::mcp::{
    McpToolDefinition, McpToolFuture, McpToolRegistration, McpToolRegistry,
};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};

pub(super) fn register(
    context: &Context,
    operations: &Arc<FileOperationsService>,
) -> Result<(), String> {
    let Ok(registry) = context.try_service::<McpToolRegistry>() else {
        return Ok(());
    };
    let mut tools = Vec::new();

    let service = Arc::clone(operations);
    tools.push(tool("file-operations.rename_media", "Rename media file", "Rename one indexed media file. Requires explicit confirmation.", json!({"type":"object","properties":{"media_id":{"type":"integer","minimum":1},"new_filename":{"type":"string","minLength":1}},"required":["media_id","new_filename"],"additionalProperties":false}), true, true, move |args| {
        let service=Arc::clone(&service);
        Box::pin(async move { let r:RenameRequest=parse(args)?; if r.new_filename.trim().is_empty() || r.new_filename.contains(['\\','/']) { return Err("new_filename must be a non-empty filename without path separators".into()); } service.rename(r.media_id,r.new_filename).await.map_err(|e|e.to_string())?; Ok(json!({"ok":true,"media_id":r.media_id})) })
    }));

    let service = Arc::clone(operations);
    tools.push(tool("file-operations.move_media", "Move media", "Move 1–100 indexed media files to a destination directory. Requires explicit confirmation.", batch_path_schema(), true, true, move |args| {
        let service=Arc::clone(&service);
        Box::pin(async move { let r:BatchPathRequest=parse(args)?; validate_ids(&r.media_ids)?; let outcomes=service.bulk_move_into_folder(r.media_ids,PathBuf::from(&r.destination_folder)).await.map_err(|e|e.to_string())?; Ok(json!({"items":outcomes.iter().map(|o|json!({"media_id":o.media_id,"moved":o.moved,"error":o.error})).collect::<Vec<_>>()})) })
    }));

    let service = Arc::clone(operations);
    tools.push(tool("file-operations.copy_media", "Copy media", "Copy 1–100 indexed media files to a destination directory. Requires explicit confirmation.", batch_path_schema(), true, true, move |args| {
        let service=Arc::clone(&service);
        Box::pin(async move { let r:BatchPathRequest=parse(args)?; validate_ids(&r.media_ids)?; let outcomes=service.bulk_copy_to_folder(r.media_ids,PathBuf::from(&r.destination_folder)).await.map_err(|e|e.to_string())?; Ok(json!({"items":outcomes.iter().map(|o|json!({"media_id":o.media_id,"copied":o.copied,"error":o.error})).collect::<Vec<_>>()})) })
    }));

    let service = Arc::clone(operations);
    tools.push(tool("file-operations.delete_media", "Send media to Recycle Bin", "Send 1–100 media files to the operating system Recycle Bin and remove their FileAtlas rows. Requires explicit confirmation.", json!({"type":"object","properties":{"media_ids":{"type":"array","items":{"type":"integer","minimum":1},"minItems":1,"maxItems":100}},"required":["media_ids"],"additionalProperties":false}), true, true, move |args| {
        let service=Arc::clone(&service);
        Box::pin(async move { let r:BatchRequest=parse(args)?; validate_ids(&r.media_ids)?; let outcomes=service.bulk_delete_to_recycle_bin(r.media_ids).await.map_err(|e|e.to_string())?; Ok(json!({"items":outcomes.iter().map(|o|json!({"media_id":o.media_id,"removed_from_database":o.removed_from_database,"recycled":o.recycled,"error":o.error})).collect::<Vec<_>>() })) })
    }));

    let registration = registry
        .register("fileatlas.file-operations", tools)
        .map_err(|e| e.to_string())?;
    let _publication = context
        .provide(Arc::new(FileOperationsMcpRegistration {
            _registration: registration,
        }))
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn batch_path_schema() -> Value {
    json!({"type":"object","properties":{"media_ids":{"type":"array","items":{"type":"integer","minimum":1},"minItems":1,"maxItems":100},"destination_folder":{"type":"string","minLength":1}},"required":["media_ids","destination_folder"],"additionalProperties":false})
}

fn validate_ids(ids: &[i64]) -> Result<(), String> {
    if !(1..=100).contains(&ids.len()) || ids.iter().any(|id| *id <= 0) {
        Err("media_ids must contain 1 to 100 positive IDs".into())
    } else {
        Ok(())
    }
}

fn tool<F>(
    name: &str,
    title: &str,
    description: &str,
    schema: Value,
    mutating: bool,
    confirmation_required: bool,
    handler: F,
) -> McpToolDefinition
where
    F: Fn(Value) -> McpToolFuture + Send + Sync + 'static,
{
    McpToolDefinition {
        name: name.into(),
        title: title.into(),
        description: description.into(),
        input_schema: schema.as_object().cloned().unwrap_or_default(),
        mutating,
        confirmation_required,
        handler: Arc::new(handler),
    }
}

fn parse<T: DeserializeOwned>(value: Value) -> Result<T, String> {
    serde_json::from_value(value).map_err(|e| format!("invalid parameters: {e}"))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenameRequest {
    media_id: i64,
    new_filename: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BatchPathRequest {
    media_ids: Vec<i64>,
    destination_folder: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BatchRequest {
    media_ids: Vec<i64>,
}

struct FileOperationsMcpRegistration {
    _registration: Arc<McpToolRegistration>,
}
impl Service for FileOperationsMcpRegistration {
    const NAME: &'static str = "fileatlas/file-operations-mcp-registration";
}
