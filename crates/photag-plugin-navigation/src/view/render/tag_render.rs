use super::super::*;
use gpui_kit::assets::IconName;

pub(in crate::view) fn render_tag(
    tag: &photag_plugin_database::Tag,
    expanded: &HashSet<i64>,
    query: &str,
    search_match: TagSearchMatch,
    included: &[i64],
    excluded: &[i64],
    cx: &mut Context<SidebarView>,
) -> Div {
    let depth = tag.full_path.matches('>').count();
    let tag_id = tag.id;
    let is_included = included.contains(&tag_id);
    let is_excluded = excluded.contains(&tag_id);
    let selected = is_included || is_excluded;
    let [red, green, blue] = tag
        .color
        .as_deref()
        .and_then(crate::parse_hex_color)
        .unwrap_or([120, 120, 130]);
    let tag_color = (u32::from(red) << 16) | (u32::from(green) << 8) | u32::from(blue);
    let tag_icon = theme::tag_icon(tag.icon.as_deref());
    let row_group = format!("navigation-tag-row-{}", tag.id);
    let select_name_id = tag.id;
    let name = Button::new(format!("navigation-tag-name-{}", tag.id))
        .ghost()
        .compact()
        .flex_1()
        .min_w_0()
        .text_size(px(12.))
        .when(is_included, |button| {
            button.icon(theme::small_icon(IconName::Check))
        })
        .when(is_excluded, |button| {
            button.icon(theme::small_icon(IconName::CircleSlash))
        })
        .label(tag.name.clone())
        .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
            this.toggle_tag_selection(select_name_id, event.modifiers(), cx)
        }));
    let drag_name = div()
        .id(format!("navigation-tag-drag-{}", tag.id))
        .on_drag(TagDragPayload { source_id: tag.id }, |payload, _, _, cx| {
            cx.new(|_| TagDragGhost {
                label: format!("Move tag {}", payload.source_id),
            })
        })
        .child(name);
    let is_expanded = !query.is_empty() || expanded.contains(&tag.id);
    let toggle_id = tag.id;
    let toggle = Button::new(format!("navigation-tag-expand-{toggle_id}"))
        .ghost()
        .compact()
        .w(px(17.))
        .icon(theme::small_icon(if is_expanded {
            IconName::ChevronDown
        } else {
            IconName::ChevronRight
        }))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.toggle_tag(toggle_id);
            cx.notify();
        }));
    let menu_id = tag.id;
    let menu_display = tag.display;
    let view = cx.entity().downgrade();
    let actions = Button::new(format!("navigation-tag-actions-{menu_id}"))
        .compact()
        .opacity(0.72)
        .group_hover(row_group.clone(), |style| style.opacity(1.))
        .icon(theme::small_icon(IconName::Ellipsis))
        .accessibility_label(photag_ui_bridge::i18n::tr("Tag actions"))
        .dropdown_menu(move |mut menu, _, _| {
            let child_view = view.clone();
            menu = menu.item(
                PopupMenuItem::new(photag_ui_bridge::i18n::tr("Create child tag")).on_click(
                    move |_, window, cx| {
                        let _ = child_view.update(cx, |this, cx| {
                            this.parent_tag_id = Some(menu_id);
                            this.tag_parent
                                .update(cx, |state, cx| state.set_value("", window, cx));
                            this.tag_name
                                .update(cx, |state, cx| state.set_value("", window, cx));
                            this.tag_create_open = true;
                            cx.notify();
                        });
                    },
                ),
            );
            let merge_view = view.clone();
            menu = menu.item(
                PopupMenuItem::new(photag_ui_bridge::i18n::tr("Merge into…")).on_click(
                    move |_, window, cx| {
                        let _ = merge_view.update(cx, |this, cx| {
                            this.begin_merge(menu_id, window, cx);
                        });
                    },
                ),
            );
            let edit_view = view.clone();
            menu = menu.item(
                PopupMenuItem::new(photag_ui_bridge::i18n::tr("Edit fields")).on_click(
                    move |_, window, cx| {
                        let _ = edit_view.update(cx, |this, cx| {
                            this.begin_edit_tag(menu_id, window, cx);
                        });
                    },
                ),
            );
            let rename_view = view.clone();
            menu = menu.item(
                PopupMenuItem::new(photag_ui_bridge::i18n::tr("Rename…")).on_click(
                    move |_, window, cx| {
                        let _ = rename_view.update(cx, |this, cx| {
                            this.begin_rename_tag(menu_id, window, cx);
                        });
                    },
                ),
            );
            let display_view = view.clone();
            menu = menu.item(
                PopupMenuItem::new(if menu_display {
                    "Hide chips"
                } else {
                    "Show chips"
                })
                .on_click(move |_, _, cx| {
                    let _ = display_view.update(cx, |this, cx| {
                        this.toggle_tag_display(menu_id, !menu_display, cx);
                        cx.notify();
                    });
                }),
            );
            let parent_view = view.clone();
            menu = menu.item(
                PopupMenuItem::new(photag_ui_bridge::i18n::tr("Use as parent")).on_click(
                    move |_, window, cx| {
                        let _ = parent_view.update(cx, |this, cx| {
                            this.use_tag_as_parent(menu_id, window, cx);
                            this.tag_create_open = true;
                            cx.notify();
                        });
                    },
                ),
            );
            let move_view = view.clone();
            menu = menu.item(
                PopupMenuItem::new(photag_ui_bridge::i18n::tr("Move…")).on_click(
                    move |_, window, cx| {
                        let _ = move_view.update(cx, |this, cx| {
                            this.move_source_id = Some(menu_id);
                            this.move_target_tag_id = None;
                            this.tag_move_target.update(cx, |input, cx| {
                                input.set_value("", window, cx);
                            });
                            cx.notify();
                        });
                    },
                ),
            );
            let style_view = view.clone();
            menu = menu.item(
                PopupMenuItem::new(photag_ui_bridge::i18n::tr("Edit appearance…")).on_click(
                    move |_, window, cx| {
                        let _ = style_view.update(cx, |this, cx| {
                            this.begin_edit_tag(menu_id, window, cx);
                        });
                    },
                ),
            );
            let delete_view = view.clone();
            menu.item(
                PopupMenuItem::new(photag_ui_bridge::i18n::tr("Delete tag…")).on_click(
                    move |_, _, cx| {
                        let _ = delete_view.update(cx, |this, cx| {
                            this.request_delete_tag(menu_id, cx);
                            cx.notify();
                        });
                    },
                ),
            )
        });
    let children = if is_expanded {
        let rows = tag
            .children
            .iter()
            .filter(|child| tag_matches(child, query, search_match))
            .map(|child| render_tag(child, expanded, query, search_match, included, excluded, cx))
            .collect::<Vec<_>>();
        if rows.is_empty() {
            Vec::new()
        } else {
            vec![
                div()
                    .relative()
                    .child(
                        div()
                            .absolute()
                            .left(px(3. + depth as f32 * 12. + 9.))
                            .top(px(0.))
                            .bottom(px(0.))
                            .w(px(1.))
                            .bg(rgb(theme::accent_alpha(theme::TEXT_MUTED, 0.32))),
                    )
                    .children(rows),
            ]
        }
    } else {
        Vec::new()
    };
    let before_id = tag.id;
    let before = div()
        .w_full()
        .h(px(8.))
        .opacity(0.)
        .items_center()
        .justify_center()
        .text_xs()
        .text_color(rgb(theme::TEXT_MUTED))
        .child(photag_ui_bridge::i18n::tr("Drop above to place before"))
        .drag_over::<TagDragPayload>(|style, _, _, _| {
            style
                .opacity(1.)
                .bg(rgb(theme::accent_alpha(theme::accent(), 0.22)))
        })
        .on_drop::<TagDragPayload>(cx.listener(move |this, payload: &TagDragPayload, _, cx| {
            this.move_tag_to(
                payload.source_id,
                photag_plugin_database::TagDropPos::Before(before_id),
                cx,
            );
        }));
    let into_id = tag.id;
    let row =
        div()
            .v_flex()
            .relative()
            .child(
                div()
                    .group(row_group)
                    .relative()
                    .h_flex()
                    .items_center()
                    .ml(px(3. + depth as f32 * 12.))
                    .mr(px(3.))
                    .pl(px(6.))
                    .pr(px(5.))
                    .h(px(theme::SIDEBAR_ROW_HEIGHT))
                    .rounded(px(9.))
                    .bg(rgba((tag_color << 8) | 0x28))
                    .border_1()
                    .border_color(rgba((tag_color << 8) | 0x70))
                    .when(selected, |row| {
                        row.bg(rgba((tag_color << 8) | 0x72))
                            .border_color(rgba((tag_color << 8) | 0xc0))
                            .child(
                                div()
                                    .absolute()
                                    .left(px(1.))
                                    .top(px(5.))
                                    .w(px(3.))
                                    .h(px(16.))
                                    .rounded(px(1.5))
                                    .bg(rgb(tag_color)),
                            )
                    })
                    .child(
                        div()
                            .h_flex()
                            .flex_1()
                            .min_w_0()
                            .items_center()
                            .gap(px(5.))
                            .child(toggle)
                            .child(div().w(px(7.)).h(px(7.)).rounded_full().bg(rgb((u32::from(
                                red,
                            ) << 16)
                                | (u32::from(green) << 8)
                                | u32::from(blue))))
                            .child(
                                theme::small_icon(tag_icon).text_color(rgb(theme::TEXT_SECONDARY)),
                            )
                            .child(drag_name.flex_1().min_w_0())
                            .drag_over::<TagDragPayload>(move |style, _, _, _| {
                                style.bg(rgba((tag_color << 8) | 0x50))
                            })
                            .on_drop::<TagDragPayload>(cx.listener(
                                move |this, payload: &TagDragPayload, _, cx| {
                                    this.move_tag_to(
                                        payload.source_id,
                                        photag_plugin_database::TagDropPos::Into(into_id),
                                        cx,
                                    );
                                },
                            )),
                    )
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap(px(3.))
                            .flex_shrink_0()
                            .child(
                                div()
                                    .w(px(28.))
                                    .h_flex()
                                    .items_center()
                                    .justify_end()
                                    .flex_shrink_0()
                                    .px(px(5.))
                                    .py(px(1.))
                                    .rounded(px(7.))
                                    .bg(rgb(theme::SURFACE_ELEVATED))
                                    .text_size(px(10.))
                                    .text_color(rgb(theme::TEXT_MUTED))
                                    .child(tag.media_count.to_string()),
                            )
                            .child(actions),
                    ),
            )
            .when(depth > 0, |row| {
                row.child(
                    div()
                        .absolute()
                        .left(px(3. + (depth - 1) as f32 * 12. + 9.))
                        .top(px(8. + theme::SIDEBAR_ROW_HEIGHT / 2.))
                        .w(px(8.))
                        .h(px(1.))
                        .bg(rgb(theme::accent_alpha(theme::TEXT_MUTED, 0.32))),
                )
            })
            .children(children);
    let after_id = tag.id;
    let after = div()
        .w_full()
        .h(px(8.))
        .opacity(0.)
        .items_center()
        .justify_center()
        .text_xs()
        .text_color(rgb(theme::TEXT_MUTED))
        .child(photag_ui_bridge::i18n::tr("Drop below to place after"))
        .drag_over::<TagDragPayload>(|style, _, _, _| {
            style
                .opacity(1.)
                .bg(rgb(theme::accent_alpha(theme::accent(), 0.22)))
        })
        .on_drop::<TagDragPayload>(cx.listener(move |this, payload: &TagDragPayload, _, cx| {
            this.move_tag_to(
                payload.source_id,
                photag_plugin_database::TagDropPos::After(after_id),
                cx,
            );
        }));
    div().v_flex().child(before).child(row).child(after)
}

