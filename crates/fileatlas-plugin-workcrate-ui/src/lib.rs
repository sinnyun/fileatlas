//! WorkCrate project and board UI contribution.
use ::gpui::{Context as ViewContext, Window, px};
use cordis::{Context, Plugin};
use fileatlas_plugin_workcrate_boards::WorkCrateBoardsService;
use fileatlas_plugin_workcrate_export::WorkCrateExportService;
use fileatlas_plugin_workcrate_projects::{WorkCrateProjectsError, WorkCrateProjectsService};
use fileatlas_ui_bridge::{
    panel_catalog::{PanelCatalog, PanelDefinition, PanelMultiplicity},
    theme,
};
use gpui_kit::{base::StyledExt as _, *};
use std::sync::Arc;
use thiserror::Error;

const OWNER: &str = "fileatlas.workcrate-ui";
const PANEL: &str = "workcrate/overview";
const COMMAND: &str = "workcrate/open";

pub struct WorkCrateUiPlugin;
impl Plugin for WorkCrateUiPlugin {
    type Config = ();
    type Input = ();
    type PrepareError = std::convert::Infallible;
    type ApplyError = UiError;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        OWNER.into()
    }
    fn prepare(&self, value: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(value)
    }
    async fn apply(&self, context: Context, _input: &Self::Input) -> Result<(), Self::ApplyError> {
        let projects = context
            .try_service::<WorkCrateProjectsService>()
            .map_err(plugin_error)?;
        let boards = context
            .try_service::<WorkCrateBoardsService>()
            .map_err(plugin_error)?;
        let exports = context
            .try_service::<WorkCrateExportService>()
            .map_err(plugin_error)?;
        let thumbnails = context
            .try_service::<fileatlas_plugin_thumbnails::ThumbnailService>()
            .map_err(plugin_error)?;
        let file_operations = context
            .try_service::<fileatlas_plugin_file_operations::FileOperationsService>()
            .map_err(plugin_error)?;
        let bridge = context
            .try_service::<fileatlas_ui_bridge::UiBridgeService>()
            .map_err(plugin_error)?;
        let ui_state = Arc::new(view::WorkCrateUiState::default());
        let _ui_state = context
            .provide(Arc::clone(&ui_state))
            .map_err(plugin_error)?;
        crate::mcp::register(
            &context,
            &projects,
            &boards,
            &exports,
            &bridge.panels,
            &ui_state,
        )
        .map_err(plugin_error)?;
        let panels = Arc::clone(&bridge.panels);
        let command_panels = Arc::clone(&panels);
        let settings = context
            .try_service::<fileatlas_plugin_settings::SettingsService>()
            .ok();
        let _command = context
            .on::<fileatlas_ui_bridge::UiCommandInvoked, _>(cordis::event::observer_sync(
                move |_, id| {
                    if id == COMMAND {
                        let only = settings
                            .as_ref()
                            .and_then(|s| s.snapshot().ok())
                            .is_some_and(|s| {
                                s.plugin_launch_mode(OWNER)
                                    == fileatlas_plugin_settings::PluginLaunchMode::PanelOnly
                            });
                        let _ = command_panels.open_presented(PANEL, only);
                    }
                    Ok::<_, std::convert::Infallible>(())
                },
            ))
            .map_err(plugin_error)?;
        fileatlas_ui_bridge::register_for_plugin(
            &context,
            OWNER,
            [
                fileatlas_ui_bridge::UiContribution::new(
                    PANEL,
                    fileatlas_ui_bridge::ContributionKind::Panel {
                        label: "WorkCrate 项目画板".into(),
                        slot: PANEL.into(),
                        order: 0,
                    },
                ),
                fileatlas_ui_bridge::UiContribution::new(
                    COMMAND,
                    fileatlas_ui_bridge::ContributionKind::Command {
                        label: "打开 WorkCrate 项目画板".into(),
                        needs: fileatlas_ui_bridge::CommandNeeds::None,
                    },
                ),
            ],
        )
        .map_err(plugin_error)?;
        let p = Arc::clone(&projects);
        let b = Arc::clone(&boards);
        let e = Arc::clone(&exports);
        let surfaces = Arc::clone(&bridge.surfaces);
        fileatlas_ui_bridge::panel_catalog::register_panel_for_plugin(
            &context,
            OWNER,
            PanelDefinition {
                options: fileatlas_ui_bridge::panels::PanelOptions {
                    id: PANEL.into(),
                    title: "WorkCrate".into(),
                    modal: false,
                    default_size: (1560., 980.),
                    min_size: (760., 560.),
                },
                multiplicity: PanelMultiplicity::Single,
                factory: Arc::new(move |window, app| {
                    crate::view::panel(
                        Arc::clone(&p),
                        Arc::clone(&b),
                        Arc::clone(&e),
                        Arc::clone(&surfaces),
                        Arc::clone(&ui_state),
                        Arc::clone(&thumbnails),
                        Arc::clone(&file_operations),
                        window,
                        app,
                    )
                }),
            },
        )
        .map_err(plugin_error)?;
        fileatlas_ui_bridge::register_views_for_plugin(&context, OWNER, [sidebar_button(panels)])
            .map_err(plugin_error)?;
        Ok(())
    }
}

fn plugin_error(error: impl std::fmt::Display) -> UiError {
    UiError::Plugin(error.to_string())
}

#[derive(Debug, Error)]
pub enum UiError {
    #[error("Cordis service or UI registration failed: {0}")]
    Plugin(String),
    #[error(transparent)]
    Projects(#[from] WorkCrateProjectsError),
    #[error("board service failed: {0}")]
    Boards(String),
}

fn sidebar_button(panels: Arc<PanelCatalog>) -> fileatlas_ui_bridge::UiViewContribution {
    fileatlas_ui_bridge::UiViewContribution::new(
        "workcrate/sidebar",
        "navigation.launcher-shortcuts",
        12,
        move |_, app| {
            let panels = Arc::clone(&panels);
            app.new(|_| WorkCrateSidebarButton { panels }).into()
        },
    )
}

struct WorkCrateSidebarButton {
    panels: Arc<PanelCatalog>,
}
impl Render for WorkCrateSidebarButton {
    fn render(&mut self, _: &mut Window, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .id("workcrate-open")
            .v_flex()
            .w(px(42.))
            .h(px(54.))
            .items_center()
            .justify_center()
            .gap(px(4.))
            .rounded(px(6.))
            .text_xs()
            .text_color(rgb(theme::text_secondary()))
            .hover(|s| s.bg(rgba(theme::fill_hover())))
            .on_click(cx.listener(|this, _, _, _| {
                if let Err(error) = this.panels.open(PANEL) {
                    tracing::error!(%error, "could not open WorkCrate panel");
                }
            }))
            .child(theme::icon(gpui_kit::assets::IconName::Briefcase))
            .child("项目")
    }
}

mod mcp;
mod view;
