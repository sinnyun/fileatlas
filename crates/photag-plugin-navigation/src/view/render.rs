use super::*;
use gpui_kit::component::button::ButtonVariants;

mod folder_row;
mod tag_render;
use folder_row::folder_row;
use tag_render::{render_subdirectory, render_tag};

impl Render for SidebarView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(request) = self.service.take_ui_request() {
            match request {
                crate::NavigationUiRequest::OpenCreate => {
                    self.close_tag_modal();
                    self.tag_create_open = true;
                }
                crate::NavigationUiRequest::OpenEdit(id) => {
                    self.close_tag_modal();
                    self.begin_edit_tag(id, window, cx);
                }
                crate::NavigationUiRequest::OpenDelete(id) => {
                    self.close_tag_modal();
                    self.request_delete_tag(id, cx);
                }
                crate::NavigationUiRequest::OpenMerge(id) => {
                    self.close_tag_modal();
                    self.begin_merge(id, window, cx);
                }
                crate::NavigationUiRequest::CloseModal => self.close_tag_modal(),
            }
            cx.notify();
        }
        let locale_revision = photag_ui_bridge::i18n::revision();
        if self.locale_revision != locale_revision {
            self.locale_revision = locale_revision;
            self.folder_path.update(cx, |input, cx| {
                input.set_placeholder(photag_ui_bridge::i18n::tr("Folder path"), window, cx)
            });
            self.new_folder_name.update(cx, |input, cx| {
                input.set_placeholder(photag_ui_bridge::i18n::tr("Folder name"), window, cx)
            });
            self.tag_name.update(cx, |input, cx| {
                input.set_placeholder(photag_ui_bridge::i18n::tr("Tag name"), window, cx)
            });
            self.tag_search.update(cx, |input, cx| {
                input.set_placeholder(photag_ui_bridge::i18n::tr("Search tags…"), window, cx)
            });
            self.merge_search.update(cx, |input, cx| {
                input.set_placeholder(
                    photag_ui_bridge::i18n::tr("Filter target tags…"),
                    window,
                    cx,
                )
            });
        }

        if self.restore_selection_pending {
            self.restore_selection_pending = false;
            let cordis = self.cordis.clone();
            let folder_id = self.restored_folder_id;
            let path = self.restored_path.clone();
            cx.spawn(async move |_, _| {
                if let Err(error) = cordis
                    .emit::<FolderSelectionChanged>(Routing::Unscoped, folder_id)
                    .await
                {
                    eprintln!("restored folder selection dispatch failed: {error}");
                }
                if let Err(error) = cordis
                    .emit::<DirectoryPathSelected>(Routing::Unscoped, path)
                    .await
                {
                    eprintln!("restored directory selection dispatch failed: {error}");
                }
            })
            .detach();
        }
        // Source main.rs:1282-1296 and sidebar.rs:228-275.
        let mut panel = div()
            .v_flex()
            .size_full()
            .gap(px(1.))
            .pl(px(10.))
            .pr(px(6.))
            .pt(px(2.))
            .pb(px(8.));
        let folder_view = cx.entity().downgrade();
        match &self.snapshot {
            Ok(snapshot) => {
                if self.snapshot.is_ok() {
                    panel = panel.child(
                        folder_row(
                            "navigation-all-media".into(),
                            photag_ui_bridge::i18n::tr("All files"),
                            snapshot.folders.iter().map(|node| node.media_count).sum(),
                            self.session
                                .snapshot()
                                .map(|s| {
                                    s.selected_folder_id.is_none() && s.selected_path.is_none()
                                })
                                .unwrap_or(true)
                                && self.included_tags.is_empty()
                                && self.excluded_tags.is_empty(),
                            theme::icons::LIBRARY,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.select_folder(None, cx);
                            this.select_tag(None, cx);
                            cx.notify();
                        })),
                    );
                    panel = panel.child(
                        div()
                            .group("folder-header")
                            .h_flex()
                            .items_center()
                            .justify_between()
                            .pt(px(2.))
                            .pb(px(4.))
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .text_color(rgb(theme::TEXT_MUTED))
                                    .child(photag_ui_bridge::i18n::tr("Folders").to_uppercase()),
                            )
                            .child(
                                div()
                                    .h_flex()
                                    .gap_1()
                                    .child(
                                        Button::new("navigation-add-folder")
                                            .ghost()
                                            .compact()
                                            .w(px(22.))
                                            .h(px(22.))
                                            .label(photag_ui_bridge::i18n::tr(&("+").to_string()))
                                            .on_click(
                                                cx.listener(|this, _, _, cx| this.pick_folder(cx)),
                                            ),
                                    )
                                    .child(
                                        Button::new("navigation-folder-options")
                                            .ghost()
                                            .opacity(0.)
                                            .group_hover("folder-header", |style| style.opacity(1.))
                                            .label("⋯")
                                            .dropdown_menu(move |mut menu, _, _| {
                                                let path_view = folder_view.clone();
                                                menu = menu.item(
                                                    PopupMenuItem::new(photag_ui_bridge::i18n::tr(
                                                        "Add folder by path…",
                                                    ))
                                                    .on_click(move |_, _, cx| {
                                                        let _ = path_view.update(cx, |this, cx| {
                                                            this.show_folder_path_entry = true;
                                                            cx.notify();
                                                        });
                                                    }),
                                                );
                                                let refresh_view = folder_view.clone();
                                                menu.item(
                                                    PopupMenuItem::new(photag_ui_bridge::i18n::tr(
                                                        "Refresh folders",
                                                    ))
                                                    .on_click(move |_, _, cx| {
                                                        let _ =
                                                            refresh_view.update(cx, |this, cx| {
                                                                this.refresh();
                                                                cx.notify();
                                                            });
                                                    }),
                                                )
                                            }),
                                    ),
                            ),
                    );
                    if self.show_folder_path_entry {
                        panel = panel.child(
                            div()
                                .h_flex()
                                .gap_1()
                                .child(Input::new(&self.folder_path).flex_1())
                                .child(
                                    Button::new("navigation-browse-folder")
                                        .label(photag_ui_bridge::i18n::tr("Browse…"))
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.pick_folder(cx)),
                                        ),
                                )
                                .child(
                                    Button::new("navigation-add-folder-path")
                                        .label(photag_ui_bridge::i18n::tr("Add"))
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.add_folder(cx)),
                                        ),
                                ),
                        );
                    }
                    if let Some(parent) = &self.new_folder_parent {
                        panel = panel.child(
                            div()
                                .v_flex()
                                .gap_1()
                                .child(div().text_xs().child(format!("New folder in {}", parent)))
                                .child(Input::new(&self.new_folder_name))
                                .child(
                                    div()
                                        .h_flex()
                                        .gap_1()
                                        .child(
                                            Button::new("navigation-create-subfolder")
                                                .label(photag_ui_bridge::i18n::tr("Create"))
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.create_subfolder(cx)
                                                })),
                                        )
                                        .child(
                                            Button::new("navigation-cancel-subfolder")
                                                .label(photag_ui_bridge::i18n::tr("Cancel"))
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.new_folder_parent = None;
                                                    cx.notify();
                                                })),
                                        ),
                                ),
                        );
                    }
                    for node in &snapshot.folders {
                        let expanded = self.expanded_directories.get(&node.folder.path);
                        let row_group = format!("navigation-folder-row-{}", node.folder.id);
                        panel = panel.child(
                            div()
                                .group(row_group.clone())
                                .h_flex()
                                .w_full()
                                .items_center()
                                .gap_0()
                                .child(
                                    Button::new(format!(
                                        "navigation-expand-root-{}",
                                        node.folder.id
                                    ))
                                    .ghost()
                                    .compact()
                                    .p_0()
                                    .font_family(theme::ICON_FONT_FAMILY)
                                    .text_size(px(8.))
                                    .label(photag_ui_bridge::i18n::tr(
                                        &(if expanded.is_some() {
                                            theme::icons::CHEVRON_DOWN
                                        } else {
                                            theme::icons::CHEVRON_RIGHT
                                        })
                                        .to_string(),
                                    ))
                                    // Source: sidebar::arrow_slot (16×16) inside a 26 px row.
                                    .w(px(theme::SIDEBAR_ARROW_SLOT))
                                    .h(px(theme::SIDEBAR_ROW_HEIGHT))
                                    .on_click(cx.listener({
                                        let path = node.folder.path.clone();
                                        move |this, _, _, cx| {
                                            this.toggle_directory(&path);
                                            cx.notify();
                                        }
                                    })),
                                )
                                .child(
                                    folder_row(
                                        format!("navigation-select-{}", node.folder.id),
                                        node.folder.name.clone(),
                                        node.media_count,
                                        self.session
                                            .snapshot()
                                            .map(|s| {
                                                s.selected_folder_id == Some(node.folder.id)
                                                    && s.selected_path.is_none()
                                            })
                                            .unwrap_or(false),
                                        if expanded.is_some() {
                                            theme::icons::FOLDER_OPEN
                                        } else {
                                            theme::icons::FOLDER
                                        },
                                    )
                                    // Source: sidebar::SIDEBAR_ROW_H and theme::FONT_SIZE_BUTTON.
                                    .flex_1()
                                    .min_w_0()
                                    .h(px(theme::SIDEBAR_ROW_HEIGHT))
                                    .text_size(px(theme::SOURCE_FONT_SIZE_BUTTON))
                                    .on_click(cx.listener({
                                        let folder_id = node.folder.id;
                                        move |this, _, _, cx| {
                                            this.select_folder(Some(folder_id), cx);
                                            cx.notify();
                                        }
                                    })),
                                )
                                .child({
                                    let folder_id = node.folder.id;
                                    let folder_path = node.folder.path.clone();
                                    let folder_view = cx.entity().downgrade();
                                    Button::new(format!("navigation-folder-menu-{folder_id}"))
                                        .opacity(0.)
                                        .group_hover(row_group, |style| style.opacity(1.))
                                        .label(photag_ui_bridge::i18n::tr(&("⋯").to_string()))
                                        .w(px(theme::SIDEBAR_ROW_HEIGHT))
                                        .h(px(theme::SIDEBAR_ROW_HEIGHT))
                                        .dropdown_menu(move |mut menu, _, _| {
                                            let create_path = folder_path.clone();
                                            let create_view = folder_view.clone();
                                            menu = menu.item(
                                                PopupMenuItem::new(photag_ui_bridge::i18n::tr(
                                                    "New subfolder…",
                                                ))
                                                .on_click(move |_, window, cx| {
                                                    let _ = create_view.update(cx, |this, cx| {
                                                        this.request_new_subfolder(
                                                            create_path.clone(),
                                                            window,
                                                            cx,
                                                        )
                                                    });
                                                }),
                                            );
                                            let rescan_view = folder_view.clone();
                                            menu = menu.item(
                                                PopupMenuItem::new(photag_ui_bridge::i18n::tr(
                                                    "Rescan",
                                                ))
                                                .on_click(move |_, _, cx| {
                                                    let _ = rescan_view.update(cx, |this, cx| {
                                                        this.rescan(folder_id, cx);
                                                        cx.notify();
                                                    });
                                                }),
                                            );
                                            let open_path = folder_path.clone();
                                            let open_view = folder_view.clone();
                                            menu = menu.item(
                                                PopupMenuItem::new(photag_ui_bridge::i18n::tr(
                                                    "Open in file manager",
                                                ))
                                                .on_click(move |_, _, cx| {
                                                    let _ = open_view.update(cx, |this, _| {
                                                        this.open_folder_in_explorer(&open_path)
                                                    });
                                                }),
                                            );
                                            let remove_view = folder_view.clone();
                                            menu.item(
                                                PopupMenuItem::new(photag_ui_bridge::i18n::tr(
                                                    "Remove",
                                                ))
                                                .on_click(move |_, _, cx| {
                                                    let _ = remove_view.update(cx, |this, cx| {
                                                        this.remove_folder(folder_id, cx);
                                                        cx.notify();
                                                    });
                                                }),
                                            )
                                        })
                                }),
                        );
                        if let Some(children) = expanded {
                            panel = panel.children(children.iter().map(|child| {
                                render_subdirectory(child, &self.expanded_directories, cx)
                            }));
                        }
                    }
                }
                if self.snapshot.is_ok() {
                    let tag_view = cx.entity().downgrade();
                    panel = panel.child(
                        div()
                            .h_flex()
                            .justify_between()
                            .items_center()
                            .pt_3()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(theme::TEXT_SECONDARY))
                                    .child(photag_ui_bridge::i18n::tr("Tags")),
                            )
                            .child(
                                div()
                                    .h_flex()
                                    .gap_1()
                                    .child(
                                        Button::new("navigation-toggle-tag-search")
                                            .ghost()
                                            .font_family(theme::ICON_FONT_FAMILY)
                                            .label(theme::icons::SEARCH)
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.show_tag_search_controls =
                                                    !this.show_tag_search_controls;
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Button::new("navigation-create-tag")
                                            .ghost()
                                            .label(photag_ui_bridge::i18n::tr("+"))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.tag_create_open = true;
                                                cx.notify();
                                            })),
                                    ),
                            ),
                    );
                    let prefs = self
                        .preferences
                        .read()
                        .map(|prefs| *prefs)
                        .unwrap_or_default();
                    self.tag_search_match = prefs.search_match;
                    if self.show_tag_search_controls {
                        let clear_view = tag_view.clone();
                        panel = panel.child(
                            div()
                                .v_flex()
                                .gap_1()
                                .child(Input::new(&self.tag_search))
                                .child(
                                    div()
                                        .h_flex()
                                        .gap_1()
                                        .child(
                                            Button::new("navigation-tag-mode-select")
                                                .label(photag_ui_bridge::i18n::tr(
                                                    &(if self.exclude_mode {
                                                        "Select"
                                                    } else {
                                                        "✓ Select"
                                                    })
                                                    .to_string(),
                                                ))
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.set_tag_click_mode(false, cx);
                                                })),
                                        )
                                        .child(
                                            Button::new("navigation-tag-mode-exclude")
                                                .label(photag_ui_bridge::i18n::tr(
                                                    &(if self.exclude_mode {
                                                        "⊘ Exclude"
                                                    } else {
                                                        "Exclude"
                                                    })
                                                    .to_string(),
                                                ))
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.set_tag_click_mode(true, cx);
                                                })),
                                        )
                                        .child(
                                            Button::new("navigation-tag-select-all")
                                                .label(photag_ui_bridge::i18n::tr("All"))
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    let tags = this
                                                        .snapshot
                                                        .as_ref()
                                                        .map(|snapshot| snapshot.tags.clone())
                                                        .unwrap_or_default();
                                                    this.select_all_tags(&tags, cx);
                                                })),
                                        )
                                        .child(
                                            Button::new("navigation-clear-tag-filter")
                                                .label(photag_ui_bridge::i18n::tr("Clear"))
                                                .on_click(move |_, _, cx| {
                                                    let _ = clear_view.update(cx, |this, cx| {
                                                        this.select_tag(None, cx);
                                                        cx.notify();
                                                    });
                                                }),
                                        ),
                                )
                                .child(div().text_xs().child(match prefs.exclude_modifier {
                                    TagExcludeModifier::None => "No exclude modifier",
                                    TagExcludeModifier::Ctrl => "Ctrl + click toggles other mode",
                                    TagExcludeModifier::Shift => "Shift + click toggles other mode",
                                    TagExcludeModifier::Alt => "Alt + click toggles other mode",
                                })),
                        );
                        if !self.included_tags.is_empty() || !self.excluded_tags.is_empty() {
                            panel = panel.child(
                                div()
                                    .h_flex()
                                    .gap_1()
                                    .child(
                                        Button::new("navigation-tag-clear-included")
                                            .label(photag_ui_bridge::i18n::tr(
                                                &(format!(
                                                    "Clear ✓ ({})",
                                                    self.included_tags.len()
                                                ))
                                                .to_string(),
                                            ))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.included_tags.clear();
                                                this.emit_tag_selection(cx);
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Button::new("navigation-tag-clear-excluded")
                                            .label(photag_ui_bridge::i18n::tr(
                                                &(format!(
                                                    "Clear ⊘ ({})",
                                                    self.excluded_tags.len()
                                                ))
                                                .to_string(),
                                            ))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.excluded_tags.clear();
                                                this.emit_tag_selection(cx);
                                                cx.notify();
                                            })),
                                    ),
                            );
                        }
                    }
                    let query = self.tag_search.read(cx).value().trim().to_owned();
                    panel.children(
                        snapshot
                            .tags
                            .iter()
                            .filter(|tag| tag_matches(tag, &query, self.tag_search_match))
                            .map(|tag| {
                                render_tag(
                                    tag,
                                    &self.expanded_tags,
                                    &query,
                                    self.tag_search_match,
                                    &self.included_tags,
                                    &self.excluded_tags,
                                    cx,
                                )
                            }),
                    )
                } else {
                    panel
                }
            }
            Err(error) => panel.child(
                div()
                    .text_sm()
                    .text_color(rgb(theme::DANGER))
                    .child(format!("Could not load library: {error}")),
            ),
        }
    }
}

