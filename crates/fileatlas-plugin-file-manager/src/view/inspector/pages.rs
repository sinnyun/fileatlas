//! Executable extension catalog for the file inspector.
use super::*;
use fileatlas_ui_bridge::components::WorkbenchPaint as _;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;

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
            .gap(px(fileatlas_ui_bridge::theme::space_standard()))
            .p(px(fileatlas_ui_bridge::theme::space_expanded()))
            .children(extensions.into_iter().map(|view| {
                div()
                    .v_flex()
                    .w_full()
                    .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                    .p(px(fileatlas_ui_bridge::theme::space_standard()))
                    .workbench_corner(fileatlas_ui_bridge::components::CornerRole::Small)
                    .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Panel)
                    .child(div().workbench_assistant().child(view.contribution.id))
                    .child(
                        div()
                            .workbench_assistant()
                            .child(format!("{} · {}", view.owner, view.contribution.slot)),
                    )
            }))
            .into_any_element()
    }
}
