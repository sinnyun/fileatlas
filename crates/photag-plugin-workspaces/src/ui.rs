use crate::{
    CURRENT_SNAPSHOT_SURFACE_KEY, ColumnDefinition, ProgramWorkspace, SELECTION_SURFACE_KEY,
    WorkspaceService,
};
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::InputState;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use photag_ui_bridge::{UiSurfaceState, UiViewContribution, i18n::tr, theme};
use std::sync::Arc;

const SURFACE: &str = "workspace-manager";

pub(super) fn manager_view(
    service: Arc<WorkspaceService>,
    surfaces: Arc<UiSurfaceState>,
) -> UiViewContribution {
    UiViewContribution::new(
        "workspace/manager-panel",
        "workspace.modal",
        25,
        move |window, app| {
            let input = app.new(|cx| InputState::new(window, cx).placeholder(tr("Workspace name")));
            let body = app.new(|_| WorkspaceManager {
                service: Arc::clone(&service),
                surfaces: Arc::clone(&surfaces),
                input: input.clone(),
                editing: None,
                error: None,
            });
            let visible = Arc::clone(&surfaces);
            let close = Arc::clone(&surfaces);
            let panel_surfaces = Arc::clone(&surfaces);
            photag_ui_bridge::panels::mount(
                body.into(),
                photag_ui_bridge::panels::PanelOptions::new(
                    SURFACE,
                    tr("Program workspaces"),
                    (620., 520.),
                ),
                panel_surfaces,
                move || visible.is_open(SURFACE),
                move || {
                    let _ = close.set_open(SURFACE, false);
                },
                app,
            )
        },
    )
}

struct WorkspaceManager {
    service: Arc<WorkspaceService>,
    surfaces: Arc<UiSurfaceState>,
    input: Entity<InputState>,
    editing: Option<String>,
    error: Option<String>,
}

impl WorkspaceManager {
    fn perform(
        &mut self,
        action: impl FnOnce(&WorkspaceService) -> Result<(), crate::WorkspaceError>,
        cx: &mut Context<Self>,
    ) {
        self.error = action(&self.service).err().map(|error| error.to_string());
        cx.notify();
    }
}

