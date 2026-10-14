use super::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::ButtonVariants;

mod panel_resize;
use panel_resize::PanelSide;

pub(super) struct PhoTagShell {
    pub(super) catalog: Arc<UiCatalog>,
    pub(super) views: Vec<(String, u64, String, Option<String>, gpui_kit::AnyView)>,
    pub(super) view_catalog: Arc<photag_ui_bridge::UiViewCatalog>,
    pub(super) cordis: CordisContext,
    pub(super) executor: tokio::runtime::Handle,
    pub(super) session: Arc<photag_plugin_session::SessionService>,
    pub(super) surfaces: Arc<photag_ui_bridge::UiSurfaceState>,
    pub(super) sidebar_width: f32,
    pub(super) inspector_width: f32,
    pub(super) viewport_width: f32,
    pub(super) capture: Arc<photag_plugin_mcp::CaptureWindowHandle>,
    pub(super) drag_origin_x: f32,
    pub(super) drag_origin_width: f32,
    pub(super) resizing_panel: Option<PanelSide>,
    pub(super) tools_open: bool,
    pub(super) sidebar_open: bool,
    pub(super) inspector_open: bool,
    pub(super) panel_signature: (bool, Option<String>),
}

impl Render for PhoTagShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
        let mut status_views = Vec::new();
        let mut tool_views = Vec::new();
        let mut sidebar_views = Vec::new();
        let mut content_views = Vec::new();
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
                "workspace.status" => status_views.push(view.clone()),
                "command.tools" => tool_views.push(view.clone()),
                "workspace.sidebar" => sidebar_views.push(view.clone()),
                "gallery.content" => content_views.push(view.clone()),
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
        modal_views.extend(active_panel);
        let mut tool_commands = Vec::new();
        let mut settings_available = false;
        for button in contributions.iter().filter_map(|owned| {
            let ContributionKind::Command { label } = &owned.contribution.kind else {
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
                                eprintln!("command dispatch failed: {error}");
                            }
                        });
                    }),
            )
        }) {
            tool_commands.push(button);
        }
        let divider = rgb(theme::SEPARATOR);
        let settings_entry = {
            let cordis = self.cordis.clone();
            let executor = self.executor.clone();
            div()
                .v_flex()
                .w(px(48.))
                .flex_shrink_0()
                .h(px(56.))
                .items_center()
                .justify_center()
                .gap(px(4.))
                .flex_shrink_0()
                .border_r_1()
                .border_t_1()
                .border_color(divider)
                .id("shell-settings-entry")
                .cursor_pointer()
                .hover(|style| style.bg(rgba(theme::FILL_HOVER)))
                .child(
                    Icon::new(IconName::Settings)
                        .size(px(16.))
                        .text_color(rgb(theme::TEXT_SECONDARY)),
                )
                .child(
                    div()
                        .text_size(px(10.))
                        .child(photag_ui_bridge::i18n::tr("Settings")),
                )
                .on_click(move |_, _, _| {
                    let cordis = cordis.clone();
                    let executor = executor.clone();
                    executor.spawn(async move {
                        if let Err(error) = cordis
                            .emit::<UiCommandInvoked>(Routing::Unscoped, "settings/toggle".into())
                            .await
                        {
                            eprintln!("settings toggle failed: {error}");
                        }
                    });
                })
        };
        let sidebar_toggle = Button::new("shell-sidebar")
            .ghost()
            .compact()
            .icon(if self.sidebar_open {
                IconName::PanelLeftClose
            } else {
                IconName::PanelLeft
            })
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
            .icon(IconName::PanelRight)
            .toggled(self.inspector_open)
            .tooltip(photag_ui_bridge::i18n::tr("Toggle inspector"))
            .on_click(cx.listener(|this, _, _, cx| {
                let next = !this.surfaces.is_open("inspector");
                if let Err(error) = this.surfaces.set_open("inspector", next) {
                    eprintln!("inspector toggle failed: {error}");
                }
                this.inspector_open = next;
                cx.notify();
            }));
        // v5 title band: logo zone tracks the sidebar width; breadcrumb fills
        // the content column; size search Tools and the inspector toggle sit
        // on the right, mirroring docs/reference-ui/final-v5/00-main-library.png.
        let title_band = div()
            .h_flex()
            .w_full()
            .flex_shrink_0()
            .h(px(theme::TITLE_BAND_HEIGHT))
            .border_b_1()
            .border_color(divider)
            .child(
                div()
                    .h_flex()
                    .h_full()
                    .items_center()
                    .gap(px(8.))
                    .pl(px(10.))
                    .pr(px(12.))
                    .when(self.sidebar_open, |zone| {
                        zone.w(px(self.sidebar_width))
                            .border_r_1()
                            .border_color(divider)
                            .pr(px(8.))
                    })
                    .child(sidebar_toggle)
                    .child(
                        Icon::new(IconName::Aperture)
                            .size(px(20.))
                            .text_color(rgb(theme::accent())),
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
                    .gap(px(10.))
                    .px(px(16.))
                    .children(breadcrumb_views),
            )
            .child(
                div()
                    .h_flex()
                    .h_full()
                    .items_center()
                    .gap(px(10.))
                    .pr(px(10.))
                    .children(size_views)
                    .children(search_views)
                    .child(
                        Button::new("shell-tools")
                            .ghost()
                            .compact()
                            .label(photag_ui_bridge::i18n::tr("Tools"))
                            .dropdown_caret(true)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.tools_open = !this.tools_open;
                                let _ = this.surfaces.set_open("tools", this.tools_open);
                                cx.notify();
                            })),
                    )
                    .child(inspector_toggle),
            );
        let mut sidebar_column = div()
            .v_flex()
            .w(px(self.sidebar_width))
            .h_full()
            .flex_shrink_0()
            .min_h_0()
            .border_r_1()
            .border_color(divider)
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
        let mut content_column = div()
            .v_flex()
            .flex_1()
            .h_full()
            .min_h_0()
            .min_w_0()
            .relative()
            .overflow_hidden()
            .bg(rgb(theme::SURFACE_PANEL))
            .child(
                div()
                    .h_flex()
                    .w_full()
                    .flex_shrink_0()
                    .items_center()
                    .gap(px(6.))
                    .px(px(16.))
                    .pt(px(10.))
                    .pb(px(6.))
                    .children(filter_views),
            )
            .child(
                div()
                    .v_flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_hidden()
                    .children(content_views),
            )
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
                    return;
                }
                if this.surfaces.take_suppressed_escape("settings") {
                    return;
                }
                if this.surfaces.is_open("settings") && !this.surfaces.is_capturing_key("settings")
                {
                    if let Err(error) = this.surfaces.set_open("settings", false) {
                        eprintln!("could not close Settings surface: {error}");
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
                        row.child(
                            div()
                                .v_flex()
                                .w(px(self.inspector_width))
                                .h_full()
                                .flex_shrink_0()
                                .min_h_0()
                                .border_l_1()
                                .border_color(divider)
                                .bg(rgb(theme::SURFACE_CARD))
                                .children(inspector_views),
                        )
                    })
                    .when(self.sidebar_open, |row| {
                        row.child(self.panel_splitter(PanelSide::Sidebar, cx))
                    })
                    .when(self.inspector_open, |row| {
                        row.child(self.panel_splitter(PanelSide::Inspector, cx))
                    }),
            )
            .child(
                // v5 status band: selection summary and featured actions.
                div()
                    .h_flex()
                    .w_full()
                    .flex_shrink_0()
                    .h(px(theme::STATUS_BAR_HEIGHT))
                    .items_center()
                    .border_t_1()
                    .border_color(divider)
                    .px(px(16.))
                    .children(status_views),
            );
        if self.tools_open {
            shell = shell.child(
                div()
                    .absolute()
                    .top(px(theme::TITLE_BAND_HEIGHT + 6.))
                    .right(px(48.))
                    .w(px(300.))
                    .max_h(px(600.))
                    .v_flex()
                    .gap(px(6.))
                    .p(px(10.))
                    .rounded(px(theme::ROUND_LG))
                    .border_1()
                    .border_color(rgba(theme::SEPARATOR_LIGHT))
                    .bg(rgba(theme::modal_rgba(theme::SURFACE_MODAL)))
                    .overflow_y_scrollbar()
                    .children(tool_views)
                    .children(tool_commands)
                    .children(auxiliary_views),
            );
        }
        if !modal_views.is_empty() {
            if self.surfaces.any_panel_open() {
                shell = shell.child(
                    div()
                        .absolute()
                        .inset_0()
                        .bg(rgba(0x00000060))
                        .occlude()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation()),
                );
            }
            shell = shell.child(div().absolute().inset_0().children(modal_views));
        }
        shell
    }
}
