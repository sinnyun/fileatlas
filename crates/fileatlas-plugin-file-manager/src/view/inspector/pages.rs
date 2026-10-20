//! Executable extension catalog for the file inspector.
use super::*;

impl FileManagerView {
    pub(super) fn inspector_extensions(&self) -> AnyElement {
        let Ok(views) = self.ui_views.snapshot() else {
            return inspector_message("Extensions could not be loaded.");
        };
        let extensions = views
            .into_iter()
            .filter(|view| {
                view.owner != "fileatlas.file_manager"
                    && view.contribution.slot.starts_with("file_manager.inspector")
            })
            .collect::<Vec<_>>();
        if extensions.is_empty() {
            return inspector_message("No extensions are available.");
        }
        div()
            .v_flex()
            .w_full()
            .h_full()
            .min_h_0()
            .gap(px(8.))
            .p(px(14.))
            .children(extensions.into_iter().map(|view| {
                div()
                    .v_flex()
                    .w_full()
                    .gap(px(2.))
                    .p(px(8.))
                    .rounded(px(theme::ROUND_SM))
                    .bg(rgb(theme::surface_panel()))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(theme::text_primary()))
                            .child(view.contribution.id),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(theme::text_secondary()))
                            .child(format!("{} · {}", view.owner, view.contribution.slot)),
                    )
            }))
            .into_any_element()
    }
}
