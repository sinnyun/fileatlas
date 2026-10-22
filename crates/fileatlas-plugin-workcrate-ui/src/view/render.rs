use super::canvas::CANVAS_INSPECTOR_WIDTH;
use super::*;
enum TierTarget {
    Projects,
    Pages,
}
impl Render for WorkCratePanel {
    fn render(&mut self, _: &mut Window, cx: &mut ViewContext<Self>) -> impl IntoElement {
        self.renders += 1;
        let state=serde_json::json!({"screen":if self.selected_project.is_none(){"projects"}else if self.selected_board.is_none(){"pages"}else{"editor"},"project_id":self.selected_project,"board_id":self.selected_board,"page_count":self.boards_view.len(),"busy":self.busy,"preview_pending":self.preview_queue.len()+usize::from(self.preview_current.is_some()),"previews_ready":self.boards_view.iter().filter(|c|c.image.is_some()).count(),"boards":self.boards_view.iter().map(|c|serde_json::json!({"id":c.board.id,"native_id":c.board.native_id,"index":c.board.index,"title":c.board.title,"width_pt":c.board.width_pt,"height_pt":c.board.height_pt,"info":c.board.info,"preview_ready":c.image.is_some(),"canvas_revision":c.canvas_revision,"error":c.error})).collect::<Vec<_>>(),"message":self.message,"editing":self.editing.as_ref().map(|(id,key)|format!("{id}#{key}")),"field_saves":self.field_saves.iter().map(|(id,key)|format!("{id}#{key}")).collect::<Vec<_>>(),"reloads":self.reloads,"renders":self.renders,"drops":self.drops,"auto_sync":{"syncs":self.auto_syncs,"pending":self.auto_sync,"error":self.auto_sync_error},"connection":if self.ui_state.connection(){"connected"}else{"not_connected"},"executable":self.config.executable,"port":self.config.port,"document":self.document,"selected_pages":self.selected_pages,"multi_select":self.multi_select,"sort_by_name":self.sort_by_name,"show_archived":self.show_archived,"object_status":self.selected_board.as_ref().map(|_|self.canvas_object_status()),"canvas":{"busy":self.canvas_busy,"zoom":self.canvas_zoom,"state":self.canvas,"object":self.canvas_object,"drag":self.canvas_drag.as_ref().map(|drag|serde_json::json!({"kind":match drag.kind{super::canvas::CanvasDragKind::Body=>"move".to_owned(),super::canvas::CanvasDragKind::Handle(index)=>format!("handle{index}")},"origin":drag.origin,"start":[drag.start.0,drag.start.1],"current":[drag.current.0,drag.current.1]}))}}).to_string();
        if self.state_gate.changed(state.clone()) {
            let _ = self.surfaces.set_value("workcrate.ui", &state);
        }
        let content = if self.selected_project.is_none() {
            self.render_projects(cx)
        } else if self.selected_board.is_none() {
            self.render_pages(cx)
        } else {
            self.render_editor(cx)
        };
        // One fixed-height status line: showing a save message must not shrink the editor
        // and make the whole page jump on every field save.
        let status = if self.busy {
            "正在处理…".to_owned()
        } else if !self.field_saves.is_empty() {
            "正在保存页面内容…".to_owned()
        } else if self.auto_sync {
            "正在获取 VectorCraft 的更改…".to_owned()
        } else {
            self.message.clone().unwrap_or_default()
        };
        let mut root = div()
            .id("workcrate-panel")
            .size_full()
            .v_flex()
            .bg(rgb(theme::surface_base()))
            .text_color(rgb(theme::text_primary()))
            .child(self.tier_band(cx));
        // The pages screen has its own hero header; the editor keeps the slim
        // context bar with the project-level actions.
        if self.selected_board.is_some() {
            root = root.child(self.screen_bar(cx));
        }
        root.child(content).child(
            div()
                .h(px(24.))
                .flex_shrink_0()
                .px_3()
                .h_flex()
                .items_center()
                .text_size(px(11.))
                .text_color(rgb(theme::text_muted()))
                .gap_3()
                .child(div().flex_1().min_w_0().truncate().child(status))
                .when(self.selected_board.is_some(), |el| {
                    el.child(self.canvas_object_footer())
                }),
        )
    }
}
impl WorkCratePanel {
    /// The shared three-tier entry band from the 2026-10-21 user drafts; the
    /// active tier is a card, reachable tiers navigate, the rest stay muted.
    fn tier_band(&self, cx: &mut ViewContext<Self>) -> gpui::Stateful<gpui::Div> {
        use gpui_kit::assets::IconName;
        let project = self
            .selected_project
            .as_ref()
            .and_then(|id| self.items.iter().find(|p| &p.id == id));
        let in_project = project.is_some();
        let in_editor = in_project && self.selected_board.is_some();
        let page_no = self
            .selected_board
            .as_ref()
            .and_then(|b| self.boards_view.iter().position(|c| &c.board.id == b))
            .map(|i| format!("{:02}", i + 1));
        div()
            .id("workcrate-tier-band")
            .h_flex()
            .items_center()
            .justify_center()
            .gap_2()
            .px_3()
            .py(px(8.))
            .border_b_1()
            .border_color(rgb(theme::separator()))
            .child(tracking::control(
                "workcrate-projects-home",
                self.tier_pill(
                    "workcrate-projects-home",
                    IconName::Briefcase,
                    "项目管理 (Tier 1)",
                    None,
                    !in_project,
                    in_project.then_some(TierTarget::Projects),
                    cx,
                ),
                &self.surfaces,
            ))
            .child(tracking::control(
                "workcrate-pages-home",
                self.tier_pill(
                    "workcrate-pages-home",
                    IconName::LayoutGrid,
                    "页面管理 (Tier 2)",
                    project.map(|p| p.name.clone()),
                    in_project && !in_editor,
                    in_project.then_some(TierTarget::Pages),
                    cx,
                ),
                &self.surfaces,
            ))
            .child(tracking::control(
                "workcrate-tier-studio",
                self.tier_pill(
                    "workcrate-tier-studio",
                    IconName::PenTool,
                    "装配工作台 (Tier 3)",
                    page_no,
                    in_editor,
                    None,
                    cx,
                ),
                &self.surfaces,
            ))
            .child(tracking::control(
                "project-defaults",
                Button::new("workcrate-project-defaults")
                    .outline()
                    .compact()
                    .label(if in_project {
                        "项目默认信息"
                    } else {
                        "程序默认信息"
                    })
                    .on_click(cx.listener(|view, _, _, cx| {
                        if let Err(error) = view.panels.open("workcrate/project-defaults") {
                            view.message = Some(error.to_string());
                            cx.notify();
                        }
                    })),
                &self.surfaces,
            ))
    }

