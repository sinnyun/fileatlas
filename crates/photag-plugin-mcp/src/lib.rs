#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
//! MCP server adapter for PhoTag's Cordis-owned application services.

mod input_activity;
mod params;
mod screenshot;
mod settings_page;
mod tools_commands;
mod tools_gallery_ui;
mod tools_read;
mod tools_ui;
mod tools_write;

use std::sync::Arc;

use cordis::{Context, Plugin, Service};
use params::*;
use photag_plugin_database::SortOrder;
use photag_plugin_gallery::GalleryService;
use photag_plugin_metadata_export::MetadataExportService;
use photag_plugin_navigation::NavigationService;
use photag_plugin_quicklook::QuickLookService;
use photag_plugin_screenshot::ScreenshotService;
use photag_plugin_tagging::TaggingService;
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::{
        router::tool::{ToolRoute, ToolRouter},
        wrapper::Parameters,
    },
    model::{
        CallToolRequestParams, CallToolResult, ContentBlock, ListToolsResult,
        PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
    },
    tool, tool_handler, tool_router,
    transport::{
        stdio,
        streamable_http_server::{
            StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
        },
    },
};
use serde_json::json;

/// Native PhoTag window handle, supplied by the Shell once GPUI creates it.
#[derive(Default)]
pub struct CaptureWindowHandle {
    native: std::sync::atomic::AtomicIsize,
    pointer: std::sync::Mutex<Option<McpPointerInput>>,
    key: std::sync::Mutex<Option<String>>,
    input_activity: std::sync::Mutex<input_activity::InputActivity>,
}

/// Same-process GPUI input, dispatched by the shell after its render lease ends.
#[derive(Clone, Copy)]
pub struct McpPointerInput {
    pub kind: &'static str,
    pub x: f32,
    pub y: f32,
    /// Vertical logical-pixel wheel delta; used only by `scroll`.
    pub scroll_y: f32,
}

impl Service for CaptureWindowHandle {
    const NAME: &'static str = "photag/mcp-window-handle";
}

impl CaptureWindowHandle {
    pub fn queue_key(&self, key: String) -> Result<(), String> {
        let mut pending = self.key.lock().map_err(|_| "MCP key queue unavailable")?;
        if pending.is_some() {
            return Err("MCP key request is still pending".into());
        }
        *pending = Some(key);
        Ok(())
    }
    pub fn key_pending(&self) -> bool {
        self.key.lock().is_ok_and(|p| p.is_some())
    }
    pub fn take_key(&self) -> Option<String> {
        self.key.lock().ok().and_then(|mut p| p.take())
    }
    pub fn set(&self, handle: isize) {
        self.native
            .store(handle, std::sync::atomic::Ordering::Release);
    }

    pub fn get(&self) -> isize {
        self.native.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn queue_pointer(&self, input: McpPointerInput) -> Result<(), String> {
        let mut pending = self
            .pointer
            .lock()
            .map_err(|_| "MCP pointer queue is unavailable")?;
        if pending.is_some() {
            return Err("MCP pointer request is still pending".into());
        }
        *pending = Some(input);
        Ok(())
    }

    pub fn pointer_pending(&self) -> bool {
        self.pointer.lock().is_ok_and(|pending| pending.is_some())
    }

    pub fn take_pointer(&self) -> Option<McpPointerInput> {
        self.pointer
            .lock()
            .ok()
            .and_then(|mut pending| pending.take())
    }
}

/// Starts an in-process Streamable HTTP server for the running app. The legacy
/// `--mcp` launch mode remains available for isolated stdio debugging.
pub struct McpPlugin {
    stdio: bool,
}

impl McpPlugin {
    pub fn in_process() -> Self {
        Self { stdio: false }
    }

