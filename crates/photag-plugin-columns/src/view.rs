use std::sync::Arc;

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::*;
use photag_plugin_workspaces::ColumnLayout;
use photag_ui_bridge::{UiViewContribution, refresh};

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
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut controls = div()
            .h_flex()
            .items_center()
            .gap(px(4.))
            .child(layout_button(
                "columns-layout-single",
                "单栏",
                ColumnLayout::Single,
                self.state.layout,
                Arc::clone(&self.service),
                cx,
            ))
            .child(layout_button(
                "columns-layout-dual",
                "双栏",
                ColumnLayout::Dual,
                self.state.layout,
                Arc::clone(&self.service),
                cx,
            ))
            .child(layout_button(
                "columns-layout-quad",
                "四栏",
                ColumnLayout::Quad,
                self.state.layout,
                Arc::clone(&self.service),
                cx,
            ));

        if self.state.layout.column_count() > 1 {
            controls = controls.child(
                div()
                    .w(px(1.))
                    .h(px(20.))
                    .mx(px(4.))
                    .bg(rgb(photag_ui_bridge::theme::SEPARATOR)),
            );
            for index in 0..self.state.layout.column_count() {
                let service = Arc::clone(&self.service);
                let selected = self.state.active_index == index;
                let label = (index + 1).to_string();
                let button = Button::new(ElementId::Name(format!("columns-active-{index}").into()))
                    .compact()
                    .label(label)
                    .on_click(cx.listener(move |_, _, _, _| {
                        if let Err(error) = service.activate(index) {
                            tracing::error!(%error, "could not activate gallery column");
                        }
                    }));
                controls = controls.child(if selected {
                    button.primary()
                } else {
                    button.ghost()
                });
            }
        }

        controls.into_any_element()
    }
}

fn layout_button(
    id: &'static str,
    label: &'static str,
    layout: ColumnLayout,
    current: ColumnLayout,
    service: Arc<ColumnLayoutService>,
    cx: &mut Context<ColumnToolbar>,
) -> Button {
    let button = Button::new(id)
        .compact()
        .label(label)
        .on_click(cx.listener(move |_, _, _, _| {
            if let Err(error) = service.set_layout(layout) {
                tracing::error!(%error, "could not change gallery column layout");
            }
        }));
    if current == layout {
        button.primary()
    } else {
        button.ghost()
    }
}
