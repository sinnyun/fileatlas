//! FileAtlas canvas plugin; the local engine copy supplies the editing model.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
mod extensions;
mod hooks;
mod images;
mod mcp;
mod store;
mod view;
use cordis::{Context, Plugin};
pub use extensions::{CanvasExtensionService, CanvasToolRegistration, register_tool_for_plugin};
pub use fileatlas_contracts::canvas::{CanvasToolContribution, CanvasToolGroup, CanvasToolIcon};
use fileatlas_ui_bridge::{ContributionKind, UiContribution, UiViewContribution};
use gpui::AppContext as _;
use std::sync::{Arc, Mutex};
pub use store::WhiteboardService;

const OWNER: &str = "fileatlas.whiteboard";
const PANEL: &str = "whiteboard/panel";
async fn background<T: Send + 'static>(
    cx: &gpui::AsyncApp,
    operation: impl FnOnce() -> T + Send + 'static,
) -> T {
    cx.background_executor()
        .spawn(async move { operation() })
        .await
}
pub const UPSTREAM: &str =
    "gpui-whiteboard 0.5.1 (Zorite d94ade3eb162c041b71994681356fcbcb4dcfd8e)";

pub struct WhiteboardPlugin;
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct WhiteboardError(String);
impl Plugin for WhiteboardPlugin {
    type Config = ();
    type Input = ();
    type PrepareError = std::convert::Infallible;
    type ApplyError = WhiteboardError;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        OWNER.into()
    }
    fn prepare(&self, _: ()) -> Result<(), Self::PrepareError> {
        Ok(())
    }
    async fn apply(&self, context: Context, _: &()) -> Result<(), WhiteboardError> {
        setup(context).map_err(WhiteboardError)
    }
}
fn setup(context: Context) -> Result<(), String> {
    fileatlas_canvas_engine::set_ui_translator(fileatlas_ui_bridge::i18n::tr);
    let bridge = context
        .try_service::<fileatlas_ui_bridge::UiBridgeService>()
        .map_err(|e| e.to_string())?;
    let settings = context
        .try_service::<fileatlas_plugin_settings::SettingsService>()
        .map_err(|e| e.to_string())?;
    let root = std::env::var_os("FILEATLAS_DEV_TEST_DIR")
        .map(std::path::PathBuf::from)
        .or_else(|| dirs::data_local_dir().map(|p| p.join("FileAtlas")))
        .ok_or("无法确定应用数据目录")?
        .join("whiteboards");
    let service = WhiteboardService::open(root)?;
    let image_cache_stats = Arc::new(Mutex::new(images::ImageCacheStats::default()));
    let cleanup = Arc::clone(&service);
    context
        .effect_sync(move || {
            if let Err(error) = cleanup.stop() {
                tracing::error!(%error, "whiteboard final flush failed");
            }
        })
        .map_err(|e| e.to_string())?;
    let _publication = context
        .provide(Arc::clone(&service))
        .map_err(|e| e.to_string())?;
    let thumbnails = context
        .try_service::<fileatlas_plugin_thumbnails::ThumbnailService>()
        .map_err(|e| e.to_string())?;
    let resources = context
        .try_service::<fileatlas_plugin_media_resources::MediaResourceService>()
        .map_err(|e| e.to_string())?;
    let eagle_links = context
        .try_service::<fileatlas_plugin_eagle_links::EagleLinkService>()
        .ok();
    let quicklook = context
        .try_service::<fileatlas_plugin_quicklook::QuickLookService>()
        .map_err(|e| e.to_string())?;
    let cordis = context.clone();
    let executor = tokio::runtime::Handle::current();
    let dispatch: Arc<extensions::CanvasCommandDispatch> = Arc::new(move |command_id| {
        let cordis = cordis.clone();
        executor.spawn(async move {
            if let Err(error) = cordis
                .emit::<fileatlas_ui_bridge::UiCommandInvoked>(
                    cordis::Routing::Unscoped,
                    command_id,
                )
                .await
            {
                tracing::error!(%error, "canvas extension command dispatch failed");
            }
        });
    });
    let extensions = Arc::new(extensions::CanvasExtensionService::new(dispatch));
    let _extension_publication = context
        .provide(Arc::clone(&extensions))
        .map_err(|e| e.to_string())?;
    let panels = Arc::clone(&bridge.panels);
    let command_panels = Arc::clone(&panels);
    let _command = context
        .on::<fileatlas_ui_bridge::UiCommandInvoked, _>(cordis::event::observer_sync(
            move |_, command| {
                if command == "whiteboard/open" {
                    let only = settings
                        .snapshot()
                        .map(|s| {
                            s.plugin_launch_mode(OWNER)
                                == fileatlas_plugin_settings::PluginLaunchMode::PanelOnly
                        })
                        .unwrap_or(false);
                    command_panels
                        .open_presented(PANEL, only)
                        .map_err(std::io::Error::other)?;
                }
                Ok::<_, std::io::Error>(())
            },
        ))
        .map_err(|e| e.to_string())?;
    fileatlas_ui_bridge::register_for_plugin(
        &context,
        OWNER,
        [
            UiContribution::new(
                PANEL,
                ContributionKind::Panel {
                    label: "画板".into(),
                    slot: PANEL.into(),
                    order: 0,
                },
            ),
            UiContribution::new(
                "whiteboard/open",
                ContributionKind::Command {
                    label: "打开画板".into(),
                    needs: fileatlas_ui_bridge::CommandNeeds::None,
                },
            ),
            UiContribution::new(
                "whiteboard/sidebar",
                ContributionKind::Widget {
                    label: "画板".into(),
                    slot: "navigation.launcher-shortcuts".into(),
                    order: 9,
                },
            ),
        ],
    )
    .map_err(|e| e.to_string())?;
    let panel_service = Arc::clone(&service);
    let panel_cache_stats = Arc::clone(&image_cache_stats);
    let panel_extensions = Arc::clone(&extensions);
    fileatlas_ui_bridge::panel_catalog::register_panel_for_plugin(
        &context,
        OWNER,
        fileatlas_ui_bridge::panel_catalog::PanelDefinition {
            options: fileatlas_ui_bridge::panels::PanelOptions {
                id: PANEL.into(),
                title: "FileAtlas 画布".into(),
                modal: false,
                default_size: (2060., 1150.),
                min_size: (760., 520.),
            },
            multiplicity: fileatlas_ui_bridge::panel_catalog::PanelMultiplicity::Single,
            factory: Arc::new(move |window, app| {
                view::panel(
                    Arc::clone(&panel_service),
                    Arc::clone(&thumbnails),
                    Arc::clone(&resources),
                    Arc::clone(&quicklook),
                    eagle_links.clone(),
                    Arc::clone(&panel_extensions),
                    Arc::clone(&panel_cache_stats),
                    window,
                    app,
                )
            }),
        },
    )
    .map_err(|e| e.to_string())?;
    let rail_panels = Arc::clone(&panels);
    fileatlas_ui_bridge::register_views_for_plugin(
        &context,
        OWNER,
        [UiViewContribution::new(
            "whiteboard/sidebar-view",
            "navigation.launcher-shortcuts",
            9,
            move |_, app| {
                app.new(|_| view::Rail {
                    panels: Arc::clone(&rail_panels),
                })
                .into()
            },
        )],
    )
    .map_err(|e| e.to_string())?;
    mcp::register(&context, service, panels, image_cache_stats)?;
    Ok(())
}
