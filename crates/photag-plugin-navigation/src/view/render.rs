use super::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::ButtonVariants;

mod folder_row;
mod pages;
mod scan_feedback;
mod tag_render;
use folder_row::folder_row;
pub(super) use tag_render::merge_target_buttons;
use tag_render::{directory_children_match, render_subdirectory, render_tag};

impl SidebarView {
    pub(super) fn render_breadcrumb(&mut self, cx: &mut Context<Self>) -> Div {
        let saved = self.session.snapshot().unwrap_or_default();
        let all_selected = saved.selected_folder_id.is_none()
            && saved.selected_path.is_none()
            && self.included_tags.is_empty()
            && self.excluded_tags.is_empty();
        let location = saved
            .selected_path
            .as_deref()
            .and_then(|path| Path::new(path).file_name())
            .and_then(|name| name.to_str())
            .map(str::to_owned)
            .or_else(|| {
                saved.selected_folder_id.and_then(|id| {
                    self.snapshot
                        .as_ref()
                        .ok()?
                        .folders
                        .iter()
                        .find(|node| node.folder.id == id)
                        .map(|node| node.folder.name.clone())
                })
            })
            .or_else(|| {
                self.included_tags.first().and_then(|id| {
                    self.snapshot.as_ref().ok().and_then(|snapshot| {
                        find_tag(&snapshot.tags, *id).map(|tag| tag.name.clone())
                    })
                })
            });
        div()
            .h_flex()
            .items_center()
            .gap(px(8.))
            .child(
                div()
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child(theme::small_icon(IconName::Library)),
            )
            .child(
                Button::new("navigation-breadcrumb-all-files")
                    .ghost()
                    .compact()
                    .label(photag_ui_bridge::i18n::tr("Favorites"))
                    .text_color(rgb(if all_selected {
                        theme::TEXT_PRIMARY
                    } else {
                        theme::TEXT_SECONDARY
                    }))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.select_folder(None, cx);
                        this.select_tag(None, cx);
                        cx.notify();
                    })),
            )
            .when_some(location, |row, label| {
                row.child(div().text_color(rgb(theme::TEXT_MUTED)).child("/"))
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(rgb(theme::TEXT_PRIMARY))
                            .child(label),
                    )
            })
    }
}

impl SidebarView {
    pub(super) fn consume_ui_request(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(request) = self.service.take_ui_request() {
            self.refresh();
            match request {
                crate::NavigationUiRequest::AllFiles => {
                    self.close_tag_modal();
                    self.select_folder(None, cx);
                    self.select_tag(None, cx);
                }
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
                crate::NavigationUiRequest::OpenMove(id) => {
                    self.close_tag_modal();
                    self.move_source_id = Some(id);
                    self.move_target_tag_id = None;
                }
                crate::NavigationUiRequest::ExpandFolder(id) => {
                    if let Some(path) = self
                        .snapshot
                        .as_ref()
                        .ok()
                        .and_then(|snapshot| {
                            snapshot.folders.iter().find(|node| node.folder.id == id)
                        })
                        .map(|node| node.folder.path.clone())
                    {
                        self.set_navigation_page(NavigationPage::Folders);
                        self.expand_directory(&path);
                    }
                }
                crate::NavigationUiRequest::ExpandPath(path) => {
                    if std::path::Path::new(&path).is_dir() {
                        self.set_navigation_page(NavigationPage::Folders);
                        self.expand_directory(&path);
                    }
                }
                crate::NavigationUiRequest::SelectPath(path) => {
                    self.set_navigation_page(NavigationPage::Folders);
                    if std::path::Path::new(&path).is_dir() {
                        let ancestors = Path::new(&path)
                            .ancestors()
                            .skip(1)
                            .take_while(|ancestor| {
                                self.snapshot.as_ref().is_ok_and(|snapshot| {
                                    snapshot
                                        .folders
                                        .iter()
                                        .any(|node| ancestor.starts_with(&node.folder.path))
                                })
                            })
                            .map(|ancestor| ancestor.to_string_lossy().into_owned())
                            .collect::<Vec<_>>();
                        for ancestor in ancestors.into_iter().rev() {
                            if self.service.browser_enabled() {
                                self.queue_directory(&ancestor);
                            } else {
                                self.expanded_directories.insert(
                                    ancestor.clone(),
                                    self.service.subdirectories(&ancestor),
                                );
                            }
                        }
                        self.persist_directory_expansion();
                        self.select_path(path, cx);
                    }
                }
                crate::NavigationUiRequest::CloseModal => self.close_tag_modal(),
            }
            cx.notify();
        }
    }
}

