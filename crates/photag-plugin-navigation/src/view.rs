//! Native GPUI sidebar rendering for the navigation plugin.

use std::sync::Arc;

use gpui_kit::base::StyledExt as _;
use gpui_kit::*;
use photag_ui_bridge::UiViewContribution;

use crate::{NavigationService, NavigationSnapshot};

pub(super) fn sidebar(service: Arc<NavigationService>) -> UiViewContribution {
    UiViewContribution::new(
        "navigation/sidebar-view",
        "workspace.sidebar",
        0,
        move |_, app| {
            let view_service = Arc::clone(&service);
            app.new(|_| SidebarView::new(view_service)).into()
        },
    )
}

struct SidebarView {
    snapshot: Result<NavigationSnapshot, String>,
}

impl SidebarView {
    fn new(service: Arc<NavigationService>) -> Self {
        Self {
            snapshot: service.snapshot().map_err(|error| error.to_string()),
        }
    }
}

impl Render for SidebarView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut panel = div()
            .v_flex()
            .size_full()
            .gap_1()
            .bg(rgb(0x20232b))
            .p_3()
            .child(div().text_sm().font_semibold().child("LIBRARY"));
        match &self.snapshot {
            Ok(snapshot) => {
                panel = panel.child(
                    div()
                        .pt_2()
                        .text_xs()
                        .text_color(rgb(0x9aa0ad))
                        .child("Folders"),
                );
                for node in &snapshot.folders {
                    panel = panel.child(
                        div()
                            .w_full()
                            .rounded_md()
                            .px_2()
                            .py_1()
                            .text_sm()
                            .child(format!("{}  {}", node.folder.name, node.media_count)),
                    );
                }
                panel = panel.child(
                    div()
                        .pt_3()
                        .text_xs()
                        .text_color(rgb(0x9aa0ad))
                        .child("Tags"),
                );
                panel.children(snapshot.tags.iter().map(render_tag))
            }
            Err(error) => panel.child(
                div()
                    .text_sm()
                    .text_color(rgb(0xef7777))
                    .child(format!("Could not load library: {error}")),
            ),
        }
    }
}

fn render_tag(tag: &photag_plugin_database::db::queries::Tag) -> Div {
    let indent = "  ".repeat(tag.full_path.matches('>').count());
    div()
        .v_flex()
        .child(
            div()
                .w_full()
                .px_2()
                .py_1()
                .text_sm()
                .child(format!("{indent}{}  {}", tag.name, tag.media_count)),
        )
        .children(tag.children.iter().map(render_tag))
}