    fn tier_pill(
        &self,
        id: &str,
        icon: gpui_kit::assets::IconName,
        label: &str,
        meta: Option<String>,
        active: bool,
        target: Option<TierTarget>,
        cx: &mut ViewContext<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let pill = div()
            .id(id.to_owned())
            .h_flex()
            .items_center()
            .gap_2()
            .h(px(34.))
            .px(px(theme::SPACE_3))
            .rounded(px(theme::ROUND_MD))
            .text_size(px(theme::FONT_SIZE_SMALL))
            .child(theme::small_icon(icon))
            .child(label.to_owned())
            .children(meta.map(|meta| {
                div()
                    .text_color(rgb(theme::text_muted()))
                    .child(format!("· {meta}"))
            }));
        if active {
            pill.workbench_card(false).font_semibold()
        } else if let Some(target) = target {
            pill.cursor_pointer()
                .text_color(rgb(theme::text_secondary()))
                .hover(|style| style.bg(rgb(theme::surface_panel())))
                .on_click(cx.listener(move |view, _, _, cx| match target {
                    TierTarget::Projects => {
                        if view.canvas.is_some() {
                            view.leave_canvas(cx);
                        }
                        view.selected_project = None;
                        view.selected_board = None;
                        view.editing = None;
                        let _ = view.ui_state.navigate(None, None);
                        view.reload(cx);
                    }
                    TierTarget::Pages => {
                        if view.canvas.is_some() {
                            view.leave_canvas(cx);
                        }
                        view.selected_board = None;
                        view.editing = None;
                        let _ = view.ui_state.navigate(view.selected_project.clone(), None);
                        cx.notify();
                    }
                }))
        } else {
            pill.text_color(rgb(theme::text_muted()))
        }
    }

