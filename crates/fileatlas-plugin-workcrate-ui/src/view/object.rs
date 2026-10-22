//! The canvas editing panel. Every control here calls one official VectorCraft command;
//! FileAtlas stores no artwork of its own, so the panel is a form over the official truth.
use super::*;
use gpui_kit::component::{Sizable as _, input::NumberInput};

const TEXT_SIZE: f64 = 18.;
pub(super) const IMAGE_FILTERS: [&str; 12] = [
    "png", "jpg", "jpeg", "gif", "webp", "tif", "tiff", "bmp", "psd", "ai", "eps", "svg",
];

fn kind_label(kind: &str) -> String {
    match kind.to_lowercase().as_str() {
        "type" => "文字".into(),
        "image" => "图片".into(),
        "rectangle" => "矩形".into(),
        "ellipse" => "椭圆".into(),
        "path" => "路径".into(),
        "group" => "组".into(),
        other if other.is_empty() => "未知".into(),
        other => other.into(),
    }
}

fn link_status_label(status: &str) -> String {
    match status.to_lowercase().as_str() {
        "" => "未知".into(),
        "up-to-date" | "uptodate" | "ok" => "已是最新".into(),
        "missing" => "源文件缺失".into(),
        "modified" | "outdated" => "源文件已更新".into(),
        "embedded" => "已嵌入".into(),
        other => other.into(),
    }
}

