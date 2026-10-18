#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
//! MCP server adapter for PhoTag's Cordis-owned application services.

mod input_activity;
mod params;
mod screenshot;
mod tools_commands;
mod tools_debug;
mod tools_dev_test;
mod tools_gallery_ui;
mod tools_read;
mod tools_ui;
mod tools_write;

use std::sync::Arc;

use cordis::{Context, Plugin, Service};
use params::*;
use photag_plugin_database::SortOrder;
use photag_plugin_dev_test::DevTestService;
use photag_plugin_file_operations::FileOperationsService;
use photag_plugin_gallery::GalleryService;
use photag_plugin_launcher::LauncherService;
use photag_plugin_metadata_export::MetadataExportService;
use photag_plugin_navigation::NavigationService;
use photag_plugin_quicklook::QuickLookService;
use photag_plugin_tagging::TaggingService;
use photag_plugin_versions::VersionsService;
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock, ServerCapabilities, ServerConfig},
    tool, tool_handler, tool_router,
    transport::stdio,
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

    fn get(&self) -> isize {
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

/// Starts an stdio server. Mutating tools are disabled unless the user opts in
/// through `PHOTAG_MCP_ALLOW_MUTATIONS=1` in the MCP server environment.
pub struct McpPlugin;

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
        let file_operations = context
            .try_service::<FileOperationsService>()
            .map_err(|error| McpPluginError::Dependency(error.to_string()))?;
        let capture = context
            .try_service::<CaptureWindowHandle>()
            .map_err(|error| McpPluginError::Dependency(error.to_string()))?;
        let dev_test = context.try_service::<DevTestService>().ok();
        let launcher = context.try_service::<LauncherService>().ok();
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
        let versions = context
            .try_service::<VersionsService>()
            .map_err(|error| McpPluginError::Dependency(error.to_string()))?;

        let server = PhoTagMcpServer::new(
            gallery,
            navigation,
            tagging,
            quicklook,
            metadata_export,
            file_operations,
            capture,
            dev_test,
            launcher,
            i18n,
            surfaces,
            command_catalog,
            history,
            versions,
            context.clone(),
        );
        let task = tokio::spawn(async move {
            match server.serve(stdio()).await {
                Ok(server) => {
                    if let Err(error) = server.waiting().await {
                        tracing::error!("PhoTag MCP server stopped: {error}");
                    }
                }
                Err(error) => tracing::error!("PhoTag MCP server failed: {error}"),
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
    #[error("MCP lifecycle guard publication failed: {0}")]
    Publish(String),
}

#[derive(Clone)]
struct PhoTagMcpServer {
    context: Context,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
    command_catalog: Arc<photag_ui_bridge::UiCatalog>,
    history: Arc<photag_plugin_history::HistoryService>,
    versions: Arc<VersionsService>,
    gallery: Arc<GalleryService>,
    navigation: Arc<NavigationService>,
    tagging: Arc<TaggingService>,
    quicklook: Arc<QuickLookService>,
    metadata_export: Arc<MetadataExportService>,
    file_operations: Arc<FileOperationsService>,
    capture: Arc<CaptureWindowHandle>,
    dev_test: Option<Arc<DevTestService>>,
    launcher: Option<Arc<LauncherService>>,
    i18n: Option<Arc<photag_plugin_i18n::I18nService>>,
    surfaces: Arc<photag_ui_bridge::UiSurfaceState>,
    mutations_enabled: bool,
}

impl PhoTagMcpServer {
    fn new(
        gallery: Arc<GalleryService>,
        navigation: Arc<NavigationService>,
        tagging: Arc<TaggingService>,
        quicklook: Arc<QuickLookService>,
        metadata_export: Arc<MetadataExportService>,
        file_operations: Arc<FileOperationsService>,
        capture: Arc<CaptureWindowHandle>,
        dev_test: Option<Arc<DevTestService>>,
        launcher: Option<Arc<LauncherService>>,
        i18n: Option<Arc<photag_plugin_i18n::I18nService>>,
        surfaces: Arc<photag_ui_bridge::UiSurfaceState>,
        command_catalog: Arc<photag_ui_bridge::UiCatalog>,
        history: Arc<photag_plugin_history::HistoryService>,
        versions: Arc<VersionsService>,
        context: Context,
    ) -> Self {
        Self {
            context,
            tool_router: Self::read_tool_router()
                + Self::write_tool_router()
                + Self::version_write_tool_router()
                + Self::dev_test_tool_router()
                + Self::commands_tool_router()
                + Self::ui_tool_router()
                + Self::gallery_ui_tool_router()
                + Self::debug_tool_router(),
            i18n,
            surfaces,
            command_catalog,
            history,
            versions,
            gallery,
            navigation,
            tagging,
            quicklook,
            metadata_export,
            file_operations,
            capture,
            dev_test,
            launcher,
            mutations_enabled: std::env::var("PHOTAG_MCP_ALLOW_MUTATIONS")
                .is_ok_and(|value| value == "1"),
        }
    }

    fn mutation_guard(&self) -> Result<(), String> {
        if self.mutations_enabled {
            Ok(())
        } else {
            Err("修改工具当前已禁用。只有在 MCP 服务环境中设置 PHOTAG_MCP_ALLOW_MUTATIONS=1 才会启用。".into())
        }
    }
}

impl PhoTagMcpServer {
    fn mutate_tags(&self, request: TagMutationRequest, assign: bool) -> String {
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        if let Err(error) = validate_batch(&request.media_ids) {
            return error_json(&error);
        }
        if !(1..=50).contains(&request.tag_ids.len()) {
            return error_json("tag_ids 数量必须为 1 到 50。");
        }
        let result = if assign {
            self.tagging.assign(&request.media_ids, &request.tag_ids)
        } else {
            self.tagging.remove(&request.media_ids, &request.tag_ids)
        };
        match result {
            Ok(()) => "{\"ok\":true}".into(),
            Err(error) => error_json(&error.to_string()),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for PhoTagMcpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(rmcp::model::Implementation::new("photag-mcp", env!("CARGO_PKG_VERSION")))
            .with_instructions("PhoTag 本机 MCP 接口。该服务运行在 MCP 客户端启动的 PhoTag 子进程内，可读取和控制这个窗口对应的应用实例。开发测试工具仅在 PHOTAG_DEV_PLUGINS 启用 dev-test 时可用。评分、标签和文件修改仅在启动环境显式设置 PHOTAG_MCP_ALLOW_MUTATIONS=1 后启用；文件操作的授权应由 MCP 客户端配置为每次询问用户。")
    }
}

fn validate_batch(ids: &[i64]) -> Result<(), String> {
    if !(1..=100).contains(&ids.len()) {
        Err("media_ids 数量必须为 1 到 100。".into())
    } else {
        Ok(())
    }
}

fn error_json(error: &str) -> String {
    tracing::error!(%error,"MCP request rejected");
    serde_json::to_string(&json!({ "error": error }))
        .unwrap_or_else(|_| "{\"error\":\"序列化失败\"}".into())
}
