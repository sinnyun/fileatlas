pub(super) use super::preview::{figcraft_fit_scale, page_preview, physical_label, pixel_label};
use super::*;

impl Render for WorkCratePanel {
    fn render(&mut self, window: &mut Window, cx: &mut ViewContext<Self>) -> impl IntoElement {
        // Cached service snapshot only; a settings page close must not leave stale menu builders.
        if let Ok(presets) = self.projects.field_presets() {
            self.preset_fields = presets;
        }
        self.sync_text_editor(window, cx);
        self.device_scale = f64::from(window.scale_factor());
        let layouts = self
            .boards_view
            .iter()
            .find(|c| Some(&c.board.id) == self.selected_board.as_ref())
            .map(|c| {
                c.board
                    .placements
                    .keys()
                    .filter_map(|id| c.board.image_layout(id).ok().map(|l| (id.clone(), l)))
                    .collect::<std::collections::BTreeMap<_, _>>()
            });
        let state=serde_json::json!({"screen":if self.selected_project.is_none(){"projects"}else if self.selected_board.is_none(){"pages"}else{"editor"},"busy":self.busy,"preview_cache_hits":self.preview_hits,"previews_ready":self.boards_view.iter().filter(|c| c.image.is_some()).count(),"display_images":self.boards_view.iter().map(|c| c.originals.len()).sum::<usize>(),"display_rgba_bytes":self.boards_view.iter().flat_map(|c| c.originals.values()).map(|i| i.as_bytes(0).map_or(0,|b| b.len())).sum::<usize>(),"export_status":self.export_status,"project_id":self.selected_project,"board_id":self.selected_board,"page_count":self.boards_view.len(),"selected_pages":self.selected_pages,"multi_select":self.multi_select,"selected_text":self.selected_text,"text_draft":self.text_draft(cx),"text_inline":self.text_inline,"text_options":self.options_diagnostics(),"text_format_busy":self.text_format_busy,"text_write_active":self.text_write_active,"load_generation":self.load_generation,"canvas_image_id":self.canvas.as_ref().map(|i|format!("{:?}",i.id)),"display_image_ids":self.boards_view.iter().flat_map(|c|c.originals.iter()).map(|(id,i)|(id.clone(),format!("{:?}",i.id))).collect::<std::collections::BTreeMap<_,_>>(),"text_count":self.boards_view.iter().find(|c|Some(&c.board.id)==self.selected_board.as_ref()).map(|c|c.board.texts.len()),"text_frames":self.boards_view.iter().find(|c|Some(&c.board.id)==self.selected_board.as_ref()).map(|c|c.board.texts.keys().filter_map(|id|c.board.text_frame(id).ok().map(|f|(id.clone(),f))).collect::<std::collections::BTreeMap<_,_>>()),"selected_image":self.selected_image,"layouts":layouts,"crop_mode":self.crop_mode,"edit_saving":self.edit_saving,"gesture":self.gesture.as_ref().map(|g| serde_json::json!({"asset":g.asset,"frame":g.current.frame,"content":g.current.content})),"editing":self.editing,"message":self.message}).to_string();
        if self.state_gate.changed(state.clone()) {
            let _ = self.surfaces.set_value("workcrate.ui", &state);
        }
        let mut toolbar = div()
            .h_flex()
            .items_center()
            .gap_2()
            .p_3()
            .border_b_1()
            .border_color(rgb(theme::separator()))
            .child(
                Button::new("workcrate-projects-home")
                    .ghost()
                    .label("项目管理")
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.selected_project = None;
                        view.selected_board = None;
                        view.canvas = None;
                        view.editing = None;
                        view.reload(cx);
                    })),
            );
        if let Some(project) = self
            .selected_project
            .as_ref()
            .and_then(|id| self.items.iter().find(|p| &p.id == id))
        {
            toolbar = toolbar.child("/").child(
                Button::new("workcrate-pages-home")
                    .ghost()
                    .label(project.name.clone())
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.selected_board = None;
                        view.canvas = None;
                        view.editing = None;
                        cx.notify();
                    })),
            );
            if self.selected_board.is_some() {
                toolbar = toolbar.child("/ 页面编辑");
            }
        }
        toolbar = toolbar.child(div().flex_1());
        if self.selected_project.is_none() {
            toolbar = toolbar
                .child(
                    fileatlas_ui_bridge::components::search_input(&self.search_input).w(px(220.)),
                )
                .child(
                    Button::new("workcrate-search")
                        .ghost()
                        .label("搜索")
                        .on_click(cx.listener(|view, _, _, cx| {
                            view.query = view.search_input.read(cx).value().to_lowercase();
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("workcrate-sort")
                        .ghost()
                        .label(if self.sort_by_name {
                            "名称排序"
                        } else {
                            "最近创建"
                        })
                        .on_click(cx.listener(|view, _, _, cx| {
                            view.sort_by_name = !view.sort_by_name;
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("workcrate-archived")
                        .ghost()
                        .label(if self.show_archived {
                            "已归档"
                        } else {
                            "进行中"
                        })
                        .on_click(cx.listener(|view, _, _, cx| {
                            view.show_archived = !view.show_archived;
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("workcrate-new-project")
                        .primary()
                        .label("新建项目")
                        .disabled(self.busy)
                        .on_click(cx.listener(|view, _, _, cx| view.create_project(cx))),
                );
        } else if self.selected_board.is_none() {
            toolbar = toolbar
                .child(
                    Button::new("project-file-references")
                        .ghost()
                        .label("添加文件引用")
                        .disabled(self.busy)
                        .on_click(cx.listener(|view, _, _, cx| view.add_references(cx))),
                )
                .child(
                    Button::new("workcrate-new-page")
                        .primary()
                        .label("新增页面")
                        .disabled(self.busy)
                        .on_click(cx.listener(|view, _, _, cx| view.create_board(cx))),
                );
        }
        if self.selected_project.is_some() && self.selected_board.is_none() {
            toolbar = toolbar.child(
                Button::new("pages-refresh-links")
                    .outline()
                    .label("更新项目图片")
                    .disabled(self.busy)
                    .on_click(cx.listener(|view, _, _, cx| view.refresh_project_links(cx))),
            );
        }
        let content = if self.selected_project.is_none() {
            self.render_projects(cx)
        } else if self.selected_board.is_none() {
            self.render_pages(cx)
        } else {
            self.render_editor(cx).into_any_element()
        };
        div()
            .id("workcrate-panel")
            .size_full()
            .v_flex()
            .bg(rgb(theme::surface_base()))
            .child(toolbar)
            .child(content)
            .when(self.busy, |el| {
                el.child(
                    div().px_3().pb_2().text_xs().child(
                        self.export_status
                            .clone()
                            .unwrap_or_else(|| "正在处理…".into()),
                    ),
                )
            })
            .when_some(self.message.clone(), |el, message| {
                el.child(div().px_3().pb_2().text_xs().child(message))
            })
    }
}

impl WorkCratePanel {
    fn render_projects(&mut self, cx: &mut ViewContext<Self>) -> gpui::AnyElement {
        let width = ((self.panel_content_size().0 - 96.) / 3.).max(210.) as f32;
        let mut grid = div()
            .id("workcrate-project-grid")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .h_flex()
            .flex_wrap()
            .content_start()
            .items_start()
            .gap(px(24.))
            .p(px(24.));
        let mut items = self.items.clone();
        items.sort_by_key(|p| std::cmp::Reverse(p.created_at));
        if self.sort_by_name {
            items.sort_by_key(|p| p.name.to_lowercase());
        }
        for project in items.into_iter().filter(|p| {
            p.archived == self.show_archived && p.name.to_lowercase().contains(&self.query)
        }) {
            let id = project.id.clone();
            let menu_id = id.clone();
            let name = project.name.clone();
            let mut card = div()
                .id(format!("workcrate-project-{id}"))
                .relative()
                .v_flex()
                .w(px(width))
                .workbench_card(false)
                .p_3()
                .gap_3()
                .child(
                    div()
                        .h_flex()
                        .items_center()
                        .child(
                            Button::new(format!("project-title-{id}"))
                                .ghost()
                                .font_semibold()
                                .text_size(px(20.))
                                .label(name.clone())
                                .on_click(cx.listener(move |view, _, _, cx| {
                                    view.selected_project = Some(id.clone());
                                    view.selected_board = None;
                                    view.manage_id = None;
                                    view.reload(cx);
                                })),
                        )
                        .child(div().flex_1())
                        .child(super::tracking::control(
                            &format!("project-menu-{menu_id}"),
                            Button::new(format!("project-menu-{menu_id}"))
                                .ghost()
                                .icon(theme::small_icon(gpui_kit::assets::IconName::Ellipsis))
                                .tooltip("项目操作")
                                .on_click(cx.listener(move |view, _, _, cx| {
                                    view.manage_id = if view.manage_id.as_ref() == Some(&menu_id) {
                                        None
                                    } else {
                                        Some(menu_id.clone())
                                    };
                                    cx.notify();
                                })),
                            &self.surfaces,
                        )),
                );
            let id = project.id.clone();
            card = card.child(
                div()
                    .id(format!("project-cover-{id}"))
                    .w_full()
                    .h(px((width - 24.) * 0.92))
                    .rounded(px(8.))
                    .overflow_hidden()
                    .bg(rgb(theme::surface_faint()))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        self.project_covers
                            .get(&id)
                            .cloned()
                            .map(|image| {
                                img(image)
                                    .size_full()
                                    .object_fit(ObjectFit::Cover)
                                    .into_any_element()
                            })
                            .unwrap_or_else(|| {
                                div()
                                    .text_color(rgb(theme::text_muted()))
                                    .child("选择项目主图")
                                    .into_any_element()
                            }),
                    )
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.selected_project = Some(id.clone());
                        view.selected_board = None;
                        view.reload(cx);
                    })),
            );
            if self.manage_id.as_ref() == Some(&project.id) {
                let target = project.id.clone();
                let value = name.clone();
                let mut menu = div()
                    .absolute()
                    .top(px(38.))
                    .right(px(6.))
                    .w(px(160.))
                    .v_flex()
                    .gap_1()
                    .workbench_section()
                    .p_2()
                    .child(super::tracking::control(
                        &format!("project-rename-{target}"),
                        Button::new(format!("project-rename-{target}"))
                            .ghost()
                            .label("重命名")
                            .on_click(cx.listener(move |view, _, window, cx| {
                                view.begin_edit(target.clone(), "project", &value, window, cx)
                            })),
                        &self.surfaces,
                    ));
                for (action, label) in [
                    ("cover", "更换主图"),
                    ("duplicate", "复制项目"),
                    (
                        "archive",
                        if project.archived {
                            "恢复项目"
                        } else {
                            "归档项目"
                        },
                    ),
                    ("delete", "删除项目"),
                ] {
                    let target = project.id.clone();
                    let action = action.to_owned();
                    let control_id = format!("project-{action}-{target}");
                    menu = menu.child(super::tracking::control(
                        &control_id,
                        Button::new(control_id.clone())
                            .ghost()
                            .label(label)
                            .disabled(self.busy)
                            .on_click(cx.listener(move |view, _, _, cx| {
                                view.project_action(target.clone(), &action, cx)
                            })),
                        &self.surfaces,
                    ));
                }
                card = card.child(menu);
            }
            if self
                .editing
                .as_ref()
                .is_some_and(|(target, key)| target == &project.id && key == "project")
            {
                card = card.child(self.edit_row(cx));
            }
            grid = grid.child(card);
        }
        grid.child(
            Button::new("workcrate-add-project-tile")
                .outline()
                .w(px(width))
                .h(px(180.))
                .label("＋ 新建项目")
                .on_click(cx.listener(|view, _, _, cx| view.create_project(cx))),
        )
        .into_any_element()
    }

    pub(super) fn open_page(&mut self, id: &str, cx: &mut ViewContext<Self>) {
        for card in &mut self.boards_view {
            if card.board.id != id {
                card.originals.clear();
                card.sources.clear();
            }
        }
        self.selected_text = None;
        self.text_editor = None;
        self.text_gesture = None;
        self.text_images.clear();
        self.selected_board = Some(id.into());
        self.gesture = None;
        self.crop_mode = false;
        self.canvas = None;
        self.editing = None;
        if let Some(board) = self
            .boards_view
            .iter()
            .find(|card| card.board.id == id)
            .map(|card| card.board.clone())
        {
            self.render_canvas(board, cx);
            self.proxy_request = None;
            self.load_editor_images(cx);
        }
        cx.notify();
    }
    pub(super) fn panel_content_size(&self) -> (f64, f64) {
        let id = self
            .surfaces
            .open_panel_ids()
            .into_iter()
            .find(|id| id.starts_with(&format!("{PANEL}@")))
            .unwrap_or_else(|| PANEL.into());
        self.surfaces
            .value(&format!("panel.{id}.content_bounds"))
            .and_then(|v| {
                let values = v
                    .split(',')
                    .map(str::parse::<f64>)
                    .collect::<Result<Vec<_>, _>>()
                    .ok()?;
                (values.len() == 4).then_some((values[2], values[3]))
            })
            .unwrap_or((1240., 860.))
    }
}