    pub fn stdio() -> Self {
        Self { stdio: true }
    }
}

impl Plugin for McpPlugin {
    type Config = ();
    type Input = ();
    type PrepareError = std::convert::Infallible;
    type ApplyError = McpPluginError;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        "photag.mcp".into()
    }

    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }

    #[tracing::instrument(skip_all, err)]
    async fn apply(&self, context: Context, _config: &Self::Input) -> Result<(), Self::ApplyError> {
        let gallery = context
            .try_service::<GalleryService>()
            .map_err(|error| McpPluginError::Dependency(error.to_string()))?;
        let navigation = context
            .try_service::<NavigationService>()
            .map_err(|error| McpPluginError::Dependency(error.to_string()))?;
        let tagging = context
            .try_service::<TaggingService>()
            .map_err(|error| McpPluginError::Dependency(error.to_string()))?;
        let quicklook = context
            .try_service::<QuickLookService>()
            .map_err(|error| McpPluginError::Dependency(error.to_string()))?;
        let metadata_export = context
            .try_service::<MetadataExportService>()
            .map_err(|error| McpPluginError::Dependency(error.to_string()))?;
        let capture = context
            .try_service::<CaptureWindowHandle>()
            .map_err(|error| McpPluginError::Dependency(error.to_string()))?;
        let screenshot = context.try_service::<ScreenshotService>().ok();
        let i18n = context
            .try_service::<photag_plugin_i18n::I18nService>()
            .ok();
        let surfaces = context
            .try_service::<photag_ui_bridge::UiBridgeService>()
            .map_err(|error| McpPluginError::Dependency(error.to_string()))?;
        let command_catalog = surfaces.catalog.clone();
        let surfaces = surfaces.surfaces.clone();
        let history = context
            .try_service::<photag_plugin_history::HistoryService>()
            .map_err(|error| McpPluginError::Dependency(error.to_string()))?;
        let columns = context
            .try_service::<photag_plugin_columns::ColumnLayoutService>()
            .ok();
        let workspaces = context
            .try_service::<photag_plugin_workspaces::WorkspaceService>()
            .ok();
        let settings = context
            .try_service::<photag_plugin_settings::SettingsService>()
            .map_err(|error| McpPluginError::Dependency(error.to_string()))?;
        photag_ui_bridge::register_views_for_plugin(
            &context,
            "photag.mcp",
            [settings_page::contribution(Arc::clone(&settings))],
        )
        .map_err(|error| McpPluginError::Publish(error.to_string()))?;
        let server = PhoTagMcpServer::new(
            gallery,
            navigation,
            tagging,
            quicklook,
            metadata_export,
            capture,
            screenshot,
            i18n,
            surfaces,
            command_catalog,
            history,
            context
                .try_service::<photag_contracts::mcp::McpToolRegistry>()
                .map_err(|error| McpPluginError::Dependency(error.to_string()))?,
            settings,
            columns,
            workspaces,
            context.clone(),
        )?;
        let use_stdio = self.stdio;
        let listener = if use_stdio {
            None
        } else {
            let port = std::env::var("PHOTAG_MCP_HTTP_PORT")
                .ok()
                .and_then(|value| value.parse::<u16>().ok())
                .unwrap_or(38473);
            Some(
                tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
                    .await
                    .map_err(|error| McpPluginError::Bind(error.to_string()))?,
            )
        };
        let task = tokio::spawn(async move {
            if use_stdio {
                match server.serve(stdio()).await {
                    Ok(server) => {
                        if let Err(error) = server.waiting().await {
                            tracing::error!("PhoTag MCP stdio server stopped: {error}");
                        }
                    }
                    Err(error) => tracing::error!("PhoTag MCP stdio server failed: {error}"),
                }
            } else if let Some(listener) = listener {
                let port = listener
                    .local_addr()
                    .map(|address| address.port())
                    .unwrap_or(38473);
                let service = StreamableHttpService::new(
                    move || Ok(server.clone()),
                    Arc::new(LocalSessionManager::default()),
                    StreamableHttpServerConfig::default(),
                );
                let app = axum::Router::new().route_service("/mcp", service);
                tracing::info!(address = %format!("http://127.0.0.1:{port}/mcp"), "PhoTag in-process MCP endpoint ready");
                if let Err(error) = axum::serve(listener, app).await {
                    tracing::error!("PhoTag in-process MCP server stopped: {error}");
                }
            }
        });
        let abort = task.abort_handle();
        if let Err(error) = context.provide(Arc::new(McpTaskGuard(abort.clone()))) {
            // The transport task must not outlive a plugin whose lifecycle
            // guard could not be registered with Cordis.
            abort.abort();
            return Err(McpPluginError::Publish(error.to_string()));
        }
        Ok(())
    }
}

struct McpTaskGuard(tokio::task::AbortHandle);

impl Service for McpTaskGuard {
    const NAME: &'static str = "photag/mcp-task-guard";
}

