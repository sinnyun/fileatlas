//! Same-process runtime diagnostic tools, sharing the drawer's actual state.
use super::*;
use rmcp::schemars;
use serde::Deserialize;
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct DebugQuery {
    pub category: Option<String>,
    /// Exact ERROR/WARN/INFO/DEBUG/TRACE, or omitted for all.
    pub level: Option<String>,
    pub search: Option<String>,
    /// Zero based. Defaults to the drawer's current page/filter.
    pub page: Option<usize>,
    pub page_size: Option<usize>,
    /// True matches the drawer: hide routine repaint telemetry. Defaults to false for raw audits.
    pub hide_ui_refresh: Option<bool>,
}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct DebugControl {
    /// open/close/toggle/category/level/search/page/page_size/pause/follow/select/clear.
    pub action: String,
    pub value: Option<String>,
}
#[tool_router(router=debug_tool_router,vis="pub(super)")]
impl PhoTagMcpServer {
    #[tool(
        description = "Read bounded runtime logs, times, errors, span durations and the actual debug drawer state. Does not create diagnostic records."
    )]
    fn get_debug_info(&self, Parameters(query): Parameters<DebugQuery>) -> String {
        let Ok(s) = self
            .context
            .try_service::<photag_plugin_debug::DebugService>()
        else {
            return error_json("Debug service unavailable");
        };
        let ui = s.ui_state();
        let mut filter = ui.filter.clone();
        // Preserve full raw observations for existing idle/performance audits.
        filter.hide_ui_refresh = query.hide_ui_refresh.unwrap_or(false);
        if let Some(v) = query.category {
            filter.category = v;
        }
        if let Some(v) = query.level {
            filter.level = v;
        }
        if let Some(v) = query.search {
            filter.search = v;
        }
        if let Some(v) = query.page {
            filter.page = v;
        }
        if let Some(v) = query.page_size {
            filter.page_size = v;
        }
        json!({"ui":ui,"logs":s.query(&filter),"view_revision":s.view_revision(true)}).to_string()
    }
    #[tool(
        description = "Control the real bottom debug drawer: open/close, filters, zero-based page, page_size=25/50/100/200, pause/follow, select ID, clear logs. Requires mutation opt-in."
    )]
    fn control_debug(&self, Parameters(request): Parameters<DebugControl>) -> String {
        if let Err(e) = self.mutation_guard() {
            return error_json(&e);
        }
        let Ok(s) = self
            .context
            .try_service::<photag_plugin_debug::DebugService>()
        else {
            return error_json("Debug service unavailable");
        };
        match s.control(&request.action, request.value.as_deref().unwrap_or("")) {
            Ok(()) => json!({"ok":true,"ui":s.ui_state()}).to_string(),
            Err(e) => error_json(&e),
        }
    }
}
