use std::sync::Arc;

use cordis::{Context as CordisContext, Plugin};
use fileatlas_ui_bridge::{UiBridgeService, UiSurfaceState, UiViewContribution, theme};
use gpui_kit::assets::IconName;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::*;

pub struct ShellControlsPlugin;
impl Plugin for ShellControlsPlugin {
    type Config = ();
    type Input = ();
    type PrepareError = std::convert::Infallible;
    type ApplyError = ShellControlsError;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        "fileatlas.shell-controls".into()
    }
    fn prepare(&self, _: ()) -> Result<(), Self::PrepareError> {
        Ok(())
    }
    async fn apply(&self, context: CordisContext, _: &()) -> Result<(), Self::ApplyError> {
        let bridge = context
            .try_service::<UiBridgeService>()
            .map_err(|e| ShellControlsError(e.to_string()))?;
        let surfaces = Arc::clone(&bridge.surfaces);
        fileatlas_ui_bridge::register_views_for_plugin(
            &context,
            "fileatlas.shell-controls",
            [
                contribution(
                    "shell/sidebar-toggle",
                    "shell.title-left-action",
                    Action::Sidebar,
                    Arc::clone(&surfaces),
                ),
                contribution(
                    "shell/inspector-toggle",
                    "shell.toolbar-actions",
                    Action::Inspector,
                    Arc::clone(&surfaces),
                ),
                contribution(
                    "shell/workspaces",
                    "shell.toolbar-actions",
                    Action::Workspaces,
                    Arc::clone(&surfaces),
                ),
                contribution(
                    "shell/tools-toggle",
                    "shell.toolbar-actions",
                    Action::Tools,
                    surfaces,
                ),
            ],
        )
        .map_err(|e| ShellControlsError(e.to_string()))?;
        Ok(())
    }
}
#[derive(Debug, thiserror::Error)]
#[error("shell control registration failed: {0}")]
pub struct ShellControlsError(String);

#[derive(Clone, Copy)]
enum Action {
    Sidebar,
    Inspector,
    Workspaces,
    Tools,
}

struct ActionView {
    action: Action,
    surfaces: Arc<UiSurfaceState>,
}
impl Render for ActionView {
    fn render(&mut self, _: &mut Window, cx: &mut gpui_kit::Context<'_, Self>) -> impl IntoElement {
        let (id, label, icon, active) = match self.action {
            Action::Sidebar => (
                "shell-sidebar",
                "Toggle sidebar",
                if self.surfaces.value("sidebar.visible").as_deref() == Some("false") {
                    IconName::PanelLeft
                } else {
                    IconName::PanelLeftClose
                },
                self.surfaces.value("sidebar.visible").as_deref() != Some("false"),
            ),
            Action::Inspector => (
                "shell-inspector",
                "Toggle inspector",
                if self.surfaces.is_open("inspector") {
                    IconName::PanelRightClose
                } else {
                    IconName::PanelRight
                },
                self.surfaces.is_open("inspector"),
            ),
            Action::Workspaces => (
                "shell-workspaces",
                "Workspaces",
                IconName::LayoutGrid,
                self.surfaces.is_open("workspace-manager"),
            ),
            Action::Tools => (
                "shell-tools",
                "Tools",
                IconName::Wrench,
                self.surfaces.is_open("tools"),
            ),
        };
        let surfaces = Arc::clone(&self.surfaces);
        Button::new(id)
            .ghost()
            .compact()
            .icon(theme::small_icon(icon))
            .tooltip(fileatlas_ui_bridge::i18n::tr(label))
            .accessibility_label(fileatlas_ui_bridge::i18n::tr(label))
            .toggled(active)
            .on_click(cx.listener(move |_, _, _, cx| {
                match self_action(id) {
                    Action::Sidebar => {
                        let visible = surfaces.value("sidebar.visible").as_deref() != Some("false");
                        let _ = surfaces
                            .set_value("sidebar.visible", if visible { "false" } else { "true" });
                    }
                    Action::Inspector => {
                        let _ = surfaces.toggle("inspector");
                    }
                    Action::Workspaces => {
                        let _ = surfaces.set_open("workspace-manager", true);
                    }
                    Action::Tools => {
                        let _ = surfaces.toggle("tools");
                    }
                }
                cx.notify();
            }))
    }
}

fn self_action(id: &str) -> Action {
    match id {
        "shell-sidebar" => Action::Sidebar,
        "shell-inspector" => Action::Inspector,
        "shell-workspaces" => Action::Workspaces,
        _ => Action::Tools,
    }
}

fn contribution(
    id: &'static str,
    slot: &'static str,
    action: Action,
    surfaces: Arc<UiSurfaceState>,
) -> UiViewContribution {
    UiViewContribution::new(id, slot, 0, move |_, app| {
        app.new(|_| ActionView {
            action,
            surfaces: Arc::clone(&surfaces),
        })
        .into()
    })
}
