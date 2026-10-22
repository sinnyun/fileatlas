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
        self.edit_input.update(cx, |input, cx| {
            input.set_value(value, window, cx);
            input.focus(window, cx);
        });
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
        if key == "project" {
            self.save_project_name(target, value, cx);
            return;
        }
        let label = match key.strip_prefix("extra:") {
            Some(name) => name.to_owned(),
            None if key == "new" => format!("字段 {value}"),
            None => field_label(&key).to_owned(),
        };
        self.save_field(target, key, value, label, cx);
    }

    /// A global project rename writes only the project index. The page document, its
    /// cards and the warm official session are unaffected, so it must not run the
    /// create/sync path that closes the canvas and reloads the whole catalogue.
    fn save_project_name(&mut self, id: String, name: String, cx: &mut ViewContext<Self>) {
        if self.busy || self.canvas_busy {
            return;
        }
        self.editing = None;
        self.message = Some("正在保存项目名称…".into());
        cx.notify();
        let projects = Arc::clone(&self.projects);
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    projects
                        .rename_project(&id, &name)
                        .map_err(|e| e.to_string())
                })
                .await;
            let _ = this.update(cx, |view: &mut WorkCratePanel, cx| {
                match result {
                    Ok(project) => {
                        if let Some(item) = view.items.iter_mut().find(|item| item.id == project.id)
                        {
                            *item = project;
                        }
                        view.message = Some("已保存：项目名称".into());
                    }
                    Err(error) => view.message = Some(format!("保存失败：{error}")),
                }
                cx.notify();
            });
        })
        .detach();
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
            "duplicate" => {
                let Some(original) = item else {
                    self.message = Some("项目不存在".into());
                    cx.notify();
                    return;
                };
                let project_name = original.name;
                let boards = self.boards.clone();
                cx.spawn(async move |this, cx| {
                    let Some(folder) = rfd::AsyncFileDialog::new().pick_folder().await else {
                        return;
                    };
                    let directory = folder.path().to_path_buf();
                    let _ = this.update(cx, |view, cx| {
                        let projects = projects.clone();
                        let boards = boards.clone();
                        view.perform(
                            move || {
                                let names = projects.projects(true).map_err(|e| e.to_string())?;
                                let mut name = format!("{project_name} 副本");
                                let mut suffix = 2;
                                while names.iter().any(|project| project.name == name) {
                                    name = format!("{project_name} 副本 {suffix}");
                                    suffix += 1;
                                }
                                let copy = projects
                                    .duplicate_project_document(&id, &name, &directory)
                                    .map_err(|e| e.to_string())?;
                                boards
                                    .refresh_project(&copy.id)
                                    .map(|_| ())
                                    .map_err(|error| {
                                        format!("副本已创建，但同步画板与预览失败：{error}")
                                    })
                            },
                            cx,
                        )
                    });
                })
                .detach();
            }
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

/// Apply one inspector field draft to a freshly read page record.
pub(super) fn apply_field(
    board: &mut WorkCrateBoard,
    key: &str,
    value: String,
) -> Result<(), String> {
    match key {
        "local" => {
            board.info.material_category = value;
            board.info.project_name_override = None;
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
            board.info.extra.insert(key[6..].to_owned(), value);
        }
        _ => return Err("未知页面字段".into()),
    }
    Ok(())
}

/// The Chinese field name shown in the save feedback for one inspector key.
pub(super) fn field_label(key: &str) -> &'static str {
    match key {
        "local" => "物料类别",
        "project" => "项目名称",
        "material" => "物料名称",
        "size" => "尺寸",
        "quantity" => "数量",
        _ => "页面字段",
    }
}

#[cfg(test)]
mod tests {
    use super::apply_field;
    use fileatlas_plugin_workcrate_boards::{PageInfo, WorkCrateBoard};

    fn board() -> WorkCrateBoard {
        WorkCrateBoard {
            id: "p:binding:1".into(),
            project_id: "p".into(),
            native_id: 1,
            index: 0,
            title: "页面".into(),
            width_pt: 420. * 72. / 25.4,
            height_pt: 297. * 72. / 25.4,
            info: PageInfo::default(),
        }
    }

    #[test]
    fn material_category_is_independent_and_can_be_cleared() {
        let mut named = board();
        assert_eq!(apply_field(&mut named, "local", "本页名称".into()), Ok(()));
        assert_eq!(named.info.material_category.as_str(), "本页名称");
        let mut inherited = named;
        assert_eq!(apply_field(&mut inherited, "local", String::new()), Ok(()));
        assert_eq!(inherited.info.material_category, "");
        assert_eq!(inherited.info.project_name_override, None);
    }

    #[test]
    fn a_new_field_starts_empty_and_a_blank_name_is_refused() {
        let mut added = board();
        assert_eq!(apply_field(&mut added, "new", "工艺".into()), Ok(()));
        assert_eq!(added.info.extra["工艺"], "");
        assert!(apply_field(&mut board(), "new", "  ".trim().to_owned()).is_err());
    }

    #[test]
    fn an_extra_edit_keeps_the_other_fields_and_the_page_name() {
        let mut edited = board();
        edited.title = "页面01".into();
        assert!(apply_field(&mut edited, "title", "手写标题".into()).is_err());
        assert!(apply_field(&mut edited, "physical", "A3".into()).is_err());
        assert_eq!(apply_field(&mut edited, "material", "纸板".into()), Ok(()));
        assert_eq!(
            apply_field(&mut edited, "extra:工艺", "切割".into()),
            Ok(())
        );
        assert_eq!(edited.title, "页面01");
        assert_eq!(edited.info.material, "纸板");
        assert_eq!(edited.info.extra["工艺"], "切割");
    }

    #[test]
    fn an_unknown_field_is_refused_rather_than_saved() {
        assert_eq!(
            apply_field(&mut board(), "mystery", "x".into()).unwrap_err(),
            "未知页面字段"
        );
    }
}
