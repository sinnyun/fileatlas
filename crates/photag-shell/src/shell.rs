use super::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::ButtonVariants;

mod native_panels;
mod panel_resize;
use panel_resize::PanelSide;

pub(super) struct PhoTagShell {
    pub(super) catalog: Arc<UiCatalog>,
    pub(super) views: Vec<(String, u64, String, Option<String>, gpui_kit::AnyView)>,
    pub(super) view_catalog_signature: Vec<(String, u64)>,
    pub(super) view_catalog: Arc<photag_ui_bridge::UiViewCatalog>,
    pub(super) cordis: CordisContext,
    pub(super) executor: tokio::runtime::Handle,
    pub(super) session: Arc<photag_plugin_session::SessionService>,
    pub(super) surfaces: Arc<photag_ui_bridge::UiSurfaceState>,
    pub(super) sidebar_width: f32,
    pub(super) inspector_width: f32,
    pub(super) viewport_width: f32,
    pub(super) capture: Arc<photag_plugin_mcp::CaptureWindowHandle>,
    pub(super) native_window_handle: isize,
    pub(super) drag_origin_x: f32,
    pub(super) drag_origin_width: f32,
    pub(super) resizing_panel: Option<PanelSide>,
    pub(super) tools_open: bool,
    pub(super) sidebar_open: bool,
    pub(super) inspector_open: bool,
    pub(super) native_panels: native_panels::NativePanels,
    pub(super) panel_signature: (bool, Option<String>),
    pub(super) column_refresh_task: Option<gpui_kit::Task<()>>,
    pub(super) column_revision: u64,
}