pub(super) fn merge_target_buttons(
    view: &SidebarView,
    source_id: i64,
    cx: &mut Context<SidebarView>,
) -> Vec<Div> {
    let Some(snapshot) = view.snapshot.as_ref().ok() else {
        return Vec::new();
    };
    let mut excluded = HashSet::new();
    if let Some(source) = find_tag(&snapshot.tags, source_id) {
        collect_tag_ids(source, &mut excluded);
    }
    let query = view.merge_search.read(cx).value().trim().to_owned();
    let mut targets = Vec::new();
    fn walk(
        tags: &[photag_plugin_database::db::queries::Tag],
        excluded: &HashSet<i64>,
        query: &str,
        mode: TagSearchMatch,
        output: &mut Vec<(i64, String)>,
    ) {
        for tag in tags {
            if !excluded.contains(&tag.id) && tag_matches(tag, query, mode) {
                output.push((
                    tag.id,
                    format!(
                        "{}{}  ({})",
                        "  ".repeat(tag.full_path.matches('>').count()),
                        tag.name,
                        tag.media_count
                    ),
                ));
            }
            walk(&tag.children, excluded, query, mode, output);
        }
    }
    walk(
        &snapshot.tags,
        &excluded,
        &query,
        view.tag_search_match,
        &mut targets,
    );
    if targets.is_empty() {
        return vec![
            div()
                .text_xs()
                .child(photag_ui_bridge::i18n::tr("No matching tags")),
        ];
    }
    targets
        .into_iter()
        .map(|(id, label)| {
            div().child(
                Button::new(format!("navigation-merge-target-{id}"))
                    .label(photag_ui_bridge::i18n::tr(&(label).to_string()))
                    .w_full()
                    .on_click(cx.listener(move |this, _, _, cx| this.merge_tag_into(id, cx))),
            )
        })
        .collect()
}
