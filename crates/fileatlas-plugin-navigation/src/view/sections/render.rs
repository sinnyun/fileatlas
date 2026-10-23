//! Rendering for the drives / quick access / grouped-favorites sidebar sections.
use fileatlas_ui_bridge::components::WorkbenchForeground as _;
use fileatlas_ui_bridge::components::WorkbenchPaint as _;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
use gpui_kit::component::button::Button;
mod editor;
mod favorites;
mod quick_access;
use super::super::render::tag_render;
use super::*;
use fileatlas_ui_bridge::components::WorkbenchStyle as _;
use gpui_kit::assets::IconName;
use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
use gpui_kit::component::progress::Progress;

fn drive_icon(drive: &crate::DriveInfo) -> IconName {
    match drive.kind {
        crate::DriveKind::Network => IconName::Globe,
        _ => IconName::HardDrive,
    }
}

fn size_label(bytes: u64) -> String {
    const TB: u64 = 1_099_511_627_776;
    const GB: u64 = 1_073_741_824;
    const MB: u64 = 1_048_576;
    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{} GB", bytes / GB)
    } else if bytes >= MB {
        format!("{} MB", bytes / MB)
    } else {
        format!("{} KB", bytes / 1_024)
    }
}

fn menu_item(
    label: &str,
    view: &WeakEntity<SidebarView>,
    action: impl Fn(&mut SidebarView, &mut Window, &mut Context<SidebarView>) + 'static,
) -> PopupMenuItem {
    let item_view = view.clone();
    PopupMenuItem::new(fileatlas_ui_bridge::i18n::tr(label)).on_click(move |_, window, cx| {
        let _ = item_view.update(cx, |this, cx| action(this, window, cx));
    })
}

fn checked_menu_item(
    label: &'static str,
    checked: bool,
    view: &WeakEntity<SidebarView>,
    action: impl Fn(&mut SidebarView, &mut Context<SidebarView>) + 'static,
) -> PopupMenuItem {
    let item_view = view.clone();
    let mut item = PopupMenuItem::new(fileatlas_ui_bridge::i18n::tr(label));
    if checked {
        item = item.icon(theme::small_icon(IconName::Check));
    }
    item.on_click(move |_, _, cx| {
        let _ = item_view.update(cx, |this, cx| action(this, cx));
    })
}

fn section_header(
    view: &WeakEntity<SidebarView>,
    key: &'static str,
    label: &str,
    collapsed: bool,
    menu: impl Fn(PopupMenu) -> PopupMenu + 'static,
    trailing: Vec<Button>,
) -> Div {
    let toggle = view.clone();
    let menu_button =
        fileatlas_ui_bridge::components::button(format!("navigation-section-menu-{key}"))
            .ghost()
            .icon(theme::small_icon(IconName::Ellipsis))
            .dropdown_menu(move |popup_menu, _, _| menu(popup_menu));
    fileatlas_ui_bridge::components::sidebar_header(
        fileatlas_ui_bridge::i18n::tr(label),
        fileatlas_ui_bridge::components::button(format!("navigation-section-chevron-{key}"))
            .ghost()
            .icon(theme::small_icon(if collapsed {
                IconName::ChevronRight
            } else {
                IconName::ChevronDown
            }))
            .on_click(move |_, _, cx| {
                let _ = toggle.update(cx, |this, cx| {
                    this.toggle_section(key);
                    cx.notify();
                });
            }),
        menu_button,
        trailing,
    )
}

fn icon_button(
    id: String,
    icon: IconName,
    handler: impl Fn(&mut SidebarView, &mut Context<SidebarView>) + 'static,
    cx: &mut Context<SidebarView>,
) -> Button {
    fileatlas_ui_bridge::components::button(id)
        .ghost()
        .icon(theme::small_icon(icon))
        .on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
}

fn expand_chevron(
    view: &WeakEntity<SidebarView>,
    id: String,
    expanded: bool,
    toggle: impl Fn(&mut SidebarView, &mut Context<SidebarView>) + 'static,
) -> Button {
    let chevron_view = view.clone();
    fileatlas_ui_bridge::components::button(id)
        .ghost()
        .icon(theme::small_icon(if expanded {
            IconName::ChevronDown
        } else {
            IconName::ChevronRight
        }))
        .on_click(move |_, _, cx| {
            let _ = chevron_view.update(cx, |this, cx| {
                toggle(this, cx);
                cx.notify();
            });
        })
}