struct TagDragGhost {
    label: String,
}

impl Render for TagDragGhost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(rgb(theme::SURFACE_PANEL))
            .text_sm()
            .child(self.label.clone())
    }
}

pub(in crate::view) fn render_subdirectory(
    node: &SubdirectoryNode,
    expanded: &HashMap<String, Vec<SubdirectoryNode>>,
    query: &str,
    selected_directory: Option<&str>,
    counts_known: bool,
    cx: &mut Context<SidebarView>,
) -> Option<Div> {
    let query = query.trim().to_lowercase();
    if !query.is_empty() && !directory_matches(node, expanded, &query) {
        return None;
    }
    let children = expanded
        .get(&node.path)
        .map(|children| {
            children
                .iter()
                .filter_map(|child| {
                    render_subdirectory(
                        child,
                        expanded,
                        &query,
                        selected_directory,
                        counts_known,
                        cx,
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let selected_path = node.path.clone();
    let menu_path = node.path.clone();
    let row_group = format!("navigation-directory-row-{}", node.path);
    let view = cx.entity().downgrade();
    let menu = Button::new(format!("navigation-directory-menu-{}", node.path))
        .absolute()
        .right(px(2.))
        .top(px(2.))
        .ghost()
        .compact()
        .opacity(0.)
        .group_hover(row_group.clone(), |style| style.opacity(1.))
        .icon(theme::small_icon(IconName::Ellipsis))
        .accessibility_label(photag_ui_bridge::i18n::tr("Folder actions"))
        .w(px(20.))
        .h(px(22.))
        .bg(rgb(theme::SURFACE_BASE))
        .dropdown_menu(move |mut menu, _, _| {
            let create_path = menu_path.clone();
            let create_view = view.clone();
            menu = menu.item(
                PopupMenuItem::new(photag_ui_bridge::i18n::tr("New subfolder…")).on_click(
                    move |_, window, cx| {
                        let _ = create_view.update(cx, |this, cx| {
                            this.request_new_subfolder(create_path.clone(), window, cx)
                        });
                    },
                ),
            );
            let open_path = menu_path.clone();
            let open_view = view.clone();
            menu.item(
                PopupMenuItem::new(photag_ui_bridge::i18n::tr("Open in file manager")).on_click(
                    move |_, _, cx| {
                        let _ = open_view
                            .update(cx, |this, _| this.open_folder_in_explorer(&open_path));
                    },
                ),
            )
        });
    let selected = selected_directory
        .is_some_and(|path| crate::sidebar_folders::same_sidebar_path(path, &node.path));
    let expand = if node.has_children {
        let directory_path = node.path.clone();
        Button::new(format!("navigation-expand-dir-{}", node.path))
            .ghost()
            .compact()
            .w(px(18.))
            .icon(theme::small_icon(
                if expanded.contains_key(&node.path) || !query.is_empty() {
                    IconName::ChevronDown
                } else {
                    IconName::ChevronRight
                },
            ))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.toggle_directory(&directory_path);
                cx.notify();
            }))
    } else {
        Button::new(format!("navigation-expand-dir-{}", node.path))
            .ghost()
            .compact()
            .w(px(18.))
            .label(" ")
    };
    let row =
        div()
            .group(row_group)
            .relative()
            .h_flex()
            .w_full()
            .h(px(26.))
            .items_center()
            .gap(px(3.))
            .rounded(px(4.))
            .px(px(3.))
            .hover(|style| style.bg(rgba(0xffffff10)))
            .when(selected, |row| row.bg(rgba((theme::accent() << 8) | 70)))
            .child(div().w(px(8.)).h(px(1.)).bg(rgba(0xffffff38)))
            .child(expand)
            .child(
                div()
                    .id(format!("navigation-open-dir-{}", node.path))
                    .h_flex()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .items_center()
                    .gap(px(6.))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.select_path(selected_path.clone(), cx)
                    }))
                    .child(theme::small_icon(if expanded.contains_key(&node.path) {
                        IconName::FolderOpen
                    } else {
                        IconName::Folder
                    }))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_xs()
                            .text_ellipsis()
                            .child(node.name.clone()),
                    )
                    .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(
                        if counts_known {
                            node.media_count.to_string()
                        } else {
                            String::new()
                        },
                    )),
            );
    let row = row.child(menu);
    let mut subtree = div().v_flex().child(row);
    if !children.is_empty() {
        subtree = subtree.child(
            div()
                .v_flex()
                .ml(px(9.))
                .pl(px(9.))
                .border_l_1()
                .border_color(rgba(0xffffff38))
                .children(children),
        );
    }
    Some(subtree)
}

