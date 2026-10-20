use super::*;

pub(super) type DisplayImages = Vec<(String, Arc<RenderImage>, Arc<RenderImage>)>;
#[derive(Clone)]
pub(super) struct DisplayRequest {
    board: WorkCrateBoard,
    targets: Vec<(String, u32)>,
    key: String,
}

impl WorkCratePanel {
    pub(super) fn reload(&mut self, cx: &mut ViewContext<Self>) {
        self.load_generation += 1;
        self.proxy_request = None;
        let generation = self.load_generation;
        let Some(project) = self.selected_project.clone() else {
            self.busy = false;
            self.boards_view.clear();
            self.load_project_covers(cx);
            cx.notify();
            return;
        };
        let boards = self.boards.clone();
        self.busy = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let models = boards.board_catalog(&project).map_err(|e| e.to_string())?;
                    Ok::<_, String>(
                        models
                            .into_iter()
                            .map(|board| {
                                let image = boards.cached_preview(&board).map(render_image);
                                BoardCard {
                                    board,
                                    image,
                                    error: None,
                                    sources: Default::default(),
                                    originals: Default::default(),
                                }
                            })
                            .collect::<Vec<_>>(),
                    )
                })
                .await;
            let _ = this.update(cx, |view, cx| {
                if view.load_generation != generation {
                    return;
                }
                view.busy = false;
                match result {
                    Ok(cards) => {
                        view.preview_hits = cards.iter().filter(|c| c.image.is_some()).count();
                        let missing = cards
                            .iter()
                            .filter(|c| c.image.is_none())
                            .map(|c| c.board.clone())
                            .collect();
                        view.boards_view = cards;
                        if let Some(board) = view
                            .boards_view
                            .iter()
                            .find(|c| Some(&c.board.id) == view.selected_board.as_ref())
                            .map(|c| c.board.clone())
                        {
                            view.render_canvas(board, cx);
                            view.load_editor_images(cx);
                        }
                        view.fill_previews(missing, generation, cx);
                    }
                    Err(e) => view.message = Some(e),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn fill_previews(
        &self,
        models: Vec<WorkCrateBoard>,
        generation: u64,
        cx: &mut ViewContext<Self>,
    ) {
        let boards = self.boards.clone();
        cx.spawn(async move |this, cx| {
            // One bounded worker. Publish each page independently, so a large project can
            // be opened and edited before its disposable cache has finished rebuilding.
            for board in models {
                let id = board.id.clone();
                let revision = boards.preview_revision(&board).ok();
                let service = boards.clone();
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        service
                            .cache_preview(&board)
                            .map(render_image)
                            .map_err(|e| e.to_string())
                    })
                    .await;
                let active = this
                    .update(cx, |view, cx| {
                        if view.load_generation != generation {
                            return false;
                        }
                        if let Some(card) = view.boards_view.iter_mut().find(|c| c.board.id == id) {
                            if view.boards.preview_revision(&card.board).ok() != revision {
                                return true;
                            }
                            match result {
                                Ok(image) => card.image = Some(image),
                                Err(e) => card.error = Some(e),
                            }
                            cx.notify();
                        }
                        true
                    })
                    .unwrap_or(false);
                if !active {
                    break;
                }
            }
        })
        .detach();
    }

    pub(super) fn load_editor_images(&mut self, _: &mut ViewContext<Self>) {
        let Some(board) = self
            .boards_view
            .iter()
            .find(|c| Some(&c.board.id) == self.selected_board.as_ref())
            .map(|c| c.board.clone())
        else {
            return;
        };
        let scale = self.fit_scale * self.device_scale;
        let targets = board
            .placements
            .iter()
            .map(|(id, p)| {
                let c = self
                    .gesture
                    .as_ref()
                    .filter(|g| &g.asset == id)
                    .map(|g| g.current.content)
                    .or_else(|| board.image_layout(id).ok().map(|l| l.content))
                    .unwrap_or([0., 0., p.placed_width_pt, p.placed_width_pt]);
                let side = (((c[2] - c[0]).max(c[3] - c[1]) * scale).ceil() as u32)
                    .max(128)
                    .next_power_of_two();
                (id.clone(), side)
            })
            .collect::<Vec<_>>();
        let key = format!(
            "{}:{targets:?}",
            self.boards.preview_revision(&board).unwrap_or_default()
        );
        if self.proxy_request.as_ref() == Some(&key) {
            return;
        }
        self.proxy_request = Some(key.clone());
        let request = DisplayRequest {
            board,
            targets,
            key,
        };
        self.display_request = Some(request.clone());
        self.request_display(request, true);
    }

    fn request_display(&mut self, request: DisplayRequest, coarse: bool) {
        let boards = self.boards.clone();
        let key = (self.load_generation, request.key, coarse);
        self.proxy_model.request(key, move || {
            request
                .targets
                .into_iter()
                .map(|(asset, side)| {
                    let pixels = boards
                        .display_image(
                            &request.board,
                            &asset,
                            if coarse { side.min(512) } else { side },
                        )
                        .map_err(|e| e.to_string())?;
                    let small = render_image(
                        image::DynamicImage::ImageRgba8(pixels.clone())
                            .thumbnail(120, 120)
                            .into_rgba8(),
                    );
                    Ok((asset, render_image(pixels), small))
                })
                .collect::<Result<DisplayImages, String>>()
        });
    }

    pub(super) fn deliver_display(&mut self, _: &mut ViewContext<Self>) -> bool {
        if !self.proxy_model.poll() {
            return false;
        }
        let Some(request) = self.display_request.clone() else {
            return false;
        };
        let Some((generation, key, coarse)) = self.proxy_model.key().cloned() else {
            return false;
        };
        if generation != self.load_generation
            || key != request.key
            || self.selected_board.as_ref() != Some(&request.board.id)
        {
            return false;
        }
        let Some(result) = self.proxy_model.value().cloned() else {
            return false;
        };
        let succeeded = result.is_ok();
        if let Some(card) = self
            .boards_view
            .iter_mut()
            .find(|c| c.board.id == request.board.id)
        {
            match result {
                Ok(images) => {
                    card.error = None;
                    for (asset, pixels, small) in images {
                        let existing = card
                            .originals
                            .get(&asset)
                            .and_then(|i| i.as_bytes(0))
                            .map_or(0, |b| b.len());
                        let incoming = pixels.as_bytes(0).map_or(0, |b| b.len());
                        if !coarse || existing < incoming {
                            card.originals.insert(asset.clone(), pixels);
                        }
                        card.sources.insert(asset, small);
                    }
                }
                Err(e) => card.error = Some(e),
            }
        }
        if coarse && succeeded {
            self.request_display(request, false);
        }
        true
    }
}