impl Drop for McpTaskGuard {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[derive(Debug, thiserror::Error)]
pub enum McpPluginError {
    #[error("required MCP dependency unavailable: {0}")]
    Dependency(String),
    #[error("MCP HTTP endpoint could not bind to localhost: {0}")]
    Bind(String),
    #[error("MCP tool contribution registry failed: {0}")]
    Registry(String),
    #[error("MCP lifecycle guard publication failed: {0}")]
    Publish(String),
}

fn contributed_tool_route(
    contribution: photag_contracts::mcp::McpToolDefinition,
    settings: Arc<photag_plugin_settings::SettingsService>,
) -> ToolRoute<PhoTagMcpServer> {
    let name = contribution.name.clone();
    let description = contribution.description.clone();
    let title = contribution.title.clone();
    let mut schema = contribution.input_schema;
    if contribution.confirmation_required {
        let properties = schema
            .entry("properties")
            .or_insert_with(|| serde_json::json!({}));
        if let Some(properties) = properties.as_object_mut() {
            properties.insert(
                "confirm".into(),
                serde_json::json!({"type":"boolean","description":"Must be true to confirm this operation."}),
            );
        }
        let required = schema
            .entry("required")
            .or_insert_with(|| serde_json::json!([]));
        if let Some(required) = required.as_array_mut() {
            if !required.iter().any(|item| item.as_str() == Some("confirm")) {
                required.push(serde_json::Value::String("confirm".into()));
            }
        }
    }
    let mut tool = Tool::new_with_raw(name, Some(description.into()), Arc::new(schema));
    tool.title = Some(title);
    let handler = contribution.handler;
    let mutating = contribution.mutating;
    let confirmation_required = contribution.confirmation_required;
    let settings = Arc::clone(&settings);
    ToolRoute::new_dyn(tool, move |context| {
        let handler = Arc::clone(&handler);
        let settings = Arc::clone(&settings);
        let mut arguments = context
            .arguments
            .map(serde_json::Value::Object)
            .unwrap_or_else(|| serde_json::json!({}));
        Box::pin(async move {
            if mutating && !mcp_tool_access_allowed(&settings, confirmation_required) {
                return Ok(CallToolResult::error(vec![ContentBlock::text(
                    if confirmation_required {
                        "This high-impact MCP tool requires Full control in Settings → MCP → AI control."
                    } else {
                        "This MCP modification requires Standard control in Settings → MCP → AI control."
                    },
                )])
                .into());
            }
            if confirmation_required {
                let confirmed = arguments
                    .get("confirm")
                    .and_then(serde_json::Value::as_bool)
                    == Some(true);
                if !confirmed {
                    return Ok(CallToolResult::error(vec![ContentBlock::text(
                        "This operation requires confirm=true.",
                    )])
                    .into());
                }
                if let Some(object) = arguments.as_object_mut() {
                    object.remove("confirm");
                }
            }
            match handler(arguments).await {
                Ok(value) => {
                    Ok(CallToolResult::success(vec![ContentBlock::text(value.to_string())]).into())
                }
                Err(error) => Ok(CallToolResult::error(vec![ContentBlock::text(error)]).into()),
            }
        })
    })
}

#[derive(Clone)]
struct PhoTagMcpServer {
    context: Context,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
    command_catalog: Arc<photag_ui_bridge::UiCatalog>,
    history: Arc<photag_plugin_history::HistoryService>,
    gallery: Arc<GalleryService>,
    navigation: Arc<NavigationService>,
    tagging: Arc<TaggingService>,
    quicklook: Arc<QuickLookService>,
    metadata_export: Arc<MetadataExportService>,
    capture: Arc<CaptureWindowHandle>,
    screenshot: Option<Arc<ScreenshotService>>,
    i18n: Option<Arc<photag_plugin_i18n::I18nService>>,
    surfaces: Arc<photag_ui_bridge::UiSurfaceState>,
    settings: Arc<photag_plugin_settings::SettingsService>,
    columns: Option<Arc<photag_plugin_columns::ColumnLayoutService>>,
    workspaces: Option<Arc<photag_plugin_workspaces::WorkspaceService>>,
    mcp_tools: Arc<photag_contracts::mcp::McpToolRegistry>,
}

impl PhoTagMcpServer {
    fn new(
        gallery: Arc<GalleryService>,
        navigation: Arc<NavigationService>,
        tagging: Arc<TaggingService>,
        quicklook: Arc<QuickLookService>,
        metadata_export: Arc<MetadataExportService>,
        capture: Arc<CaptureWindowHandle>,
        screenshot: Option<Arc<ScreenshotService>>,
        i18n: Option<Arc<photag_plugin_i18n::I18nService>>,
        surfaces: Arc<photag_ui_bridge::UiSurfaceState>,
        command_catalog: Arc<photag_ui_bridge::UiCatalog>,
        history: Arc<photag_plugin_history::HistoryService>,
        mcp_tools: Arc<photag_contracts::mcp::McpToolRegistry>,
        settings: Arc<photag_plugin_settings::SettingsService>,
        columns: Option<Arc<photag_plugin_columns::ColumnLayoutService>>,
        workspaces: Option<Arc<photag_plugin_workspaces::WorkspaceService>>,
        context: Context,
    ) -> Result<Self, McpPluginError> {
        let tool_router =
            select_tool_routes(
                Self::read_tool_router(),
                &[
                    "screenshot_folder_picker",
                    "resize_window",
                    "screenshot_window",
                ],
            ) + select_tool_routes(Self::write_tool_router(), &["screenshot_quicklook"])
                + Self::commands_tool_router()
                + select_tool_routes(Self::ui_tool_router(), &["get_ui_state", "control_ui"])
                + Self::gallery_ui_tool_router();
        Ok(Self {
            context,
            tool_router,
            i18n,
            surfaces,
            command_catalog,
            history,
            gallery,
            navigation,
            tagging,
            quicklook,
            metadata_export,
            capture,
            screenshot,
            settings,
            columns,
            workspaces,
            mcp_tools,
        })
    }

