//! Top-level composition only; the feature crate owns all whiteboard behavior.
use cordis::{Context, PreparedPlugin};
use fileatlas_plugin_settings::{PluginLaunchMode, PluginRuntimeRegistry, PluginStarter, Settings};
use std::sync::Arc;

pub(super) fn start(
    runtime: &tokio::runtime::Runtime,
    context: &Context,
    registry: &Arc<PluginRuntimeRegistry>,
    preferences: &Settings,
    enabled: bool,
) {
    let fiber = if enabled && preferences.plugin_is_enabled("fileatlas.whiteboard") {
        match runtime.block_on(context.spawn(PreparedPlugin::from_input(
            fileatlas_plugin_whiteboard::WhiteboardPlugin,
            (),
        ))) {
            Ok(fiber) => Some(fiber),
            Err(error) => {
                tracing::error!(%error, "could not start Zorite whiteboard plugin");
                None
            }
        }
    } else {
        None
    };
    let context = context.clone();
    let starter: PluginStarter = Arc::new(move || {
        let context = context.clone();
        Box::pin(async move {
            context
                .spawn(PreparedPlugin::from_input(
                    fileatlas_plugin_whiteboard::WhiteboardPlugin,
                    (),
                ))
                .await
                .map_err(|e| e.to_string())
        })
    });
    if let Err(error) = runtime.block_on(registry.register(
        "fileatlas.whiteboard",
        "FileAtlas Canvas",
        "Unified canvas workspace with shared resources and layers.",
        vec![
            PluginLaunchMode::WorkspaceAndPanel,
            PluginLaunchMode::PanelOnly,
        ],
        fiber,
        starter,
    )) {
        tracing::error!(%error, "could not register Zorite whiteboard runtime control");
    }
}
