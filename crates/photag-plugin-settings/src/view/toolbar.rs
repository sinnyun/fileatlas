use super::*;

impl SettingsView {
    pub(super) fn toolbar_settings(&self, s: &Settings, cx: &mut Context<Self>) -> Div {
        let mut b = div().v_flex().gap_2();
        let groups = [
            (
                "Zoom controls",
                vec![
                    (
                        "tb.zoom_mode",
                        "Zoom mode toggle",
                        s.toolbar.zoom_mode_button,
                    ),
                    ("tb.zoom_reset", "Reset zoom", s.toolbar.zoom_reset_button),
                    ("tb.zoom_presets", "Zoom presets", s.toolbar.zoom_presets),
                ],
            ),
            (
                "Photo manipulation",
                vec![
                    ("tb.rotate", "Rotation controls", s.toolbar.rotate_buttons),
                    ("tb.flip", "Mirror controls", s.toolbar.flip_buttons),
                ],
            ),
            (
                "Tags",
                vec![("tb.tags", "Tags dropdown", s.toolbar.tags_button)],
            ),
            (
                "File",
                vec![
                    (
                        "tb.delete",
                        "Delete to recycle bin",
                        s.toolbar.delete_button,
                    ),
                    ("tb.copy", "Copy to…", s.toolbar.copy_button),
                    ("tb.move", "Move to…", s.toolbar.move_button),
                    ("tb.explorer", "Show in Explorer", s.toolbar.explorer_button),
                    (
                        "tb.external",
                        "Open in default app",
                        s.toolbar.open_external_button,
                    ),
                ],
            ),
            (
                "View",
                vec![
                    (
                        "tb.filmstrip",
                        "Filmstrip toggle",
                        s.toolbar.filmstrip_toggle,
                    ),
                    (
                        "date",
                        "Group date-sorted gallery by month",
                        s.show_date_dividers,
                    ),
                ],
            ),
        ];
        let mut left_column = div().v_flex().flex_1().gap_2();
        let mut right_column = div().v_flex().flex_1().gap_2();
        for (index, (group, controls)) in groups.into_iter().enumerate() {
            let mut section = div()
                .v_flex()
                .gap(px(5.))
                .pt_2()
                .border_t_1()
                .border_color(rgba(theme::HAIRLINE.to_rgba()))
                .child(
                    div()
                        .text_sm()
                        .font_semibold()
                        .child(photag_ui_bridge::i18n::tr(group)),
                );
            for (k, n, v) in controls {
                section = section.child(self.toggle_button(k, n, v, cx));
            }
            if index % 2 == 0 {
                left_column = left_column.child(section);
            } else {
                right_column = right_column.child(section);
            }
        }
        b = b.child(
            div()
                .h_flex()
                .items_start()
                .gap_2()
                .child(left_column)
                .child(right_column),
        );
        b = b
            .child(div().v_flex().gap_1().pt_1().child(self.toggle_button(
                "pos.locked",
                "Lock toolbar position",
                s.toolbar_pos.locked,
                cx,
            )))
            .child(self.toggle_button(
                "filmstrip",
                "Show filmstrip by default",
                s.show_filmstrip,
                cx,
            ))
            .child(self.enum_button(
                "pos-reset",
                "Reset toolbar position".into(),
                |s| {
                    s.toolbar_pos.offset_x = 0.0;
                    s.toolbar_pos.offset_y = 0.0;
                },
                cx,
            ));
        b
    }
}
