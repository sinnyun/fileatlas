use super::*;

pub(super) struct PhoTagShell {
    pub(super) catalog: Arc<UiCatalog>,
    pub(super) views: Vec<(String, gpui_kit::AnyView)>,
    pub(super) cordis: CordisContext,
    pub(super) executor: tokio::runtime::Handle,
    pub(super) session: Arc<photag_plugin_session::SessionService>,
    pub(super) surfaces: Arc<photag_ui_bridge::UiSurfaceState>,
    pub(super) sidebar_width: f32,
    pub(super) resizing_sidebar: bool,
    pub(super) tools_open: bool,
}

impl Render for PhoTagShell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let contributions = self.catalog.snapshot().unwrap_or_default();
        let mut search_views = Vec::new();
        let mut size_views = Vec::new();
        let mut filter_views = Vec::new();
        let mut status_views = Vec::new();
        let mut tool_views = Vec::new();
        let mut sidebar_views = Vec::new();
        let mut content_views = Vec::new();
        let mut context_views = Vec::new();
        let mut auxiliary_views = Vec::new();
        let mut modal_views = Vec::new();
        for (slot, view) in &self.views {
            match slot.as_str() {
                "command.search" => search_views.push(view.clone()),
                "command.size" => size_views.push(view.clone()),
                "command.filters" => filter_views.push(view.clone()),
                "workspace.status" => status_views.push(view.clone()),
                "command.tools" => tool_views.push(view.clone()),
                "workspace.sidebar" => sidebar_views.push(view.clone()),
                "gallery.content" => content_views.push(view.clone()),
                "workspace.main" => context_views.push(view.clone()),
                "workspace.modal" => modal_views.push(view.clone()),
                _ => auxiliary_views.push(view.clone()),
            }
        }
        let mut settings_commands = Vec::new();
        let mut tool_commands = Vec::new();
        for (id, button) in contributions.iter().filter_map(|owned| {
            let ContributionKind::Command { label } = &owned.contribution.kind else {
                return None;
            };
            let command_id = owned.contribution.id.clone();
            let cordis = self.cordis.clone();
            let executor = self.executor.clone();
            Some((
                command_id.clone(),
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
            ))
        }) {
            if id == "settings/toggle" {
                settings_commands.push(button);
            } else {
                tool_commands.push(button);
            }
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
            // Source main.rs:1007-1097: full-width command band.
            .child(
                div()
                    .v_flex()
                    .flex_shrink_0()
                    .px(px(16.))
                    .pt(px(12.))
                    .pb(px(14.))
                    .gap(px(6.))
                    .child(
                        div()
                            .h_flex()
                            .w_full()
                            .items_center()
                            .gap(px(12.))
                            .h(px(32.))
                            .child(
                                div()
                                    .font_family(theme::SEMIBOLD_FONT_FAMILY)
                                    .text_size(px(15.))
                                    .text_color(rgb(theme::TEXT_SECONDARY))
                                    .child("PhoTag"),
                            )
                            .child(div().h_flex().flex_1().min_w_0().children(search_views))
                            .children(size_views)
                            .child(
                                Button::new("shell-tools")
                                    .compact()
                                    .label(photag_ui_bridge::i18n::tr("Tools"))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.tools_open = !this.tools_open;
                                        cx.notify();
                                    })),
                            )
                            .children(settings_commands),
                    )
                    .children(filter_views),
            )
            .child(
                div()
                    .h_flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .relative()
                    .child(
                        div()
                            .v_flex()
                            .w(px(self.sidebar_width))
                            .h_full()
                            .flex_shrink_0()
                            .min_h_0()
                            .children(sidebar_views),
                    )
                    .child(
                        div()
                            .v_flex()
                            .flex_1()
                            .h_full()
                            .min_h_0()
                            .min_w_0()
                            .relative()
                            .pt(px(2.))
                            .pr(px(12.))
                            .pb(px(12.))
                            .overflow_hidden()
                            .child(
                                div()
                                    .v_flex()
                                    .flex_1()
                                    .h_full()
                                    .w_full()
                                    .min_h_0()
                                    .min_w_0()
                                    .rounded(px(10.))
                                    .bg(rgba(theme::panel_rgba(theme::SURFACE_PANEL)))
                                    .pl(px(14.))
                                    .pr(px(10.))
                                    .pt(px(12.))
                                    .pb(px(10.))
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
                            ),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px(self.sidebar_width - 4.))
                            .top_0()
                            .w(px(8.))
                            .h_full()
                            .cursor_col_resize()
                            .hover(|s| s.bg(rgba(0xffffff1a)))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, _| {
                                    this.resizing_sidebar = true;
                                }),
                            )
                            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                                if this.resizing_sidebar {
                                    let next = f32::from(event.position.x)
                                        .clamp(theme::SIDEBAR_MIN_WIDTH, theme::SIDEBAR_MAX_WIDTH);
                                    if (next - this.sidebar_width).abs() > 0.5 {
                                        this.sidebar_width = next;
                                        cx.notify();
                                    }
                                }
                            }))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _, _, _| {
                                    if this.resizing_sidebar {
                                        this.resizing_sidebar = false;
                                        if let Ok(mut saved) = this.session.snapshot() {
                                            saved.sidebar_width = this.sidebar_width;
                                            if let Err(error) = this.session.replace(saved) {
                                                eprintln!(
                                                    "sidebar width session save failed: {error}"
                                                );
                                            }
                                        }
                                    }
                                }),
                            ),
                    ),
            )
            // Source main.rs:1099-1277: full-width status band.
            .child(
                div()
                    .h_flex()
                    .w_full()
                    .flex_shrink_0()
                    .px(px(16.))
                    .pt(px(7.))
                    .pb(px(9.))
                    .children(status_views),
            );
        if self.tools_open {
            shell = shell.child(
                div()
                    .absolute()
                    .top(px(52.))
                    .right(px(16.))
                    .w(px(300.))
                    .max_h(px(600.))
                    .v_flex()
                    .gap(px(6.))
                    .p(px(10.))
                    .rounded(px(8.))
                    .bg(rgba(theme::modal_rgba(theme::SURFACE_MODAL)))
                    .overflow_y_scrollbar()
                    .children(tool_views)
                    .children(tool_commands)
                    .children(auxiliary_views),
            );
        }
        if self.views.is_empty() {
            shell = shell.child(
                div()
                    .v_flex()
                    .flex_1()
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
        if !modal_views.is_empty() {
            shell = shell.child(div().absolute().inset_0().children(modal_views));
        }
        shell
    }
}
