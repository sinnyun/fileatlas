use crate::NavigationService;
use photag_contracts::mcp::{McpToolDefinition, McpToolFuture};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::sync::Arc;

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
    serde_json::from_value(value).map_err(|error| format!("invalid parameters: {error}"))
}

pub(super) fn tools(
    service: &Arc<NavigationService>,
    context: &cordis::Context,
) -> Vec<McpToolDefinition> {
    let mut tools = Vec::new();
    let add_scan_context = context.clone();
    let rescan_context = context.clone();

    let navigation = Arc::clone(service);
    tools.push(tool("navigation.add_folder", "Register library folder", "Register an existing directory and start or refresh its indexing, according to the active library mode. Requires explicit confirmation.", json!({"type":"object","properties":{"path":{"type":"string","minLength":1}},"required":["path"],"additionalProperties":false}), true, true, move |args| {
        let navigation=Arc::clone(&navigation);
        let context=add_scan_context.clone();
        Box::pin(async move {
            let request:PathRequest=parse(args)?;
            let (folder,progress)=navigation.add_folder(&request.path).map_err(|error|error.to_string())?;
            observe_scan(context,progress);
            Ok(json!({"ok":true,"folder":folder}))
        })
    }));

    let navigation = Arc::clone(service);
    tools.push(tool("navigation.remove_folder", "Remove library folder", "Remove a registered folder and its indexed rows. This does not delete files from disk; requires explicit confirmation.", json!({"type":"object","properties":{"folder_id":{"type":"integer","minimum":1}},"required":["folder_id"],"additionalProperties":false}), true, true, move |args| {
        let navigation=Arc::clone(&navigation);
        Box::pin(async move { let request:FolderRequest=parse(args)?; let folder=navigation.remove_folder(request.folder_id).map_err(|error|error.to_string())?; Ok(json!({"ok":true,"folder":folder})) })
    }));

    let navigation = Arc::clone(service);
    tools.push(tool("navigation.rescan_folder", "Rescan library folder", "Refresh the registered folder view or clear and rebuild its indexed rows, according to the active library mode. Requires explicit confirmation.", json!({"type":"object","properties":{"folder_id":{"type":"integer","minimum":1}},"required":["folder_id"],"additionalProperties":false}), true, true, move |args| {
        let navigation=Arc::clone(&navigation);
        let context=rescan_context.clone();
        Box::pin(async move { let request:FolderRequest=parse(args)?; let progress=navigation.rescan(request.folder_id).map_err(|error|error.to_string())?; observe_scan(context,progress); Ok(json!({"ok":true,"started":true,"folder_id":request.folder_id})) })
    }));

    let navigation = Arc::clone(service);
    tools.push(tool("navigation.cancel_scan", "Cancel folder scan", "Cancel indexing for one registered folder without removing its current media records.", json!({"type":"object","properties":{"folder_id":{"type":"integer","minimum":1}},"required":["folder_id"],"additionalProperties":false}), true, false, move |args| {
        let navigation=Arc::clone(&navigation);
        Box::pin(async move { let request:FolderRequest=parse(args)?; navigation.cancel_scan(request.folder_id).map_err(|error|error.to_string())?; Ok(json!({"ok":true,"folder_id":request.folder_id})) })
    }));

    let navigation = Arc::clone(service);
    tools.push(tool("navigation.create_tag", "Create tag", "Create a hierarchical PhoTag tag, optionally under a parent tag.", json!({"type":"object","properties":{"name":{"type":"string","minLength":1,"maxLength":256},"parent_id":{"type":["integer","null"],"minimum":1}},"required":["name"],"additionalProperties":false}), true, false, move |args| {
        let navigation=Arc::clone(&navigation);
        Box::pin(async move { let request:CreateTagRequest=parse(args)?; if request.name.trim().is_empty() || request.name.len()>256 { return Err("name must be non-empty and at most 256 UTF-8 bytes".into()); } let tag=navigation.create_tag(&request.name,request.parent_id).map_err(|error|error.to_string())?; Ok(json!({"tag":tag})) })
    }));

    let navigation = Arc::clone(service);
    tools.push(tool("navigation.rename_tag", "Rename tag", "Rename one tag while preserving its hierarchy and associations.", json!({"type":"object","properties":{"tag_id":{"type":"integer","minimum":1},"name":{"type":"string","minLength":1,"maxLength":256}},"required":["tag_id","name"],"additionalProperties":false}), true, false, move |args| {
        let navigation=Arc::clone(&navigation);
        Box::pin(async move { let request:RenameTagRequest=parse(args)?; if request.name.trim().is_empty() || request.name.len()>256 { return Err("name must be non-empty and at most 256 UTF-8 bytes".into()); } navigation.rename_tag(request.tag_id,&request.name).map_err(|error|error.to_string())?; Ok(json!({"ok":true,"tag_id":request.tag_id})) })
    }));

    let navigation = Arc::clone(service);
    tools.push(tool("navigation.delete_tag", "Delete tag", "Delete one tag and its associations without deleting media files. Requires explicit confirmation.", json!({"type":"object","properties":{"tag_id":{"type":"integer","minimum":1}},"required":["tag_id"],"additionalProperties":false}), true, true, move |args| {
        let navigation=Arc::clone(&navigation);
        Box::pin(async move { let request:TagIdRequest=parse(args)?; let affected=navigation.delete_tag(request.tag_id,false).map_err(|error|error.to_string())?; Ok(json!({"ok":true,"tag_id":request.tag_id,"affected_media_ids":affected})) })
    }));

    let navigation = Arc::clone(service);
    tools.push(tool("navigation.manage_favorite", "Manage folder favorite", "Rename a folder favorite alias or change its order; does not rename or move local files.", json!({"type":"object","properties":{"action":{"type":"string","enum":["rename","move"]},"folder_id":{"type":"integer","minimum":1},"name":{"type":"string"},"delta":{"type":"integer"}},"required":["action","folder_id"],"additionalProperties":false}), true, false, move |args| {
        let navigation=Arc::clone(&navigation);
        Box::pin(async move {
            let request:FavoriteRequest=parse(args)?;
            match request.action.as_str() {
                "rename"=>navigation.rename_favorite(request.folder_id,request.name.as_deref().unwrap_or("")),
                "move"=>navigation.move_favorite(request.folder_id,request.delta.unwrap_or(0)),
                _=>return Err("action must be rename or move".into()),
            }.map_err(|error|error.to_string())?;
            let _=navigation.request_ui(crate::NavigationUiRequest::CloseModal);
            Ok(json!({"updated":request.folder_id}))
        })
    }));

    tools
}

fn observe_scan(
    context: cordis::Context,
    progress: std::sync::mpsc::Receiver<Option<photag_plugin_indexing::ScanProgress>>,
) {
    let runtime = tokio::runtime::Handle::current();
    std::thread::spawn(move || {
        while let Ok(update) = progress.recv() {
            if update.is_none() {
                break;
            }
        }
        if let Err(error) = runtime.block_on(
            context.emit::<photag_contracts::MediaIndexChanged>(cordis::Routing::Unscoped, ()),
        ) {
            tracing::error!(%error, "navigation MCP scan completion dispatch failed");
        }
    });
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PathRequest {
    path: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FolderRequest {
    folder_id: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateTagRequest {
    name: String,
    parent_id: Option<i64>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenameTagRequest {
    tag_id: i64,
    name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TagIdRequest {
    tag_id: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FavoriteRequest {
    action: String,
    folder_id: i64,
    name: Option<String>,
    delta: Option<isize>,
}
