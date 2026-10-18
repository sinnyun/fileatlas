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
        let confirm_label = photag_ui_bridge::i18n::tr("Confirm");
        let cancel_label = photag_ui_bridge::i18n::tr("Cancel");
        let browse_label = photag_ui_bridge::i18n::tr("Browse…");
        let mut row = div()
            .h_flex()
            .gap_1()
            .items_center()
            .py(px(2.))
            .child(photag_ui_bridge::components::text_input(&self.sidebar_editor_input).flex_1())
            .child(
                Button::new("navigation-sidebar-editor-confirm")
                    .primary()
                    .compact()
                    .label(confirm_label)
                    .on_click(cx.listener(|this, _, _, cx| this.confirm_sidebar_editor(cx))),
            );
        if show_browse {
            row = row.child(
                Button::new("navigation-sidebar-editor-browse")
                    .ghost()
                    .compact()
                    .label(browse_label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.picker_editor = Some(editor.clone());
                        this.pick_folder(cx);
                    })),
            );
        }
        let row = row.child(
            Button::new("navigation-sidebar-editor-cancel")
                .ghost()
                .compact()
                .label(cancel_label)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.sidebar_editor = None;
                    this.sidebar_editor_error = None;
                    cx.notify();
                })),
        );
        Some(div().v_flex().gap_1().child(row).when_some(
            self.sidebar_editor_error.clone(),
            |column, error| {
                column.child(div().text_xs().text_color(rgb(theme::DANGER)).child(error))
            },
        ))
    }
}
