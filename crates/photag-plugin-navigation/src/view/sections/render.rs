//! Rendering for the drives / quick access / grouped-favorites sidebar sections.
mod editor;
mod favorites;
mod quick_access;
use super::super::render::tag_render;
use super::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
use gpui_kit::component::progress::Progress;

fn drive_icon(drive: &crate::DriveInfo) -> IconName {
    match drive.kind {
        crate::DriveKind::Network => IconName::Globe,
        _ => IconName::HardDrive,
    }
}

fn usage_text(used_bytes: u64, total_bytes: u64) -> String {
    let gb = 1_073_741_824u64;
    format!(
        "{} / {}",
        group_thousands(used_bytes / gb),
        group_thousands(total_bytes / gb)
    )
}

fn menu_item(
    label: &str,
    view: &WeakEntity<SidebarView>,
    action: impl Fn(&mut SidebarView, &mut Window, &mut Context<SidebarView>) + 'static,
) -> PopupMenuItem {
    let item_view = view.clone();
    PopupMenuItem::new(photag_ui_bridge::i18n::tr(label)).on_click(move |_, window, cx| {
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
    let mut item = PopupMenuItem::new(photag_ui_bridge::i18n::tr(label));
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
    let menu_button = Button::new(format!("navigation-section-menu-{key}"))
        .ghost()
        .compact()
        .w(px(22.))
        .h(px(22.))
        .icon(theme::small_icon(IconName::Ellipsis))
        .dropdown_menu(move |popup_menu, _, _| menu(popup_menu));
    photag_ui_bridge::components::sidebar_header(
        photag_ui_bridge::i18n::tr(label),
        Button::new(format!("navigation-section-chevron-{key}"))
            .ghost()
            .compact()
            .p_0()
            .w(px(theme::SIDEBAR_ARROW_SLOT))
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
    Button::new(id)
        .ghost()
        .compact()
        .w(px(22.))
        .h(px(22.))
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
    Button::new(id)
        .ghost()
        .compact()
        .p_0()
        .w(px(theme::SIDEBAR_ARROW_SLOT))
        .h(px(24.))
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
                .flex_1()
                .min_w_0()
                .h_flex()
                .items_center()
                .gap(px(6.))
                .h(px(24.))
                .rounded(px(4.))
                .px(px(4.))
                .hover(|style| style.bg(rgba(theme::FILL_HOVER)))
                .when(selected, |style| style.bg(rgb(theme::selection_color())))
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
                        .text_color(rgb(if selected {
                            theme::accent_hover()
                        } else {
                            theme::TEXT_SECONDARY
                        })),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(12.5))
                        .text_ellipsis()
                        .child(label),
                ),
        )
        .child(
            Button::new(format!("navigation-{id_key}-menu"))
                .ghost()
                .compact()
                .w(px(22.))
                .h(px(22.))
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
            .ml(px(22.))
            .pl(px(8.))
            .border_l_1()
            .border_color(rgba(theme::BORDER_STRONG))
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
                Button::new("navigation-drives-toggle-network")
                    .ghost()
                    .compact()
                    .w(px(22.))
                    .h(px(22.))
                    .icon(theme::small_icon(IconName::Globe))
                    .text_color(rgb(if network_shown {
                        theme::accent()
                    } else {
                        theme::TEXT_MUTED
                    }))
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
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(theme::DANGER))
                            .child(format!(
                                "{}: {error}",
                                photag_ui_bridge::i18n::tr("Sidebar changes not saved")
                            )),
                    )
                    .child(
                        Button::new("sidebar-save-retry")
                            .ghost()
                            .compact()
                            .label(photag_ui_bridge::i18n::tr("Retry"))
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
                            .text_xs()
                            .text_color(rgb(theme::TEXT_MUTED))
                            .child(photag_ui_bridge::i18n::tr("No drives found")),
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
                            .gap(px(2.))
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
                                    .gap(px(2.))
                                    .rounded(px(4.))
                                    .px(px(4.))
                                    .py(px(2.))
                                    .hover(|style| style.bg(rgba(theme::FILL_HOVER)))
                                    .when(selected, |style| style.bg(rgb(theme::selection_color())))
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
                                            .gap(px(6.))
                                            .child(
                                                theme::small_icon(drive_icon(&drive)).text_color(
                                                    rgb(if selected {
                                                        theme::accent_hover()
                                                    } else {
                                                        theme::TEXT_SECONDARY
                                                    }),
                                                ),
                                            )
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .text_size(px(12.5))
                                                    .text_ellipsis()
                                                    .child(drive.display_name()),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(10.5))
                                                    .text_color(rgb(theme::TEXT_MUTED))
                                                    .child(if drive.usage_error.is_some() {
                                                        photag_ui_bridge::i18n::tr("Unavailable")
                                                    } else {
                                                        usage_text(
                                                            drive.used_bytes,
                                                            drive.total_bytes,
                                                        )
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
                                Button::new(format!("navigation-drive-menu-{root}"))
                                    .ghost()
                                    .compact()
                                    .w(px(22.))
                                    .h(px(22.))
                                    .icon(theme::small_icon(IconName::Ellipsis))
                                    .dropdown_menu(move |menu, _, _| {
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
                                    }),
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
                section = section.child(
                    div()
                        .text_xs()
                        .text_color(rgb(theme::DANGER))
                        .child(error.clone()),
                );
            }
            None => {
                section = section.child(
                    div()
                        .text_xs()
                        .text_color(rgb(theme::TEXT_MUTED))
                        .child(photag_ui_bridge::i18n::tr("Loading drives…")),
                );
            }
        }
        section
    }
}
