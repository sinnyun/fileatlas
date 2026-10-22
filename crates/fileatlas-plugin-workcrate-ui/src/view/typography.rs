use super::*;

impl WorkCratePanel {
    pub(super) fn canvas_typography(&self, cx: &mut ViewContext<Self>) -> gpui::Div {
        let mut panel = div()
            .v_flex()
            .gap_2()
            .child(
                div()
                    .h_flex()
                    .gap_2()
                    .child(
                        tracking::control(
                            "canvas-font-input",
                            fileatlas_ui_bridge::components::text_input(&self.canvas_font_input)
                                .w_full(),
                            &self.surfaces,
                        )
                        .flex_1()
                        .min_w_0(),
                    )
                    .child(
                        tracking::control(
                            "canvas-size-input",
                            fileatlas_ui_bridge::components::text_input(&self.canvas_size_input)
                                .w_full(),
                            &self.surfaces,
                        )
                        .w(px(84.))
                        .flex_shrink_0(),
                    ),
            )
            .child(self.canvas_action(
                "canvas-apply-style",
                "应用字体与字号",
                |v, cx| v.canvas_apply_style(None, cx),
                cx,
            ));
        let mut row = div().h_flex().flex_wrap().gap_2();
        for (id, label) in [("left", "左对齐"), ("center", "居中"), ("right", "右对齐")] {
            row = row.child(self.canvas_action(
                &format!("canvas-align-{id}"),
                label,
                move |v, cx| v.canvas_apply_style(Some(id), cx),
                cx,
            ));
        }
        panel = panel.child(row);
        panel
    }

    fn canvas_apply_style(&mut self, justify: Option<&'static str>, cx: &mut ViewContext<Self>) {
        let Some(object) = self.canvas_object.as_ref().filter(|o| o.kind == "type") else {
            return;
        };
        let id = object.id;
        let font = self.canvas_font_input.read(cx).value().trim().to_owned();
        let raw = self.canvas_size_input.read(cx).value().trim().to_owned();
        let size = if justify.is_some() || raw.is_empty() {
            None
        } else {
            match raw.parse::<f64>() {
                Ok(n) if n.is_finite() && n > 0. => Some(n),
                _ => {
                    self.message = Some("字号需为大于 0 的有限数字（pt）。".into());
                    cx.notify();
                    return;
                }
            }
        };
        if justify.is_none() && font.is_empty() && size.is_none() {
            self.message = Some("请填写字体名称或字号。".into());
            cx.notify();
            return;
        }
        self.canvas_job(
            move |boards| {
                boards
                    .canvas_text_style(
                        id,
                        (justify.is_none() && !font.is_empty()).then_some(font.as_str()),
                        size,
                        justify,
                    )
                    .map_err(|e| e.to_string())
            },
            cx,
        );
    }
}
