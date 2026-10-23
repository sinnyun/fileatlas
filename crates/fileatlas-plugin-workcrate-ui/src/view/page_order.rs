use super::*;
use fileatlas_ui_bridge::components::WorkbenchPaint as _;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
use gpui_kit::assets::IconName;

#[derive(Clone)]
pub(super) struct PageDrag {
    project: String,
    id: String,
    order: Vec<String>,
    title: String,
}
impl Render for PageDrag {
    fn render(&mut self, _: &mut Window, _: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .px(px(fileatlas_ui_bridge::theme::space_expanded()))
            .py(px(fileatlas_ui_bridge::theme::space_standard()))
            .workbench_corner(fileatlas_ui_bridge::components::CornerRole::Medium)
            .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Panel)
            .workbench_body()
            .child(format!("移动 {}", self.title))
    }
}

impl WorkCratePanel {
    fn order_locked(&self) -> bool {
        self.busy || self.canvas_busy || !self.field_saves.is_empty() || self.editing.is_some()
    }
    pub(super) fn page_order_controls(
        &self,
        id: &str,
        index: usize,
        cx: &mut ViewContext<Self>,
    ) -> gpui::Div {
        let mut row = div()
            .h_flex()
            .items_center()
            .gap(px(fileatlas_ui_bridge::theme::space_compact()));
        let locked = self.order_locked();
        for (key, icon, to) in [
            ("up", IconName::ArrowUp, index.checked_sub(1)),
            (
                "down",
                IconName::ArrowDown,
                (index + 1 < self.boards_view.len()).then_some(index + 1),
            ),
        ] {
            let id = id.to_owned();
            row = row.child(super::tracking::control(
                &format!("page-{key}-{id}"),
                fileatlas_ui_bridge::components::button(format!("page-{key}-{id}"))
                    .ghost()
                    .icon(theme::small_icon(icon))
                    .tooltip(if key == "up" {
                        "上移一页"
                    } else {
                        "下移一页"
                    })
                    .disabled(locked || to.is_none())
                    .on_click(cx.listener(move |view, _, _, cx| {
                        if let Some(to) = to {
                            view.move_page(&id, to, cx);
                        }
                    })),
                &self.surfaces,
            ));
        }
        let payload = PageDrag {
            project: self.selected_project.clone().unwrap_or_default(),
            id: id.into(),
            order: self
                .boards_view
                .iter()
                .map(|c| c.board.id.clone())
                .collect(),
            title: self.boards_view[index].board.title.clone(),
        };
        row.child(super::tracking::control(
            &format!("page-drag-{id}"),
            div()
                .id(format!("page-drag-{id}"))
                .px(px(fileatlas_ui_bridge::theme::space_standard()))
                .py(px(fileatlas_ui_bridge::theme::space_compact()))
                .workbench_assistant()
                .child("⋮⋮")
                .when(!locked, |el| {
                    el.cursor_pointer()
                        .on_drag(payload, |value, _, _, cx| cx.new(|_| value.clone()))
                }),
            &self.surfaces,
        ))
    }
    pub(super) fn drop_page(
        &mut self,
        payload: &PageDrag,
        target: &str,
        cx: &mut ViewContext<Self>,
    ) {
        if self.selected_project.as_deref() != Some(&payload.project)
            || payload.order
                != self
                    .boards_view
                    .iter()
                    .map(|c| c.board.id.clone())
                    .collect::<Vec<_>>()
        {
            self.message = Some("页面列表已变化，请重新拖动".into());
            cx.notify();
            return;
        }
        if let Some(to) = self.boards_view.iter().position(|c| c.board.id == target) {
            self.move_page(&payload.id, to, cx);
        }
    }
    fn move_page(&mut self, id: &str, to: usize, cx: &mut ViewContext<Self>) {
        if self.order_locked() {
            return;
        }
        let mut order = self
            .boards_view
            .iter()
            .map(|c| c.board.id.clone())
            .collect::<Vec<_>>();
        let Some(from) = order.iter().position(|value| value == id) else {
            return;
        };
        if from == to || to >= order.len() {
            return;
        }
        let Some(project) = self.selected_project.clone() else {
            return;
        };
        let moved = order.remove(from);
        order.insert(to, moved);
        let boards = self.boards.clone();
        self.order_job(
            move || {
                boards
                    .reorder_pages(&project, &order)
                    .map_err(|e| e.to_string())
            },
            cx,
        );
    }
}

