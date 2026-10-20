use super::*;

impl WorkCratePanel {
    pub(super) fn restore_crop(&mut self, cx: &mut ViewContext<Self>) {
        let (Some(id), Some(asset)) = (self.selected_board.clone(), self.selected_image.clone())
        else {
            return;
        };
        let boards = Arc::clone(&self.boards);
        self.crop_mode = false;
        self.perform(
            move || {
                boards
                    .restore_image_crop(&id, &asset)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            },
            cx,
        );
    }

    pub(super) fn refresh_project_links(&mut self, cx: &mut ViewContext<Self>) {
        let Some(id) = self.selected_project.clone() else {
            return;
        };
        let boards = Arc::clone(&self.boards);
        self.busy = true;
        self.message = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    boards
                        .refresh_project_images(&id)
                        .map_err(|e| e.to_string())
                })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                view.message = Some(match result {
                    Ok(report) => format!(
                        "项目图片：已更新 {}，未变化 {}，失败 {}{}",
                        report.updated,
                        report.unchanged,
                        report.failures.len(),
                        if report.failures.is_empty() {
                            String::new()
                        } else {
                            format!("；{}", report.failures.join("；"))
                        }
                    ),
                    Err(error) => error,
                });
                view.reload(cx);
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn open_image_source(&mut self, asset: &str, cx: &mut ViewContext<Self>) {
        let path = self
            .boards_view
            .iter()
            .find(|c| Some(&c.board.id) == self.selected_board.as_ref())
            .and_then(|c| c.board.placements.get(asset))
            .and_then(|p| p.source_document.clone());
        let Some(path) = path else {
            self.message = Some("请先为图片设置扩展链接文件".into());
            cx.notify();
            return;
        };
        let operations = Arc::clone(&self.file_operations);
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    operations
                        .open_linked_file(std::path::Path::new(&path))
                        .map_err(|e| e.to_string())
                })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.message = Some(match result {
                    Ok(()) => "已打开扩展源文件；编辑导出覆盖图片后，点击更新项目图片".into(),
                    Err(error) => error,
                });
                cx.notify();
            });
        })
        .detach();
    }
}
