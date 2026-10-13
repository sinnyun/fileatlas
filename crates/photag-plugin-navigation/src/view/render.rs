use super::*;

mod tag_render;
use tag_render::{render_subdirectory, render_tag};

impl Render for SidebarView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
        let mut panel = div()
            .v_flex()
            .size_full()
            .gap_1()
            .bg(rgb(theme::SURFACE_BASE))
            .p_3()
            .child(div().text_sm().font_semibold().child("LIBRARY"));
        panel = panel.child(
            div()
                .h_flex()
                .gap_1()
                .child(
                    Button::new("navigation-tab-folders")
                        .label(if self.active_tab == SidebarTab::Folders {
                            "Folders ✓"
                        } else {
                            "Folders"
                        })
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.switch_tab(SidebarTab::Folders, window, cx)
                        })),
                )
                .child(
                    Button::new("navigation-tab-tags")
                        .label(if self.active_tab == SidebarTab::Tags {
                            "Tags ✓"
                        } else {
                            "Tags"
                        })
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.switch_tab(SidebarTab::Tags, window, cx)
                        })),
                ),
        );
        match &self.snapshot {
            Ok(snapshot) => {
                if self.active_tab == SidebarTab::Folders {
                    panel = panel.child(
                        div()
                            .h_flex()
                            .justify_between()
                            .pt_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(theme::TEXT_SECONDARY))
                                    .child("Folders"),
                            )
                            .child(Button::new("navigation-refresh").label("Refresh").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.refresh();
                                    cx.notify();
                                }),
                            )),
                    );
                    panel = panel.child(
                        div()
                            .h_flex()
                            .gap_1()
                            .child(Input::new(&self.folder_path).flex_1())
                            .child(
                                Button::new("navigation-browse-folder")
                                    .label("Browse…")
                                    .on_click(cx.listener(|this, _, _, cx| this.pick_folder(cx))),
                            )
                            .child(
                                Button::new("navigation-add-folder")
                                    .label("Add")
                                    .on_click(cx.listener(|this, _, _, cx| this.add_folder(cx))),
                            ),
                    );
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
                                                .label("Create")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.create_subfolder(cx)
                                                })),
                                        )
                                        .child(
                                            Button::new("navigation-cancel-subfolder")
                                                .label("Cancel")
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
                            .label("All Media")
                            .on_click(cx.listener(|this, _, _, cx| this.select_folder(None, cx))),
                    );
                    for node in &snapshot.folders {
                        let expanded = self.expanded_directories.get(&node.folder.path);
                        panel = panel.child(
                            div()
                                .v_flex()
                                .gap_1()
                                .child(
                                    div().w_full().rounded_md().px_2().py_1().text_sm().child(
                                        format!("{}  {}", node.folder.name, node.media_count),
                                    ),
                                )
                                .child(
                                    div()
                                        .h_flex()
                                        .gap_1()
                                        .child(
                                            Button::new(format!(
                                                "navigation-expand-root-{}",
                                                node.folder.id
                                            ))
                                            .label(if expanded.is_some() {
                                                "Collapse"
                                            } else {
                                                "Expand"
                                            })
                                            .on_click(cx.listener({
                                                let path = node.folder.path.clone();
                                                move |this, _, _, cx| {
                                                    this.toggle_directory(&path);
                                                    cx.notify();
                                                }
                                            })),
                                        )
                                        .child(
                                            Button::new(format!(
                                                "navigation-select-{}",
                                                node.folder.id
                                            ))
                                            .label("Open")
                                            .on_click(cx.listener({
                                                let folder_id = node.folder.id;
                                                move |this, _, _, cx| {
                                                    this.select_folder(Some(folder_id), cx)
                                                }
                                            })),
                                        )
                                        .child(
                                            Button::new(format!(
                                                "navigation-rescan-{}",
                                                node.folder.id
                                            ))
                                            .label("Rescan")
                                            .on_click(cx.listener({
                                                let folder_id = node.folder.id;
                                                move |this, _, _, cx| {
                                                    this.rescan(folder_id, cx);
                                                    cx.notify();
                                                }
                                            })),
                                        )
                                        .child(
                                            Button::new(format!(
                                                "navigation-remove-{}",
                                                node.folder.id
                                            ))
                                            .label("Remove")
                                            .on_click(cx.listener({
                                                let folder_id = node.folder.id;
                                                move |this, _, _, cx| {
                                                    this.remove_folder(folder_id, cx);
                                                    cx.notify();
                                                }
                                            })),
                                        )
                                        .child({
                                            let folder_id = node.folder.id;
                                            let folder_path = node.folder.path.clone();
                                            let folder_view = cx.entity().downgrade();
                                            Button::new(format!(
                                                "navigation-folder-menu-{folder_id}"
                                            ))
                                            .label("⋯")
                                            .dropdown_menu(move |mut menu, _, _| {
                                                let create_path = folder_path.clone();
                                                let create_view = folder_view.clone();
                                                menu = menu.item(
                                                    PopupMenuItem::new("New subfolder…").on_click(
                                                        move |_, window, cx| {
                                                            let _ = create_view.update(
                                                                cx,
                                                                |this, cx| {
                                                                    this.request_new_subfolder(
                                                                        create_path.clone(),
                                                                        window,
                                                                        cx,
                                                                    )
                                                                },
                                                            );
                                                        },
                                                    ),
                                                );
                                                let rescan_view = folder_view.clone();
                                                menu = menu.item(
                                                    PopupMenuItem::new("Rescan").on_click(
                                                        move |_, _, cx| {
                                                            let _ = rescan_view.update(
                                                                cx,
                                                                |this, cx| {
                                                                    this.rescan(folder_id, cx);
                                                                    cx.notify();
                                                                },
                                                            );
                                                        },
                                                    ),
                                                );
                                                let open_path = folder_path.clone();
                                                let open_view = folder_view.clone();
                                                menu = menu.item(
                                                    PopupMenuItem::new("Open in file manager")
                                                        .on_click(move |_, _, cx| {
                                                            let _ =
                                                                open_view.update(cx, |this, _| {
                                                                    this.open_folder_in_explorer(
                                                                        &open_path,
                                                                    )
                                                                });
                                                        }),
                                                );
                                                let remove_view = folder_view.clone();
                                                menu.item(PopupMenuItem::new("Remove").on_click(
                                                    move |_, _, cx| {
                                                        let _ =
                                                            remove_view.update(cx, |this, cx| {
                                                                this.remove_folder(folder_id, cx);
                                                                cx.notify();
                                                            });
                                                    },
                                                ))
                                            })
                                        }),
                                ),
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
                                    .child("Tags"),
                            )
                            .child(
                                Button::new("navigation-clear-tag-filter")
                                    .label("Clear")
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
                                    .label(if self.exclude_mode {
                                        "Select"
                                    } else {
                                        "✓ Select"
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.exclude_mode = false;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("navigation-tag-mode-exclude")
                                    .label(if self.exclude_mode {
                                        "⊘ Exclude"
                                    } else {
                                        "Exclude"
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.exclude_mode = true;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("navigation-tag-select-all")
                                    .label("All")
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
                                        .label(format!("Clear ✓ ({})", self.included_tags.len()))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.included_tags.clear();
                                            this.emit_tag_selection(cx);
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("navigation-tag-clear-excluded")
                                        .label(format!("Clear ⊘ ({})", self.excluded_tags.len()))
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
                            .child("Tag management"),
                    );
                    panel = panel.child(
                        div()
                            .h_flex()
                            .gap_1()
                            .child(Input::new(&self.tag_name).flex_1())
                            .child(Input::new(&self.tag_parent).flex_1())
                            .child(
                                Button::new("navigation-create-tag")
                                    .label("Create")
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
                    panel = panel.child(
                        div()
                            .h_flex()
                            .gap_1()
                            .child(
                                Button::new("navigation-tag-appearance-children")
                                    .label(if self.apply_tag_appearance_to_children {
                                        "✓ Apply to children"
                                    } else {
                                        "Apply to children"
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.apply_tag_appearance_to_children =
                                            !this.apply_tag_appearance_to_children;
                                        cx.notify();
                                    })),
                            )
                            .when(self.apply_tag_appearance_to_children, |panel| {
                                panel.child(
                                    Button::new("navigation-tag-appearance-skip-edited")
                                        .label(if self.exclude_edited_tag_children {
                                            "✓ Skip edited children"
                                        } else {
                                            "Skip edited children"
                                        })
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
                                    .child("Set fields, then use a tag row action"),
                            ),
                    );
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
