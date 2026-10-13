use super::super::*;

pub(super) fn render_tag(
    tag: &photag_plugin_database::db::queries::Tag,
    expanded: &HashSet<i64>,
    query: &str,
    search_match: TagSearchMatch,
    included: &[i64],
    excluded: &[i64],
    cx: &mut Context<SidebarView>,
) -> Div {
    let indent = "  ".repeat(tag.full_path.matches('>').count());
    let tag_id = tag.id;
    let is_included = included.contains(&tag_id);
    let is_excluded = excluded.contains(&tag_id);
    let selected = is_included || is_excluded;
    let [red, green, blue] = tag
        .color
        .as_deref()
        .and_then(crate::parse_hex_color)
        .unwrap_or([120, 120, 130]);
    let tag_icon = tag.icon.as_deref().filter(|icon| !icon.is_empty());
    let row_group = format!("navigation-tag-row-{}", tag.id);
    let select_name_id = tag.id;
    let name = Button::new(format!("navigation-tag-name-{}", tag.id))
        .ghost()
        .compact()
        .flex_1()
        .min_w_0()
        .text_size(px(12.))
        .label(format!(
            "{indent}{}{}  {}",
            tag.name,
            if is_included {
                " ✓"
            } else if is_excluded {
                " ⊘"
            } else {
                ""
            },
            tag.media_count
        ))
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
        .label(photag_ui_bridge::i18n::tr(
            &(if is_expanded {
                theme::icons::CHEVRON_DOWN
            } else {
                theme::icons::CHEVRON_RIGHT
            })
            .to_string(),
        ))
        .font_family(theme::ICON_FONT_FAMILY)
        .text_size(px(8.))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.toggle_tag(toggle_id);
            cx.notify();
        }));
    let menu_id = tag.id;
    let menu_display = tag.display;
    let view = cx.entity().downgrade();
    let actions = Button::new(format!("navigation-tag-actions-{menu_id}"))
        .compact()
        .opacity(0.)
        .group_hover(row_group.clone(), |style| style.opacity(1.))
        .label(photag_ui_bridge::i18n::tr(&("⋯").to_string()))
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
        tag.children
            .iter()
            .filter(|child| tag_matches(child, query, search_match))
            .map(|child| render_tag(child, expanded, query, search_match, included, excluded, cx))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let before_id = tag.id;
    let before = div()
        .w_full()
        .h(px(20.))
        .items_center()
        .justify_center()
        .text_xs()
        .text_color(rgb(theme::TEXT_MUTED))
        .child(photag_ui_bridge::i18n::tr("Drop above to place before"))
        .drag_over::<TagDragPayload>(|style, _, _, _| {
            style.bg(rgb(theme::accent_alpha(theme::accent(), 0.22)))
        })
        .on_drop::<TagDragPayload>(cx.listener(move |this, payload: &TagDragPayload, _, cx| {
            this.move_tag_to(
                payload.source_id,
                photag_plugin_database::db::queries::TagDropPos::Before(before_id),
                cx,
            );
        }));
    let into_id = tag.id;
    let row = div()
        .v_flex()
        .child(
            div()
                .group(row_group)
                .relative()
                .h_flex()
                .items_center()
                .rounded(px(4.))
                .when(selected, |row| {
                    row.bg(rgba(theme::accent_alpha(theme::accent(), 65. / 255.)))
                        .child(
                            div()
                                .absolute()
                                .left(px(1.))
                                .top(px(5.))
                                .w(px(3.))
                                .h(px(16.))
                                .rounded(px(1.5))
                                .bg(rgb(theme::accent())),
                        )
                })
                .child(
                    div()
                        .h_flex()
                        .flex_1()
                        .min_w_0()
                        .items_center()
                        .gap_1()
                        .child(toggle)
                        .child(div().w_2().h_2().rounded_sm().bg(rgb((u32::from(red) << 16)
                            | (u32::from(green) << 8)
                            | u32::from(blue))))
                        .when_some(tag_icon, |row, icon| {
                            row.child(div().text_size(px(10.)).child(icon.to_owned()))
                        })
                        .child(drag_name)
                        .drag_over::<TagDragPayload>(|style, _, _, _| {
                            style.bg(rgb(theme::accent_alpha(theme::accent(), 0.22)))
                        })
                        .on_drop::<TagDragPayload>(cx.listener(
                            move |this, payload: &TagDragPayload, _, cx| {
                                this.move_tag_to(
                                    payload.source_id,
                                    photag_plugin_database::db::queries::TagDropPos::Into(into_id),
                                    cx,
                                );
                            },
                        )),
                )
                .child(actions),
        )
        .children(children);
    let after_id = tag.id;
    let after = div()
        .w_full()
        .h(px(20.))
        .items_center()
        .justify_center()
        .text_xs()
        .text_color(rgb(theme::TEXT_MUTED))
        .child(photag_ui_bridge::i18n::tr("Drop below to place after"))
        .drag_over::<TagDragPayload>(|style, _, _, _| {
            style.bg(rgb(theme::accent_alpha(theme::accent(), 0.22)))
        })
        .on_drop::<TagDragPayload>(cx.listener(move |this, payload: &TagDragPayload, _, cx| {
            this.move_tag_to(
                payload.source_id,
                photag_plugin_database::db::queries::TagDropPos::After(after_id),
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

pub(super) fn render_subdirectory(
    node: &SubdirectoryNode,
    expanded: &HashMap<String, Vec<SubdirectoryNode>>,
    cx: &mut Context<SidebarView>,
) -> Div {
    let children = expanded
        .get(&node.path)
        .map(|children| {
            children
                .iter()
                .map(|child| render_subdirectory(child, expanded, cx))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let selected_path = node.path.clone();
    let menu_path = node.path.clone();
    let row_group = format!("navigation-directory-row-{}", node.path);
    let view = cx.entity().downgrade();
    let menu = Button::new(format!("navigation-directory-menu-{}", node.path))
        .opacity(0.)
        .group_hover(row_group.clone(), |style| style.opacity(1.))
        .label(photag_ui_bridge::i18n::tr(&("⋯").to_string()))
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
    let mut row = div().group(row_group).h_flex().pl_3().gap_1().child(
        Button::new(format!("navigation-open-dir-{}", node.path))
            .ghost()
            .compact()
            .flex_1()
            .min_w_0()
            .text_xs()
            .label(format!("{}  {}", node.name, node.media_count))
            .on_click(
                cx.listener(move |this, _, _, cx| this.select_path(selected_path.clone(), cx)),
            ),
    );
    row = row.child(menu);
    if node.has_children {
        let directory_path = node.path.clone();
        row = row.child(
            Button::new(format!("navigation-expand-dir-{}", node.path))
                .ghost()
                .compact()
                .label(photag_ui_bridge::i18n::tr(
                    &(if expanded.contains_key(&node.path) {
                        theme::icons::CHEVRON_DOWN
                    } else {
                        theme::icons::CHEVRON_RIGHT
                    })
                    .to_string(),
                ))
                .font_family(theme::ICON_FONT_FAMILY)
                .text_size(px(8.))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.toggle_directory(&directory_path);
                    cx.notify();
                })),
        );
    }
    row.children(children)
}
