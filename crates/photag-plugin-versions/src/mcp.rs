use crate::VersionsService;
use cordis::{Context, Service};
use photag_contracts::mcp::{
    McpToolDefinition, McpToolFuture, McpToolRegistration, McpToolRegistry,
};
use photag_plugin_database::DatabaseService;
use photag_plugin_indexing::IndexingService;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::sync::Arc;

pub(super) fn register(
    context: &Context,
    versions: &Arc<VersionsService>,
    database: &Arc<DatabaseService>,
    indexing: &Arc<IndexingService>,
) -> Result<(), String> {
    let Ok(registry) = context.try_service::<McpToolRegistry>() else {
        return Ok(());
    };
    let mut tools = Vec::new();

    let database_service = Arc::clone(database);
    let indexing_service = Arc::clone(indexing);
    let database = Arc::clone(&database_service);
    let service = Arc::clone(versions);
    tools.push(tool(
        "versions.list",
        "List file versions",
        "List durable Lore versions for one indexed media record.",
        media_schema(),
        false,
        false,
        move |args| {
            let database = Arc::clone(&database);
            let service = Arc::clone(&service);
            Box::pin(async move {
                let r: MediaRequest = parse(args)?;
                let path = media_path(&database, r.media_id)?;
                let versions = service.list(&path).await.map_err(|e| e.to_string())?;
                Ok(json!({"media_id":r.media_id,"path":path,"versions":versions}))
            })
        },
    ));

    let database = Arc::clone(&database_service);
    let indexing = Arc::clone(&indexing_service);
    let service = Arc::clone(versions);
    tools.push(tool("versions.file_state", "Inspect file version state", "Compare an indexed file's current bytes with its saved revisions without changing the file.", media_schema(), false, false, move |args| {
        let database=Arc::clone(&database); let service=Arc::clone(&service);
        Box::pin(async move { let r:MediaRequest=parse(args)?; let path=media_path(&database,r.media_id)?; let state=service.file_state(&path).await.map_err(|e|e.to_string())?; Ok(json!({"media_id":r.media_id,"path":path,"state":state})) })
    }));

    let database = Arc::clone(&database_service);
    let service = Arc::clone(versions);
    tools.push(tool("versions.list_branches", "List file branches", "Read a file's Lore branch graph, revision chains, and pending merge conflicts.", json!({"type":"object","properties":{"media_id":{"type":"integer","minimum":1},"include_archived":{"type":"boolean","default":false}},"required":["media_id"],"additionalProperties":false}), false, false, move |args| {
        let database=Arc::clone(&database); let service=Arc::clone(&service);
        Box::pin(async move { let r:BranchListRequest=parse(args)?; let path=media_path(&database,r.media_id)?; let graph=service.branch_graph(&path,r.include_archived).await.map_err(|e|e.to_string())?; Ok(json!({"media_id":r.media_id,"path":path,"graph":graph})) })
    }));

    let database = Arc::clone(&database_service);
    let service = Arc::clone(versions);
    let event_context = context.clone();
    tools.push(tool("versions.create", "Save file version", "Create a durable Lore snapshot of one indexed file. Requires explicit confirmation.", json!({"type":"object","properties":{"media_id":{"type":"integer","minimum":1},"title":{"type":"string","minLength":1,"maxLength":240}},"required":["media_id","title"],"additionalProperties":false}), true, true, move |args| {
        let database=Arc::clone(&database); let service=Arc::clone(&service); let context=event_context.clone();
        Box::pin(async move { let r:CreateRequest=parse(args)?; let path=media_path(&database,r.media_id)?; let version=service.create_version(&path,&r.title).await.map_err(|e|e.to_string())?; emit_media_changed(&context).await?; emit_history_changed(&context,r.media_id).await?; Ok(json!({"ok":true,"media_id":r.media_id,"version":version})) })
    }));

    let database = Arc::clone(&database_service);
    let service = Arc::clone(versions);
    let event_context = context.clone();
    tools.push(tool("versions.activate", "Restore file version", "Restore one saved revision to the indexed file's original path. Unsaved edits must first be saved as a version; requires explicit confirmation.", json!({"type":"object","properties":{"media_id":{"type":"integer","minimum":1},"version_id":{"type":"string","minLength":1}},"required":["media_id","version_id"],"additionalProperties":false}), true, true, move |args| {
        let database=Arc::clone(&database); let indexing=Arc::clone(&indexing); let service=Arc::clone(&service); let context=event_context.clone();
        Box::pin(async move { let r:ActivateRequest=parse(args)?; let path=media_path(&database,r.media_id)?; service.activate(&path,&r.version_id).await.map_err(|e|e.to_string())?; indexing.refresh_restored_file(r.media_id)?; emit_media_changed(&context).await?; emit_history_changed(&context,r.media_id).await?; Ok(json!({"ok":true,"media_id":r.media_id,"version_id":r.version_id,"activated":true})) })
    }));

    let database = Arc::clone(&database_service);
    let indexing = Arc::clone(&indexing_service);
    let service = Arc::clone(versions);
    let event_context = context.clone();
    tools.push(tool("versions.manage_branch", "Manage file branch", "Create, switch, archive, merge, resolve, or abort a Lore branch operation. File-changing operations refresh PhoTag after completion; requires explicit confirmation.", json!({"type":"object","properties":{"media_id":{"type":"integer","minimum":1},"action":{"type":"string","enum":["create","switch","archive","merge","resolve","abort"]},"name":{"type":"string"},"source_branch":{"type":"string"},"strategy":{"type":"string","enum":["ours","theirs"]},"overwrite":{"type":"boolean","default":false}},"required":["media_id","action"],"additionalProperties":false}), true, true, move |args| {
        let database=Arc::clone(&database); let indexing=Arc::clone(&indexing); let service=Arc::clone(&service); let context=event_context.clone();
        Box::pin(async move {
            let r:ManageBranchRequest=parse(args)?; let path=media_path(&database,r.media_id)?; let mut file_changed=false;
            let result=match r.action.as_str(){
                "create"=>json!({"ok":true,"action":"create","branch":service.create_branch(&path,r.name.as_deref().ok_or("create requires name")?).await.map_err(|e|e.to_string())?}),
                "switch"=>{let branch=service.switch_branch(&path,r.name.as_deref().ok_or("switch requires name")?,r.overwrite).await.map_err(|e|e.to_string())?; file_changed=true; json!({"ok":true,"action":"switch","branch":branch})},
                "archive"=>{let name=r.name.as_deref().ok_or("archive requires name")?; service.archive_branch(&path,name).await.map_err(|e|e.to_string())?; json!({"ok":true,"action":"archive","branch":name})},
                "merge"=>{let merge=service.merge_branch(&path,r.source_branch.as_deref().ok_or("merge requires source_branch")?).await.map_err(|e|e.to_string())?; file_changed=merge.completed; json!({"ok":true,"action":"merge","result":merge})},
                "resolve"=>{let merge=service.resolve_merge(&path,r.strategy.as_deref().ok_or("resolve requires strategy=ours|theirs")?).await.map_err(|e|e.to_string())?; file_changed=merge.completed; json!({"ok":true,"action":"resolve","result":merge})},
                "abort"=>{service.abort_merge(&path).await.map_err(|e|e.to_string())?; json!({"ok":true,"action":"abort"})},
                _=>return Err("action must be create, switch, archive, merge, resolve, or abort".into()),
            };
            if file_changed { indexing.refresh_restored_file(r.media_id)?; emit_media_changed(&context).await?; }
            emit_history_changed(&context,r.media_id).await?;
            Ok(result)
        })
    }));

    let registration = registry
        .register("photag.versions", tools)
        .map_err(|e| e.to_string())?;
    let _publication = context
        .provide(Arc::new(VersionsMcpRegistration {
            _registration: registration,
        }))
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn media_schema() -> Value {
    json!({"type":"object","properties":{"media_id":{"type":"integer","minimum":1}},"required":["media_id"],"additionalProperties":false})
}
fn media_path(database: &DatabaseService, id: i64) -> Result<String, String> {
    if id <= 0 {
        return Err("media_id must be positive".into());
    }
    database
        .get_media_by_id(id)
        .map_err(|e| e.to_string())?
        .map(|media| media.full_path)
        .ok_or_else(|| "media record was not found".into())
}
async fn emit_media_changed(context: &Context) -> Result<(), String> {
    context
        .emit::<photag_contracts::MediaIndexChanged>(cordis::Routing::Unscoped, ())
        .await
        .map_err(|e| e.to_string())
}
async fn emit_history_changed(context: &Context, media_id: i64) -> Result<(), String> {
    context
        .emit::<photag_contracts::MediaVersionHistoryChanged>(cordis::Routing::Unscoped, media_id)
        .await
        .map_err(|e| e.to_string())
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
struct MediaRequest {
    media_id: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BranchListRequest {
    media_id: i64,
    #[serde(default)]
    include_archived: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateRequest {
    media_id: i64,
    title: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ActivateRequest {
    media_id: i64,
    version_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManageBranchRequest {
    media_id: i64,
    action: String,
    name: Option<String>,
    source_branch: Option<String>,
    strategy: Option<String>,
    #[serde(default)]
    overwrite: bool,
}
struct VersionsMcpRegistration {
    _registration: Arc<McpToolRegistration>,
}
impl Service for VersionsMcpRegistration {
    const NAME: &'static str = "photag/versions-mcp-registration";
}
