use super::*;
use gpui_kit::component::Disableable as _;
impl SidebarView {
    /// Independent sibling frames, sharing header geometry with drives/quick access.
    pub(in crate::view) fn render_favorite_groups(
        &mut self,
        mut region: Div,
        cx: &mut Context<Self>,
    ) -> Div {
        let weak = cx.entity().downgrade();
        let config = self.sidebar_config();
        let browse = self.service.browser_enabled();
        let selected_path = self
            .session
            .snapshot()
            .ok()
            .and_then(|saved| saved.selected_path);
        if let Ok(mut headers) = self.favorite_header_bounds.write() {
            headers.retain(|id, _| config.favorite_groups.iter().any(|group| &group.id == id));
        }
        for group in config.favorite_groups {
            let group_collapsed = self.collapsed_groups.contains(&group.id);
            let group_view = weak.clone();
            let group_id = group.id.clone();
            let menu_view = weak.clone();
            let menu_group = group.id.clone();
            let menu = Button::new(format!("navigation-group-menu-{}", group.id))
                .ghost()
                .compact()
                .w(px(22.))
                .h(px(22.))
                .icon(theme::small_icon(IconName::Ellipsis))
                .dropdown_menu(move |menu, _, _| {
                    let current_view = menu_view.clone();
                    let current_id = menu_group.clone();
                    let rename_view = menu_view.clone();
                    let rename_id = menu_group.clone();
                    let delete_view = menu_view.clone();
                    let delete_id = menu_group.clone();
                    let add_view = menu_view.clone();
                    let add_id = menu_group.clone();
                    menu.item(menu_item(
                        "Add current folder to group",
                        &current_view,
                        move |this, _, cx| {
                            this.add_current_favorite(current_id.clone(), cx);
                        },
                    ))
                    .item(menu_item(
                        "Add folder by path…",
                        &add_view,
                        move |this, window, cx| {
                            this.open_sidebar_editor(
                                SidebarEditor::AddFavorite {
                                    group_id: add_id.clone(),
                                },
                                "",
                                window,
                                cx,
                            );
                        },
                    ))
                    .item(menu_item(
                        "Rename",
                        &rename_view,
                        move |this, window, cx| {
                            let initial = this
                                .sidebar_config()
                                .favorite_groups
                                .iter()
                                .find(|g| g.id == rename_id)
                                .map(|g| g.name.clone())
                                .unwrap_or_default();
                            this.open_sidebar_editor(
                                SidebarEditor::RenameGroup {
                                    group_id: rename_id.clone(),
                                },
                                &initial,
                                window,
                                cx,
                            );
                        },
                    ))
                    .item(menu_item(
                        "Delete group",
                        &delete_view,
                        move |this, _, cx| {
                            this.remove_favorite_group(delete_id.clone(), cx);
                        },
                    ))
                });
            let toggle = expand_chevron(
                &group_view,
                format!("navigation-group-chevron-{}", group.id),
                !group_collapsed,
                move |this, _| this.toggle_favorite_group(&group_id),
            );
            let add_id = group.id.clone();
            let add = icon_button(
                format!("navigation-group-add-{}", group.id),
                IconName::Plus,
                move |this, cx| this.add_current_favorite(add_id.clone(), cx),
                cx,
            )
            .disabled(self.current_favorite_path().is_none())
            .accessibility_label(fileatlas_ui_bridge::i18n::tr("Add current folder to group"));
            let measured = Arc::clone(&self.favorite_header_bounds);
            let measured_id = group.id.clone();
            let header = fileatlas_ui_bridge::components::sidebar_header(
                group.name.clone(),
                toggle,
                menu,
                vec![add],
            )
            .relative()
            .child(
                canvas(
                    move |bounds, _, _| {
                        if let Ok(mut headers) = measured.write() {
                            headers.insert(
                                measured_id.clone(),
                                [
                                    f32::from(bounds.origin.x),
                                    f32::from(bounds.origin.y),
                                    f32::from(bounds.size.width),
                                    f32::from(bounds.size.height),
                                ],
                            );
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            );
            let mut block = div().v_flex().w_full().child(header);
            if matches!(&self.sidebar_editor,
                Some(SidebarEditor::RenameGroup {group_id} | SidebarEditor::AddFavorite {group_id}
                  | SidebarEditor::RenameFavorite {group_id,..}) if group_id == &group.id)
                && let Some(editor) = self.render_sidebar_editor_row(cx)
            {
                block = block.child(editor);
            }
            if group_collapsed {
                region = region.child(sidebar_region(block));
                continue;
            }
            let mut rows = div().v_flex().w_full();
            for folder in group.folders.clone() {
                let basename = Path::new(&folder.path)
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned());
                let display = folder
                    .display_name
                    .clone()
                    .filter(|name| Some(name) != basename.as_ref())
                    .unwrap_or_else(|| crate::location::display_path(&folder.path));
                let expanded = self.expanded_directories.contains_key(&folder.path);
                let selected = selected_path.as_deref().is_some_and(|selected| {
                    crate::sidebar_folders::same_sidebar_path(selected, &folder.path)
                });
                let row_view = weak.clone();
                let registered_id = self
                    .snapshot
                    .as_ref()
                    .ok()
                    .and_then(|snapshot| {
                        snapshot.folders.iter().find(|node| {
                            crate::sidebar_folders::same_sidebar_path(
                                &node.folder.path,
                                &folder.path,
                            )
                        })
                    })
                    .map(|node| node.folder.id);
                let menu_view = row_view.clone();
                let row_path = folder.path.clone();
                let rename_view = weak.clone();
                let rename_group_id = group.id.clone();
                let rename_folder_id = folder.id.clone();
                let remove_view = weak.clone();
                let remove_group_id = group.id.clone();
                let remove_folder_id = folder.id.clone();
                rows = rows.child(path_row(
                    &row_view,
                    format!("fav-{}", folder.id),
                    display,
                    IconName::Folder,
                    folder.path.clone(),
                    expanded,
                    selected,
                    browse,
                    move |menu| {
                        let menu = if let Some(id) = registered_id {
                            favorite_menu(menu, id, row_path.clone(), menu_view.clone())
                        } else {
                            menu
                        };
                        let open_view = menu_view.clone();
                        let open_path = row_path.clone();
                        let add_view = menu_view.clone();
                        let add_path = row_path.clone();
                        let rename_view = rename_view.clone();
                        let rename_group_id = rename_group_id.clone();
                        let rename_folder_id = rename_folder_id.clone();
                        let remove_view = remove_view.clone();
                        let remove_group_id = remove_group_id.clone();
                        let remove_folder_id = remove_folder_id.clone();
                        menu.item(menu_item("Index folder", &add_view, move |this, _, cx| {
                            this.add_folder_path(add_path.clone(), cx);
                        }))
                        .item(menu_item(
                            "Open in file manager",
                            &open_view,
                            move |this, _, _| {
                                this.open_folder_in_explorer(&open_path);
                            },
                        ))
                        .item(menu_item(
                            "Rename",
                            &rename_view,
                            move |this, window, cx| {
                                let initial = this
                                    .sidebar_config()
                                    .favorite_groups
                                    .iter()
                                    .find(|g| g.id == rename_group_id)
                                    .and_then(|g| {
                                        g.folders.iter().find(|f| f.id == rename_folder_id)
                                    })
                                    .and_then(|f| f.display_name.clone())
                                    .unwrap_or_default();
                                this.open_sidebar_editor(
                                    SidebarEditor::RenameFavorite {
                                        group_id: rename_group_id.clone(),
                                        folder_id: rename_folder_id.clone(),
                                    },
                                    &initial,
                                    window,
                                    cx,
                                );
                            },
                        ))
                        .item(menu_item(
                            "Remove from favorites",
                            &remove_view,
                            move |this, _, cx| {
                                this.remove_favorite_folder(
                                    remove_group_id.clone(),
                                    remove_folder_id.clone(),
                                    cx,
                                );
                            },
                        ))
                    },
                ));
                if expanded {
                    if let Some(column) = children_column(self, &folder.path, cx) {
                        rows = rows.child(column);
                    }
                }
            }
            region = region.child(sidebar_region(block.child(rows)));
        }
        if let Some(error) = &self.favorite_action_error {
            region = region.child(
                div()
                    .text_xs()
                    .text_color(rgb(theme::danger_text()))
                    .child(error.clone()),
            );
        }
        if let Some(form) = self.render_favorite_folder_creation(cx) {
            region = region.child(form);
        }
        region = region.child(
            Button::new("navigation-favorites-add-group")
                .outline()
                .w_full()
                .flex_shrink_0()
                .h(px(30.))
                .mt(px(6.))
                .icon(theme::small_icon(IconName::Plus))
                .label(fileatlas_ui_bridge::i18n::tr("New group"))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_sidebar_editor(SidebarEditor::CreateGroup, "", window, cx)
                })),
        );
        if matches!(self.sidebar_editor, Some(SidebarEditor::CreateGroup))
            && let Some(editor) = self.render_sidebar_editor_row(cx)
        {
            region = region.child(editor);
        }
        region
    }
}
