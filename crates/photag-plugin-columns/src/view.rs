use std::sync::Arc;

use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::*;
use photag_plugin_workspaces::ColumnLayout;
use photag_ui_bridge::{UiViewContribution, components, refresh, theme};

use crate::{ColumnLayoutService, ColumnState};

pub(super) fn toolbar(
    service: Arc<ColumnLayoutService>,
    workspaces: Arc<photag_plugin_workspaces::WorkspaceService>,
) -> UiViewContribution {
    UiViewContribution::new("columns/toolbar", "command.columns", 0, move |_, app| {
        let service = Arc::clone(&service);
        app.new(|cx| {
            let refresh_task =
                refresh::listen(cx, service.wake(), |view: &mut ColumnToolbar, _| {
                    let latest = view.service.snapshot();
                    if latest
                        .as_ref()
                        .is_ok_and(|state| state.revision == view.state.revision)
                    {
                        return false;
                    }
                    if let Ok(latest) = latest {
                        view.state = latest;
                        true
                    } else {
                        false
                    }
                });
            let workspace_wake = workspaces.wake();
            let workspace_service = Arc::clone(&workspaces);
            let layout_service = Arc::clone(&service);
            let workspace_task =
                refresh::listen(cx, workspace_wake, move |view: &mut ColumnToolbar, _| {
                    let Ok(workspace) = workspace_service.active() else {
                        return false;
                    };
                    if view.state.layout == workspace.layout {
                        return false;
                    }
                    match layout_service.set_layout(workspace.layout) {
                        Ok(state) => {
                            view.state = state;
                            true
                        }
                        Err(error) => {
                            tracing::error!(%error, "could not apply activated workspace layout");
                            false
                        }
                    }
                });
            ColumnToolbar {
                _refresh_task: refresh_task,
                _workspace_task: workspace_task,
                state: service.snapshot().unwrap_or_default(),
                service,
            }
        })
        .into()
    })
}

struct ColumnToolbar {
    _refresh_task: Task<()>,
    _workspace_task: Task<()>,
    state: ColumnState,
    service: Arc<ColumnLayoutService>,
}

impl Render for ColumnToolbar {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let state = self.state;
        let service = Arc::clone(&self.service);
        components::icon_button(
            "columns-layout-menu",
            match state.layout {
                ColumnLayout::Single => gpui_kit::assets::IconName::Square,
                ColumnLayout::Dual => gpui_kit::assets::IconName::Columns2,
                ColumnLayout::Quad => gpui_kit::assets::IconName::Grid2x2,
            },
        )
        .w(px(theme::ICON_BUTTON_SIZE
            + theme::ICON_SIZE_SMALL
            + theme::SPACE_1))
        .tooltip(photag_ui_bridge::i18n::tr(
            "Column layout and active column",
        ))
        .accessibility_label(photag_ui_bridge::i18n::tr(
            "Column layout and active column",
        ))
        .dropdown_caret(true)
        .dropdown_menu(move |mut menu, _, _| {
            for (label, layout) in [
                ("Single column", ColumnLayout::Single),
                ("Two columns", ColumnLayout::Dual),
                ("Four columns", ColumnLayout::Quad),
            ] {
                let layout_service = Arc::clone(&service);
                menu = menu.item(
                    PopupMenuItem::new(photag_ui_bridge::i18n::tr(label))
                        .checked(state.layout == layout)
                        .on_click(move |_, _, _| {
                            if let Err(error) = layout_service.set_layout(layout) {
                                tracing::error!(%error, "could not change gallery column layout");
                            }
                        }),
                );
            }
            if state.layout.column_count() > 1 {
                for index in 0..state.layout.column_count() {
                    let column_service = Arc::clone(&service);
                    menu = menu.item(
                        PopupMenuItem::new(format!(
                            "{} {}",
                            photag_ui_bridge::i18n::tr("Active column"),
                            index + 1
                        ))
                        .checked(state.active_index == index)
                        .on_click(move |_, _, _| {
                            if let Err(error) = column_service.activate(index) {
                                tracing::error!(%error, "could not activate gallery column");
                            }
                        }),
                    );
                }
            }
            menu
        })
        .into_any_element()
    }
}
