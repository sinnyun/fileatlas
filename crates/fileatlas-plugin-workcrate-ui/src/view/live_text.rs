//! Coalesced text-only writes. Input entities and image textures survive each commit.
use super::*;

pub(super) struct Draft {
    board: String,
    id: String,
    body: String,
    size: f64,
    color: String,
    frame: [f64; 4],
}

impl WorkCratePanel {
    pub(super) fn live_text_draft(&self, cx: &ViewContext<Self>) -> Option<Draft> {
        let e = self.text_editor.as_ref()?;
        let size = e.size.read(cx).value().parse::<f64>().ok()?;
        let width = e.width.read(cx).value().parse::<f64>().ok()?;
        let height = e.height.read(cx).value().parse::<f64>().ok()?;
        if !size.is_finite()
            || !(0.1..=1296.).contains(&size)
            || !width.is_finite()
            || !height.is_finite()
            || width < 4.
            || height < 4.
        {
            return None;
        }
        Some(Draft {
            board: e.board.clone(),
            id: e.id.clone(),
            body: e.body.read(cx).value().to_string(),
            size,
            color: format!("#{:06x}", theme::packed(e.color.read(cx).value()?)),
            frame: [
                e.frame[0],
                e.frame[1],
                e.frame[0] + width,
                e.frame[1] + height,
            ],
        })
    }

    pub(super) fn queue_text_update(&mut self, cx: &mut ViewContext<Self>) {
        let Some(draft) = self.live_text_draft(cx) else {
            return;
        };
        self.pending_text
            .insert((draft.board.clone(), draft.id.clone()), draft);
        self.text_revision += 1;
        cx.notify();
        if self.text_write_active {
            return;
        }
        self.text_write_active = true;
        fileatlas_ui_bridge::refresh::poll(
            cx,
            std::time::Duration::from_millis(100),
            |view, cx| {
                use fileatlas_ui_bridge::refresh::PollResult;
                if view.text_commit_active || view.text_format_busy {
                    return PollResult::until(false, true);
                }
                let Some((_, draft)) = view.pending_text.pop_first() else {
                    view.text_write_active = false;
                    return PollResult::until(true, false);
                };
                view.text_commit_active = true;
                view.commit_text_draft(draft, cx);
                PollResult::until(false, true)
            },
        );
    }

    fn commit_text_draft(&mut self, draft: Draft, cx: &mut ViewContext<Self>) {
        let service = self.boards.clone();
        let revision = self.text_revision;
        let scale = self.fit_scale * self.device_scale;
        cx.spawn(async move |this, cx| {
            let id = draft.id.clone();
            let result = cx
                .background_executor()
                .spawn(async move {
                    let board = service
                        .update_text(
                            &draft.board,
                            &draft.id,
                            &draft.body,
                            draft.size,
                            &draft.color,
                            draft.frame,
                        )
                        .map_err(|e| e.to_string())?;
                    let rendered = service
                        .render_text(&board.id, &draft.id, scale)
                        .map_err(|e| e.to_string())?;
                    let pixels = image::RgbaImage::from_raw(
                        rendered.width,
                        rendered.height,
                        rendered.to_straight(),
                    )
                    .ok_or("文本像素无效")?;
                    Ok::<_, String>((board, render_image(pixels)))
                })
                .await;
            let _ = this.update(cx, |view, cx| {
                let changed = !view.pending_text.is_empty();
                match result {
                    Ok((board, pixels)) => {
                        let selected = view.selected_board.as_ref() == Some(&board.id);
                        if let Some(card) =
                            view.boards_view.iter_mut().find(|c| c.board.id == board.id)
                        {
                            card.board = board.clone();
                        }
                        let pending_same = view
                            .pending_text
                            .contains_key(&(board.id.clone(), id.clone()));
                        if let Some(e) = view
                            .text_editor
                            .as_mut()
                            .filter(|e| e.board == board.id && e.id == id)
                        {
                            // Keep the live entity/caret and any newer draft intact.
                            if pending_same
                                || e.body.read(cx).value().to_string() == board.texts[&id].text
                            {
                                e.source_revision =
                                    serde_json::to_string(&board.texts[&id]).unwrap_or_default();
                            }
                            if !pending_same {
                                e.frame = board.text_frame(&id).unwrap_or(e.frame);
                            }
                        }
                        if selected && !pending_same {
                            view.text_images.insert(id.clone(), pixels);
                        }
                        if !changed {
                            if view.edit_saving {
                                view.text_gesture = None;
                                view.edit_saving = false;
                            }
                            let board_id = board.id;
                            fileatlas_ui_bridge::refresh::poll(
                                cx,
                                std::time::Duration::from_millis(600),
                                move |view, cx| {
                                    if view.text_revision == revision {
                                        view.refresh_page_preview(board_id.clone(), cx);
                                    }
                                    fileatlas_ui_bridge::refresh::PollResult::until(false, false)
                                },
                            );
                        }
                    }
                    Err(e) => {
                        view.message = Some(e);
                    }
                }
                view.text_commit_active = false;
                if !changed && view.edit_saving {
                    view.edit_saving = false;
                    view.text_gesture = None;
                }
                cx.notify();
            });
        })
        .detach();
    }
}