impl Render for SidebarView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.consume_ui_request(window, cx);
        let locale_revision = photag_ui_bridge::i18n::revision();
        if self.locale_revision != locale_revision {
            self.locale_revision = locale_revision;
            self.folder_path.update(cx, |input, cx| {
                input.set_placeholder(photag_ui_bridge::i18n::tr("Folder path"), window, cx)
            });
            self.folder_search.update(cx, |input, cx| {
                input.set_placeholder(photag_ui_bridge::i18n::tr("Search folders…"), window, cx)
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
        if let Some(value) = self.pending_folder_search.take() {
            self.folder_search.update(cx, |input, cx| {
                input.set_value(&value, window, cx);
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
                    tracing::error!("restored folder selection dispatch failed: {error}");
                }
                if let Err(error) = cordis
                    .emit::<DirectoryPathSelected>(Routing::Unscoped, path)
                    .await
                {
                    tracing::error!("restored directory selection dispatch failed: {error}");
                }
            })
            .detach();
        }
        // Source main.rs:1282-1296 and sidebar.rs:228-275.
        let rail = pages::render_rail(self, cx);
        if self.active_page == NavigationPage::Extensions {
            return div()
                .h_flex()
                .size_full()
                .child(rail)
                .child(pages::render_extensions(self));
        }
        let mut panel = div()
            .v_flex()
            .size_full()
            .flex_1()
            .min_w_0()
            .gap(px(1.))
            .pl(px(10.))
            .pr(px(6.))
            .pt(px(2.))
            .pb(px(8.));
        if self.active_page == NavigationPage::Folders {
            panel = scan_feedback::append(panel, self, cx);
        }
        let folder_view = cx.entity().downgrade();
        let panel = match &self.snapshot {
            Ok(snapshot) => {
                if self.active_page == NavigationPage::Folders {
                    let query = self.folder_search.read(cx).value().trim().to_owned();
                    let lowered_query = query.to_lowercase();
                    panel = panel.child(
                        div()
                            .h_flex()
                            .items_center()
                            .justify_between()
                            .pt(px(2.))
                            .pb(px(5.))
                            .child(
                                div()
                                    .font_semibold()
                                    .text_size(px(15.))
                                    .child(photag_ui_bridge::i18n::tr("Folder favorites")),
                            )
                            .child(
                                div()
                                    .h_flex()
                                    .gap_1()
                                    .child(
                                        Button::new("navigation-add-folder")
                                            .disabled(self.service.folder_picker_pending())
                                            .ghost()
                                            .compact()
                                            .w(px(22.))
                                            .h(px(22.))
                                            .icon(theme::small_icon(IconName::Plus))
                                            .accessibility_label(photag_ui_bridge::i18n::tr(
                                                "Add folder",
                                            ))
                                            .on_click(
                                                cx.listener(|this, _, _, cx| this.pick_folder(cx)),
                                            ),
                                    )
                                    .child(
                                        Button::new("navigation-folder-options")
                                            .ghost()
                                            .compact()
                                            .w(px(22.))
                                            .h(px(22.))
                                            .icon(theme::small_icon(IconName::Ellipsis))
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
                    panel = panel.child(Input::new(&self.folder_search).w_full());
                    panel = panel.child(
                        folder_row(
                            "navigation-all-media".into(),
                            photag_ui_bridge::i18n::tr("Favorites"),
                            snapshot.folders.iter().map(|node| node.media_count).sum(),
                            self.session
                                .snapshot()
                                .map(|s| {
                                    s.selected_folder_id.is_none() && s.selected_path.is_none()
                                })
                                .unwrap_or(true)
                                && self.included_tags.is_empty()
                                && self.excluded_tags.is_empty(),
                            gpui_kit::assets::IconName::Files,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.select_folder(None, cx);
                            this.select_tag(None, cx);
                            cx.notify();
                        })),
                    );
                    if self.show_folder_path_entry {
                        panel = panel.child(
                            div()
                                .h_flex()
                                .gap_1()
                                .child(Input::new(&self.folder_path).flex_1())
                                .child(
                                    Button::new("navigation-open-path")
                                        .label(photag_ui_bridge::i18n::tr("Open folder"))
                                        .on_click(cx.listener(|view, _, _, cx| {
                                            let path =
                                                view.folder_path.read(cx).value().trim().to_owned();
                                            if let Err(error) = view.service.request_ui(
                                                crate::NavigationUiRequest::SelectPath(path),
                                            ) {
                                                view.snapshot = Err(error.to_string());
                                            }
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("navigation-browse-folder")
                                        .disabled(self.service.folder_picker_pending())
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
                    let visible_folders = snapshot
                        .folders
                        .iter()
                        .filter(|node| {
                            lowered_query.is_empty()
                                || node.folder.name.to_lowercase().contains(&lowered_query)
                                || node.folder.path.to_lowercase().contains(&lowered_query)
                                || directory_children_match(
                                    &node.folder.path,
                                    &self.expanded_directories,
                                    &lowered_query,
                                )
                        })
                        .collect::<Vec<_>>();
                    if !lowered_query.is_empty() && visible_folders.is_empty() {
                        panel = panel.child(
                            div()
                                .text_xs()
                                .text_color(rgb(theme::TEXT_MUTED))
                                .child(photag_ui_bridge::i18n::tr("No matching folders")),
                        );
                    }
                    for node in visible_folders {
                        let expanded = self.expanded_directories.get(&node.folder.path);
                        let row_group = format!("navigation-folder-row-{}", node.folder.id);
                        panel = panel.child(
                            div()
                                .group(row_group.clone())
                                .relative()
                                .h_flex()
                                .w_full()
                                .items_center()
                                .gap_0()
                                .hover(|style| style.bg(rgba(0xffffff10)))
                                .child(
                                    Button::new(format!(
                                        "navigation-expand-root-{}",
                                        node.folder.id
                                    ))
                                    .ghost()
                                    .compact()
                                    .p_0()
                                    .icon(theme::small_icon(if expanded.is_some() {
                                        IconName::ChevronDown
                                    } else {
                                        IconName::ChevronRight
                                    }))
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
                                                (s.selected_folder_id == Some(node.folder.id)
                                                    && s.selected_path.is_none())
                                                    || s.selected_path.as_deref()
                                                        == Some(node.folder.path.as_str())
                                            })
                                            .unwrap_or(false),
                                        if expanded.is_some() {
                                            gpui_kit::assets::IconName::FolderOpen
                                        } else {
                                            gpui_kit::assets::IconName::Folder
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
                                        .absolute()
                                        .right(px(2.))
                                        .top(px(2.))
                                        .ghost()
                                        .compact()
                                        .opacity(0.)
                                        .group_hover(row_group, |style| style.opacity(1.))
                                        .icon(theme::small_icon(IconName::Ellipsis))
                                        .accessibility_label(photag_ui_bridge::i18n::tr(
                                            "Folder actions",
                                        ))
                                        .w(px(20.))
                                        .h(px(22.))
                                        .bg(rgb(theme::SURFACE_BASE))
                                        .dropdown_menu(move |menu, _, _| {
                                            favorite_menu(
                                                menu,
                                                folder_id,
                                                folder_path.clone(),
                                                folder_view.clone(),
                                            )
                                        })
                                }),
                        );
                        if let Some(children) = expanded {
                            let child_rows = children
                                .iter()
                                .filter_map(|child| {
                                    render_subdirectory(
                                        child,
                                        &self.expanded_directories,
                                        &query,
                                        self.session
                                            .snapshot()
                                            .ok()
                                            .and_then(|saved| saved.selected_path)
                                            .as_deref(),
                                        cx,
                                    )
                                })
                                .collect::<Vec<_>>();
                            if !child_rows.is_empty() {
                                panel = panel.child(
                                    div()
                                        .v_flex()
                                        .ml(px(22.))
                                        .pl(px(8.))
                                        .border_l_1()
                                        .border_color(rgba(0xffffff38))
                                        .children(child_rows),
                                );
                            }
                        }
                    }
                }
                if self.active_page == NavigationPage::Tags {
                    let tag_view = cx.entity().downgrade();
                    panel = panel.child(
                        div()
                            .h_flex()
                            .justify_between()
                            .items_center()
                            .mt(px(8.))
                            .pt(px(8.))
                            .border_t_1()
                            .border_color(rgb(theme::SEPARATOR))
                            .child(
                                div()
                                    .h_flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .text_xs()
                                    .text_color(rgb(theme::TEXT_MUTED))
                                    .child(div().child(theme::small_icon(IconName::Tag)))
                                    .child(photag_ui_bridge::i18n::tr("Tags").to_uppercase()),
                            )
                            .child(
                                div()
                                    .h_flex()
                                    .gap_1()
                                    .child(
                                        Button::new("navigation-toggle-tag-search")
                                            .ghost()
                                            .icon(theme::small_icon(IconName::Search))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.show_tag_search_controls =
                                                    !this.show_tag_search_controls;
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Button::new("navigation-create-tag")
                                            .ghost()
                                            .icon(theme::small_icon(IconName::Plus))
                                            .accessibility_label(photag_ui_bridge::i18n::tr(
                                                "Create tag",
                                            ))
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
                                                .when(!self.exclude_mode, |button| {
                                                    button.icon(theme::small_icon(IconName::Check))
                                                })
                                                .label(photag_ui_bridge::i18n::tr("Select"))
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.set_tag_click_mode(false, cx);
                                                })),
                                        )
                                        .child(
                                            Button::new("navigation-tag-mode-exclude")
                                                .when(self.exclude_mode, |button| {
                                                    button.icon(theme::small_icon(
                                                        IconName::CircleSlash,
                                                    ))
                                                })
                                                .label(photag_ui_bridge::i18n::tr("Exclude"))
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
                                    TagExcludeModifier::None => {
                                        photag_ui_bridge::i18n::tr("No exclude modifier")
                                    }
                                    TagExcludeModifier::Ctrl => photag_ui_bridge::i18n::tr_args(
                                        "{modifier} + click toggles other mode",
                                        &[("modifier", "Ctrl".into())],
                                    ),
                                    TagExcludeModifier::Shift => photag_ui_bridge::i18n::tr_args(
                                        "{modifier} + click toggles other mode",
                                        &[("modifier", "Shift".into())],
                                    ),
                                    TagExcludeModifier::Alt => photag_ui_bridge::i18n::tr_args(
                                        "{modifier} + click toggles other mode",
                                        &[("modifier", "Alt".into())],
                                    ),
                                })),
                        );
                        if !self.included_tags.is_empty() || !self.excluded_tags.is_empty() {
                            panel = panel.child(
                                div()
                                    .h_flex()
                                    .gap_1()
                                    .child(
                                        Button::new("navigation-tag-clear-included")
                                            .label(photag_ui_bridge::i18n::tr_args(
                                                "Included tags ({count})",
                                                &[("count", self.included_tags.len().to_string())],
                                            ))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.included_tags.clear();
                                                this.emit_tag_selection(cx);
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Button::new("navigation-tag-clear-excluded")
                                            .label(photag_ui_bridge::i18n::tr_args(
                                                "Excluded tags ({count})",
                                                &[("count", self.excluded_tags.len().to_string())],
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
        };
        div().h_flex().size_full().child(rail).child(panel)
    }
}