impl Render for WorkspaceManager {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.surfaces.is_open(SURFACE) {
            return div().size_0().into_any_element();
        }
        let Ok(document) = self.service.snapshot() else {
            return div()
                .text_color(rgb(theme::danger_text()))
                .child(tr("Could not read program workspaces"))
                .into_any_element();
        };
        let active = self.service.active().ok().map(|w| w.id);
        let mut entries = div().v_flex().gap(px(theme::SPACE_2));
        for workspace in document.workspaces {
            let id = workspace.id.clone();
            let is_active = active.as_deref() == Some(id.as_str());
            let is_default = document.default_workspace_id == id;
            let is_editing = self.editing.as_deref() == Some(id.as_str());
            let select_id = id.clone();
            let default_id = id.clone();
            let rename_id = id.clone();
            let delete_id = id.clone();
            let rename_name = workspace.name.clone();
            let mut row = div()
                .h_flex()
                .items_center()
                .gap(px(theme::SPACE_2))
                .px(px(theme::SPACE_2))
                .py(px(theme::SPACE_1))
                .rounded(px(4.))
                .border_1()
                .border_color(rgb(theme::separator()));
            if is_editing {
                row = row
                    .child(photag_ui_bridge::components::text_input(&self.input).flex_1())
                    .child(
                        Button::new(format!("workspace-rename-save-{id}"))
                            .primary()
                            .compact()
                            .label(tr("Save name"))
                            .on_click(cx.listener(move |view, _, window, cx| {
                                let name = view.input.read(cx).value().trim().to_owned();
                                let target = id.clone();
                                view.perform(|service| service.rename(&target, &name), cx);
                                view.editing = None;
                                let _ = window;
                            })),
                    );
            } else {
                row = row
                    .child(
                        div().flex_1().min_w_0().v_flex().child(
                            div()
                                .h_flex()
                                .items_center()
                                .gap(px(theme::SPACE_1))
                                .child(if is_default {
                                    tr("Default workspace")
                                } else {
                                    workspace.name.clone()
                                })
                                .when(is_default, |this| {
                                    this.child(
                                        div()
                                            .text_color(rgb(theme::accent_text()))
                                            .text_size(px(theme::FONT_SIZE_SMALL))
                                            .child(tr("Default")),
                                    )
                                })
                                .when(is_active, |this| {
                                    this.child(
                                        div()
                                            .text_color(rgb(theme::text_secondary()))
                                            .text_size(px(theme::FONT_SIZE_SMALL))
                                            .child(tr("Active")),
                                    )
                                }),
                        ),
                    )
                    .child(
                        Button::new(format!("workspace-select-{id}"))
                            .ghost()
                            .compact()
                            .label(tr("Open"))
                            .on_click(cx.listener(move |view, _, _, cx| {
                                let id = select_id.clone();
                                let selected = view.service.activate(&id);
                                view.error = selected.err().map(|error| error.to_string());
                                let _ = view.surfaces.set_value(SELECTION_SURFACE_KEY, &id);
                                if let Ok(workspace) = view.service.active() {
                                    let _ = view.surfaces.set_value(
                                        "workspace.layout",
                                        &format!("{:?}", workspace.layout).to_lowercase(),
                                    );
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new(format!("workspace-default-{id}"))
                            .ghost()
                            .compact()
                            .label(tr("Set default"))
                            .on_click(cx.listener(move |view, _, _, cx| {
                                let id = default_id.clone();
                                view.perform(|service| service.set_default(&id), cx);
                            })),
                    )
                    .child(
                        Button::new(format!("workspace-rename-{id}"))
                            .ghost()
                            .compact()
                            .label(tr("Rename"))
                            .on_click(cx.listener(move |view, _, window, cx| {
                                view.input.update(cx, |input, cx| {
                                    input.set_value(&rename_name, window, cx)
                                });
                                view.editing = Some(rename_id.clone());
                                cx.notify();
                            })),
                    );
                if !is_default {
                    row = row.child(
                        Button::new(format!("workspace-delete-{id}"))
                            .ghost()
                            .compact()
                            .label(tr("Delete"))
                            .on_click(cx.listener(move |view, _, _, cx| {
                                let id = delete_id.clone();
                                view.perform(|service| service.remove(&id), cx);
                            })),
                    );
                }
            }
            entries = entries.child(row);
        }
        let name = self.input.read(cx).value().trim().to_owned();
        let error = self.error.clone();
        div().v_flex().gap(px(theme::SPACE_3)).p(px(theme::SPACE_4)).w_full().max_h(px(440.)).overflow_y_scrollbar()
            .child(div().font_semibold().child(tr("Program workspaces")))
            .child(div().text_size(px(theme::FONT_SIZE_SMALL)).text_color(rgb(theme::text_secondary()))
                .child(tr("Save launch layouts with independent folders and view settings for each column.")))
            .child(entries)
            .child(div().h_flex().items_center().gap(px(theme::SPACE_2))
                .child(photag_ui_bridge::components::text_input(&self.input).flex_1())
                .child(Button::new("workspace-create").primary().label(tr("Create workspace"))
                    .on_click(cx.listener(move |view, _, _, cx| {
                        let name = if name.is_empty() { tr("New workspace") } else { name.clone() };
                        let current = view
                            .surfaces
                            .value(CURRENT_SNAPSHOT_SURFACE_KEY)
                            .and_then(|json| serde_json::from_str::<ProgramWorkspace>(&json).ok())
                            .or_else(|| view.service.active().ok());
                        let layout = current.as_ref().map(|w| w.layout).unwrap_or_default();
                        let columns = current.map(|w| w.columns).unwrap_or_else(|| vec![ColumnDefinition::default()]);
                        let id = format!("workspace-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos());
                        let workspace = ProgramWorkspace { id: id.clone(), name, layout, columns };
                        view.error = view.service.upsert(workspace).err().map(|error| error.to_string());
                        if view.error.is_none() { view.error = view.service.activate(&id).err().map(|e| e.to_string()); let _ = view.surfaces.set_value(SELECTION_SURFACE_KEY, &id); }
                        if view.error.is_none() { let _ = view.surfaces.set_value("workspace.layout", &format!("{:?}", layout).to_lowercase()); }
                        cx.notify();
                    })))
                .child(Button::new("workspace-save-current").ghost().label(tr("Save current"))
                    .on_click(cx.listener(|view, _, _, cx| {
                        let active = view.service.active();
                        let workspace = view.surfaces.value(CURRENT_SNAPSHOT_SURFACE_KEY)
                            .map(|json| serde_json::from_str::<ProgramWorkspace>(&json).map_err(crate::WorkspaceError::from))
                            .unwrap_or(active);
                        view.error = workspace.and_then(|workspace| view.service.save_active(workspace)).err().map(|error| error.to_string());
                        cx.notify();
                    }))))
            .when_some(error, |this, error| this.child(div().text_color(rgb(theme::danger_text())).child(error)))
            .into_any_element()
    }
}
