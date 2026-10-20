use super::*;
impl SidebarView {
    pub(in crate::view) fn render_quick_access_section(&mut self, cx: &mut Context<Self>) -> Div {
        let collapsed = self.collapsed_sections.contains("quick_access");
        let weak = cx.entity().downgrade();
        let config = self.sidebar_config();
        let browse = self.service.browser_enabled();
        let selected_path = self
            .session
            .snapshot()
            .ok()
            .and_then(|saved| saved.selected_path)
            .clone();
        let add_view = weak.clone();
        let mut section = div().v_flex().child(section_header(
            &weak,
            "quick_access",
            "Quick access",
            collapsed,
            move |menu| {
                let add_menu = add_view.clone();
                menu.item(menu_item("Add folder", &add_menu, |this, window, cx| {
                    this.open_sidebar_editor(SidebarEditor::AddQuickAccess, "", window, cx);
                }))
            },
            vec![icon_button(
                "navigation-quick-add".into(),
                IconName::Plus,
                |this, cx| {
                    this.picker_editor = Some(SidebarEditor::AddQuickAccess);
                    this.pick_folder(cx);
                },
                cx,
            )],
        ));
        if collapsed {
            return section;
        }
        let system = self
            .system_folders_read
            .value()
            .and_then(|r| r.as_ref().ok())
            .cloned()
            .unwrap_or_default();
        for (label, path) in system {
            let path = path.to_string_lossy().into_owned();
            let expanded = self.expanded_directories.contains_key(&path);
            let selected = selected_path
                .as_deref()
                .is_some_and(|selected| crate::sidebar_folders::same_sidebar_path(selected, &path));
            let row_view = weak.clone();
            let menu_view = row_view.clone();
            let row_path = path.clone();
            let display = fileatlas_ui_bridge::i18n::tr(&label);
            section = section.child(path_row(
                &row_view,
                format!("quick-sys-{}", label.to_lowercase()),
                display,
                IconName::Folder,
                path.clone(),
                expanded,
                selected,
                browse,
                move |menu| {
                    let open_view = menu_view.clone();
                    let open_path = row_path.clone();
                    let add_view = menu_view.clone();
                    let add_path = row_path.clone();
                    menu.item(menu_item(
                        "Add to folder bookmarks",
                        &add_view,
                        move |this, _, cx| {
                            this.add_folder_path(add_path.clone(), cx);
                        },
                    ))
                    .item(menu_item(
                        "Open in file manager",
                        &open_view,
                        move |this, _, _| {
                            this.open_folder_in_explorer(&open_path);
                        },
                    ))
                },
            ));
            if expanded {
                if let Some(column) = children_column(self, &path, cx) {
                    section = section.child(column);
                }
            }
        }
        for entry in config.quick_access.clone() {
            let display = entry.display_name.clone().unwrap_or_else(|| {
                Path::new(&entry.path)
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| entry.path.clone())
            });
            let expanded = self.expanded_directories.contains_key(&entry.path);
            let selected = selected_path.as_deref().is_some_and(|selected| {
                crate::sidebar_folders::same_sidebar_path(selected, &entry.path)
            });
            let row_view = weak.clone();
            let menu_view = row_view.clone();
            let row_path = entry.path.clone();
            let rename_view = weak.clone();
            let rename_path = entry.path.clone();
            let remove_view = weak.clone();
            let remove_path = entry.path.clone();
            section = section.child(path_row(
                &row_view,
                format!("quick-user-{}", entry.path),
                display,
                IconName::Folder,
                entry.path.clone(),
                expanded,
                selected,
                browse,
                move |menu| {
                    let open_view = menu_view.clone();
                    let open_path = row_path.clone();
                    let add_view = menu_view.clone();
                    let add_path = row_path.clone();
                    let rename_view = rename_view.clone();
                    let rename_path = rename_path.clone();
                    let remove_view = remove_view.clone();
                    let remove_path = remove_path.clone();
                    let menu = menu.item(menu_item(
                        "Add to folder bookmarks",
                        &add_view,
                        move |this, _, cx| {
                            this.add_folder_path(add_path.clone(), cx);
                        },
                    ));
                    let menu = menu.item(menu_item(
                        "Open in file manager",
                        &open_view,
                        move |this, _, _| {
                            this.open_folder_in_explorer(&open_path);
                        },
                    ));
                    let menu = menu.item(menu_item(
                        "Rename",
                        &rename_view,
                        move |this, window, cx| {
                            let initial = this
                                .sidebar_config()
                                .quick_access
                                .iter()
                                .find(|item| item.path == rename_path)
                                .and_then(|item| item.display_name.clone())
                                .unwrap_or_default();
                            this.open_sidebar_editor(
                                SidebarEditor::RenameQuickAccess {
                                    path: rename_path.clone(),
                                },
                                &initial,
                                window,
                                cx,
                            );
                        },
                    ));
                    menu.item(menu_item(
                        "Remove from quick access",
                        &remove_view,
                        move |this, _, cx| {
                            this.remove_quick_access(remove_path.clone(), cx);
                        },
                    ))
                },
            ));
            if expanded {
                if let Some(column) = children_column(self, &entry.path, cx) {
                    section = section.child(column);
                }
            }
        }
        section
    }
}
