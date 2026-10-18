use crate::DebugService;
use cordis::{Context, Service};
use photag_contracts::mcp::{
    McpToolDefinition, McpToolFuture, McpToolRegistration, McpToolRegistry,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

pub(super) fn register(context: &Context, service: &Arc<DebugService>) -> Result<(), String> {
    let Ok(registry) = context.try_service::<McpToolRegistry>() else {
        return Ok(());
    };
    let query_service = Arc::clone(service);
    let query = tool(
        "debug.query",
        "Query runtime diagnostics",
        "Read bounded runtime logs and the actual debug drawer state.",
        json!({"type":"object","properties":{"category":{"type":"string"},"level":{"type":"string"},"search":{"type":"string"},"page":{"type":"integer","minimum":0},"page_size":{"type":"integer","minimum":1,"maximum":200},"hide_ui_refresh":{"type":"boolean","default":false}},"additionalProperties":false}),
        false,
        move |args| {
            let service = Arc::clone(&query_service);
            Box::pin(async move {
                let request: DebugQuery = parse(args)?;
                let ui = service.ui_state();
                let mut filter = ui.filter.clone();
                filter.hide_ui_refresh = request.hide_ui_refresh.unwrap_or(false);
                if let Some(value) = request.category {
                    filter.category = value;
                }
                if let Some(value) = request.level {
                    filter.level = value;
                }
                if let Some(value) = request.search {
                    filter.search = value;
                }
                if let Some(value) = request.page {
                    filter.page = value;
                }
                if let Some(value) = request.page_size {
                    filter.page_size = value;
                }
                Ok(
                    json!({"ui":ui,"logs":service.query(&filter),"view_revision":service.view_revision(true)}),
                )
            })
        },
    );

    let control_service = Arc::clone(service);
    let control = tool(
        "debug.control",
        "Control debug drawer",
        "Control the real debug drawer, filters, paging, selection, copy/export, or clear. Requires mutation permission.",
        json!({"type":"object","properties":{"action":{"type":"string","enum":["open","close","toggle","category","level","search","page","page_size","pause","follow","select","select_ids","select_range","copy","export","clear"]},"value":{"type":"string"}},"required":["action"],"additionalProperties":false}),
        true,
        move |args| {
            let service = Arc::clone(&control_service);
            Box::pin(async move {
                let request: DebugControl = parse(args)?;
                service.control(&request.action, request.value.as_deref().unwrap_or(""))?;
                Ok(json!({"ok":true,"ui":service.ui_state()}))
            })
        },
    );

    let selected_service = Arc::clone(service);
    let selected = tool(
        "debug.selected_logs",
        "Read selected debug logs",
        "Read selected retained logs in the same text format used by Copy logs and Export.",
        json!({"type":"object","properties":{},"additionalProperties":false}),
        false,
        move |_| {
            let service = Arc::clone(&selected_service);
            Box::pin(async move {
                let text = service.selected_text()?;
                Ok(json!({"text":text,"selected_ids":service.ui_state().selected_ids}))
            })
        },
    );

    let registration = registry
        .register("photag.debug", vec![query, control, selected])
        .map_err(|e| e.to_string())?;
    let _publication = context
        .provide(Arc::new(DebugMcpRegistration {
            _registration: registration,
        }))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DebugQuery {
    category: Option<String>,
    level: Option<String>,
    search: Option<String>,
    page: Option<usize>,
    page_size: Option<usize>,
    hide_ui_refresh: Option<bool>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DebugControl {
    action: String,
    value: Option<String>,
}

fn parse<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, String> {
    serde_json::from_value(value).map_err(|error| format!("invalid parameters: {error}"))
}
fn tool<F>(
    name: &str,
    title: &str,
    description: &str,
    schema: Value,
    mutating: bool,
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
        confirmation_required: false,
        handler: Arc::new(handler),
    }
}
struct DebugMcpRegistration {
    _registration: Arc<McpToolRegistration>,
}
impl Service for DebugMcpRegistration {
    const NAME: &'static str = "photag/debug-mcp-registration";
}