/// The official program reports Win32 extended-length paths; the panel shows the plain one.
fn display_path(path: &str) -> String {
    path.strip_prefix(r"\\?\").unwrap_or(path).to_owned()
}

fn value_row(label: &str, value: String) -> gpui::Div {
    div()
        .h_flex()
        .items_start()
        .gap_2()
        .child(
            div()
                .w(px(72.))
                .flex_shrink_0()
                .text_xs()
                .text_color(rgb(theme::text_muted()))
                .child(label.to_owned()),
        )
        .child(div().flex_1().min_w_0().text_xs().child(value))
}

impl WorkCratePanel {
    pub(super) fn canvas_action(
        &self,
        id: &str,
        label: &str,
        run: impl Fn(&mut Self, &mut ViewContext<Self>) + 'static,
        cx: &mut ViewContext<Self>,
    ) -> gpui::Div {
        super::tracking::control(
            id,
            Button::new(id.to_owned())
                .outline()
                .compact()
                .label(label.to_owned())
                .disabled(
                    self.canvas.is_none()
                        || (id == "workcrate-canvas-set-text"
                            && !self
                                .canvas_object
                                .as_ref()
                                .is_some_and(|o| o.kind == "type"))
                        || (id == "workcrate-canvas-links-update"
                            && !self.canvas_object.as_ref().is_some_and(|o| o.linked)),
                )
                .on_click(cx.listener(move |view, _, _, cx| {
                    if view.busy || view.canvas_busy {
                        view.message = Some("画布操作进行中，请稍候…".into());
                        cx.notify();
                        return;
                    }
                    run(view, cx);
                })),
            &self.surfaces,
        )
    }

    fn number_field(&self, id: &str, label: &str, index: usize) -> gpui::Div {
        div()
            .v_flex()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(theme::text_muted()))
                    .child(label.to_owned()),
            )
            .child(super::tracking::control(
                id,
                NumberInput::new(&self.canvas_numbers[index])
                    .small()
                    .w_full(),
                &self.surfaces,
            ))
            .flex_1()
            .min_w_0()
    }

    pub(super) fn canvas_object_panel(&self, cx: &mut ViewContext<Self>) -> gpui::AnyElement {
        let selected = self
            .canvas
            .as_ref()
            .map(|snapshot| snapshot.selection.len())
            .unwrap_or_default();
        let object = self.canvas_object.as_ref();
        let mut object_body = div().v_flex().gap_1().child(
            div()
                .text_xs()
                .text_color(rgb(theme::text_muted()))
                .child(if selected == 0 {
                    "在画布上单击或框选对象后即可编辑。".to_owned()
                } else {
                    format!("已选中 {selected} 个对象。")
                }),
        );
        if let Some(object) = object {
            object_body = object_body
                .child(value_row("名称", object.name.clone()))
                .child(value_row("类型", kind_label(&object.kind)));
        }
        let mut panel = div()
            .id("workcrate-canvas-object")
            .v_flex()
            .flex_shrink_0()
            .gap_3()
            .p_3()
            .child(super::inspector::section_card(
                "画布对象",
                "官方选区",
                object_body,
            ));
        if selected > 0 {
            panel = panel.child(super::inspector::section_card(
                "位置与尺寸",
                "官方 bounds",
                div()
                    .v_flex()
                    .gap_1()
                    .child(
                        div()
                            .h_flex()
                            .gap_2()
                            .child(self.number_field("canvas-x", "X 位置", 0))
                            .child(self.number_field("canvas-y", "Y 位置", 1)),
                    )
                    .child(
                        div()
                            .h_flex()
                            .gap_2()
                            .child(self.number_field("canvas-width", "宽度", 2))
                            .child(self.number_field("canvas-height", "高度", 3)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(theme::text_muted()))
                            .child("单位为 pt，X/Y 为文档坐标，取对象左上角。"),
                    )
                    .child(self.canvas_action(
                        "workcrate-canvas-apply-bounds",
                        "应用位置与尺寸",
                        |v, cx| {
                            v.canvas_apply_bounds(cx);
                        },
                        cx,
                    )),
            ));
        }
        let is_text = object.is_some_and(|o| o.kind.eq_ignore_ascii_case("type"));
        panel = panel.child(super::inspector::section_card(
            if is_text { "文字内容" } else { "文字" },
            "官方文本",
            div()
                .v_flex()
                .gap_1()
                .child(super::tracking::control(
                    "canvas-text-input",
                    fileatlas_ui_bridge::components::textarea(&self.canvas_text_input)
                        .w_full()
                        .h(px(120.)),
                    &self.surfaces,
                ))
                .child(
                    div()
                        .h_flex()
                        .flex_wrap()
                        .gap_2()
                        .child(self.canvas_action(
                            "workcrate-canvas-set-text",
                            "更改文字",
                            |v, cx| {
                                v.canvas_apply_text(cx);
                            },
                            cx,
                        ))
                        .child(self.canvas_action(
                            "workcrate-canvas-create-text",
                            "新建文字",
                            |v, cx| v.canvas_create_text(cx),
                            cx,
                        )),
                ),
        ));
        if object.is_some_and(|o| o.kind.eq_ignore_ascii_case("image")) {
            let links_body = div()
                .v_flex()
                .gap_1()
                .child(value_row(
                    "状态",
                    object
                        .map(|o| link_status_label(&o.status))
                        .unwrap_or_default(),
                ))
                .child(value_row(
                    "文件",
                    object
                        .map(|o| display_path(&o.link_path))
                        .filter(|path| !path.is_empty())
                        .unwrap_or_else(|| "已嵌入，不链接外部文件".into()),
                ))
                .child(
                    div()
                        .h_flex()
                        .flex_wrap()
                        .gap_2()
                        .child(self.canvas_action(
                            "workcrate-canvas-links-update",
                            "更新链接",
                            |v, cx| v.canvas_update_links(cx),
                            cx,
                        ))
                        .child(self.canvas_action(
                            "workcrate-canvas-links-relink",
                            "替换文件",
                            |v, cx| v.canvas_relink(cx),
                            cx,
                        )),
                );
            panel = panel.child(super::inspector::section_card(
                "链接图片",
                "外部文件",
                links_body,
            ));
        }
        if is_text {
            panel = panel.child(super::inspector::section_card(
                "文字格式",
                "字体与字号",
                self.canvas_typography(cx),
            ));
        }
        panel = panel.child(super::inspector::section_card(
            "画板",
            "添加图片",
            div()
                .v_flex()
                .gap_1()
                .child(
                    div()
                        .h_flex()
                        .flex_wrap()
                        .gap_2()
                        .child(self.canvas_action(
                            "workcrate-canvas-place-embed",
                            "添加图片",
                            |v, cx| v.canvas_pick_and_place(false, cx),
                            cx,
                        ))
                        .child(self.canvas_action(
                            "workcrate-canvas-place-link",
                            "链接图片",
                            |v, cx| v.canvas_pick_and_place(true, cx),
                            cx,
                        )),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(theme::text_muted()))
                        .child("添加的图片落在画板中心；链接方式可随时更新或替换。"),
                ),
        ));
        panel.into_any_element()
    }

    fn canvas_selection(&self) -> (Vec<u64>, CanvasGeometry) {
        let snapshot = self.canvas.clone().unwrap_or_default();
        (snapshot.selection.clone(), snapshot.artboard.clone())
    }

    fn canvas_number_values(&self, cx: &mut ViewContext<Self>) -> Option<[f64; 4]> {
        let values: Vec<f64> = self
            .canvas_numbers
            .iter()
            .filter_map(|input| input.read(cx).value().trim().parse::<f64>().ok())
            .collect();
        <[f64; 4]>::try_from(values).ok()
    }

    pub(super) fn canvas_apply_bounds(&mut self, cx: &mut ViewContext<Self>) {
        let Some([x, y, width, height]) = self.canvas_number_values(cx) else {
            self.message = Some("位置与尺寸需要填写 4 个数字。".into());
            cx.notify();
            return;
        };
        if ![x, y, width, height].iter().all(|n| n.is_finite()) || width <= 0. || height <= 0. {
            self.message = Some("位置必须是有限数字，宽高必须大于 0 pt。".into());
            cx.notify();
            return;
        }
        let (ids, _) = self.canvas_selection();
        if ids.is_empty() {
            self.message = Some("请先选择要调整的对象。".into());
            cx.notify();
            return;
        }
        let bounds = CanvasGeometry {
            x,
            y,
            width,
            height,
        };
        self.canvas_job(
            move |boards| {
                boards
                    .canvas_set_bounds(&ids, &bounds)
                    .map_err(|e| e.to_string())
            },
            cx,
        );
    }

    pub(super) fn canvas_apply_text(&mut self, cx: &mut ViewContext<Self>) {
        if !self
            .canvas_object
            .as_ref()
            .is_some_and(|object| object.kind.eq_ignore_ascii_case("type"))
        {
            self.message = Some("请先点选一个文字对象，再更改它的内容。".into());
            cx.notify();
            return;
        }
        let text = self.canvas_text_input.read(cx).value().to_string();
        let (ids, _) = self.canvas_selection();
        let [id] = ids.as_slice() else {
            self.message = Some("请只点选一个文字对象后再更改内容。".into());
            cx.notify();
            return;
        };
        let target = *id;
        self.canvas_job(
            move |boards| {
                boards
                    .canvas_set_text(target, &text)
                    .map_err(|e| e.to_string())
            },
            cx,
        );
    }

    pub(super) fn canvas_create_text(&mut self, cx: &mut ViewContext<Self>) {
        let text = self.canvas_text_input.read(cx).value().to_string();
        if text.trim().is_empty() {
            self.message = Some("请先在文字框中输入内容。".into());
            cx.notify();
            return;
        }
        let (_, artboard) = self.canvas_selection();
        let (x, y) = (
            artboard.x + artboard.width / 2.,
            artboard.y + artboard.height / 2.,
        );
        self.canvas_job(
            move |boards| {
                boards
                    .canvas_create_text(x, y, &text, TEXT_SIZE)
                    .map_err(|e| e.to_string())
            },
            cx,
        );
    }

    pub(super) fn canvas_pick_and_place(&mut self, link: bool, cx: &mut ViewContext<Self>) {
        let Some(snapshot) = self.canvas.clone() else {
            return;
        };
        let artboard = snapshot.artboard.clone();
        let target = snapshot.board_id;
        cx.spawn(async move |this, cx| {
            let Some(file) = rfd::AsyncFileDialog::new()
                .set_title(if link {
                    "选择要链接的图片"
                } else {
                    "选择要添加的图片"
                })
                .add_filter("图片", &IMAGE_FILTERS)
                .pick_file()
                .await
            else {
                return;
            };
            let path = file.path().to_path_buf();
            let centre = [
                artboard.x + artboard.width / 2.,
                artboard.y + artboard.height / 2.,
            ];
            let _ = this.update(cx, |view: &mut WorkCratePanel, cx| {
                if view.canvas.is_none() || view.selected_board.as_ref() != Some(&target) {
                    return;
                }
                view.canvas_job(
                    move |boards| {
                        boards
                            .canvas_place(&path, centre, link)
                            .map_err(|e| e.to_string())
                    },
                    cx,
                );
            });
        })
        .detach();
    }

    pub(super) fn canvas_update_links(&mut self, cx: &mut ViewContext<Self>) {
        let (ids, _) = self.canvas_selection();
        if ids.is_empty() {
            return;
        }
        self.canvas_job(
            move |boards| boards.canvas_links_update(&ids).map_err(|e| e.to_string()),
            cx,
        );
    }

    pub(super) fn canvas_relink(&mut self, cx: &mut ViewContext<Self>) {
        let (ids, _) = self.canvas_selection();
        let target = self.selected_board.clone();
        if ids.is_empty() {
            return;
        }
        cx.spawn(async move |this, cx| {
            let Some(file) = rfd::AsyncFileDialog::new()
                .set_title("选择替换的图片文件")
                .add_filter("图片", &IMAGE_FILTERS)
                .pick_file()
                .await
            else {
                return;
            };
            let path = file.path().to_path_buf();
            let _ = this.update(cx, |view: &mut WorkCratePanel, cx| {
                if view.canvas.is_none()
                    || view.selected_board != target
                    || view.canvas_selection().0 != ids
                {
                    return;
                }
                view.canvas_job(
                    move |boards| {
                        boards
                            .canvas_links_relink(&ids, &path)
                            .map_err(|e| e.to_string())
                    },
                    cx,
                );
            });
        })
        .detach();
    }
}