impl Render for PhoTagShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.column_refresh_task.is_none() {
            if let Ok(service) = self
                .cordis
                .try_service::<photag_plugin_columns::ColumnLayoutService>()
            {
                let wake = service.wake();
                self.column_revision = service.snapshot().map(|state| state.revision).unwrap_or(0);
                self.column_refresh_task = Some(photag_ui_bridge::refresh::listen(
                    cx,
                    wake,
                    move |shell, _cx| {
                        let Ok(state) = service.snapshot() else {
                            return false;
                        };
                        if state.revision == shell.column_revision {
                            return false;
                        }
                        shell.column_revision = state.revision;
                        true
                    },
                ));
            }
        }
        self.viewport_width = f32::from(window.bounds().size.width);
        self.dispatch_mcp_pointer(window, cx);
        let mut previous = std::mem::take(&mut self.views);
        self.views = self
            .view_catalog
            .snapshot()
            .unwrap_or_default()
            .into_iter()
            .filter(|entry| entry.contribution.slot != "settings.language")
            .map(|entry| {
                let c = entry.contribution;
                let view = if let Some(index) =
                    previous.iter().position(|(id, generation, _, _, _)| {
                        id == &c.id && *generation == entry.generation
                    }) {
                    previous.remove(index).4
                } else {
                    (c.factory)(window, cx)
                };
                (c.id, entry.generation, c.slot, c.panel_id, view)
            })
            .collect();
        self.tools_open = self.surfaces.is_open("tools");
        self.inspector_open = self.surfaces.is_open("inspector");
        let contributions = self.catalog.snapshot().unwrap_or_default();
        let mut breadcrumb_views = Vec::new();
        let mut search_views = Vec::new();
        let mut size_views = Vec::new();
        let mut filter_views = Vec::new();
        let mut column_control_views = Vec::new();
        let mut status_views = Vec::new();
        let mut debug_status = Vec::new();
        let mut bottom_views = Vec::new();
        let mut tool_views = Vec::new();
        let mut sidebar_views = Vec::new();
        let mut content_views = Vec::new();
        let mut column_addresses: [Vec<gpui_kit::AnyView>; 4] = std::array::from_fn(|_| Vec::new());
        let mut column_contents: [Vec<gpui_kit::AnyView>; 4] = std::array::from_fn(|_| Vec::new());
        let mut column_filters: [Vec<gpui_kit::AnyView>; 4] = std::array::from_fn(|_| Vec::new());
        let mut column_searches: [Vec<gpui_kit::AnyView>; 4] = std::array::from_fn(|_| Vec::new());
        let mut column_sizes: [Vec<gpui_kit::AnyView>; 4] = std::array::from_fn(|_| Vec::new());
        let mut column_status: [Vec<gpui_kit::AnyView>; 4] = std::array::from_fn(|_| Vec::new());
        let mut column_tools: [Vec<gpui_kit::AnyView>; 4] = std::array::from_fn(|_| Vec::new());
        let mut column_inspectors: [Vec<gpui_kit::AnyView>; 4] =
            std::array::from_fn(|_| Vec::new());
        let mut column_modals: [Vec<gpui_kit::AnyView>; 4] = std::array::from_fn(|_| Vec::new());
        let mut context_views = Vec::new();
        let mut inspector_views = Vec::new();
        let mut auxiliary_views = Vec::new();
        let mut modal_views = Vec::new();
        let mut active_panel = None;
        let active = self.surfaces.value("panel.active");
        for (_, _, slot, panel_id, view) in &self.views {
            match slot.as_str() {
                "command.breadcrumb" => breadcrumb_views.push(view.clone()),
                "command.search" => search_views.push(view.clone()),
                "command.size" => size_views.push(view.clone()),
                "command.filters" => filter_views.push(view.clone()),
                "command.columns" => column_control_views.push(view.clone()),
                slot if slot.starts_with("command.column.") => {
                    let mut parts = slot.split('.');
                    let _ = parts.next();
                    let _ = parts.next();
                    if let (Some(index), Some(region)) = (parts.next(), parts.next()) {
                        if let Ok(index) = index.parse::<usize>() {
                            if index < 4 {
                                match region {
                                    "filters" => column_filters[index].push(view.clone()),
                                    "search" => column_searches[index].push(view.clone()),
                                    "size" => column_sizes[index].push(view.clone()),
                                    "status" => column_status[index].push(view.clone()),
                                    "tools" => column_tools[index].push(view.clone()),
                                    "inspector" => column_inspectors[index].push(view.clone()),
                                    "modal" => column_modals[index].push(view.clone()),
                                    _ => auxiliary_views.push(view.clone()),
                                }
                            }
                        }
                    }
                }
                "workspace.status" => status_views.push(view.clone()),
                "workspace.debug-status" => debug_status.push(view.clone()),
                "workspace.bottom" => bottom_views.push(view.clone()),
                "command.tools" => tool_views.push(view.clone()),
                "workspace.sidebar" => sidebar_views.push(view.clone()),
                "gallery.content" => content_views.push(view.clone()),
                slot if slot.starts_with("gallery.column.") => {
                    let mut parts = slot.split('.');
                    let _ = parts.next();
                    let _ = parts.next();
                    if let (Some(index), Some(region)) = (parts.next(), parts.next()) {
                        if let Ok(index) = index.parse::<usize>() {
                            if index < 4 {
                                match region {
                                    "address" => column_addresses[index].push(view.clone()),
                                    "content" => column_contents[index].push(view.clone()),
                                    _ => auxiliary_views.push(view.clone()),
                                }
                            }
                        }
                    }
                }
                "gallery.inspector" => inspector_views.push(view.clone()),
                "workspace.main" => context_views.push(view.clone()),
                "workspace.modal" => {
                    if panel_id.is_some() && panel_id == &active {
                        active_panel = Some(view.clone());
                    } else {
                        modal_views.push(view.clone());
                    }
                }
                _ => auxiliary_views.push(view.clone()),
            }
        }
        let active_column = self
            .cordis
            .try_service::<photag_plugin_columns::ColumnLayoutService>()
            .ok()
            .and_then(|service| service.snapshot().ok())
            .map(|state| state.active_index)
            .unwrap_or(0)
            .min(3);
        filter_views.extend(column_filters[active_column].clone());
        search_views.extend(column_searches[active_column].clone());
        status_views.extend(column_status[active_column].clone());
        tool_views.extend(column_tools[active_column].clone());
        inspector_views.extend(column_inspectors[active_column].clone());
        modal_views.extend(column_modals[active_column].clone());
        modal_views.extend(active_panel);
        let mut tool_commands = Vec::new();
        let mut settings_available = false;
        for button in contributions.iter().filter_map(|owned| {
            let ContributionKind::Command { label, .. } = &owned.contribution.kind else {
                return None;
            };
            if owned.contribution.id == "settings/toggle" {
                settings_available = true;
                return None;
            }
            if owned.contribution.id == "metadata-export/export" {
                return None;
            }
            let command_id = owned.contribution.id.clone();
            let cordis = self.cordis.clone();
            let executor = self.executor.clone();
            Some(
                Button::new(command_id.clone())
                    .compact()
                    .label(photag_ui_bridge::i18n::tr(&(label.clone()).to_string()))
                    .on_click(move |_, _, _| {
                        let cordis = cordis.clone();
                        let command_id = command_id.clone();
                        executor.spawn(async move {
                            if let Err(error) = cordis
                                .emit::<UiCommandInvoked>(Routing::Unscoped, command_id)
                                .await
                            {
                                tracing::error!("command dispatch failed: {error}");
                            }
                        });
                    }),
            )
        }) {
            tool_commands.push(button);
        }
        let settings_entry = {
            let cordis = self.cordis.clone();
            let executor = self.executor.clone();
            theme::separate_region(
                div()
                    .v_flex()
                    .w(px(48.))
                    .flex_shrink_0()
                    .h(px(56.))
                    .items_center()
                    .justify_center()
                    .gap(px(4.))
                    .bg(rgb(theme::SURFACE_RAIL))
                    .child(
                        theme::icon(IconName::Settings)
                            .size(px(16.))
                            .text_color(rgb(theme::TEXT_SECONDARY)),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .child(photag_ui_bridge::i18n::tr("Settings")),
                    ),
                theme::RegionEdge::Top,
            )
            .id("shell-settings-entry")
            .cursor_pointer()
            .hover(|style| style.bg(rgba(theme::FILL_HOVER)))
            .on_click(move |_, _, _| {
                let cordis = cordis.clone();
                let executor = executor.clone();
                executor.spawn(async move {
                    if let Err(error) = cordis
                        .emit::<UiCommandInvoked>(Routing::Unscoped, "settings/toggle".into())
                        .await
                    {
                        tracing::error!("settings toggle failed: {error}");
                    }
                });
            })
        };
        let sidebar_toggle = Button::new("shell-sidebar")
            .ghost()
            .compact()
            .icon(theme::small_icon(if self.sidebar_open {
                IconName::PanelLeftClose
            } else {
                IconName::PanelLeft
            }))
            .tooltip(photag_ui_bridge::i18n::tr("Toggle sidebar"))
            .on_click(cx.listener(|this, _, _, cx| {
                this.sidebar_open = !this.sidebar_open;
                let _ = this.surfaces.set_value(
                    "sidebar.visible",
                    if this.sidebar_open { "true" } else { "false" },
                );
                cx.notify();
            }));
        let inspector_toggle = Button::new("shell-inspector")
            .ghost()
            .compact()
            .icon(theme::icon(IconName::PanelRight))
            .toggled(self.inspector_open)
            .tooltip(photag_ui_bridge::i18n::tr("Toggle inspector"))
            .on_click(cx.listener(|this, _, _, cx| {
                let next = !this.surfaces.is_open("inspector");
                if let Err(error) = this.surfaces.set_open("inspector", next) {
                    tracing::error!("inspector toggle failed: {error}");
                }
                this.inspector_open = next;
                cx.notify();
            }));
        // Shared operation band: filters/view controls and search/functions are
        // separate groups. Address navigation stays in the row below.
        let logo_zone = div()
            .h_flex()
            .h_full()
            .items_center()
            .gap(px(8.))
            .pl(px(10.))
            .pr(px(12.))
            .when(self.sidebar_open, |zone| {
                theme::separate_region(
                    zone.w(px(self.sidebar_width)).pr(px(8.)),
                    theme::RegionEdge::Right,
                )
            });
        let title_band = theme::separate_region(
            div()
                .h_flex()
                .w_full()
                .flex_shrink_0()
                .h(px(theme::TITLE_BAND_HEIGHT))
                .items_center()
                .gap(px(8.))
                .child(
                    logo_zone
                        .child(sidebar_toggle)
                        .child(
                            theme::large_icon(IconName::Aperture).text_color(rgb(theme::accent())),
                        )
                        .child(
                            div()
                                .font_family(theme::SEMIBOLD_FONT_FAMILY)
                                .text_size(px(theme::FONT_SIZE_HEADING))
                                .child("PhoTag"),
                        ),
                )
                .child(
                    div()
                        .h_flex()
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .items_center()
                        .overflow_hidden()
                        .children(filter_views)
                        .children(column_control_views),
                )
                .child(theme::separate_region(
                    div()
                        .h_flex()
                        .h_full()
                        .flex_shrink_0()
                        .items_center()
                        .gap(px(6.))
                        .pl(px(8.))
                        .pr(px(8.))
                        .children(search_views)
                        .child(
                            Button::new("shell-workspaces")
                                .ghost()
                                .compact()
                                .label(photag_ui_bridge::i18n::tr("Workspaces"))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Err(error) =
                                        this.surfaces.set_open("workspace-manager", true)
                                    {
                                        tracing::error!(
                                            "could not open workspace manager: {error}"
                                        );
                                    }
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new("shell-tools")
                                .ghost()
                                .compact()
                                .icon(theme::icon(IconName::Wrench))
                                .tooltip(photag_ui_bridge::i18n::tr("Tools"))
                                .accessibility_label(photag_ui_bridge::i18n::tr("Tools"))
                                .toggled(self.tools_open)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.tools_open = !this.tools_open;
                                    let _ = this.surfaces.set_open("tools", this.tools_open);
                                    cx.notify();
                                })),
                        ),
                    theme::RegionEdge::Left,
                ))
                .child(inspector_toggle),
            theme::RegionEdge::Bottom,
        );
        let mut sidebar_column = theme::separate_region(
            div()
                .v_flex()
                .w(px(self.sidebar_width))
                .h_full()
                .flex_shrink_0()
                .min_h_0()
                .bg(rgb(theme::SURFACE_SIDEBAR)),
            theme::RegionEdge::Right,
        )
        .child(
            div()
                .v_flex()
                .flex_1()
                .min_h_0()
                .w_full()
                .children(sidebar_views),
        );
        if settings_available {
            sidebar_column = sidebar_column.child(
                div()
                    .h_flex()
                    .w_full()
                    .flex_shrink_0()
                    .child(settings_entry),
            );
        }
        let layout = self
            .cordis
            .try_service::<photag_plugin_columns::ColumnLayoutService>()
            .ok()
            .and_then(|service| service.snapshot().ok());
        let active_column = layout.map(|state| state.active_index).unwrap_or(0);
        let column_count = layout
            .map(|state| state.layout.column_count())
            .unwrap_or(1)
            .min(4);
        let has_column_contributions = (0..column_count)
            .any(|index| !column_contents[index].is_empty() || !column_addresses[index].is_empty());
        let single_content_views = if column_contents[0].is_empty() {
            content_views.clone()
        } else {
            column_contents[0].clone()
        };
        let multi_content = if column_count == 1 {
            div()
                .v_flex()
                .flex_1()
                .min_h_0()
                .w_full()
                .overflow_hidden()
                .children(single_content_views)
                .into_any_element()
        } else if has_column_contributions {
            let column_service = self
                .cordis
                .try_service::<photag_plugin_columns::ColumnLayoutService>()
                .ok();
            let make_column = |index: usize| {
                let mut column = div()
                    .v_flex()
                    .flex_1()
                    .h_full()
                    .min_h_0()
                    .min_w(px(240.))
                    .overflow_hidden()
                    .when(index > 0, |column| {
                        column
                            .border_l_1()
                            .border_color(rgba(theme::SEPARATOR_LIGHT))
                    })
                    .when(index == active_column && column_count > 1, |column| {
                        column.border_1().border_color(rgb(theme::accent()))
                    })
                    .child(
                        div()
                            .h_flex()
                            .w_full()
                            .min_w_0()
                            .items_center()
                            .child(
                                div()
                                    .h_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .children(column_addresses[index].clone()),
                            )
                            .child(
                                div()
                                    .h_flex()
                                    .flex_shrink_0()
                                    .items_center()
                                    .children(column_sizes[index].clone()),
                            ),
                    )
                    .child(
                        div()
                            .v_flex()
                            .flex_1()
                            .min_h_0()
                            .overflow_hidden()
                            .children(column_contents[index].clone()),
                    );
                if let Some(service) = column_service.clone() {
                    // Activate before child controls handle the same pointer event.
                    // Gallery cards and controls may stop bubbling, which otherwise
                    // leaves the shell's shared toolbar and inspector on the old column.
                    column = column.capture_any_mouse_down(move |_, _, _| {
                        let _ = service.activate(index);
                    });
                }
                column
            };
            if column_count == 2 {
                div()
                    .h_flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_x_scrollbar()
                    .child(make_column(0))
                    .child(make_column(1))
                    .into_any_element()
            } else {
                div()
                    .v_flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_hidden()
                    .child(
                        div()
                            .h_flex()
                            .flex_1()
                            .min_h_0()
                            .w_full()
                            .overflow_x_scrollbar()
                            .child(make_column(0))
                            .child(make_column(1)),
                    )
                    .child(
                        div()
                            .h_flex()
                            .flex_1()
                            .min_h_0()
                            .w_full()
                            .overflow_x_scrollbar()
                            .border_t_1()
                            .border_color(rgba(theme::SEPARATOR_LIGHT))
                            .child(make_column(2))
                            .child(make_column(3)),
                    )
                    .into_any_element()
            }
        } else {
            div()
                .v_flex()
                .flex_1()
                .min_h_0()
                .w_full()
                .overflow_hidden()
                .children(content_views.clone())
                .into_any_element()
        };
        let mut content_column = div()
            .v_flex()
            .flex_1()
            .h_full()
            .min_h_0()
            .min_w_0()
            .relative()
            .overflow_hidden()
            .bg(rgb(theme::SURFACE_PANEL))
            .when(column_count == 1, |content| {
                let single_addresses = if column_addresses[0].is_empty() {
                    breadcrumb_views
                } else {
                    column_addresses[0].clone()
                };
                let single_navigation_actions = if column_sizes[0].is_empty() {
                    size_views
                } else {
                    column_sizes[0].clone()
                };
                content.child(
                    div()
                        .h_flex()
                        .w_full()
                        .flex_shrink_0()
                        .h(px(40.))
                        .items_center()
                        .gap(px(8.))
                        .px(px(16.))
                        .children(single_addresses)
                        .children(single_navigation_actions),
                )
            })
            .child(multi_content)
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .v_flex()
                    .justify_end()
                    .children(context_views),
            );
        if self.views.is_empty() {
            content_column = content_column.child(
                div()
                    .v_flex()
                    .absolute()
                    .inset_0()
                    .gap_2()
                    .overflow_y_scrollbar()
                    .p_4()
                    .children(contributions.iter().map(render_contribution))
                    .when(contributions.is_empty(), |body| {
                        body.child(
                            div()
                                .text_sm()
                                .text_color(rgb(theme::TEXT_SECONDARY))
                                .child("No feature plugins are active."),
                        )
                    }),
            );
        }
        let mut shell = div()
            .v_flex()
            .size_full()
            .bg(rgba(theme::base_rgba(theme::SURFACE_BASE)))
            .relative()
            .font_family(theme::UI_FONT_FAMILY)
            .text_size(px(theme::FONT_SIZE_BODY))
            .text_color(rgb(theme::TEXT_PRIMARY))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if !event.keystroke.key.eq_ignore_ascii_case("escape") {
                    if !this.surfaces.is_capturing_key("settings") {
                        let modifiers = event.keystroke.modifiers;
                        let mut parts = Vec::new();
                        if modifiers.control {
                            parts.push("Ctrl".to_owned());
                        }
                        if modifiers.alt {
                            parts.push("Alt".to_owned());
                        }
                        if modifiers.shift {
                            parts.push("Shift".to_owned());
                        }
                        if modifiers.platform {
                            parts.push("Win".to_owned());
                        }
                        parts.push(if event.keystroke.key.len() == 1 {
                            event.keystroke.key.to_ascii_uppercase()
                        } else {
                            event.keystroke.key.clone()
                        });
                        let chord = parts.join("+");
                        let overrides = this
                            .cordis
                            .try_service::<photag_plugin_settings::SettingsService>()
                            .ok()
                            .and_then(|service| service.snapshot().ok())
                            .map(|settings| settings.keybindings)
                            .unwrap_or_default();
                        let command_id = this
                            .catalog
                            .commands()
                            .ok()
                            .into_iter()
                            .flatten()
                            .flat_map(|command| {
                                let overrides = &overrides;
                                let chord = &chord;
                                let command_id = command.id.clone();
                                command.keybindings.into_iter().filter_map(move |binding| {
                                    let current =
                                        overrides.get(&binding.id).unwrap_or(&binding.key);
                                    (binding.scope == photag_ui_bridge::ShortcutScope::Application
                                        && !current.is_empty()
                                        && current.eq_ignore_ascii_case(&chord))
                                    .then_some(command_id.clone())
                                })
                            })
                            .next();
                        if let Some(command_id) = command_id {
                            let cordis = this.cordis.clone();
                            this.executor.spawn(async move {
                                let _ = cordis
                                    .emit::<UiCommandInvoked>(Routing::Unscoped, command_id)
                                    .await;
                            });
                            cx.notify();
                        }
                    }
                    return;
                }
                if this.surfaces.take_suppressed_escape("settings") {
                    return;
                }
                if this.surfaces.is_open("settings") && !this.surfaces.is_capturing_key("settings")
                {
                    if let Err(error) = this.surfaces.set_open("settings", false) {
                        tracing::error!("could not close Settings surface: {error}");
                    }
                    cx.notify();
                }
            }))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, cx| {
                this.move_panel_drag(event, window, cx)
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.finish_panel_drag(cx)),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.finish_panel_drag(cx)),
            )
            .map(|shell| {
                shell
                    .child(title_band)
                    .child(
                        div()
                            .h_flex()
                            .flex_1()
                            .min_h_0()
                            .w_full()
                            .relative()
                            .when(self.sidebar_open, |row| row.child(sidebar_column))
                            .child(content_column)
                            .when(self.inspector_open, |row| {
                                row.child(theme::separate_region(
                                    div()
                                        .v_flex()
                                        .w(px(self.inspector_width))
                                        .h_full()
                                        .flex_shrink_0()
                                        .min_h_0()
                                        .bg(rgb(theme::SURFACE_CARD))
                                        .children(inspector_views),
                                    theme::RegionEdge::Left,
                                ))
                            })
                            .when(self.sidebar_open, |row| {
                                row.child(self.panel_splitter(PanelSide::Sidebar, cx))
                            })
                            .when(self.inspector_open, |row| {
                                row.child(self.panel_splitter(PanelSide::Inspector, cx))
                            }),
                    )
                    .children(bottom_views)
                    .child(theme::separate_region(
                        // v5 status band: selection summary and featured actions.
                        div()
                            .h_flex()
                            .w_full()
                            .flex_shrink_0()
                            .h(px(theme::STATUS_BAR_HEIGHT))
                            .items_center()
                            .px(px(16.))
                            .child(
                                div()
                                    .h_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .children(status_views),
                            )
                            .children(debug_status),
                        theme::RegionEdge::Top,
                    ))
            });
        if self.tools_open {
            shell = shell.child(
                div()
                    .absolute()
                    .top(px(theme::TITLE_BAND_HEIGHT + 6.))
                    .right(px(48.))
                    .w(px(300.))
                    .h(px((f32::from(window.bounds().size.height)
                        - theme::TITLE_BAND_HEIGHT
                        - 24.)
                        .clamp(160., 600.)))
                    .p(px(10.))
                    .rounded(px(theme::ROUND_LG))
                    .border_1()
                    .border_color(rgba(theme::SEPARATOR_LIGHT))
                    .bg(rgba(theme::modal_rgba(theme::SURFACE_MODAL)))
                    .child(
                        div()
                            .v_flex()
                            .size_full()
                            .gap(px(6.))
                            .overflow_y_scrollbar()
                            .children(tool_views)
                            .children(tool_commands)
                            .children(auxiliary_views),
                    ),
            );
        }
        if !modal_views.is_empty() {
            if self.surfaces.open_panel_ids().iter().any(|id| {
                self.surfaces
                    .value(&format!("panel.{id}.native"))
                    .as_deref()
                    != Some("true")
            }) {
                shell = shell.child(
                    div()
                        .absolute()
                        .inset_0()
                        .bg(rgba(theme::SHADOW_OVERLAY))
                        .occlude()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation()),
                );
            }
            shell = shell.child(div().absolute().inset_0().children(modal_views));
        }
        let capture = self.capture.clone();
        // Register passive observers during paint, before dispatch reaches
        // widget handlers. They never invalidate/log or intercept input.
        shell.child(
            gpui_kit::canvas(
                |_, _, _| (),
                move |_, (), window, _| {
                    let pointer = capture.clone();
                    window.on_mouse_event(move |event: &gpui_kit::MouseMoveEvent, phase, _, _| {
                        if phase == gpui_kit::DispatchPhase::Capture {
                            pointer.observe_pointer(
                                f32::from(event.position.x),
                                f32::from(event.position.y),
                            );
                        }
                    });
                    let buttons = capture.clone();
                    window.on_mouse_event(move |_: &gpui_kit::MouseDownEvent, phase, _, _| {
                        if phase == gpui_kit::DispatchPhase::Capture {
                            buttons.observe_button();
                        }
                    });
                    let keys = capture.clone();
                    window.on_key_event(move |_: &gpui_kit::KeyDownEvent, phase, _, _| {
                        if phase == gpui_kit::DispatchPhase::Capture {
                            keys.observe_key();
                        }
                    });
                },
            )
            .absolute()
            .inset_0(),
        )
    }
}