struct OrderFrame {
    save: fileatlas_plugin_workcrate_boards::PageOrderSave,
    frame: Option<super::canvas::CanvasFrame>,
    images: BTreeMap<String, Arc<RenderImage>>,
}
impl WorkCratePanel {
    pub(super) fn receive_order_save(
        &mut self,
        save: fileatlas_plugin_workcrate_boards::PageOrderSave,
        cx: &mut ViewContext<Self>,
    ) {
        if save
            .boards
            .first()
            .is_none_or(|b| Some(&b.project_id) != self.selected_project.as_ref())
        {
            return;
        }
        self.order_job(move || Ok(save), cx);
    }
    fn order_job(
        &mut self,
        run: impl FnOnce() -> Result<fileatlas_plugin_workcrate_boards::PageOrderSave, String>
        + Send
        + 'static,
        cx: &mut ViewContext<Self>,
    ) {
        self.busy = true;
        // An older thumbnail decode must not overwrite a newly ordered page image.
        self.load_generation += 1;
        self.preview_queue.clear();
        self.preview_current = None;
        self.order_generation += 1;
        let generation = self.order_generation;
        self.message = Some("正在保存页面顺序…".into());
        self.canvas_poll += 1; // Discard old passive probes, keep the visible canvas/frame.
        let boards = self.boards.clone();
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let save = run()?;
                    let frame = save
                        .frame
                        .clone()
                        .filter(|s| save.changed.contains(&s.board_id))
                        .map(|s| super::canvas::canvas_frame(&boards, s));
                    let mut images = BTreeMap::new();
                    for id in &save.changed {
                        if let Some(f) = &frame
                            && &f.snapshot.board_id == id
                        {
                            if let Some(image) = &f.image {
                                images.insert(id.clone(), image.clone());
                            }
                        } else {
                            let path =
                                boards.render_preview(id, 1600).map_err(|e| e.to_string())?;
                            let pixels = image::open(path).map_err(|e| e.to_string())?;
                            images.insert(id.clone(), render_image(pixels.into_rgba8()));
                        }
                    }
                    Ok::<_, String>(OrderFrame {
                        save,
                        frame,
                        images,
                    })
                })
                .await;
            let _ =
                this.update_in(cx, |view: &mut WorkCratePanel, window, cx| {
                    if view.order_generation != generation {
                        return;
                    }
                    view.busy = false;
                    match result {
                        Ok(OrderFrame {
                            save,
                            frame,
                            images,
                        }) => {
                            if save.boards.first().is_some_and(|b| {
                                Some(&b.project_id) == view.selected_project.as_ref()
                            }) {
                                let mut previous = std::mem::take(&mut view.boards_view)
                                    .into_iter()
                                    .map(|c| (c.board.id.clone(), c))
                                    .collect::<BTreeMap<_, _>>();
                                view.boards_view = save
                                    .boards
                                    .into_iter()
                                    .map(|board| {
                                        let mut card =
                                            previous.remove(&board.id).unwrap_or(BoardCard {
                                                board: board.clone(),
                                                image: None,
                                                error: None,
                                                preview_checked: false,
                                                canvas_revision: None,
                                            });
                                        if let Some(image) = images.get(&board.id) {
                                            card.image = Some(image.clone());
                                            card.error = None;
                                            card.preview_checked = true;
                                        }
                                        card.board = board;
                                        card
                                    })
                                    .collect();
                                if let Some(snapshot) = save.frame
                                    && view.selected_board.as_ref() == Some(&snapshot.board_id)
                                {
                                    if let Some(frame) = frame {
                                        view.apply_canvas_frame(frame, window, cx);
                                    } else {
                                        view.canvas = Some(snapshot);
                                    }
                                }
                                view.message = Some("页面顺序已保存".into());
                            }
                        }
                        Err(error) => view.message = Some(format!("排序失败：{error}")),
                    }
                    if view.canvas.is_some() {
                        view.start_canvas_poller(cx);
                    }
                    view.preview_queue = view
                        .boards_view
                        .iter()
                        .filter(|c| c.image.is_none())
                        .map(|c| c.board.id.clone())
                        .collect();
                    if view.preview_current.is_none() {
                        view.start_preview();
                    }
                    view.ui_state.wake().trigger();
                    cx.notify();
                });
        })
        .detach();
    }
}
