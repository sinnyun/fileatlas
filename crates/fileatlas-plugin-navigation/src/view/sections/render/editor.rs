use super::*;
impl SidebarView {
    pub(in crate::view) fn render_sidebar_editor_row(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Option<Div> {
        let editor = self.sidebar_editor.clone()?;
        let show_browse = matches!(
            editor,
            SidebarEditor::AddQuickAccess | SidebarEditor::AddFavorite { .. }
        );
        let confirm_label = fileatlas_ui_bridge::i18n::tr("Confirm");
        let cancel_label = fileatlas_ui_bridge::i18n::tr("Cancel");
        let browse_label = fileatlas_ui_bridge::i18n::tr("Browse…");
        let mut row = div()
            .h_flex()
            .gap(px(theme::space_standard()))
            .items_center()
            .py(px(fileatlas_ui_bridge::theme::space_compact()))
            .child(fileatlas_ui_bridge::components::text_input(&self.sidebar_editor_input).flex_1())
            .child(
                fileatlas_ui_bridge::components::button("navigation-sidebar-editor-confirm")
                    .primary()
                    .label(confirm_label)
                    .on_click(cx.listener(|this, _, _, cx| this.confirm_sidebar_editor(cx))),
            );
        if show_browse {
            row = row.child(
                fileatlas_ui_bridge::components::button("navigation-sidebar-editor-browse")
                    .ghost()
                    .label(browse_label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.picker_editor = Some(editor.clone());
                        this.pick_folder(cx);
                    })),
            );
        }
        let row = row.child(
            fileatlas_ui_bridge::components::button("navigation-sidebar-editor-cancel")
                .ghost()
                .label(cancel_label)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.sidebar_editor = None;
                    this.sidebar_editor_error = None;
                    cx.notify();
                })),
        );
        Some(
            div()
                .v_flex()
                .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                .child(row)
                .when_some(self.sidebar_editor_error.clone(), |column, error| {
                    column.child(div().workbench_assistant().child(error))
                }),
        )
    }
}
