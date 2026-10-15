use super::*;
impl SidebarView {
    pub(in crate::view) fn render_favorites_section(&mut self, cx: &mut Context<Self>) -> Div {
        let collapsed = self.collapsed_sections.contains("favorites");
        let weak = cx.entity().downgrade();
        let config = self.sidebar_config();
        let browse = self.service.browser_enabled();
        let selected_path = self
            .session
            .snapshot()
            .ok()
            .and_then(|saved| saved.selected_path)
            .clone();
        let group_menu_view = weak.clone();
        let mut section = div()
            .v_flex()
            .border_t_1()
            .border_color(rgb(theme::SEPARATOR))
            .child(section_header(
                &weak,
                "favorites",
                "Favorites",
                collapsed,
                move |menu| {
                    let new_view = group_menu_view.clone();
                    menu.item(menu_item("New group", &new_view, |this, window, cx| {
                        this.open_sidebar_editor(SidebarEditor::CreateGroup, "", window, cx);
                    }))
                },
                vec![],
            ));
        if collapsed {
            return section;
        }
        for group in config.favorite_groups.clone() {
            let group_collapsed = self.collapsed_groups.contains(&group.id);
            let group_view = weak.clone();
            let group_rename_id = group.id.clone();
            let group_delete_id = group.id.clone();
            let add_view = weak.clone();
            let add_group_id = group.id.clone();
            section = section.child(
                div()
                    .h_flex()
                    .w_full()
                    .items_center()
                    .child(expand_chevron(
                        &group_view,
                        format!("navigation-group-chevron-{}", group.id),
                        !group_collapsed,
                        {
                            let toggle_id = group.id.clone();
                            move |this, _| {
                                this.toggle_favorite_group(&toggle_id);
                            }
                        },
                    ))
                    .child(
                        div()
                            .id(format!("navigation-group-{}", group.id))
                            .flex_1()
                            .h_flex()
                            .items_center()
                            .gap(px(6.))
                            .h(px(24.))
                            .px(px(4.))
                            .rounded(px(4.))
                            .hover(|style| style.bg(rgba(0xffffff10)))
                            .child(
                                theme::small_icon(if group_collapsed {
                                    IconName::Star
                                } else {
                                    IconName::StarFill
                                })
                                .text_color(rgb(theme::STAR)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(12.5))
                                    .text_ellipsis()
                                    .child(group.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .text_color(rgb(theme::TEXT_MUTED))
                                    .child(group.folders.len().to_string()),
                            ),
                    )
                    .child(
                        Button::new(format!("navigation-group-menu-{}", group.id))
                            .ghost()
                            .compact()
                            .w(px(22.))
                            .h(px(22.))
                            .icon(theme::small_icon(IconName::Ellipsis))
                            .dropdown_menu(move |menu, _, _| {
                                let add_view = add_view.clone();
                                let add_group_id = add_group_id.clone();
                                let rename_view = group_view.clone();
                                let rename_id = group_rename_id.clone();
                                let delete_view = group_view.clone();
                                let delete_id = group_delete_id.clone();
                                menu.item(menu_item(
                                    "Add folder",
                                    &add_view,
                                    move |this, window, cx| {
                                        this.open_sidebar_editor(
                                            SidebarEditor::AddFavorite {
                                                group_id: add_group_id.clone(),
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
                            }),
                    ),
            );
            if group_collapsed {
                continue;
            }
            for folder in group.folders.clone() {
                let display = folder.display_name.clone().unwrap_or_else(|| {
                    Path::new(&folder.path)
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| folder.path.clone())
                });
                let expanded = self.expanded_directories.contains_key(&folder.path);
                let selected = selected_path.as_deref().is_some_and(|selected| {
                    crate::sidebar_folders::same_sidebar_path(selected, &folder.path)
                });
                let row_view = weak.clone();
                let menu_view = row_view.clone();
                let row_path = folder.path.clone();
                let rename_view = weak.clone();
                let rename_group_id = group.id.clone();
                let rename_folder_id = folder.id.clone();
                let remove_view = weak.clone();
                let remove_group_id = group.id.clone();
                let remove_folder_id = folder.id.clone();
                section = section.child(path_row(
                    &row_view,
                    format!("fav-{}", folder.id),
                    display,
                    IconName::Folder,
                    folder.path.clone(),
                    expanded,
                    selected,
                    browse,
                    move |menu| {
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
                        section = section.child(column);
                    }
                }
            }
        }
        section.child(
            Button::new("navigation-favorites-add-group")
                .ghost()
                .compact()
                .w_full()
                .justify_start()
                .label(photag_ui_bridge::i18n::tr("New group"))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_sidebar_editor(SidebarEditor::CreateGroup, "", window, cx);
                })),
        )
    }
}
