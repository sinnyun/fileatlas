use super::*;
impl Render for WorkCratePanel {
    fn render(&mut self, _: &mut Window, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let state=serde_json::json!({"screen":if self.selected_project.is_none(){"projects"}else if self.selected_board.is_none(){"pages"}else{"editor"},"project_id":self.selected_project,"board_id":self.selected_board,"page_count":self.boards_view.len(),"busy":self.busy,"preview_pending":self.preview_queue.len()+usize::from(self.preview_current.is_some()),"previews_ready":self.boards_view.iter().filter(|c|c.image.is_some()).count(),"boards":self.boards_view.iter().map(|c|serde_json::json!({"id":c.board.id,"native_id":c.board.native_id,"index":c.board.index,"title":c.board.title,"width_pt":c.board.width_pt,"height_pt":c.board.height_pt,"info":c.board.info,"preview_ready":c.image.is_some(),"error":c.error})).collect::<Vec<_>>(),"message":self.message,"connection":if self.ui_state.connection(){"connected"}else{"not_connected"},"executable":self.config.executable,"port":self.config.port,"document":self.document,"selected_pages":self.selected_pages,"multi_select":self.multi_select}).to_string();
        if self.state_gate.changed(state.clone()) {
            let _ = self.surfaces.set_value("workcrate.ui", &state);
        }
        let mut toolbar = div()
            .h_flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .p_3()
            .border_b_1()
            .border_color(rgb(theme::separator()))
            .child(self.action(
                "workcrate-projects-home",
                "项目管理",
                |v, cx| {
                    v.selected_project = None;
                    v.selected_board = None;
                    v.editing = None;
                    let _ = v.ui_state.navigate(None, None);
                    v.reload(cx);
                },
                cx,
            ));
        if let Some(p) = self
            .selected_project
            .as_ref()
            .and_then(|id| self.items.iter().find(|p| &p.id == id))
        {
            toolbar = toolbar.child("/").child(self.action(
                "workcrate-pages-home",
                &p.name.clone(),
                |v, cx| {
                    v.selected_board = None;
                    v.editing = None;
                    let _ = v.ui_state.navigate(v.selected_project.clone(), None);
                    cx.notify();
                },
                cx,
            ));
        }
        toolbar = toolbar.child(div().flex_1());
        if self.selected_project.is_none() {
            toolbar = toolbar
                .child(
                    fileatlas_ui_bridge::components::search_input(&self.search_input).w(px(200.)),
                )
                .child(self.action(
                    "workcrate-search",
                    "搜索",
                    |v, cx| {
                        v.query = v.search_input.read(cx).value().to_lowercase();
                        cx.notify();
                    },
                    cx,
                ))
                .child(self.action(
                    "workcrate-sort",
                    if self.sort_by_name {
                        "名称排序"
                    } else {
                        "最近创建"
                    },
                    |v, cx| {
                        v.sort_by_name = !v.sort_by_name;
                        cx.notify();
                    },
                    cx,
                ))
                .child(self.action(
                    "workcrate-archived",
                    if self.show_archived {
                        "已归档"
                    } else {
                        "进行中"
                    },
                    |v, cx| {
                        v.show_archived = !v.show_archived;
                        cx.notify();
                    },
                    cx,
                ))
                .child(self.action(
                    "workcrate-new-project",
                    "新建项目",
                    |v, cx| v.create_project(cx),
                    cx,
                ));
        } else {
            toolbar = toolbar
                .child(self.action(
                    "attach-document",
                    "关联工程",
                    |v, cx| v.pick_document(cx),
                    cx,
                ))
                .child(self.action(
                    "workcrate-new-page",
                    "新增 A3 页面",
                    |v, cx| v.create_board(cx),
                    cx,
                ))
                .child(self.action(
                    "refresh-native",
                    "同步工程与预览",
                    |v, cx| v.sync_project(cx),
                    cx,
                ));
        }
        let content = if self.selected_project.is_none() {
            self.render_projects(cx)
        } else if self.selected_board.is_none() {
            self.render_pages(cx)
        } else {
            self.render_editor(cx)
        };
        div()
            .id("workcrate-panel")
            .size_full()
            .v_flex()
            .bg(rgb(theme::surface_base()))
            .text_color(rgb(theme::text_primary()))
            .child(toolbar)
            .child(content)
            .when(self.busy, |e| {
                e.child(div().px_3().pb_2().text_xs().child("正在处理…"))
            })
            .when_some(self.message.clone(), |e, s| {
                e.child(div().px_3().pb_2().text_xs().child(s))
            })
    }
}
impl WorkCratePanel {
    pub(super) fn action(
        &self,
        id: &str,
        label: &str,
        run: impl Fn(&mut Self, &mut ViewContext<Self>) + 'static,
        cx: &mut ViewContext<Self>,
    ) -> gpui::Div {
        tracking::control(
            id,
            Button::new(id.to_owned())
                .outline()
                .compact()
                .label(label.to_owned())
                .disabled(self.busy)
                .on_click(cx.listener(move |v, _, _, cx| run(v, cx))),
            &self.surfaces,
        )
    }
    pub(super) fn render_editor(&self, cx: &mut ViewContext<Self>) -> gpui::AnyElement {
        let mut rail = div()
            .id("page-rail")
            .w(px(190.))
            .h_full()
            .flex_shrink_0()
            .v_flex()
            .gap_2()
            .p_3()
            .overflow_y_scroll()
            .border_r_1()
            .border_color(rgb(theme::separator()))
            .child("页面 · 物料列表");
        for c in &self.boards_view {
            let id = c.board.id.clone();
            rail = rail.child(
                div()
                    .id(format!("rail-{id}"))
                    .v_flex()
                    .workbench_card(self.selected_board.as_ref() == Some(&id))
                    .p_2()
                    .gap_2()
                    .child(c.board.title.clone())
                    .child(preview::page_preview(c, 150.))
                    .on_click(cx.listener(move |v, _, _, cx| v.open_page(&id, cx))),
            );
        }
        let Some(card) = self
            .boards_view
            .iter()
            .find(|c| Some(&c.board.id) == self.selected_board.as_ref())
        else {
            return div()
                .flex_1()
                .child("本地没有这块画板目录，请同步工程与预览。")
                .into_any_element();
        };
        let center = div()
            .flex_1()
            .min_w_0()
            .h_full()
            .v_flex()
            .gap_3()
            .p_3()
            .bg(rgb(theme::surface_faint()))
            .child(
                div()
                    .h_flex()
                    .flex_wrap()
                    .gap_2()
                    .child(self.action(
                        "open-official-editor",
                        "在 VectorCraft 中编辑",
                        |v, cx| v.open_editor(cx),
                        cx,
                    ))
                    .child(self.action(
                        "place-design-file",
                        "放置设计文件",
                        |v, cx| v.place_file(cx),
                        cx,
                    ))
                    .child(self.action(
                        "export-current-image",
                        "导出本页图片",
                        |v, cx| {
                            v.selected_pages = v.selected_board.iter().cloned().collect();
                            v.export_catalog(true, cx);
                        },
                        cx,
                    )),
            )
            .child(div().text_xs().child(format!(
                "{} · {:.1} × {:.1} pt",
                card.board.title, card.board.width_pt, card.board.height_pt
            )))
            .child(
                tracking::control(
                    "artboard",
                    div()
                        .flex_1()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            card.image
                                .clone()
                                .map(|i| {
                                    img(i)
                                        .size_full()
                                        .object_fit(ObjectFit::Contain)
                                        .into_any_element()
                                })
                                .unwrap_or_else(|| {
                                    div()
                                        .child(if card.preview_checked {
                                            "暂无本地预览，请同步工程"
                                        } else {
                                            "正在读取本地预览…"
                                        })
                                        .into_any_element()
                                }),
                        ),
                    &self.surfaces,
                )
                .flex_1()
                .min_h_0(),
            );
        div()
            .flex_1()
            .min_h_0()
            .h_flex()
            .child(rail)
            .child(center)
            .child(self.inspector(card, cx))
            .into_any_element()
    }
}
