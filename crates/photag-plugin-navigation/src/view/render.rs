use super::*;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::scroll::ScrollableElement;

mod tag_render;
use tag_render::{render_subdirectory, render_tag};

impl Render for SidebarView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
            .pb(px(8.))
            .child(div().h_flex().w_full().gap_0().h(px(32.)).children(
                [(SidebarTab::Folders, "Folders"), (SidebarTab::Tags, "Tags")].map(
                    |(tab, name)| {
                        let active = self.active_tab == tab;
                        let mut pivot = div().relative().flex_1().min_w(px(60.)).h(px(32.)).child(
                            Button::new(format!("navigation-tab-{name}"))
                                .ghost()
                                .compact()
                                .w_full()
                                .h(px(32.))
                                .text_size(px(13.))
                                .label(photag_ui_bridge::i18n::tr(name))
                                .text_color(rgb(if active {
                                    theme::TEXT_PRIMARY
                                } else {
                                    theme::TEXT_SECONDARY
                                }))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.switch_tab(tab, window, cx)
                                })),
                        );
                        if active {
                            pivot = pivot.child(
                                div()
                                    .absolute()
                                    .bottom_0()
                                    .left(relative(0.28))
                                    .right(relative(0.28))
                                    .h(px(3.))
                                    .rounded(px(1.5))
                                    .bg(rgb(theme::accent())),
                            );
                        }
                        pivot
                    },
                ),
            ))
            .child(div().h(px(6.)));
        match &self.snapshot {
            Ok(snapshot) => {
                if self.active_tab == SidebarTab::Folders {
                    panel = panel.child(
                        div()
                            .h_flex()
                            .items_center()
                            .justify_between()
                            .pt_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(theme::TEXT_SECONDARY))
                                    .child(photag_ui_bridge::i18n::tr("Folders")),
                            )
                            .child(
                                div()
                                    .h_flex()
                                    .gap_1()
                                    .child(
                                        Button::new("navigation-folder-path-toggle")
                                            .label(photag_ui_bridge::i18n::tr(
                                                &(if self.show_folder_path_entry {
                                                    "Cancel"
                                                } else {
                                                    "Path"
                                                })
                                                .to_string(),
                                            ))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.show_folder_path_entry =
                                                    !this.show_folder_path_entry;
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Button::new("navigation-add-folder")
                                            .compact()
                                            .w(px(22.))
                                            .h(px(22.))
                                            .rounded(px(5.))
                                            .bg(rgb(theme::accent()))
                                            .label(photag_ui_bridge::i18n::tr(&("+").to_string()))
                                            .on_click(
                                                cx.listener(|this, _, _, cx| this.pick_folder(cx)),
                                            ),
                                    )
                                    .child(
                                        Button::new("navigation-refresh")
                                            .label(photag_ui_bridge::i18n::tr(&("↻").to_string()))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.refresh();
                                                cx.notify();
                                            })),
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
                    panel = panel.child(
                        Button::new("navigation-all-media")
                            .label(photag_ui_bridge::i18n::tr("All Media"))
                            .on_click(cx.listener(|this, _, _, cx| this.select_folder(None, cx))),
                    );
                    for node in &snapshot.folders {
                        let expanded = self.expanded_directories.get(&node.folder.path);
                        panel = panel.child(
                            div()
                                .h_flex()
                                .w_full()
                                .items_center()
                                .gap_0()
                                .child(
                                    Button::new(format!(
                                        "navigation-expand-root-{}",
                                        node.folder.id
                                    ))
                                    .label(photag_ui_bridge::i18n::tr(
                                        &(if expanded.is_some() { "⌄" } else { "›" }).to_string(),
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
                                    Button::new(format!("navigation-select-{}", node.folder.id))
                                        .label(photag_ui_bridge::i18n::tr(
                                            &(format!(
                                                "{}  {}",
                                                node.folder.name, node.media_count
                                            ))
                                            .to_string(),
                                        ))
                                        // Source: sidebar::SIDEBAR_ROW_H and theme::FONT_SIZE_BUTTON.
                                        .flex_1()
                                        .min_w_0()
                                        .h(px(theme::SIDEBAR_ROW_HEIGHT))
                                        .text_size(px(theme::SOURCE_FONT_SIZE_BUTTON))
                                        .on_click(cx.listener({
                                            let folder_id = node.folder.id;
                                            move |this, _, _, cx| {
                                                this.select_folder(Some(folder_id), cx)
                                            }
                                        })),
                                )
                                .child({
                                    let folder_id = node.folder.id;
                                    let folder_path = node.folder.path.clone();
                                    let folder_view = cx.entity().downgrade();
                                    Button::new(format!("navigation-folder-menu-{folder_id}"))
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
                    if let Some(progress) = &self.scan_progress {
                        panel = panel.child(
                            div()
                                .text_xs()
                                .text_color(rgb(theme::TEXT_SECONDARY))
                                .child(format!(
                                    "{}: {} / {} files",
                                    progress.phase, progress.scanned, progress.total
                                )),
                        );
                    }
                }
                if self.active_tab == SidebarTab::Tags {
                    panel = panel.child(
                        div()
                            .h_flex()
                            .justify_between()
                            .pt_3()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(theme::TEXT_SECONDARY))
                                    .child(photag_ui_bridge::i18n::tr("Tags")),
                            )
                            .child(
                                Button::new("navigation-clear-tag-filter")
                                    .label(photag_ui_bridge::i18n::tr("Clear"))
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.select_tag(None, cx)),
                                    ),
                            ),
                    );
                    let prefs = self
                        .preferences
                        .read()
                        .map(|prefs| *prefs)
                        .unwrap_or_default();
                    self.tag_search_match = prefs.search_match;
                    panel = panel.child(Input::new(&self.tag_search));
                    panel = panel.child(
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
                                            &(format!("Clear ✓ ({})", self.included_tags.len()))
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
                                            &(format!("Clear ⊘ ({})", self.excluded_tags.len()))
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
                    panel = panel.child(
                        div()
                            .text_xs()
                            .text_color(rgb(theme::TEXT_SECONDARY))
                            .child(photag_ui_bridge::i18n::tr("Tag management")),
                    );
                    panel = panel.child(
                        div()
                            .h_flex()
                            .gap_1()
                            .child(Input::new(&self.tag_name).flex_1())
                            .child(Input::new(&self.tag_parent).flex_1())
                            .child(
                                Button::new("navigation-create-tag")
                                    .label(photag_ui_bridge::i18n::tr("Create"))
                                    .on_click(cx.listener(|this, _, _, cx| this.create_tag(cx))),
                            ),
                    );
                    panel = panel.child(
                        div()
                            .h_flex()
                            .gap_1()
                            .child(Input::new(&self.tag_edit_name).flex_1())
                            .child(Input::new(&self.tag_color).flex_1())
                            .child(Input::new(&self.tag_icon).flex_1()),
                    );
                    if let Some(edited_id) = self.associated_edit_tag {
                        let mut rows = Vec::new();
                        collect_association_rows(&snapshot.tags, edited_id, &mut rows);
                        let mut associations = div()
                            .v_flex()
                            .gap_1()
                            .max_h(px(120.))
                            .overflow_y_scrollbar()
                            .child(photag_ui_bridge::i18n::tr(
                                "Associated tags: assigning this tag also assigns:",
                            ));
                        for (id, path) in rows {
                            let checked = self.associated_tag_ids.contains(&id);
                            associations = associations.child(
                                Button::new(format!("navigation-associated-tag-{id}"))
                                    .label(photag_ui_bridge::i18n::tr(
                                        &(format!("{} {path}", if checked { "☑" } else { "☐" }))
                                            .to_string(),
                                    ))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.toggle_associated_tag(id, cx)
                                    })),
                            );
                        }
                        panel = panel.child(associations);
                    }
                    panel = panel.child(
                        div()
                            .h_flex()
                            .gap_1()
                            .child(
                                Button::new("navigation-tag-appearance-children")
                                    .label(photag_ui_bridge::i18n::tr(
                                        &(if self.apply_tag_appearance_to_children {
                                            "✓ Apply to children"
                                        } else {
                                            "Apply to children"
                                        })
                                        .to_string(),
                                    ))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.apply_tag_appearance_to_children =
                                            !this.apply_tag_appearance_to_children;
                                        cx.notify();
                                    })),
                            )
                            .when(self.apply_tag_appearance_to_children, |panel| {
                                panel.child(
                                    Button::new("navigation-tag-appearance-skip-edited")
                                        .label(photag_ui_bridge::i18n::tr(
                                            &(if self.exclude_edited_tag_children {
                                                "✓ Skip edited children"
                                            } else {
                                                "Skip edited children"
                                            })
                                            .to_string(),
                                        ))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.exclude_edited_tag_children =
                                                !this.exclude_edited_tag_children;
                                            cx.notify();
                                        })),
                                )
                            }),
                    );
                    panel = panel.child(
                        div()
                            .h_flex()
                            .gap_1()
                            .child(Input::new(&self.tag_move_target).flex_1())
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(theme::TEXT_SECONDARY))
                                    .child(photag_ui_bridge::i18n::tr(
                                        "Set fields, then use a tag row action",
                                    )),
                            ),
                    );
                    if let Some((tag_id, tag_name, count)) = self.confirm_delete.clone() {
                        panel = panel.child(
                            div()
                                .v_flex()
                                .gap_1()
                                .p_2()
                                .child(format!(
                                    "Delete ‘{tag_name}’? It is assigned to {count} media items."
                                ))
                                .child(photag_ui_bridge::i18n::tr(
                                    "Also remove parent tags from those media?",
                                ))
                                .child(
                                    div()
                                        .h_flex()
                                        .gap_1()
                                        .child(
                                            Button::new("navigation-delete-tag-clean")
                                                .label(photag_ui_bridge::i18n::tr(
                                                    &("Remove parent tags too").to_string(),
                                                ))
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.confirm_delete = None;
                                                    this.delete_tag(tag_id, true, cx);
                                                })),
                                        )
                                        .child(
                                            Button::new("navigation-delete-tag-keep")
                                                .label(photag_ui_bridge::i18n::tr(
                                                    &("Keep parent tags").to_string(),
                                                ))
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.confirm_delete = None;
                                                    this.delete_tag(tag_id, false, cx);
                                                })),
                                        )
                                        .child(
                                            Button::new("navigation-delete-tag-cancel")
                                                .label(photag_ui_bridge::i18n::tr("Cancel"))
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.confirm_delete = None;
                                                    cx.notify();
                                                })),
                                        ),
                                ),
                        );
                    }
                    if let Some(source_id) = self.merge_source {
                        let source_name = self
                            .snapshot
                            .as_ref()
                            .ok()
                            .and_then(|snapshot| find_tag(&snapshot.tags, source_id))
                            .map(|tag| tag.name.clone())
                            .unwrap_or_else(|| "tag".into());
                        panel = panel.child(
                            div()
                                .v_flex()
                                .gap_1()
                                .p_2()
                                .child(format!("Merge ‘{source_name}’ into…"))
                                .child(Input::new(&self.merge_search))
                                .child(
                                    Button::new("navigation-merge-children-mode")
                                        .label(photag_ui_bridge::i18n::tr(
                                            &(if self.merge_reparent_children {
                                                "Children will be re-parented under target"
                                            } else {
                                                "Children will be deleted"
                                            })
                                            .to_string(),
                                        ))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.merge_reparent_children =
                                                !this.merge_reparent_children;
                                            cx.notify();
                                        })),
                                )
                                .children(merge_target_buttons(self, source_id, cx))
                                .child(
                                    Button::new("navigation-merge-cancel")
                                        .label(photag_ui_bridge::i18n::tr("Cancel"))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.merge_source = None;
                                            cx.notify();
                                        })),
                                ),
                        );
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

fn collect_association_rows(
    tags: &[photag_plugin_database::db::queries::Tag],
    excluded_id: i64,
    rows: &mut Vec<(i64, String)>,
) {
    for tag in tags {
        if tag.id != excluded_id {
            rows.push((tag.id, tag.full_path.replace('>', " > ")));
        }
        collect_association_rows(&tag.children, excluded_id, rows);
    }
}

fn merge_target_buttons(
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