#[allow(clippy::too_many_arguments)]
fn path_row(
    view: &WeakEntity<SidebarView>,
    id_key: String,
    label: String,
    icon: IconName,
    path: String,
    expanded: bool,
    selected: bool,
    browse: bool,
    menu: impl Fn(PopupMenu) -> PopupMenu + 'static,
) -> Div {
    let chevron_view = view.clone();
    let chevron_path = path.clone();
    let activate_view = view.clone();
    let activate_path = path.clone();
    div()
        .h_flex()
        .w_full()
        .items_center()
        .child(expand_chevron(
            &chevron_view,
            format!("navigation-{id_key}-chevron"),
            expanded,
            move |this, _| {
                this.toggle_directory(&chevron_path);
            },
        ))
        .child(
            div()
                .id(format!("navigation-{id_key}"))
                .workbench_tree_row(selected)
                .flex_1()
                .min_w_0()
                .h(px(24.))
                .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                .px(px(fileatlas_ui_bridge::theme::space_compact()))
                .on_click(move |_, _, cx| {
                    let _ = activate_view.update(cx, |this, cx| {
                        if browse {
                            this.activate_sidebar_path(activate_path.clone(), cx);
                        } else {
                            this.add_folder_path(activate_path.clone(), cx);
                        }
                    });
                })
                .child(
                    theme::small_icon(if expanded { IconName::FolderOpen } else { icon })
                        .workbench_foreground(if selected {
                            fileatlas_ui_bridge::components::ForegroundRole::AccentText
                        } else {
                            fileatlas_ui_bridge::components::ForegroundRole::Secondary
                        }),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .workbench_assistant()
                        .text_ellipsis()
                        .child(label),
                ),
        )
        .child(
            fileatlas_ui_bridge::components::button(format!("navigation-{id_key}-menu"))
                .ghost()
                .icon(theme::small_icon(IconName::Ellipsis))
                .dropdown_menu(move |popup_menu, _, _| menu(popup_menu)),
        )
}

fn children_column(
    view: &mut SidebarView,
    root: &str,
    cx: &mut Context<SidebarView>,
) -> Option<Div> {
    let children = view.expanded_directories.get(root)?.clone();
    let query = view.folder_search.read(cx).value().trim().to_lowercase();
    let selected_path = view
        .session
        .snapshot()
        .ok()
        .and_then(|saved| saved.selected_path)
        .clone();
    let rows = children
        .iter()
        .filter_map(|child| {
            tag_render::render_subdirectory(
                child,
                &view.expanded_directories,
                &query,
                selected_path.as_deref(),
                false,
                cx,
            )
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return None;
    }
    Some(
        div()
            .v_flex()
            .ml(px(fileatlas_ui_bridge::theme::space_expanded()))
            .pl(px(fileatlas_ui_bridge::theme::space_standard()))
            .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Left)
            .children(rows),
    )
}

impl SidebarView {
    pub(in crate::view) fn ensure_drives_requested(&mut self) {
        let show_network = self.sidebar_config().show_network_drives;
        let key = (show_network, self.sidebar_io_revision);
        self.system_folders_read
            .request(self.sidebar_io_revision, || {
                Ok(system_quick_access()
                    .into_iter()
                    .filter_map(|(label, path)| {
                        path.filter(|p| p.is_dir()).map(|p| (label.to_owned(), p))
                    })
                    .collect())
            });
        self.drives_read
            .request(key, move || crate::enumerate_drives(key.0));
    }

