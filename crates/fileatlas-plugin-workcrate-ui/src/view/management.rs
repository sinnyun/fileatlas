use super::*;

impl WorkCratePanel {
    pub(super) fn add_references(&mut self, cx: &mut ViewContext<Self>) {
        let Some(project_id) = self.selected_project.clone() else {
            self.message = Some("请先创建项目".into());
            cx.notify();
            return;
        };
        let projects = Arc::clone(&self.projects);
        self.busy = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let picked = rfd::AsyncFileDialog::new()
                .pick_files()
                .await
                .map(|files| {
                    files
                        .into_iter()
                        .map(|file| file.path().to_path_buf())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let result = if picked.is_empty() {
                Ok(None)
            } else {
                cx.background_executor()
                    .spawn(async move {
                        projects
                            .add_references(&project_id, &picked)
                            .map_err(|error| error.to_string())
                    })
                    .await
                    .map(Some)
            };
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                match result {
                    Ok(Some(_)) => view.message = Some("已添加文件引用；原文件未复制".into()),
                    Ok(None) => {}
                    Err(error) => view.message = Some(error),
                };
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn export_current_image(&mut self, cx: &mut ViewContext<Self>) {
        let Some(id) = self.selected_board.clone() else {
            return;
        };
        let exports = Arc::clone(&self.exports);
        cx.spawn(async move |this, cx| {
            if let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter("PNG", &["png"])
                .add_filter("JPEG", &["jpg", "jpeg"])
                .set_file_name("page.png")
                .save_file()
                .await
            {
                let path = file.path().to_path_buf();
                let _ = this.update(cx, |view, cx| {
                    view.perform_export(
                        move || {
                            exports
                                .export_page_image(&id, &path)
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
    pub(super) fn perform(
        &mut self,
        job: impl FnOnce() -> Result<(), String> + Send + 'static,
        cx: &mut ViewContext<Self>,
    ) {
        self.message = None;
        self.busy = true;
        let page_id = self.selected_board.clone();
        let boards = Arc::clone(&self.boards);
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    job()?;
                    page_id
                        .map(|id| boards.board(&id).map(Some).map_err(|e| e.to_string()))
                        .unwrap_or(Ok(None))
                })
                .await;
            let _ =
                this.update(cx, |view, cx| {
                    view.busy = false;
                    match result {
                        Ok(board) => {
                            let previous = std::mem::replace(
                                &mut view.items,
                                view.projects.projects(true).unwrap_or_default(),
                            );
                            let membership_changed = previous.len() != view.items.len()
                                || previous
                                    .iter()
                                    .any(|old| !view.items.iter().any(|new| new.id == old.id));
                            let changed_covers = previous
                                .iter()
                                .filter_map(|old| {
                                    view.items
                                        .iter()
                                        .find(|new| new.id == old.id)
                                        .is_some_and(|new| new.main_image != old.main_image)
                                        .then(|| old.id.clone())
                                })
                                .collect::<std::collections::HashSet<_>>();
                            let live_projects = view
                                .items
                                .iter()
                                .map(|item| item.id.as_str())
                                .collect::<std::collections::HashSet<_>>();
                            view.project_covers
                                .retain(|id, _| live_projects.contains(id.as_str()));
                            for id in &changed_covers {
                                view.project_covers.remove(id);
                            }
                            view.editing = None;
                            if let Some(board) = board.filter(|board| {
                                view.selected_board.as_deref() == Some(board.id.as_str())
                            }) {
                                view.apply_board_update(board, cx);
                            } else if view.selected_project.as_ref().is_some_and(|id| {
                                !view.items.iter().any(|project| &project.id == id)
                            }) {
                                view.selected_project = None;
                                view.selected_board = None;
                                view.reload(cx);
                            } else if (membership_changed || !changed_covers.is_empty())
                                && view.selected_project.is_none()
                            {
                                view.load_project_covers(cx);
                            }
                        }
                        Err(error) => view.message = Some(error),
                    }
                    cx.notify();
                });
        })
        .detach();
    }

    pub(super) fn apply_board_update(&mut self, board: WorkCrateBoard, cx: &mut ViewContext<Self>) {
        let id = board.id.clone();
        let mut placements_changed = false;
        if let Some(card) = self.boards_view.iter_mut().find(|card| card.board.id == id) {
            for (asset, old) in &card.board.placements {
                let changed = board.placements.get(asset).is_none_or(|new| {
                    old.path != new.path
                        || old.sha256 != new.sha256
                        || old.reference_id != new.reference_id
                        || old.uses_preview != new.uses_preview
                });
                if changed {
                    card.originals.remove(asset);
                    card.sources.remove(asset);
                    placements_changed = true;
                }
            }
            placements_changed |= card.board.placements.len() != board.placements.len();
            card.board = board.clone();
        }
        if self
            .selected_text
            .as_ref()
            .is_some_and(|selected| !board.texts.contains_key(selected))
        {
            self.selected_text = None;
            self.text_editor = None;
            self.text_options = None;
            self.text_images.clear();
        }
        if self.selected_board.as_deref() == Some(id.as_str()) {
            if placements_changed
                && self
                    .items
                    .iter()
                    .find(|project| project.id == board.project_id)
                    .is_some_and(|project| project.main_image.is_none())
            {
                self.project_covers.remove(&board.project_id);
            }
            self.render_canvas(board, cx);
            if placements_changed {
                self.proxy_request = None;
                self.load_editor_images(cx);
            }
        }
        self.refresh_page_preview(id, cx);
    }

    pub(super) fn load_project_covers(&mut self, cx: &mut ViewContext<Self>) {
        let projects = Arc::clone(&self.projects);
        let boards = Arc::clone(&self.boards);
        let items = self
            .items
            .iter()
            .filter(|project| !self.project_covers.contains_key(&project.id))
            .cloned()
            .collect::<Vec<_>>();
        if items.is_empty() {
            return;
        }
        cx.spawn(async move |this, cx| {
            let covers = cx
                .background_executor()
                .spawn(async move {
                    items
                        .into_iter()
                        .filter_map(|p| {
                            let image = if let Some(cover) = p.main_image {
                                let path = projects.project_directory(&p.id).ok()?.join(cover.path);
                                image::open(path).ok()?.thumbnail(720, 480).into_rgba8()
                            } else {
                                let board = boards.board_catalog(&p.id).ok()?.into_iter().next()?;
                                let asset = board.placements.keys().next()?;
                                boards.display_image(&board, asset, 720).ok()?
                            };
                            Some((p.id, render_image(image)))
                        })
                        .collect()
                })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.project_covers = covers;
                cx.notify();
            });
        })
        .detach();
    }

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
        let Some((target, key)) = self.editing.clone() else {
            return;
        };
        let value = self.edit_input.read(cx).value().trim().to_owned();
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
        let boards = Arc::clone(&self.boards);
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
                    boards
                        .duplicate_project_pages(&id, &copy.id)
                        .map_err(|e| e.to_string())
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
                    if ids.iter().enumerate().any(|(index, _)| {
                        root.join(format!(
                            "page-{:02}.{}",
                            index + 1,
                            if jpeg { "jpg" } else { "png" }
                        ))
                        .exists()
                    }) {
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

    pub(super) fn image_action(&mut self, asset: String, action: &str, cx: &mut ViewContext<Self>) {
        if action == "edit-source" {
            self.open_image_source(&asset, cx);
            return;
        }
        let Some(id) = self.selected_board.clone() else {
            return;
        };
        let boards = Arc::clone(&self.boards);
        if action == "refresh" {
            self.perform(
                move || {
                    boards
                        .refresh_image(&id, &asset, None)
                        .map(|_| ())
                        .map_err(|e| e.to_string())
                },
                cx,
            );
            return;
        }
        let source = action == "link";
        cx.spawn(async move |this, cx| {
            let picker = rfd::AsyncFileDialog::new();
            let picker = if source {
                picker
            } else {
                picker.add_filter("图片", &["jpg", "jpeg", "png"])
            };
            if let Some(file) = picker.pick_file().await {
                let path = file.path().to_path_buf();
                let _ = this.update(cx, |view, cx| {
                    view.perform(
                        move || {
                            if source {
                                boards
                                    .link_source(
                                        &id,
                                        &asset,
                                        Some(path.to_string_lossy().into_owned()),
                                    )
                                    .map(|_| ())
                                    .map_err(|e| e.to_string())
                            } else {
                                boards
                                    .refresh_image(&id, &asset, Some(&path))
                                    .map(|_| ())
                                    .map_err(|e| e.to_string())
                            }
                        },
                        cx,
                    )
                });
            }
        })
        .detach();
    }
}
