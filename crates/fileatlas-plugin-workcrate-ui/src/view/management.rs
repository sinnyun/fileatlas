use super::*;
impl WorkCratePanel {
    pub(super) fn begin_edit(
        &mut self,
        target: String,
        key: &str,
        value: &str,
        window: &mut Window,
        cx: &mut ViewContext<Self>,
    ) {
        self.edit_input
            .update(cx, |input, cx| input.set_value(value, window, cx));
        self.editing = Some((target, key.into()));
        cx.notify();
    }

    pub(super) fn save_edit(&mut self, cx: &mut ViewContext<Self>) {
        let value = self.edit_input.read(cx).value().trim().to_owned();
        self.save_value(value, cx);
    }
    pub(super) fn save_value(&mut self, value: String, cx: &mut ViewContext<Self>) {
        let Some((target, key)) = self.editing.clone() else {
            return;
        };
        let projects = Arc::clone(&self.projects);
        let boards = Arc::clone(&self.boards);
        self.perform(
            move || {
                if key == "project" {
                    projects
                        .rename_project(&target, &value)
                        .map_err(|e| e.to_string())?;
                    return Ok(());
                }
                let mut board = boards.board(&target).map_err(|e| e.to_string())?;
                match key.as_str() {
                    "title" => {
                        board.title = value;
                        board.info.auto_from_first = false;
                    }
                    "physical" => board.info.size_label = value,
                    "local" => {
                        board.info.project_name_override = (!value.is_empty()).then_some(value)
                    }
                    "material" => board.info.material = value,
                    "size" => board.info.size = value,
                    "quantity" => board.info.quantity = value,
                    "new" => {
                        if value.is_empty() {
                            return Err("字段名称不能为空".into());
                        }
                        board.info.extra.insert(value, String::new());
                    }
                    key if key.starts_with("extra:") => {
                        board.info.extra.insert(key[6..].into(), value);
                    }
                    _ => return Err("未知页面字段".into()),
                }
                boards
                    .update_info(&target, &board.title, board.info)
                    .map_err(|e| e.to_string())?;
                Ok(())
            },
            cx,
        );
    }

    pub(super) fn project_action(&mut self, id: String, action: &str, cx: &mut ViewContext<Self>) {
        let projects = Arc::clone(&self.projects);
        let item = self.items.iter().find(|p| p.id == id).cloned();
        match action {
            "archive" => self.perform(
                move || {
                    projects
                        .set_archived(&id, !item.ok_or("项目不存在")?.archived)
                        .map_err(|e| e.to_string())?;
                    Ok(())
                },
                cx,
            ),
            "duplicate" => self.perform(
                move || {
                    let original = item.ok_or("项目不存在")?;
                    let names = projects.projects(true).map_err(|e| e.to_string())?;
                    let mut name = format!("{} 副本", original.name);
                    let mut suffix = 2;
                    while names.iter().any(|p| p.name == name) {
                        name = format!("{} 副本 {suffix}", original.name);
                        suffix += 1;
                    }
                    let copy = projects
                        .duplicate_project(&id, &name)
                        .map_err(|e| e.to_string())?;
                    let _ = copy;
                    Ok(())
                },
                cx,
            ),
            "cover" => {
                let thumbnails = Arc::clone(&self.thumbnails);
                cx.spawn(async move |this, cx| {
                    let picked = rfd::AsyncFileDialog::new()
                        .add_filter("项目主图", &["jpg", "jpeg", "png"])
                        .pick_file()
                        .await;
                    if let Some(file) = picked {
                        let path = file.path().to_path_buf();
                        let _ = this.update(cx, |view, cx| {
                            view.perform(
                                move || {
                                    projects
                                        .set_main_image(&id, &path, &thumbnails)
                                        .map_err(|e| e.to_string())
                                },
                                cx,
                            )
                        });
                    }
                })
                .detach();
            }
            "delete" => {
                cx.spawn(async move |this,cx| {
                    let response=rfd::AsyncMessageDialog::new().set_title("删除项目").set_description("移除项目及页面？项目数据会保留在 WorkCrate 回收目录，链接的原始文件不受影响。").set_buttons(rfd::MessageButtons::OkCancel).show().await;
                    if response==rfd::MessageDialogResult::Ok {let _=this.update(cx,|view,cx|view.perform(move ||projects.delete_project(&id).map(|_|()).map_err(|e|e.to_string()),cx));}
                }).detach();
            }
            _ => {}
        }
    }

    pub(super) fn export_catalog(&mut self, images: bool, cx: &mut ViewContext<Self>) {
        let jpeg = self.image_export_jpeg;
        let ids = self
            .boards_view
            .iter()
            .filter(|card| !images || self.selected_pages.contains(&card.board.id))
            .map(|card| card.board.id.clone())
            .collect::<Vec<_>>();
        if ids.is_empty() {
            self.message = Some("请先选择页面".into());
            cx.notify();
            return;
        }
        let exports = Arc::clone(&self.exports);
        cx.spawn(async move |this, cx| {
            if images {
                if let Some(folder) = rfd::AsyncFileDialog::new().pick_folder().await {
                    let root = folder.path().to_path_buf();
                    let check_ids = ids.clone();
                    let check_root = root.clone();
                    let exists = cx
                        .background_executor()
                        .spawn(async move {
                            check_ids.iter().enumerate().any(|(index, _)| {
                                check_root
                                    .join(format!(
                                        "page-{:02}.{}",
                                        index + 1,
                                        if jpeg { "jpg" } else { "png" }
                                    ))
                                    .exists()
                            })
                        })
                        .await;
                    if exists {
                        let response = rfd::AsyncMessageDialog::new()
                            .set_title("替换已存在的导出图片")
                            .set_description("目标目录中已有同名图片，继续将替换这些导出文件。")
                            .set_buttons(rfd::MessageButtons::OkCancel)
                            .show()
                            .await;
                        if response != rfd::MessageDialogResult::Ok {
                            return;
                        }
                    }
                    let _ = this.update(cx, |view, cx| {
                        view.perform_export(
                            move || {
                                let paths = ids
                                    .iter()
                                    .enumerate()
                                    .map(|(index, _)| {
                                        root.join(format!(
                                            "page-{:02}.{}",
                                            index + 1,
                                            if jpeg { "jpg" } else { "png" }
                                        ))
                                    })
                                    .collect::<Vec<_>>();
                                exports
                                    .export_pages_images(&ids, &paths)
                                    .map(|_| ())
                                    .map_err(|e| e.to_string())
                            },
                            cx,
                        )
                    });
                }
            } else if let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter("PDF", &["pdf"])
                .set_file_name("project.pdf")
                .save_file()
                .await
            {
                let path = file.path().to_path_buf();
                let _ = this.update(cx, |view, cx| {
                    view.perform_export(
                        move || {
                            exports
                                .export_pages_pdf(&ids, &path)
                                .map(|_| ())
                                .map_err(|e| e.to_string())
                        },
                        cx,
                    )
                });
            }
        })
        .detach();
    }
}
