use super::*;
impl WorkCratePanel {
    pub(super) fn perform(
        &mut self,
        job: impl FnOnce() -> Result<(), String> + Send + 'static,
        cx: &mut ViewContext<Self>,
    ) {
        if self.busy || self.canvas_busy || !self.field_saves.is_empty() {
            return;
        }
        let boards = self.boards.clone();
        let close_canvas = self.canvas.is_some();
        if close_canvas {
            self.clear_canvas_state();
        }
        self.busy = true;
        self.message = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    if close_canvas && boards.canvas_is_open() {
                        boards.canvas_commit().map_err(|e| e.to_string())?;
                        boards.canvas_close().map_err(|e| e.to_string())?;
                    }
                    job()
                })
                .await;
            let _ = this.update(cx, |v, cx| {
                v.busy = false;
                match result {
                    Ok(()) => {
                        v.editing = None;
                        v.message = Some("操作完成".into());
                        v.reload(cx);
                    }
                    Err(e) => v.message = Some(e),
                }
                cx.notify();
            });
        })
        .detach();
    }
    /// Exports read the saved document; they produce no catalogue or card change, so
    /// they never run the reload path. Only an open canvas session is released first.
    pub(super) fn perform_export(
        &mut self,
        job: impl FnOnce() -> Result<(), String> + Send + 'static,
        cx: &mut ViewContext<Self>,
    ) {
        if self.busy || self.canvas_busy || !self.field_saves.is_empty() {
            return;
        }
        let boards = self.boards.clone();
        let close_canvas = self.canvas.is_some();
        if close_canvas {
            self.clear_canvas_state();
        }
        self.busy = true;
        self.message = Some("正在导出…".into());
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    if close_canvas && boards.canvas_is_open() {
                        boards.canvas_commit().map_err(|e| e.to_string())?;
                        boards.canvas_close().map_err(|e| e.to_string())?;
                    }
                    job()
                })
                .await;
            let _ = this.update(cx, |v, cx| {
                v.busy = false;
                v.message = Some(match result {
                    Ok(()) => "导出完成".into(),
                    Err(error) => format!("导出失败：{error}"),
                });
                cx.notify();
            });
        })
        .detach();
    }

    /// Save one page field. Unlike [`Self::perform`] this neither blocks the panel nor
    /// reloads the project: only the changed card and its preview are patched, every other
    /// page keeps its frame, and the next field can be drafted while this one is in flight.
    /// The official session is opened once and kept warm, so the first save pays for the
    /// process and every later field save reuses it; decoding the returned frame happens on
    /// the same background thread, never on the UI thread.
    pub(super) fn save_field(
        &mut self,
        id: String,
        key: String,
        value: String,
        label: String,
        cx: &mut ViewContext<Self>,
    ) {
        if self.busy
            || self.canvas_busy
            || !self.field_saves.is_empty()
            || !self.field_saves.insert((id.clone(), key.clone()))
        {
            return;
        }
        // Saving the page the canvas shows reuses its warm session. A save for any other
        // page cannot share it, because the overlay must keep displaying its own page.
        let reuse = self.canvas.as_ref().is_some_and(|s| s.board_id == id);
        let close_canvas = self.canvas.is_some() && !reuse;
        if close_canvas {
            self.clear_canvas_state();
        }
        self.message = None;
        cx.notify();
        let boards = self.boards.clone();
        let job_id = id.clone();
        let job_key = key.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    if close_canvas && boards.canvas_is_open() {
                        boards.canvas_commit().map_err(|e| e.to_string())?;
                        boards.canvas_close().map_err(|e| e.to_string())?;
                    }
                    let warm =
                        boards.canvas_is_open() && boards.canvas_holds(&job_id).unwrap_or(false);
                    if !warm {
                        boards
                            .canvas_open_quiet(&job_id)
                            .map_err(|e| e.to_string())?;
                    }
                    let mut board = boards.board(&job_id).map_err(|e| e.to_string())?;
                    super::management::apply_field(&mut board, &job_key, value)?;
                    let mut save = match boards.update_info(&board.id, &board.title, board.info) {
                        Ok(save) => save,
                        Err(error) => {
                            if !warm {
                                let _ = boards.canvas_close();
                            }
                            return Err(error.to_string());
                        }
                    };
                    if !save.changed {
                        if !warm {
                            let _ = boards.canvas_close();
                        }
                        return Ok((save, None));
                    }
                    let frame = save
                        .frame
                        .take()
                        .map(|snapshot| super::canvas::canvas_frame(&boards, snapshot));
                    Ok((save, frame))
                })
                .await;
            let _ = this.update_in(cx, |view: &mut WorkCratePanel, window, cx| {
                view.field_saves.remove(&(id.clone(), key.clone()));
                match result {
                    Ok((save, frame)) => {
                        if view.editing.as_ref() == Some(&(id.clone(), key.clone())) {
                            view.editing = None;
                        }
                        view.apply_field_save(save, frame, window, cx);
                        view.message = Some(format!("已保存：{label}"));
                    }
                    Err(error) => view.message = Some(format!("保存失败：{error}")),
                }
                // A page picked while this save was in flight could not switch then.
                view.switch_canvas_page(cx);
                cx.notify();
            });
        })
        .detach();
    }

    /// Patch exactly the card a field save changed. The official frame is already decoded on
    /// the background thread; in preview mode it also becomes the card image, so the page is
    /// not rendered or queued a second time and no other card is touched.
    fn apply_field_save(
        &mut self,
        save: fileatlas_plugin_workcrate_boards::InfoSave,
        frame: Option<super::canvas::CanvasFrame>,
        window: &mut Window,
        cx: &mut ViewContext<Self>,
    ) {
        let fileatlas_plugin_workcrate_boards::InfoSave {
            board,
            frame: _,
            changed,
        } = save;
        let id = board.id.clone();
        if let Some(card) = self.boards_view.iter_mut().find(|c| c.board.id == id) {
            card.board = board;
            if frame.is_none() && changed {
                card.error = None;
                card.canvas_revision = None;
            }
        }
        if let Some(frame) = frame {
            self.apply_canvas_frame(frame, window, cx);
            // A save leaves the official session warm in preview mode: keep watching it so
            // an edit made through the official program still reaches this page.
            self.start_canvas_poller(cx);
            return;
        }
        if !changed {
            return;
        }
        if !self.preview_queue.contains(&id) {
            self.preview_queue.push_front(id);
        }
        if self.preview_current.is_none() {
            self.start_preview();
        }
    }
    pub(super) fn create_project(&mut self, cx: &mut ViewContext<Self>) {
        let picked = cx.spawn(async move |this, cx| {
            let Some(folder) = rfd::AsyncFileDialog::new().pick_folder().await else {
                return;
            };
            let directory = folder.path().to_path_buf();
            let _ = this.update(cx, |view, cx| {
                let projects = view.projects.clone();
                let boards = view.boards.clone();
                view.perform(
                    move || {
                        let names = projects.projects(true).map_err(|e| e.to_string())?;
                        let mut i = 1;
                        let name = loop {
                            let candidate = format!("新建项目 {i}");
                            if !names.iter().any(|project| project.name == candidate) {
                                break candidate;
                            }
                            i += 1;
                        };
                        let project = projects.create_project(&name).map_err(|e| e.to_string())?;
                        if let Err(error) = boards.create_project_document(&project.id, &directory)
                        {
                            return match projects.delete_project(&project.id) {
                                Ok(_) => Err(error.to_string()),
                                Err(cleanup) => {
                                    Err(format!("{error}; 清理未完成创建的项目记录失败：{cleanup}"))
                                }
                            };
                        }
                        Ok(())
                    },
                    cx,
                )
            });
        });
        picked.detach();
    }
    pub(super) fn create_board(&mut self, cx: &mut ViewContext<Self>) {
        let Some(id) = self.selected_project.clone() else {
            return;
        };
        let b = self.boards.clone();
        self.perform(
            move || b.create_page(&id).map(|_| ()).map_err(|e| e.to_string()),
            cx,
        );
    }
    pub(super) fn pick_document(&mut self, cx: &mut ViewContext<Self>) {
        let Some(id) = self.selected_project.clone() else {
            return;
        };
        let b = self.boards.clone();
        cx.spawn(async move |this, cx| {
            if let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter("VectorCraft 工程", &["vectorcraft"])
                .pick_file()
                .await
            {
                let path = file.path().to_path_buf();
                let _ = this.update(cx, |v, cx| {
                    v.perform(
                        move || b.attach_document(&id, &path).map_err(|e| e.to_string()),
                        cx,
                    )
                });
            }
        })
        .detach();
    }
    pub(super) fn place_file(&mut self, cx: &mut ViewContext<Self>) {
        let Some(id) = self.selected_board.clone() else {
            return;
        };
        cx.spawn(async move |this, cx| {
            if let Some(file) = rfd::AsyncFileDialog::new().pick_file().await {
                let path = file.path().to_path_buf();
                let _ = this.update(cx, |v, cx| {
                    if v.selected_board.as_deref() != Some(id.as_str()) {
                        return;
                    }
                    v.place_selected_file(id.clone(), path, cx);
                });
            }
        })
        .detach();
    }

    /// Place one file into the selected page and patch only that card. The document is
    /// written page by page, so no other card, preview or the project list is reloaded.
    fn place_selected_file(&mut self, id: String, path: PathBuf, cx: &mut ViewContext<Self>) {
        if self.busy || self.canvas_busy || !self.field_saves.is_empty() {
            return;
        }
        let boards = self.boards.clone();
        let close_canvas = self.canvas.is_some();
        if close_canvas {
            self.clear_canvas_state();
        }
        self.busy = true;
        self.message = Some("正在放置文件…".into());
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    if close_canvas && boards.canvas_is_open() {
                        boards.canvas_commit().map_err(|e| e.to_string())?;
                        boards.canvas_close().map_err(|e| e.to_string())?;
                    }
                    boards.place_file(&id, &path).map_err(|e| e.to_string())?;
                    boards.board(&id).map_err(|e| e.to_string())
                })
                .await;
            let _ = this.update(cx, |v, cx| {
                v.busy = false;
                match result {
                    Ok(board) => {
                        if let Some(card) = v
                            .boards_view
                            .iter_mut()
                            .find(|card| card.board.id == board.id)
                        {
                            card.board = board;
                            card.error = None;
                            card.canvas_revision = None;
                            if !v.preview_queue.contains(&card.board.id) {
                                v.preview_queue.push_front(card.board.id.clone());
                            }
                        }
                        if v.preview_current.is_none() {
                            v.start_preview();
                        }
                        v.message = Some("已放置文件".into());
                    }
                    Err(error) => v.message = Some(format!("放置失败：{error}")),
                }
                cx.notify();
            });
        })
        .detach();
    }
    pub(super) fn sync_project(&mut self, cx: &mut ViewContext<Self>) {
        let Some(id) = self.selected_project.clone() else {
            return;
        };
        let b = self.boards.clone();
        self.perform(
            move || {
                b.refresh_project(&id)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            },
            cx,
        );
    }
    pub(super) fn open_editor(&mut self, cx: &mut ViewContext<Self>) {
        self.canvas_handoff_editor(cx);
    }
}
