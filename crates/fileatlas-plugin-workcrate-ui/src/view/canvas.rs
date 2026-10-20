use super::render::{figcraft_fit_scale, page_preview, physical_label, pixel_label};
use super::*;

impl WorkCratePanel {
    pub(super) fn render_editor(&mut self, cx: &mut ViewContext<Self>) -> gpui::AnyElement {
        let Some(card) = self
            .boards_view
            .iter()
            .find(|c| Some(&c.board.id) == self.selected_board.as_ref())
            .cloned()
        else {
            return div().child("正在读取页面…").into_any_element();
        };
        let board = &card.board;
        let fallback = self.panel_content_size();
        let (width, height) = self
            .surfaces
            .value("workcrate.viewport")
            .and_then(|s| {
                let v = s
                    .split(',')
                    .map(str::parse::<f64>)
                    .collect::<Result<Vec<_>, _>>()
                    .ok()?;
                (v.len() == 2).then_some((v[0], v[1]))
            })
            .unwrap_or((fallback.0 - 490., fallback.1 - 150.));
        let fit = figcraft_fit_scale(
            width + 80.,
            height + 80.,
            board.source.settings.page_width,
            board.source.settings.page_height,
        );
        self.fit_scale = fit;
        self.zoom = 1.;
        if !self.busy {
            self.load_editor_images(cx);
        }
        if !self.busy && (fit * self.device_scale - self.rendered_scale).abs() > 0.005 {
            self.render_canvas(board.clone(), cx);
            self.load_editor_images(cx);
        }
        let mut rail = div()
            .id("workcrate-page-rail")
            .h_full()
            .w(px(180.))
            .flex_shrink_0()
            .min_h_0()
            .v_flex()
            .gap_3()
            .p_3()
            .border_r_1()
            .border_color(rgb(theme::separator()))
            .overflow_y_scroll()
            .child("页面");
        for (index, page) in self.boards_view.clone().into_iter().enumerate() {
            let id = page.board.id.clone();
            rail = rail.child(
                div()
                    .id(format!("rail-page-{id}"))
                    .v_flex()
                    .gap_2()
                    .workbench_card(id == board.id)
                    .p_2()
                    .child(div().text_xs().truncate().child(format!(
                        "{:02}  {}",
                        index + 1,
                        page.board.title
                    )))
                    .child(page_preview(&page, 140.))
                    .on_click(cx.listener(move |view, _, _, cx| view.open_page(&id, cx))),
            );
        }
        rail = rail.child(
            Button::new("rail-add-page")
                .outline()
                .label("＋ 添加页面")
                .on_click(cx.listener(|view, _, _, cx| view.create_board(cx))),
        );
        let command = div()
            .h_flex()
            .items_center()
            .flex_wrap()
            .gap_2()
            .p_3()
            .border_b_1()
            .border_color(rgb(theme::separator()))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(theme::text_muted()))
                    .child("A3 固定页面 · 等比排版"),
            )
            .child(super::tracking::control(
                "canvas-crop-mode",
                Button::new("canvas-crop-mode")
                    .ghost()
                    .when(self.crop_mode, |b| b.outline())
                    .compact()
                    .label(if self.crop_mode {
                        "完成裁剪"
                    } else {
                        "裁剪"
                    })
                    .disabled(self.selected_image.is_none())
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.crop_mode = !view.crop_mode;
                        cx.notify();
                    })),
                &self.surfaces,
            ))
            .child(super::tracking::control(
                "canvas-restore-crop",
                Button::new("canvas-restore-crop")
                    .ghost()
                    .compact()
                    .label("恢复原图")
                    .disabled(self.selected_image.is_none())
                    .on_click(cx.listener(|view, _, _, cx| view.restore_crop(cx))),
                &self.surfaces,
            ))
            .child(div().flex_1())
            .child(super::tracking::control(
                "canvas-refresh-project",
                Button::new("canvas-refresh-project")
                    .ghost()
                    .compact()
                    .label("更新项目图片")
                    .disabled(self.busy)
                    .on_click(cx.listener(|view, _, _, cx| view.refresh_project_links(cx))),
                &self.surfaces,
            ))
            .child(super::tracking::control(
                "canvas-add-image",
                Button::new("canvas-add-image")
                    .primary()
                    .compact()
                    .label("添加图片")
                    .disabled(self.busy)
                    .on_click(cx.listener(|view, _, window, cx| {
                        if let Some(id) = view.selected_board.clone() {
                            view.begin_edit(id, "add-images", "", window, cx);
                        }
                    })),
                &self.surfaces,
            ))
            .child(
                Button::new("canvas-export-image")
                    .ghost()
                    .compact()
                    .label("图片导出")
                    .disabled(self.busy)
                    .on_click(cx.listener(|view, _, _, cx| view.export_current_image(cx))),
            )
            .child(
                Button::new("canvas-export-pdf")
                    .ghost()
                    .compact()
                    .label("PDF 导出")
                    .disabled(self.busy)
                    .on_click(cx.listener(|view, _, _, cx| view.export(cx))),
            );
        let dw = (board.source.settings.page_width * fit) as f32;
        let dh = (board.source.settings.page_height * fit) as f32;
        let header = board.source.settings.page_height * 0.06;
        let mut pictures = div()
            .id("workcrate-image-area")
            .absolute()
            .left_0()
            .top(px((header * fit) as f32))
            .w(px(dw))
            .h(px(dh - (header * fit) as f32))
            .overflow_hidden()
            .drag_over::<fileatlas_contracts::FileManagerFileDragPayload>(|s, _, _, _| {
                s.border_2().border_color(rgb(theme::accent()))
            })
            .on_drop::<fileatlas_contracts::FileManagerFileDragPayload>(cx.listener(
                |view, p: &fileatlas_contracts::FileManagerFileDragPayload, _, cx| {
                    if p.allows_drop() {
                        view.import_images(p.paths.iter().map(PathBuf::from).collect(), cx);
                    }
                },
            ))
            .on_drop::<fileatlas_contracts::FileStagingDragPayload>(cx.listener(
                |view, p: &fileatlas_contracts::FileStagingDragPayload, _, cx| {
                    view.import_images(p.paths.iter().map(PathBuf::from).collect(), cx)
                },
            ))
            .on_drop::<gpui::ExternalPaths>(cx.listener(|view, p: &gpui::ExternalPaths, _, cx| {
                view.import_images(p.paths().to_vec(), cx)
            }));
        for (asset, _) in &board.placements {
            if let Ok(layout) = board.image_layout(asset) {
                let layout = self
                    .gesture
                    .as_ref()
                    .filter(|g| g.asset == *asset)
                    .map(|g| g.current.clone())
                    .unwrap_or(layout);
                pictures =
                    pictures.child(self.image_frame(&card, asset, &layout, fit, header, false, cx));
            }
        }
        if let Some(asset) = self.selected_image.as_ref() {
            if let Ok(layout) = board.image_layout(asset) {
                let layout = self
                    .gesture
                    .as_ref()
                    .filter(|g| g.asset == *asset)
                    .map(|g| g.current.clone())
                    .unwrap_or(layout);
                pictures =
                    pictures.child(self.image_frame(&card, asset, &layout, fit, header, true, cx));
            }
        }
        let paper = self
            .canvas
            .clone()
            .map(|image| {
                img(image)
                    .w(px(dw))
                    .h(px(dh))
                    .object_fit(ObjectFit::Contain)
                    .into_any_element()
            })
            .unwrap_or_else(|| div().child("正在读取页面信息…").into_any_element());
        let artboard = div()
            .id("workcrate-artboard")
            .relative()
            .w(px(dw))
            .h(px(dh))
            .flex_shrink_0()
            .child(paper)
            .child(super::tracking::measure(
                "artboard",
                Arc::clone(&self.surfaces),
            ))
            .child(pictures);
        let surfaces = Arc::clone(&self.surfaces);
        let entity = cx.entity_id();
        let measure = gpui::canvas(
            move |bounds, window, cx| {
                let value = format!(
                    "{},{}",
                    f32::from(bounds.size.width),
                    f32::from(bounds.size.height)
                );
                if surfaces.value("workcrate.viewport").as_deref() != Some(&value) {
                    let _ = surfaces.set_value("workcrate.viewport", &value);
                    cx.notify(entity);
                    window.request_animation_frame();
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .left_0()
        .top_0()
        .size_full();
        let center = div()
            .h_full()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .v_flex()
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_2()
                    .px_4()
                    .py_3()
                    .child(
                        div()
                            .flex_1()
                            .font_semibold()
                            .truncate()
                            .child(board.title.clone()),
                    )
                    .child(div().text_xs().child(format!(
                        "{} · {}",
                        physical_label(board),
                        pixel_label(board)
                    ))),
            )
            .child(
                div()
                    .id("workcrate-canvas-viewport")
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .justify_center()
                    .p_3()
                    .bg(rgb(theme::surface_faint()))
                    .child(artboard)
                    .child(measure),
            );
        let mut workspace = div().flex_1().min_w_0().h_full().v_flex().child(command);
        if self
            .editing
            .as_ref()
            .is_some_and(|(_, key)| key == "add-images")
        {
            workspace = workspace.child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_2()
                    .p_3()
                    .border_b_1()
                    .border_color(rgb(theme::separator()))
                    .child("图片路径")
                    .child(fileatlas_ui_bridge::components::text_input(&self.edit_input).flex_1())
                    .child(
                        Button::new("canvas-browse-images")
                            .outline()
                            .label("浏览文件…")
                            .on_click(
                                cx.listener(|view, _, window, cx| view.place_image(window, cx)),
                            ),
                    )
                    .child(super::tracking::control(
                        "canvas-confirm-images",
                        Button::new("canvas-confirm-images")
                            .primary()
                            .label("添加")
                            .on_click(cx.listener(|view, _, _, cx| {
                                let text = view.edit_input.read(cx).value().to_string();
                                view.import_images(
                                    text.split(';')
                                        .map(|p| PathBuf::from(p.trim().trim_matches('"')))
                                        .collect(),
                                    cx,
                                );
                            })),
                        &self.surfaces,
                    ))
                    .child(
                        Button::new("canvas-cancel-images")
                            .ghost()
                            .label("取消")
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.editing = None;
                                cx.notify();
                            })),
                    ),
            );
        }
        workspace = workspace.child(
            div()
                .flex_1()
                .min_h_0()
                .h_flex()
                .items_start()
                .child(rail)
                .child(center),
        );
        div()
            .flex_1()
            .min_h_0()
            .h_flex()
            .items_start()
            .child(workspace)
            .child(self.inspector(&card, cx))
            .on_mouse_move(cx.listener(|view, event: &gpui::MouseMoveEvent, _, cx| {
                if event.dragging() {
                    view.move_image_gesture(event.position, cx);
                }
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|view, _, _, cx| view.end_image_gesture(cx)),
            )
            .on_mouse_up_out(
                gpui::MouseButton::Left,
                cx.listener(|view, _, _, cx| view.end_image_gesture(cx)),
            )
            .into_any_element()
    }

    fn image_frame(
        &self,
        card: &BoardCard,
        asset: &str,
        layout: &fileatlas_plugin_workcrate_boards::ImageLayout,
        fit: f64,
        header: f64,
        overlay: bool,
        cx: &mut ViewContext<Self>,
    ) -> gpui::AnyElement {
        let f = layout.frame;
        let c = layout.content;
        let mut frame = div()
            .id(format!("canvas-image-{asset}-{overlay}"))
            .absolute()
            .left(px((f[0] * fit) as f32))
            .top(px(((f[1] - header) * fit) as f32))
            .w(px(((f[2] - f[0]) * fit) as f32))
            .h(px(((f[3] - f[1]) * fit) as f32));
        if let Some(image) = card.originals.get(asset).cloned().filter(|_| !overlay) {
            frame = frame.child(
                div().size_full().overflow_hidden().child(
                    img(image)
                        .absolute()
                        .left(px(((c[0] - f[0]) * fit) as f32))
                        .top(px(((c[1] - f[1]) * fit) as f32))
                        .w(px(((c[2] - c[0]) * fit) as f32))
                        .h(px(((c[3] - c[1]) * fit) as f32))
                        .object_fit(ObjectFit::Contain),
                ),
            );
        }
        let target = asset.to_owned();
        if !overlay {
            frame = frame.cursor(gpui::CursorStyle::OpenHand).on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |view, event: &gpui::MouseDownEvent, _, cx| {
                    view.begin_image_gesture(target.clone(), None, event.position, cx);
                    cx.stop_propagation();
                }),
            );
        }
        if overlay {
            frame = frame.child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .size_full()
                    .border_1()
                    .border_color(rgb(theme::accent())),
            );
            for corner in 0..4 {
                let target = asset.to_owned();
                let right = corner % 2 == 1;
                let bottom = corner >= 2;
                frame = frame.child(
                    div()
                        .id(format!("image-handle-{asset}-{corner}"))
                        .absolute()
                        .left(px(if right {
                            ((f[2] - f[0]) * fit) as f32 - 5.
                        } else {
                            -5.
                        }))
                        .top(px(if bottom {
                            ((f[3] - f[1]) * fit) as f32 - 5.
                        } else {
                            -5.
                        }))
                        .size(px(10.))
                        .border_2()
                        .border_color(rgb(theme::accent()))
                        .bg(rgb(theme::surface_base()))
                        .cursor(if right == bottom {
                            gpui::CursorStyle::ResizeUpLeftDownRight
                        } else {
                            gpui::CursorStyle::ResizeUpRightDownLeft
                        })
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |view, event: &gpui::MouseDownEvent, _, cx| {
                                view.begin_image_gesture(
                                    target.clone(),
                                    Some(corner),
                                    event.position,
                                    cx,
                                );
                                cx.stop_propagation();
                            }),
                        ),
                );
            }
        }
        frame.into_any_element()
    }
}
