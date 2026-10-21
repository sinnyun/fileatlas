use super::*;
impl WorkCratePanel {
    pub(super) fn perform(
        &mut self,
        job: impl FnOnce() -> Result<(), String> + Send + 'static,
        cx: &mut ViewContext<Self>,
    ) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.message = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move { job() }).await;
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
    pub(super) fn perform_export(
        &mut self,
        job: impl FnOnce() -> Result<(), String> + Send + 'static,
        cx: &mut ViewContext<Self>,
    ) {
        self.perform(job, cx);
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
        let b = self.boards.clone();
        cx.spawn(async move |this, cx| {
            if let Some(file) = rfd::AsyncFileDialog::new().pick_file().await {
                let path = file.path().to_path_buf();
                let _ = this.update(cx, |v, cx| {
                    v.perform(
                        move || b.place_file(&id, &path).map_err(|e| e.to_string()),
                        cx,
                    )
                });
            }
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
        let Some(id) = self.selected_board.clone() else {
            return;
        };
        let b = self.boards.clone();
        self.perform(move || b.open_editor(&id).map_err(|e| e.to_string()), cx);
    }
}
