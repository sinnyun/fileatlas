//! Lifecycle-owned tools; all theme application goes through Settings and UI Bridge.
use super::*;
use photag_contracts::mcp::{McpToolDefinition, McpToolRegistration, McpToolRegistry};
use serde_json::json;
struct Registration {
    _registration: Arc<McpToolRegistration>,
}
impl Service for Registration {
    const NAME: &'static str = "photag/material-theme/mcp";
}
pub fn register(context: &Context, service: Arc<MaterialThemeService>) -> Result<(), String> {
    let Ok(registry) = context.try_service::<McpToolRegistry>() else {
        return Ok(());
    };
    let state_service = service.clone();
    let state = McpToolDefinition {
        name: "material-theme.state".into(),
        title: "Material generator state".into(),
        description: "Read saved generator parameters, output path, generation/error state.".into(),
        input_schema: json!({"type":"object","properties":{},"additionalProperties":false})
            .as_object()
            .cloned()
            .unwrap_or_default(),
        mutating: false,
        confirmation_required: false,
        handler: Arc::new(move |_| {
            let service = state_service.clone();
            Box::pin(async move { Ok(json!(service.snapshot())) })
        }),
    };
    let generate=McpToolDefinition{
        name:"material-theme.generate".into(),title:"Generate native GPUI theme".into(),
        description:"Generate light/dark GPUI ThemeSet JSON from a #RRGGBB seed, save plugin parameters, optionally apply through Settings. Files stay in the current profile themes directory.".into(),
        input_schema:json!({"type":"object","properties":{"seed":{"type":"string"},"variant":{"enum":["neutral","tonal-spot","monochrome"]},"contrast":{"type":"number","minimum":0,"maximum":1},"apply":{"type":"boolean"}},"required":["seed"],"additionalProperties":false}).as_object().cloned().unwrap_or_default(),
        mutating:true,confirmation_required:false,
        handler:Arc::new(move|args|{let service=service.clone();Box::pin(async move{
            let mut p=Preferences::default();
            p.seed=args.get("seed").and_then(|v|v.as_str()).ok_or("seed required")?.into();
            if let Some(value)=args.get("variant"){p.variant=value.as_str().ok_or("variant must be string")?.into();}
            if let Some(value)=args.get("contrast"){p.contrast=value.as_f64().ok_or("contrast must be number")?;}
            Ok(json!(service.generate(p,args.get("apply").and_then(|v|v.as_bool()).unwrap_or(true))?))
        })}),
    };
    let registration = registry
        .register("photag.material-theme", [state, generate])
        .map_err(|e| e.to_string())?;
    let _registration = context
        .provide(Arc::new(Registration {
            _registration: registration,
        }))
        .map_err(|e| e.to_string())?;
    Ok(())
}
