use super::*;

impl WorkCratePanel {
    pub(super) fn place_image(&mut self, window: &mut Window, cx: &mut ViewContext<Self>) {
        if self.selected_board.is_none() || self.busy {
            return;
        }
        let dialog = rfd::AsyncFileDialog::new()
            .set_parent(window)
            .add_filter("图片文件", &["jpg", "jpeg", "png"]);
        cx.spawn(async move |this, cx| {
            let picked = dialog.pick_files().await;
            if let Some(files) = picked {
                let paths = files.into_iter().map(|f| f.path().to_path_buf()).collect();
                let _ = this.update(cx, |view, cx| view.import_images(paths, cx));
            }
        })
        .detach();
    }

    /// The visible picker and all drag receivers share the same ingestion path.
    pub(super) fn import_images(&mut self, paths: Vec<PathBuf>, cx: &mut ViewContext<Self>) {
        let (Some(project), Some(board)) =
            (self.selected_project.clone(), self.selected_board.clone())
        else {
            self.message = Some("请先打开需要添加图片的页面".into());
            cx.notify();
            return;
        };
        if self.busy {
            return;
        }
        let accepted: Vec<_> = paths
            .into_iter()
            .filter(|path| {
                path.extension().and_then(|v| v.to_str()).is_some_and(|v| {
                    matches!(v.to_ascii_lowercase().as_str(), "jpg" | "jpeg" | "png")
                })
            })
            .collect();
        if accepted.is_empty() {
            self.message = Some("画布仅支持 JPG / JPEG / PNG 图片".into());
            cx.notify();
            return;
        }
        let projects = Arc::clone(&self.projects);
        let boards = Arc::clone(&self.boards);
        self.perform(
            move || {
                for (index, raw) in accepted.iter().enumerate() {
                    let path = std::fs::canonicalize(raw).map_err(|e| e.to_string())?;
                    let references = projects
                        .add_references(&project, std::slice::from_ref(&path))
                        .map_err(|e| e.to_string())?;
                    let reference = references
                        .iter()
                        .find(|r| std::fs::canonicalize(&r.path).ok().as_ref() == Some(&path))
                        .ok_or_else(|| "未能添加图片引用".to_owned())?;
                    boards
                        .place_reference(
                            &board,
                            &reference.id,
                            24. + index as f64 * 18.,
                            72. + index as f64 * 18.,
                            420.,
                            false,
                        )
                        .map_err(|e| e.to_string())?;
                }
                Ok(())
            },
            cx,
        );
    }
}
