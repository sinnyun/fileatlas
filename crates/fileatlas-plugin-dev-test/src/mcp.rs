use crate::DevTestService;
use cordis::{Context, Service};
use fileatlas_contracts::mcp::{
    McpToolDefinition, McpToolFuture, McpToolRegistration, McpToolRegistry,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

pub(super) fn register(context: &Context, service: &Arc<DevTestService>) -> Result<(), String> {
    let Ok(registry) = context.try_service::<McpToolRegistry>() else {
        return Ok(());
    };
    let status_service = Arc::clone(service);
    let status = tool(
        "dev-test.status",
        "Read development test status",
        "Read isolated fixture task status and progress.",
        json!({"type":"object","properties":{},"additionalProperties":false}),
        false,
        move |_| {
            let service = Arc::clone(&status_service);
            Box::pin(async move {
                Ok(
                    json!({"available":true,"running":service.is_running(),"status":service.status()}),
                )
            })
        },
    );
    let scan_service = Arc::clone(service);
    let scan = tool(
        "dev-test.scan_fixture",
        "Scan media fixture",
        "Scan the configured development media fixture with the production indexer. Runs asynchronously.",
        json!({"type":"object","properties":{},"additionalProperties":false}),
        true,
        move |_| {
            let service = Arc::clone(&scan_service);
            Box::pin(async move {
                let accepted = service.scan_fixture();
                Ok(
                    json!({"accepted":accepted,"running":service.is_running(),"status":service.status()}),
                )
            })
        },
    );
    let inject_service = Arc::clone(service);
    let inject = tool(
        "dev-test.inject_virtual",
        "Inject virtual media rows",
        "Replace isolated development database rows for FileManager scale validation. No physical files are created.",
        json!({"type":"object","properties":{"count":{"type":"integer","enum":[1000,10000,100000]}},"required":["count"],"additionalProperties":false}),
        true,
        move |args| {
            let service = Arc::clone(&inject_service);
            Box::pin(async move {
                let request: CountRequest = parse(args)?;
                let accepted = service.inject_virtual(request.count);
                Ok(
                    json!({"accepted":accepted,"requested_count":request.count,"running":service.is_running(),"status":service.status()}),
                )
            })
        },
    );
    let clear_service = Arc::clone(service);
    let clear = tool(
        "dev-test.clear_virtual",
        "Clear virtual media rows",
        "Remove only reserved virtual test rows from the isolated development database.",
        json!({"type":"object","properties":{},"additionalProperties":false}),
        true,
        move |_| {
            let service = Arc::clone(&clear_service);
            Box::pin(async move {
                let accepted = service.clear_virtual();
                Ok(
                    json!({"accepted":accepted,"running":service.is_running(),"status":service.status()}),
                )
            })
        },
    );
    let registration = registry
        .register("fileatlas.dev-test", vec![status, scan, inject, clear])
        .map_err(|e| e.to_string())?;
    let _publication = context
        .provide(Arc::new(DevTestMcpRegistration {
            _registration: registration,
        }))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CountRequest {
    count: usize,
}
fn parse<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, String> {
    serde_json::from_value(value).map_err(|e| format!("invalid parameters: {e}"))
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
struct DevTestMcpRegistration {
    _registration: Arc<McpToolRegistration>,
}
impl Service for DevTestMcpRegistration {
    const NAME: &'static str = "fileatlas/dev-test-mcp-registration";
}
