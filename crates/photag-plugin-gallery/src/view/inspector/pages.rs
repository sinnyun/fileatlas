//! Read-only Inspector History and executable extension catalog pages.

use super::*;

impl GalleryView {
    pub(super) fn inspector_history(&self) -> AnyElement {
        let Some(history) = &self.history else {
            return inspector_message("History service is unavailable.");
        };
        let undo = history.undo_description();
        let redo = history.redo_description();
        if undo.is_none() && redo.is_none() {
            return inspector_message("No available media history.");
        }
        let mut body = div()
            .v_flex()
            .w_full()
            .h_full()
            .min_h_0()
            .gap(px(8.))
            .p(px(14.))
            .child(inspector_message(
                "Recent actions are global and are not tied to a single file.",
            ));
        if let Some(undo) = undo {
            body = body.child(info_row("Undo", undo));
        }
        if let Some(redo) = redo {
            body = body.child(info_row("Redo", redo));
        }
        body.into_any_element()
    }

    pub(super) fn inspector_extensions(&self) -> AnyElement {
        let Ok(views) = self.ui_views.snapshot() else {
            return inspector_message("Extensions could not be loaded.");
        };
        let extensions = views
            .into_iter()
            .filter(|view| {
                view.owner != "photag.gallery"
                    && view.contribution.slot.starts_with("gallery.inspector")
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
                    .bg(rgb(theme::SURFACE_PANEL))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(theme::TEXT_PRIMARY))
                            .child(view.contribution.id),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(theme::TEXT_SECONDARY))
                            .child(format!("{} · {}", view.owner, view.contribution.slot)),
                    )
            }))
            .into_any_element()
    }
}
