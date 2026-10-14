use super::*;

pub(super) struct SidebarBreadcrumb {
    sidebar: Entity<SidebarView>,
    _subscription: Subscription,
}

impl Render for SidebarBreadcrumb {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sidebar
            .update(cx, |sidebar, cx| sidebar.render_breadcrumb(cx))
    }
}

pub(super) fn breadcrumb_view(
    sidebar: Entity<SidebarView>,
    app: &mut App,
) -> Entity<SidebarBreadcrumb> {
    app.new(|cx| SidebarBreadcrumb {
        _subscription: cx.observe(&sidebar, |_, _, cx| cx.notify()),
        sidebar,
    })
}

pub(super) struct SidebarTagModal {
    sidebar: Entity<SidebarView>,
    _subscription: Subscription,
}

impl Render for SidebarTagModal {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sidebar
            .update(cx, |sidebar, cx| {
                sidebar.consume_ui_request(window, cx);
                sidebar.render_tag_modal(cx)
            })
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key.eq_ignore_ascii_case("escape") {
                    let sidebar = this.sidebar.clone();
                    let _ = sidebar.update(cx, |sidebar, cx| {
                        sidebar.close_tag_modal();
                        cx.notify();
                    });
                    cx.notify();
                }
            }))
    }
}

pub(super) fn tag_modal_view(
    sidebar: Entity<SidebarView>,
    app: &mut App,
) -> Entity<SidebarTagModal> {
    app.new(|cx| SidebarTagModal {
        _subscription: cx.observe(&sidebar, |_, _, cx| cx.notify()),
        sidebar,
    })
}
