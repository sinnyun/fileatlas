use super::*;

impl WorkCratePanel {
    pub(super) fn relocate_project(&mut self, id: String, cx: &mut ViewContext<Self>) {
        let boards = self.boards.clone();
        self.manage_id = None;
        self.perform(
            move || {
                boards
                    .relocate_project(&id)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            },
            cx,
        );
    }

    pub(super) fn open_project_folder(&mut self, cx: &mut ViewContext<Self>) {
        let boards = self.boards.clone();
        cx.spawn(async move |this, cx| {
            let Some(folder) = rfd::AsyncFileDialog::new()
                .set_title("打开或重新定位 WorkCrate 项目文件夹")
                .pick_folder()
                .await
            else {
                return;
            };
            let folder = folder.path().to_owned();
            let _ = this.update(cx, |view, cx| {
                if view.busy || view.canvas_busy || !view.field_saves.is_empty() {
                    return;
                }
                view.clear_canvas_state();
                view.perform(
                    move || {
                        boards
                            .open_project_folder(&folder)
                            .map(|_| ())
                            .map_err(|e| e.to_string())
                    },
                    cx,
                );
            });
        })
        .detach();
    }
}