pub(in crate::view) fn directory_children_match(
    path: &str,
    expanded: &HashMap<String, Vec<SubdirectoryNode>>,
    query: &str,
) -> bool {
    expanded.get(path).is_some_and(|children| {
        children
            .iter()
            .any(|child| directory_matches(child, expanded, query))
    })
}

fn directory_matches(
    node: &SubdirectoryNode,
    expanded: &HashMap<String, Vec<SubdirectoryNode>>,
    query: &str,
) -> bool {
    node.name.to_lowercase().contains(query)
        || node.path.to_lowercase().contains(query)
        || directory_children_match(&node.path, expanded, query)
}

pub(in crate::view) fn merge_target_buttons(
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
        tags: &[photag_plugin_database::Tag],
        excluded: &HashSet<i64>,
        query: &str,
        mode: TagSearchMatch,
        output: &mut Vec<(i64, String, usize, i64)>,
    ) {
        for tag in tags {
            if !excluded.contains(&tag.id) && tag_matches(tag, query, mode) {
                output.push((
                    tag.id,
                    tag.name.clone(),
                    tag.full_path.matches('>').count(),
                    tag.media_count,
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
        .map(|(id, name, depth, count)| {
            div().child(
                Button::new(format!("navigation-merge-target-{id}"))
                    .ghost()
                    .w_full()
                    .pl(px(8. + depth as f32 * 14.))
                    .label(photag_ui_bridge::i18n::tr(&format!("{}  ({count})", name)))
                    .on_click(cx.listener(move |this, _, _, cx| this.merge_tag_into(id, cx))),
            )
        })
        .collect()
}
