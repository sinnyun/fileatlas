use crate::{MetadataExportService, MetadataExportStatus};
use cordis::{Context, Service};
use photag_contracts::mcp::{McpToolDefinition, McpToolRegistration, McpToolRegistry};
use serde::Deserialize;
use serde_json::json;
use std::{path::PathBuf, sync::Arc, time::Duration};

pub(super) fn register(
    context: &Context,
    export: &Arc<MetadataExportService>,
) -> Result<(), String> {
    let Ok(registry) = context.try_service::<McpToolRegistry>() else {
        return Ok(());
    };
    let service = Arc::clone(export);
    let tool = McpToolDefinition {
        name: "metadata-export.export".into(),
        title: "Export metadata backup".into(),
        description: "Write PhoTag's complete metadata backup JSON to the supplied path. Requires explicit confirmation because it creates or replaces a file.".into(),
        input_schema: json!({"type":"object","properties":{"path":{"type":"string","minLength":1}},"required":["path"],"additionalProperties":false})
            .as_object().cloned().unwrap_or_default(),
        mutating: true,
        confirmation_required: true,
        handler: Arc::new(move |arguments| {
            let service = Arc::clone(&service);
            Box::pin(async move {
                let request: ExportRequest = serde_json::from_value(arguments)
                    .map_err(|error| format!("invalid parameters: {error}"))?;
                if request.path.trim().is_empty() {
                    return Err("path must not be empty".into());
                }
                service.set_destination(PathBuf::from(request.path))
                    .map_err(|error| error.to_string())?;
                service.request_export().map_err(|error| error.to_string())?;
                tokio::time::timeout(Duration::from_secs(600), async {
                    loop {
                        match service.status().map_err(|error| error.to_string())? {
                            MetadataExportStatus::Succeeded { path, records } => {
                                return Ok(json!({"ok":true,"path":path,"records":records}));
                            }
                            MetadataExportStatus::Failed(error) => return Err(error),
                            MetadataExportStatus::Idle | MetadataExportStatus::Exporting => {
                                tokio::time::sleep(Duration::from_millis(100)).await;
                            }
                        }
                    }
                })
                .await
                .map_err(|_| "metadata export timed out after 10 minutes".to_owned())?
            })
        }),
    };
    let registration = registry
        .register("photag.metadata-export", [tool])
        .map_err(|error| error.to_string())?;
    let _publication = context
        .provide(Arc::new(MetadataExportMcpRegistration {
            _registration: registration,
        }))
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportRequest {
    path: String,
}

struct MetadataExportMcpRegistration {
    _registration: Arc<McpToolRegistration>,
}

impl Service for MetadataExportMcpRegistration {
    const NAME: &'static str = "photag/metadata-export-mcp-registration";
}
