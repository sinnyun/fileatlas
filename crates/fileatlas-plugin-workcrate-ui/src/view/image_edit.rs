use super::*;
use fileatlas_plugin_workcrate_boards::ImageLayout;

#[derive(Clone)]
pub(super) struct ImageGesture {
    pub asset: String,
    pub start: gpui::Point<gpui::Pixels>,
    pub before: ImageLayout,
    pub current: ImageLayout,
    pub corner: Option<usize>,
    pub crop: bool,
}

impl WorkCratePanel {
    pub(super) fn begin_image_gesture(
        &mut self,
        asset: String,
        corner: Option<usize>,
        position: gpui::Point<gpui::Pixels>,
        cx: &mut ViewContext<Self>,
    ) {
        if self.edit_saving {
            return;
        }
        let Some(card) = self
            .boards_view
            .iter()
            .find(|c| Some(&c.board.id) == self.selected_board.as_ref())
        else {
            return;
        };
        let Ok(layout) = card.board.image_layout(&asset) else {
            return;
        };
        self.selected_text = None;
        self.text_editor = None;
        self.text_gesture = None;
        self.selected_image = Some(asset.clone());
        self.gesture = Some(ImageGesture {
            asset,
            start: position,
            before: layout.clone(),
            current: layout,
            corner,
            crop: self.crop_mode && corner.is_some(),
        });
        cx.notify();
    }

    pub(super) fn move_image_gesture(
        &mut self,
        position: gpui::Point<gpui::Pixels>,
        cx: &mut ViewContext<Self>,
    ) {
        let page = self
            .boards_view
            .iter()
            .find(|c| Some(&c.board.id) == self.selected_board.as_ref())
            .map(|c| {
                (
                    c.board.source.settings.page_width,
                    c.board.source.settings.page_height,
                )
            })
            .unwrap_or((1190.55, 841.89));
        let Some(g) = &mut self.gesture else {
            return;
        };
        let mut dx = f64::from(position.x - g.start.x) / self.fit_scale;
        let mut dy = f64::from(position.y - g.start.y) / self.fit_scale;
        let f = g.before.frame;
        let mut next = f;
        if let Some(corner) = g.corner {
            let left = corner == 0 || corner == 2;
            let top = corner < 2;
            if g.crop {
                let c = g.before.content;
                if left {
                    next[0] = (f[0] + dx).clamp(c[0], f[2] - 4.);
                } else {
                    next[2] = (f[2] + dx).clamp(f[0] + 4., c[2]);
                }
                if top {
                    next[1] = (f[1] + dy).clamp(c[1], f[3] - 4.);
                } else {
                    next[3] = (f[3] + dy).clamp(f[1] + 4., c[3]);
                }
                g.current.content = c;
            } else {
                let w = f[2] - f[0];
                let h = f[3] - f[1];
                let vx = if left { -w } else { w };
                let vy = if top { -h } else { h };
                let ax = if left { f[2] } else { f[0] };
                let ay = if top { f[3] } else { f[1] };
                let min_scale = (8. / w).max(8. / h).max(0.001);
                let max_scale = ((if left { ax } else { page.0 - ax }) / w)
                    .min((if top { ay - page.1 * 0.06 } else { page.1 - ay }) / h)
                    .max(min_scale);
                let scale =
                    (1. + (dx * vx + dy * vy) / (w * w + h * h)).clamp(min_scale, max_scale);
                next = [
                    ax + (f[0] - ax) * scale,
                    ay + (f[1] - ay) * scale,
                    ax + (f[2] - ax) * scale,
                    ay + (f[3] - ay) * scale,
                ];
                let c = g.before.content;
                g.current.content = [
                    ax + (c[0] - ax) * scale,
                    ay + (c[1] - ay) * scale,
                    ax + (c[2] - ax) * scale,
                    ay + (c[3] - ay) * scale,
                ];
            }
        } else {
            dx = dx.clamp(-f[0], (page.0 - f[2]).max(-f[0]));
            dy = dy.clamp(
                page.1 * 0.06 - f[1],
                (page.1 - f[3]).max(page.1 * 0.06 - f[1]),
            );
            next = [f[0] + dx, f[1] + dy, f[2] + dx, f[3] + dy];
            let c = g.before.content;
            g.current.content = [c[0] + dx, c[1] + dy, c[2] + dx, c[3] + dy];
        }
        if g.current.frame != next {
            g.current.frame = next;
            cx.notify();
        }
    }

    pub(super) fn end_image_gesture(&mut self, cx: &mut ViewContext<Self>) {
        if self.edit_saving {
            return;
        }
        let (Some(g), Some(id)) = (self.gesture.clone(), self.selected_board.clone()) else {
            return;
        };
        if g.current.frame == g.before.frame {
            self.gesture = None;
            cx.notify();
            return;
        }
        self.edit_saving = true;
        let boards = Arc::clone(&self.boards);
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    boards
                        .set_image_frame(&id, &g.asset, g.current.frame, g.crop)
                        .map_err(|e| e.to_string())
                })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.edit_saving = false;
                view.gesture = None;
                match result {
                    Ok(board) => {
                        let id = board.id.clone();
                        if let Some(card) = view.boards_view.iter_mut().find(|c| c.board.id == id) {
                            card.board = board;
                        }
                        view.proxy_request = None;
                        view.load_editor_images(cx);
                        view.refresh_page_preview(id, cx);
                    }
                    Err(e) => view.message = Some(e),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn refresh_page_preview(&self, id: String, cx: &mut ViewContext<Self>) {
        let boards = Arc::clone(&self.boards);
        cx.spawn(async move |this, cx| {
            let requested = boards
                .board(&id)
                .ok()
                .map(|b| serde_json::to_string(&b.source).unwrap_or_default());
            let image = cx
                .background_executor()
                .spawn({
                    let boards = Arc::clone(&boards);
                    let id = id.clone();
                    async move {
                        let board = boards.board(&id).ok()?;
                        boards.cache_preview(&board).ok().map(render_image)
                    }
                })
                .await;
            let _ = this.update(cx, |view, cx| {
                if let Some(card) = view.boards_view.iter_mut().find(|c| c.board.id == id) {
                    if requested
                        == Some(serde_json::to_string(&card.board.source).unwrap_or_default())
                    {
                        card.image = image;
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }
}