    /// Slim bar under the tier band holding the current context and the
    /// project-level actions; the projects screen has its own header instead.
    fn screen_bar(&self, cx: &mut ViewContext<Self>) -> gpui::Div {
        use gpui_kit::assets::IconName;
        let title = if self.selected_board.is_some() {
            self.boards_view
                .iter()
                .find(|c| Some(&c.board.id) == self.selected_board.as_ref())
                .map(|c| c.board.title.clone())
                .unwrap_or_default()
        } else {
            self.selected_project
                .as_ref()
                .and_then(|id| self.items.iter().find(|p| &p.id == id))
                .map(|p| p.name.clone())
                .unwrap_or_default()
        };
        div()
            .h_flex()
            .items_center()
            .gap_2()
            .px_3()
            .py(px(6.))
            .border_b_1()
            .border_color(rgb(theme::separator()))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .font_semibold()
                    .text_size(px(theme::FONT_SIZE_BODY))
                    .child(title),
            )
            .child(self.screen_action(
                "attach-document",
                Some(IconName::Link2),
                "关联工程",
                false,
                |v, cx| v.pick_document(cx),
                cx,
            ))
            .child(self.screen_action(
                "workcrate-new-page",
                Some(IconName::Plus),
                "新增 A3 页面",
                false,
                |v, cx| v.create_board(cx),
                cx,
            ))
            .child(self.screen_action(
                "refresh-native",
                Some(IconName::RefreshCw),
                "同步工程与预览",
                false,
                |v, cx| v.sync_project(cx),
                cx,
            ))
    }

    pub(super) fn screen_action(
        &self,
        id: &str,
        icon: Option<gpui_kit::assets::IconName>,
        label: &str,
        primary: bool,
        run: impl Fn(&mut Self, &mut ViewContext<Self>) + 'static,
        cx: &mut ViewContext<Self>,
    ) -> gpui::Div {
        let button = {
            let mut button = Button::new(id.to_owned());
            if let Some(icon) = icon {
                button = button.icon(theme::icon(icon));
            }
            let button = button
                .label(label.to_owned())
                .compact()
                .rounded(px(theme::ROUND_MD));
            let button = if primary {
                button.primary()
            } else {
                button.outline()
            };
            button
                .disabled(self.busy)
                .on_click(cx.listener(move |v, _, _, cx| {
                    if !v.field_saves.is_empty() {
                        v.message = Some("正在保存页面内容，请稍候…".into());
                        cx.notify();
                        return;
                    }
                    if v.canvas_busy {
                        v.message = Some("画布操作进行中，请稍候…".into());
                        cx.notify();
                        return;
                    }
                    run(v, cx);
                }))
        };
        tracking::control(id, button, &self.surfaces)
    }

    /// Icon-only toolbar control with the same busy guards as `screen_action`.
    pub(super) fn icon_action(
        &self,
        id: &str,
        icon: gpui_kit::assets::IconName,
        tooltip: &str,
        run: impl Fn(&mut Self, &mut ViewContext<Self>) + 'static,
        cx: &mut ViewContext<Self>,
    ) -> gpui::Div {
        tracking::control(
            id,
            fileatlas_ui_bridge::components::icon_button(id, icon)
                .tooltip(tooltip)
                .on_click(cx.listener(move |v, _, _, cx| {
                    if !v.field_saves.is_empty() {
                        v.message = Some("正在保存页面内容，请稍候…".into());
                        cx.notify();
                        return;
                    }
                    if v.canvas_busy {
                        v.message = Some("画布操作进行中，请稍候…".into());
                        cx.notify();
                        return;
                    }
                    run(v, cx);
                })),
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
            .child(
                div()
                    .text_size(px(theme::FONT_SIZE_TINY))
                    .text_color(rgb(theme::text_muted()))
                    .child("页面 · 物料列表"),
            );
        for (index, c) in self.boards_view.iter().enumerate() {
            let id = c.board.id.clone();
            let active = self.selected_board.as_ref() == Some(&id);
            rail = rail.child(tracking::control(
                &format!("rail-{id}"),
                div()
                    .id(format!("rail-{id}"))
                    .v_flex()
                    .workbench_card(active)
                    .when(active, |el| {
                        el.border_1().border_color(rgb(theme::accent()))
                    })
                    .p_2()
                    .gap_1()
                    .child(preview::page_preview(c, 150.))
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .font_semibold()
                                    .text_size(px(theme::FONT_SIZE_TINY))
                                    .text_color(rgb(theme::accent()))
                                    .child(format!("{:02}", index + 1)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(theme::FONT_SIZE_TINY))
                                    .child(c.board.title.clone()),
                            ),
                    )
                    .child(self.page_order_controls(&id, index, cx))
                    .drag_over::<super::page_order::PageDrag>(|style, _, _, _| {
                        style.border_color(rgb(theme::accent_hover()))
                    })
                    .on_drop::<super::page_order::PageDrag>(cx.listener({
                        let id = id.clone();
                        move |view, payload, _, cx| view.drop_page(payload, &id, cx)
                    }))
                    .on_click(cx.listener(move |v, _, _, cx| v.open_page(&id, cx))),
                &self.surfaces,
            ));
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
            .bg(rgb(theme::surface_faint()))
            .child(self.canvas_toolbar(cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .v_flex()
                    .gap_3()
                    .p_3()
                    .child(
                        div().h_flex().items_center().gap_2().child(
                            div()
                                .text_size(px(theme::FONT_SIZE_TINY))
                                .text_color(rgb(theme::text_muted()))
                                .child(card.board.title.clone()),
                        ),
                    )
                    .child(self.canvas_area(cx)),
            );
        div()
            .flex_1()
            .min_h_0()
            .h_flex()
            .child(rail)
            .child(center)
            .child(
                div()
                    .id("workcrate-editor-rail")
                    .h_full()
                    .w(px(CANVAS_INSPECTOR_WIDTH))
                    .flex_shrink_0()
                    .min_h_0()
                    .v_flex()
                    .border_l_1()
                    .border_color(rgb(theme::separator()))
                    .overflow_y_scroll()
                    .child(self.inspector(card, cx))
                    .child(self.canvas_object_panel(cx)),
            )
            .into_any_element()
    }
}
