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
        let p = self.projects.clone();
        self.perform(
            move || {
                let names = p.projects(true).map_err(|e| e.to_string())?;
                let mut i = 1;
                let name = loop {
                    let s = format!("新建项目 {i}");
                    if !names.iter().any(|p| p.name == s) {
                        break s;
                    }
                    i += 1;
                };
                p.create_project(&name)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            },
            cx,
        );
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
    pub(super) fn connect(&mut self, cx: &mut ViewContext<Self>) {
        let b = self.boards.clone();
        let s = self.ui_state.clone();
        let port = self.port_input.read(cx).value().parse::<u16>();
        let Ok(port) = port else {
            self.message = Some("端口应为 1–65535".into());
            cx.notify();
            return;
        };
        let mut config = self.config.clone();
        config.port = port;
        self.perform(
            move || {
                b.configure(config).map_err(|e| e.to_string())?;
                let result = b.connect();
                s.set_connection(result.is_ok());
                result.map(|_| ()).map_err(|e| e.to_string())
            },
            cx,
        );
    }
    pub(super) fn pick_executable(&mut self, cx: &mut ViewContext<Self>) {
        let b = self.boards.clone();
        let config = self.config.clone();
        let state = self.ui_state.clone();
        cx.spawn(async move |this, cx| {
            if let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter("VectorCraft 程序", &["exe"])
                .pick_file()
                .await
            {
                let mut config = config;
                config.executable = file.path().to_string_lossy().into_owned();
                let _ = this.update(cx, |v, cx| {
                    v.perform(
                        move || {
                            b.configure(config).map_err(|e| e.to_string())?;
                            state.set_connection(false);
                            Ok(())
                        },
                        cx,
                    )
                });
            }
        })
        .detach();
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
    pub(super) fn save_project(&mut self, cx: &mut ViewContext<Self>) {
        let Some(id) = self.selected_project.clone() else {
            return;
        };
        let b = self.boards.clone();
        self.perform(move || b.save_project(&id).map_err(|e| e.to_string()), cx);
    }
    pub(super) fn open_editor(&mut self, cx: &mut ViewContext<Self>) {
        let Some(id) = self.selected_board.clone() else {
            return;
        };
        let b = self.boards.clone();
        self.perform(move || b.open_editor(&id).map_err(|e| e.to_string()), cx);
    }
}