    pub(in crate::view) fn render_drives_section(&mut self, cx: &mut Context<Self>) -> Div {
        self.ensure_drives_requested();
        let collapsed = self.collapsed_sections.contains("drives");
        let weak = cx.entity().downgrade();
        let network_shown = self.sidebar_config().show_network_drives;
        let header_menu_view = weak.clone();
        let toggle_view = weak.clone();
        let section = div().v_flex().child(section_header(
            &weak,
            "drives",
            "Drives",
            collapsed,
            move |menu| {
                menu.item(checked_menu_item(
                    "Show network drives",
                    network_shown,
                    &header_menu_view,
                    |this, cx| this.set_show_network_drives(true, cx),
                ))
                .item(menu_item(
                    "Refresh drives",
                    &header_menu_view,
                    |view, _, cx| {
                        view.refresh_sidebar_roots();
                        cx.notify();
                    },
                ))
                .item(checked_menu_item(
                    "Hide network drives",
                    !network_shown,
                    &header_menu_view,
                    |this, cx| this.set_show_network_drives(false, cx),
                ))
            },
            vec![
                fileatlas_ui_bridge::components::button("navigation-drives-toggle-network")
                    .ghost()
                    .icon(theme::small_icon(IconName::Globe))
                    .workbench_foreground(if network_shown {
                        fileatlas_ui_bridge::components::ForegroundRole::Accent
                    } else {
                        fileatlas_ui_bridge::components::ForegroundRole::Muted
                    })
                    .on_click(move |_, _, cx| {
                        let _ = toggle_view.update(cx, |this, cx| {
                            this.toggle_network_drives(cx);
                        });
                    }),
            ],
        ));
        let section = section.when_some(self.sidebar_save_error.clone(), |column, error| {
            column.child(
                div()
                    .v_flex()
                    .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                    .child(div().workbench_assistant().child(format!(
                        "{}: {error}",
                        fileatlas_ui_bridge::i18n::tr("Sidebar changes not saved")
                    )))
                    .child(
                        fileatlas_ui_bridge::components::button("sidebar-save-retry")
                            .ghost()
                            .label(fileatlas_ui_bridge::i18n::tr("Retry"))
                            .on_click(cx.listener(|view, _, _, cx| view.retry_sidebar_save(cx))),
                    ),
            )
        });
        if collapsed {
            return section;
        }
        let browse = self.service.browser_enabled();
        let selected_path = self
            .session
            .snapshot()
            .ok()
            .and_then(|saved| saved.selected_path)
            .clone();
        let mut section = section;
        match self.drives_read.value().cloned() {
            Some(Ok(drives)) => {
                if drives.is_empty() {
                    section = section.child(
                        div()
                            .workbench_assistant()
                            .child(fileatlas_ui_bridge::i18n::tr("No drives found")),
                    );
                }
                for drive in drives {
                    let root = drive.root.clone();
                    let expanded = self.expanded_directories.contains_key(&root);
                    let selected = selected_path
                        .as_deref()
                        .is_some_and(|path| crate::sidebar_folders::in_sidebar_root(path, &root));
                    let chevron_view = weak.clone();
                    let chevron_root = root.clone();
                    let activate_view = weak.clone();
                    let activate_root = root.clone();
                    let menu_view = weak.clone();
                    let menu_root = root.clone();
                    let ratio = if drive.total_bytes > 0 {
                        drive.used_bytes as f32 / drive.total_bytes as f32
                    } else {
                        0.
                    };
                    section = section.child(
                        div()
                            .h_flex()
                            .w_full()
                            .items_start()
                            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                            .child(expand_chevron(
                                &chevron_view,
                                format!("navigation-drive-chevron-{root}"),
                                expanded,
                                move |this, _| this.toggle_drive(&chevron_root),
                            ))
                            .child(
                                div()
                                    .id(format!("navigation-drive-{root}"))
                                    .flex_1()
                                    .min_w_0()
                                    .v_flex()
                                    .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                                    .workbench_corner(fileatlas_ui_bridge::components::CornerRole::Small)
                                    .px(px(fileatlas_ui_bridge::theme::space_compact()))
                                    .py(px(fileatlas_ui_bridge::theme::space_compact()))
                                    .workbench_selectable_surface(selected)
                                    .on_click(move |_, _, cx| {
                                        let _ = activate_view.update(cx, |this, cx| {
                                            if browse {
                                                this.activate_sidebar_path(
                                                    activate_root.clone(),
                                                    cx,
                                                );
                                            } else {
                                                this.add_folder_path(activate_root.clone(), cx);
                                            }
                                        });
                                    })
                                    .child(
                                        div()
                                            .h_flex()
                                            .items_center()
                                            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                                            .child(
                                                theme::small_icon(drive_icon(&drive)).workbench_foreground(
                                                    if selected {
                                                        fileatlas_ui_bridge::components::ForegroundRole::Selection
                                                    } else {
                                                        fileatlas_ui_bridge::components::ForegroundRole::Secondary
                                                    },
                                                ),
                                            )
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .workbench_assistant()
                                                    .text_ellipsis()
                                                    .child(drive.display_name()),
                                            )
                                            .child(
                                                div()
                                                    .min_w_0()
                                                    .overflow_hidden()
                                                    .workbench_assistant()
                                                    .text_ellipsis()
                                                    .child(if drive.usage_error.is_some() {
                                                        fileatlas_ui_bridge::i18n::tr("Unavailable")
                                                    } else {
                                                        size_label(drive.total_bytes)
                                                    }),
                                            ),
                                    )
                                    .child(
                                        Progress::new(format!(
                                            "navigation-drive-usage-{}",
                                            drive.root
                                        ))
                                        .value(ratio.clamp(0., 1.) * 100.)
                                        .w_full(),
                                    ),
                            )
                            .child(
                                fileatlas_ui_bridge::components::button(format!(
                                    "navigation-drive-menu-{root}"
                                ))
                                .ghost()
                                .icon(theme::small_icon(IconName::Ellipsis))
                                .dropdown_menu(
                                    move |menu, _, _| {
                                        let add_view = menu_view.clone();
                                        let add_path = menu_root.clone();
                                        let open_view = menu_view.clone();
                                        let open_path = menu_root.clone();
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
                                ),
                            ),
                    );
                    if expanded {
                        if let Some(column) = children_column(self, &root, cx) {
                            section = section.child(column);
                        }
                    }
                }
            }
            Some(Err(error)) => {
                section = section.child(div().workbench_assistant().child(error.clone()));
            }
            None => {
                section = section.child(
                    div()
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr("Loading drives…")),
                );
            }
        }
        section
    }
}
