use crate::{LauncherService, PinScope};
use fileatlas_contracts::mcp::{McpToolDefinition, McpToolFuture};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};

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

pub(super) fn tools(service: &Arc<LauncherService>) -> Vec<McpToolDefinition> {
    let mut tools = Vec::new();

    let launcher = Arc::clone(service);
    tools.push(tool("launcher.add_program", "Add program", "Add an executable or shortcut to the launcher list without running it.", json!({"type":"object","properties":{"path":{"type":"string","minLength":1}},"required":["path"],"additionalProperties":false}), true, false, move |args| {
        let launcher = Arc::clone(&launcher);
        Box::pin(async move {
            let request: PathRequest = parse(args)?;
            launcher.add_manual(&PathBuf::from(&request.path)).map_err(|error| error.to_string())?;
            Ok(json!({"ok":true,"path":request.path}))
        })
    }));

    let launcher = Arc::clone(service);
    tools.push(tool("launcher.set_pin", "Set program pin", "Set a launcher's pin scope to none, category, or all programs.", json!({"type":"object","properties":{"id":{"type":"integer","minimum":1},"scope":{"type":"string","enum":["none","category","all"]}},"required":["id","scope"],"additionalProperties":false}), true, false, move |args| {
        let launcher = Arc::clone(&launcher);
        Box::pin(async move {
            let request: PinRequest = parse(args)?;
            let scope = match request.scope.as_str() {"none"=>PinScope::None,"category"=>PinScope::Category,"all"=>PinScope::AllPrograms,_=>return Err("scope must be none, category, or all".into())};
            launcher.set_pin_scope(request.id, scope).map_err(|error| error.to_string())?;
            Ok(json!({"ok":true,"id":request.id,"scope":request.scope}))
        })
    }));

    let launcher = Arc::clone(service);
    tools.push(tool("launcher.create_category", "Create launcher category", "Create a category in the program launcher.", json!({"type":"object","properties":{"name":{"type":"string","minLength":1,"maxLength":80}},"required":["name"],"additionalProperties":false}), true, false, move |args| {
        let launcher = Arc::clone(&launcher);
        Box::pin(async move { let request: NameRequest=parse(args)?; launcher.add_category(&request.name).map_err(|error|error.to_string())?; Ok(json!({"ok":true,"name":request.name})) })
    }));

    let launcher = Arc::clone(service);
    tools.push(tool("launcher.rename_category", "Rename launcher category", "Rename a program launcher category.", json!({"type":"object","properties":{"id":{"type":"integer","minimum":1},"name":{"type":"string","minLength":1,"maxLength":80}},"required":["id","name"],"additionalProperties":false}), true, false, move |args| {
        let launcher = Arc::clone(&launcher);
        Box::pin(async move { let request: IdNameRequest=parse(args)?; launcher.rename_category(request.id,&request.name).map_err(|error|error.to_string())?; Ok(json!({"ok":true,"id":request.id,"name":request.name})) })
    }));

    let launcher = Arc::clone(service);
    tools.push(tool("launcher.delete_category", "Delete launcher category", "Delete a category and move its programs to General. Requires explicit confirmation.", json!({"type":"object","properties":{"id":{"type":"integer","minimum":1}},"required":["id"],"additionalProperties":false}), true, true, move |args| {
        let launcher = Arc::clone(&launcher);
        Box::pin(async move { let request: IdRequest=parse(args)?; launcher.delete_category(request.id).map_err(|error|error.to_string())?; Ok(json!({"deleted":true,"id":request.id})) })
    }));

    let launcher = Arc::clone(service);
    tools.push(tool("launcher.update_program", "Update launcher program", "Change a program display name, category, or hidden state; unspecified values are preserved.", json!({"type":"object","properties":{"id":{"type":"integer","minimum":1},"name":{"type":"string","minLength":1},"category_id":{"type":"integer","minimum":1},"hidden":{"type":"boolean"}},"required":["id"],"additionalProperties":false}), true, false, move |args| {
        let launcher = Arc::clone(&launcher);
        Box::pin(async move {
            let request: UpdateRequest=parse(args)?;
            if request.name.is_none() && request.category_id.is_none() && request.hidden.is_none() { return Err("provide name, category_id, or hidden".into()); }
            let entry=launcher.snapshot().entries.into_iter().find(|entry|entry.id==request.id).ok_or_else(||"program not found".to_owned())?;
            let name=request.name.as_deref().unwrap_or(&entry.display_name);
            let category=request.category_id.unwrap_or(entry.category_id);
            let hidden=request.hidden.unwrap_or(entry.hidden);
            launcher.update_entry(request.id,name,category,hidden).map_err(|error|error.to_string())?;
            Ok(json!({"updated":true,"id":request.id,"display_name":name,"category_id":category,"hidden":hidden}))
        })
    }));

    let launcher = Arc::clone(service);
    tools.push(tool(
        "launcher.refresh",
        "Refresh launcher discovery",
        "Refresh installed program discovery without launching or uninstalling programs.",
        json!({"type":"object","properties":{},"additionalProperties":false}),
        true,
        false,
        move |_| {
            let launcher = Arc::clone(&launcher);
            Box::pin(async move {
                launcher
                    .refresh_discovery()
                    .map_err(|error| error.to_string())?;
                Ok(json!({"started":true}))
            })
        },
    ));

    let launcher = Arc::clone(service);
    tools.push(tool("launcher.remove_program", "Remove program entry", "Remove a program from the launcher list without uninstalling its files. Requires explicit confirmation.", json!({"type":"object","properties":{"id":{"type":"integer","minimum":1}},"required":["id"],"additionalProperties":false}), true, true, move |args| {
        let launcher=Arc::clone(&launcher);
        Box::pin(async move { let request:IdRequest=parse(args)?; launcher.remove_entry(request.id).map_err(|error|error.to_string())?; Ok(json!({"removed":true,"id":request.id})) })
    }));

    let launcher = Arc::clone(service);
    tools.push(tool("launcher.uninstall_program", "Uninstall program", "Run the registered uninstaller for a program. This can remove software and requires explicit confirmation.", json!({"type":"object","properties":{"id":{"type":"integer","minimum":1}},"required":["id"],"additionalProperties":false}), true, true, move |args| {
        let launcher=Arc::clone(&launcher);
        Box::pin(async move { let request:IdRequest=parse(args)?; launcher.uninstall_confirmed(request.id).map_err(|error|error.to_string())?; Ok(json!({"uninstall_started":true,"id":request.id})) })
    }));

    tools
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PathRequest {
    path: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PinRequest {
    id: i64,
    scope: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NameRequest {
    name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IdNameRequest {
    id: i64,
    name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IdRequest {
    id: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateRequest {
    id: i64,
    name: Option<String>,
    category_id: Option<i64>,
    hidden: Option<bool>,
}
