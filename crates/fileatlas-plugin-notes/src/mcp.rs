use std::sync::Arc;

use cordis::{Context, Service};
use fileatlas_contracts::mcp::{
    McpToolDefinition, McpToolFuture, McpToolRegistration, McpToolRegistry,
};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::{NotesError, NotesService};

pub(super) fn register(context: &Context, service: &Arc<NotesService>) -> Result<(), NotesError> {
    let Ok(registry) = context.try_service::<McpToolRegistry>() else {
        return Ok(());
    };
    let service = Arc::clone(service);
    let mut tools = Vec::new();
    let notes = Arc::clone(&service);
    tools.push(tool(
        "notes.list",
        "List Markdown notes and metadata",
        false,
        false,
        json!({"type":"object","properties":{},"additionalProperties":false}),
        move |_| {
            let notes = Arc::clone(&notes);
            Box::pin(async move { Ok(json!({"items": notes.list().map_err(|e|e.to_string())?})) })
        },
    ));
    let notes = Arc::clone(&service);
    tools.push(tool("notes.search", "Search note titles, contents and tags", false, false, json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"],"additionalProperties":false}), move |args| {
        let notes = Arc::clone(&notes); Box::pin(async move { let request: Query = parse(args)?; Ok(json!({"items": notes.search(&request.query).map_err(|e|e.to_string())?})) })
    }));
    let notes = Arc::clone(&service);
    tools.push(tool("notes.read", "Read a note and its backlinks", false, false, json!({"type":"object","properties":{"id":{"type":"string","minLength":1}},"required":["id"],"additionalProperties":false}), move |args| {
        let notes = Arc::clone(&notes); Box::pin(async move { let request: Id = parse(args)?; Ok(json!(notes.read(&request.id).map_err(|e|e.to_string())?)) })
    }));
    let notes = Arc::clone(&service);
    tools.push(tool("notes.create", "Create a Markdown note", true, false, json!({"type":"object","properties":{"title":{"type":"string","minLength":1},"folder":{"type":"string"},"body":{"type":"string"}},"required":["title"],"additionalProperties":false}), move |args| {
        let notes = Arc::clone(&notes); Box::pin(async move { let request: Create = parse(args)?; Ok(json!(notes.create(&request.title, request.folder.as_deref(), request.body.as_deref()).map_err(|e|e.to_string())?)) })
    }));
    let notes = Arc::clone(&service);
    tools.push(tool("notes.update", "Update note body and metadata", true, false, json!({"type":"object","properties":{"id":{"type":"string","minLength":1},"body":{"type":"string"},"category":{"type":"string"},"status":{"type":"string","enum":["draft","in_progress","completed","archived"]},"tags":{"type":"array","items":{"type":"string"}}},"required":["id","body"],"additionalProperties":false}), move |args| {
        let notes = Arc::clone(&notes); Box::pin(async move { let request: Update = parse(args)?; Ok(json!(notes.update(&request.id, &request.body, request.category.as_deref(), request.status.as_deref(), request.tags).map_err(|e|e.to_string())?)) })
    }));
    let notes = Arc::clone(&service);
    tools.push(tool("notes.move", "Move a note to another nested folder", true, false, json!({"type":"object","properties":{"id":{"type":"string","minLength":1},"folder":{"type":"string"}},"required":["id"],"additionalProperties":false}), move |args| {
        let notes = Arc::clone(&notes); Box::pin(async move { let request: MoveNote = parse(args)?; Ok(json!(notes.move_note(&request.id, request.folder.as_deref()).map_err(|e|e.to_string())?)) })
    }));
    let notes = Arc::clone(&service);
    tools.push(tool("notes.rename", "Rename a note and update wiki links", true, false, json!({"type":"object","properties":{"id":{"type":"string","minLength":1},"title":{"type":"string","minLength":1}},"required":["id","title"],"additionalProperties":false}), move |args| {
        let notes = Arc::clone(&notes); Box::pin(async move { let request: Rename = parse(args)?; Ok(json!(notes.rename(&request.id, &request.title).map_err(|e|e.to_string())?)) })
    }));
    let notes = Arc::clone(&service);
    tools.push(tool("notes.create_folder", "Create a nested notes folder", true, false, json!({"type":"object","properties":{"parent":{"type":"string"},"name":{"type":"string","minLength":1}},"required":["name"],"additionalProperties":false}), move |args| {
        let notes = Arc::clone(&notes); Box::pin(async move { let request: Folder = parse(args)?; Ok(json!(notes.create_folder(request.parent.as_deref(), &request.name).map_err(|e|e.to_string())?)) })
    }));
    let notes = Arc::clone(&service);
    tools.push(tool("notes.trash", "Move a note to the recoverable trash folder", true, true, json!({"type":"object","properties":{"id":{"type":"string","minLength":1}},"required":["id"],"additionalProperties":false}), move |args| {
        let notes = Arc::clone(&notes); Box::pin(async move { let request: Id = parse(args)?; let path = notes.trash(&request.id).map_err(|e|e.to_string())?; let relative = path.strip_prefix(notes.root()).map_err(|_| "trash path escaped notes root")?; Ok(json!({"trashed":relative.to_string_lossy().replace('\\', "/")})) })
    }));
    let notes = Arc::clone(&service);
    tools.push(tool("notes.outline", "List Markdown headings", false, false, json!({"type":"object","properties":{"id":{"type":"string","minLength":1}},"required":["id"],"additionalProperties":false}), move |args| {
        let notes = Arc::clone(&notes); Box::pin(async move { let request: Id = parse(args)?; Ok(json!({"items": notes.outline(&request.id).map_err(|e|e.to_string())?})) })
    }));
    let notes = Arc::clone(&service);
    tools.push(tool("notes.restore", "Restore a trashed note", true, false, json!({"type":"object","properties":{"path":{"type":"string","minLength":1},"folder":{"type":"string"}},"required":["path"],"additionalProperties":false}), move |args| {
        let notes = Arc::clone(&notes); Box::pin(async move { let request: Restore = parse(args)?; Ok(json!(notes.restore(&request.path, request.folder.as_deref()).map_err(|e|e.to_string())?)) })
    }));
    let notes = Arc::clone(&service);
    tools.push(tool("notes.insert_image", "Copy an image into the note assets and return its Markdown link", true, false, json!({"type":"object","properties":{"id":{"type":"string","minLength":1},"source_path":{"type":"string","minLength":1}},"required":["id","source_path"],"additionalProperties":false}), move |args| {
        let notes = Arc::clone(&notes); Box::pin(async move { let request: Image = parse(args)?; let markdown = notes.insert_image(&request.id, std::path::Path::new(&request.source_path)).map_err(|e|e.to_string())?; Ok(json!({"markdown":markdown})) })
    }));
    let notes = Arc::clone(&service);
    tools.push(tool("notes.rename_folder", "Rename a nested notes folder", true, false, json!({"type":"object","properties":{"path":{"type":"string","minLength":1},"name":{"type":"string","minLength":1}},"required":["path","name"],"additionalProperties":false}), move |args| {
        let notes = Arc::clone(&notes); Box::pin(async move { let request: RenameFolder = parse(args)?; Ok(json!(notes.rename_folder(&request.path, &request.name).map_err(|e|e.to_string())?)) })
    }));
    let notes = Arc::clone(&service);
    tools.push(tool("notes.move_folder", "Move a nested notes folder", true, false, json!({"type":"object","properties":{"path":{"type":"string","minLength":1},"parent":{"type":"string"}},"required":["path"],"additionalProperties":false}), move |args| {
        let notes = Arc::clone(&notes); Box::pin(async move { let request: MoveFolder = parse(args)?; Ok(json!(notes.move_folder(&request.path, request.parent.as_deref()).map_err(|e|e.to_string())?)) })
    }));
    let notes = Arc::clone(&service);
    tools.push(tool("notes.trash_folder", "Move a nested notes folder to the recoverable trash", true, true, json!({"type":"object","properties":{"path":{"type":"string","minLength":1}},"required":["path"],"additionalProperties":false}), move |args| {
        let notes = Arc::clone(&notes); Box::pin(async move { let request: PathRequest = parse(args)?; let path = notes.trash_folder(&request.path).map_err(|e|e.to_string())?; Ok(json!({"path":path})) })
    }));
    let notes = Arc::clone(&service);
    tools.push(tool(
        "notes.trash_list",
        "List recoverable notes and folders",
        false,
        false,
        json!({"type":"object","properties":{},"additionalProperties":false}),
        move |_| {
            let notes = Arc::clone(&notes);
            Box::pin(
                async move { Ok(json!({"items":notes.list_trash().map_err(|e|e.to_string())?})) },
            )
        },
    ));
    let notes = Arc::clone(&service);
    tools.push(tool("notes.restore_folder", "Restore a trashed notes folder", true, false, json!({"type":"object","properties":{"path":{"type":"string","minLength":1},"parent":{"type":"string"}},"required":["path"],"additionalProperties":false}), move |args| {
        let notes = Arc::clone(&notes); Box::pin(async move { let request: RestoreFolder = parse(args)?; Ok(json!(notes.restore_folder(&request.path, request.parent.as_deref()).map_err(|e|e.to_string())?)) })
    }));
    let notes = Arc::clone(&service);
    tools.push(tool("notes.delete_trash", "Permanently delete a note or folder from trash", true, true, json!({"type":"object","properties":{"path":{"type":"string","minLength":1}},"required":["path"],"additionalProperties":false}), move |args| {
        let notes = Arc::clone(&notes); Box::pin(async move { let request: PathRequest = parse(args)?; notes.delete_trash(&request.path).map_err(|e|e.to_string())?; Ok(json!({"deleted":request.path})) })
    }));
    let registration = registry
        .register("fileatlas.notes", tools)
        .map_err(|e| NotesError::Plugin(e.to_string()))?;
    let _publication = context
        .provide(Arc::new(RegistrationOwner {
            _registration: registration,
        }))
        .map_err(|e| NotesError::Plugin(e.to_string()))?;
    Ok(())
}

fn tool<F>(
    name: &str,
    description: &str,
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
        title: description.into(),
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
struct Id {
    id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Query {
    query: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Create {
    title: String,
    folder: Option<String>,
    body: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Update {
    id: String,
    body: String,
    category: Option<String>,
    status: Option<String>,
    tags: Option<Vec<String>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rename {
    id: String,
    title: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MoveNote {
    id: String,
    folder: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Folder {
    parent: Option<String>,
    name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Restore {
    path: String,
    folder: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RestoreFolder {
    path: String,
    parent: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Image {
    id: String,
    source_path: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenameFolder {
    path: String,
    name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MoveFolder {
    path: String,
    parent: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PathRequest {
    path: String,
}
struct RegistrationOwner {
    _registration: Arc<McpToolRegistration>,
}
impl Service for RegistrationOwner {
    const NAME: &'static str = "fileatlas/notes-mcp-registration";
}