    fn mutation_guard(&self) -> Result<(), String> {
        if mcp_access_level(&self.settings).allows_standard() {
            Ok(())
        } else {
            Err("请在设置 → MCP → AI 控制中启用“常规控制”或“完全控制”。".into())
        }
    }

    fn full_control_guard(&self) -> Result<(), String> {
        if mcp_access_level(&self.settings).allows_full() {
            Ok(())
        } else {
            Err("此高影响操作需要在设置 → MCP → AI 控制中启用“完全控制”。".into())
        }
    }

    fn mutations_enabled(&self) -> bool {
        mcp_access_level(&self.settings).allows_standard()
    }
}

fn mcp_access_level(
    settings: &photag_plugin_settings::SettingsService,
) -> photag_plugin_settings::McpAccessLevel {
    settings
        .snapshot()
        .map(|snapshot| snapshot.mcp_access_level)
        .unwrap_or_default()
}

fn mcp_tool_access_allowed(
    settings: &photag_plugin_settings::SettingsService,
    high_impact: bool,
) -> bool {
    let level = mcp_access_level(settings);
    if high_impact {
        level.allows_full()
    } else {
        level.allows_standard()
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for PhoTagMcpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(rmcp::model::Implementation::new("photag-mcp", env!("CARGO_PKG_VERSION")))
            .with_instructions("PhoTag 本机 MCP 接口。正常模式由已运行的 PhoTag GUI 进程通过 localhost Streamable HTTP 提供；功能插件通过 Cordis 生命周期注册自己的 namespaced 工具、参数 schema 和处理器。开发测试工具仅在 PHOTAG_DEV_PLUGINS 启用 dev-test 时可用。MCP 权限由设置 → MCP → AI 控制分为关闭、常规控制和完全控制；高影响操作需要完全控制且调用参数 confirm=true。")
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let mut tools = self
            .tool_router
            .list_all()
            .into_iter()
            .filter(|tool| host_tool_name(&tool.name))
            .collect::<Vec<_>>();
        tools.extend(self.contributed_tool_router()?.list_all());
        tools.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::CallToolResponse, McpError> {
        let contributed = self.contributed_tool_router()?;
        let target = if contributed.has_route(&request.name) {
            &contributed
        } else if host_tool_name(&request.name) {
            &self.tool_router
        } else {
            return Ok(CallToolResult::error(vec![ContentBlock::text(
                "This legacy unnamespaced feature tool has been retired; use the feature-owned namespaced tool listed by tools/list.",
            )]).into());
        };
        target
            .call(rmcp::handler::server::tool::ToolCallContext::new(
                self, request, context,
            ))
            .await
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        if host_tool_name(name) {
            self.tool_router.get(name).cloned()
        } else {
            self.contributed_tool_router().ok()?.get(name).cloned()
        }
    }
}

fn host_tool_name(name: &str) -> bool {
    matches!(
        name,
        "screenshot_window"
            | "screenshot_folder_picker"
            | "screenshot_quicklook"
            | "resize_window"
            | "get_ui_state"
            | "control_ui"
            | "control_navigation"
            | "control_gallery"
            | "command_list"
            | "command_run"
    )
}

fn select_tool_routes<S: Send + Sync + 'static>(
    router: ToolRouter<S>,
    names: &[&str],
) -> ToolRouter<S> {
    let mut selected = ToolRouter::new();
    for name in names {
        if let Some(route) = router.map.get(*name) {
            selected.add_route(route.clone());
        }
    }
    selected
}

impl PhoTagMcpServer {
    fn contributed_tool_router(&self) -> Result<ToolRouter<Self>, McpError> {
        let mut router = ToolRouter::new();
        let contributions = self
            .mcp_tools
            .snapshot()
            .map_err(|error| McpError::internal_error(error.to_string(), None))?;
        for contribution in contributions {
            router.add_route(contributed_tool_route(
                contribution,
                Arc::clone(&self.settings),
            ));
        }
        Ok(router)
    }
}

fn error_json(error: &str) -> String {
    tracing::error!(%error,"MCP request rejected");
    serde_json::to_string(&json!({ "error": error }))
        .unwrap_or_else(|_| "{\"error\":\"序列化失败\"}".into())
}
