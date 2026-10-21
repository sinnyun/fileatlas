use super::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::scroll::ScrollableElement as _;

mod pages;
mod scan_feedback;
pub(in crate::view) mod tag_render;
pub(super) use tag_render::merge_target_buttons;
use tag_render::render_tag;

impl SidebarView {
    pub(super) fn consume_ui_request(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(request) = self.service.take_ui_request() {
            if !matches!(
                request,
                crate::NavigationUiRequest::SelectPath(_)
                    | crate::NavigationUiRequest::SelectPathPreservingPage(_)
                    | crate::NavigationUiRequest::ExpandPath(_)
                    | crate::NavigationUiRequest::AddressBar(_)
            ) {
                self.refresh();
            }
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
                    self.select_directory_request(path, true, cx);
                }
                crate::NavigationUiRequest::SelectPathPreservingPage(path) => {
                    self.select_directory_request(path, false, cx);
                }
                crate::NavigationUiRequest::CloseModal => self.close_tag_modal(),
                crate::NavigationUiRequest::Sidebar(request) => {
                    self.apply_sidebar_request(request, window, cx);
                }
                crate::NavigationUiRequest::AddressBar(request) => {
                    self.apply_address_request(request, window, cx);
                }
            }
            self.publish_sidebar_state(cx);
            cx.notify();
        }
    }

    fn select_directory_request(
        &mut self,
        path: String,
        switch_page: bool,
        cx: &mut Context<Self>,
    ) {
        if switch_page {
            self.set_navigation_page(NavigationPage::Folders);
        }
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
                    self.expanded_directories
                        .insert(ancestor.clone(), self.service.subdirectories(&ancestor));
                }
            }
            self.persist_directory_expansion();
            self.select_path(path, cx);
        }
    }
}

impl Render for SidebarView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_plugin_slot_views(window, cx);
        self.consume_ui_request(window, cx);
        let locale_revision = fileatlas_ui_bridge::i18n::revision();
        if self.locale_revision != locale_revision {
            self.locale_revision = locale_revision;
            self.folder_path.update(cx, |input, cx| {
                input.set_placeholder(fileatlas_ui_bridge::i18n::tr("Folder path"), window, cx)
            });
            self.folder_search.update(cx, |input, cx| {
                input.set_placeholder(fileatlas_ui_bridge::i18n::tr("Search folders…"), window, cx)
            });
            self.new_folder_name.update(cx, |input, cx| {
                input.set_placeholder(fileatlas_ui_bridge::i18n::tr("Folder name"), window, cx)
            });
            self.tag_name.update(cx, |input, cx| {
                input.set_placeholder(fileatlas_ui_bridge::i18n::tr("Tag name"), window, cx)
            });
            self.tag_search.update(cx, |input, cx| {
                input.set_placeholder(fileatlas_ui_bridge::i18n::tr("Search tags…"), window, cx)
            });
            self.merge_search.update(cx, |input, cx| {
                input.set_placeholder(
                    fileatlas_ui_bridge::i18n::tr("Filter target tags…"),
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
            let tags = TagFilterSelection {
                included: self.included_tags.clone(),
                excluded: self.excluded_tags.clone(),
            };
            cx.spawn(async move |_, _| {
                let result = if path.is_some() {
                    cordis
                        .emit::<DirectoryPathSelected>(Routing::Unscoped, path)
                        .await
                } else if !tags.included.is_empty() || !tags.excluded.is_empty() {
                    cordis
                        .emit::<TagFilterSelectionChanged>(Routing::Unscoped, tags)
                        .await
                } else {
                    cordis
                        .emit::<FolderSelectionChanged>(Routing::Unscoped, folder_id)
                        .await
                };
                if let Err(error) = result {
                    tracing::error!(%error,"restored navigation dispatch failed");
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
        if self.active_page == NavigationPage::ProjectTree {
            let content = self
                .project_tree_views
                .iter()
                .map(|(_, _, view)| view.clone())
                .collect::<Vec<_>>();
            let page = div()
                .h_flex()
                .size_full()
                .child(rail)
                .child(div().v_flex().flex_1().min_w_0().h_full().children(content));
            return page;
        }
        let mut panel = div()
            .v_flex()
            .min_h_0()
            .size_full()
            .flex_1()
            .min_w_0()
            .gap(px(1.))
            .pl(px(10.))
            .pr(px(6.))
            .pt(px(2.))
            .pb(px(8.));
        if self.active_page == NavigationPage::Folders {
            panel = panel
                .child(sidebar_region(self.render_drives_section(cx)))
                .child(sidebar_region(self.render_quick_access_section(cx)));
            if matches!(
                self.sidebar_editor,
                Some(SidebarEditor::AddQuickAccess | SidebarEditor::RenameQuickAccess { .. })
            ) && let Some(editor_row) = self.render_sidebar_editor_row(cx)
            {
                panel = panel.child(editor_row);
            }
            panel = scan_feedback::append(panel, self, cx);
        }
        let panel = match &self.snapshot {
            Ok(snapshot) => {
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
                            .border_color(rgb(theme::separator()))
                            .child(
                                div()
                                    .h_flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .text_xs()
                                    .text_color(rgb(theme::text_muted()))
                                    .child(div().child(theme::small_icon(IconName::Tag)))
                                    .child(fileatlas_ui_bridge::i18n::tr("Tags").to_uppercase()),
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
                                            .accessibility_label(fileatlas_ui_bridge::i18n::tr(
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
                                .child(fileatlas_ui_bridge::components::text_input(
                                    &self.tag_search,
                                ))
                                .child(
                                    div()
                                        .h_flex()
                                        .gap_1()
                                        .child(
                                            Button::new("navigation-tag-mode-select")
                                                .when(!self.exclude_mode, |button| {
                                                    button.icon(theme::small_icon(IconName::Check))
                                                })
                                                .label(fileatlas_ui_bridge::i18n::tr("Select"))
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
                                                .label(fileatlas_ui_bridge::i18n::tr("Exclude"))
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.set_tag_click_mode(true, cx);
                                                })),
                                        )
                                        .child(
                                            Button::new("navigation-tag-select-all")
                                                .label(fileatlas_ui_bridge::i18n::tr("All"))
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
                                                .label(fileatlas_ui_bridge::i18n::tr("Clear"))
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
                                        fileatlas_ui_bridge::i18n::tr("No exclude modifier")
                                    }
                                    TagExcludeModifier::Ctrl => fileatlas_ui_bridge::i18n::tr_args(
                                        "{modifier} + click toggles other mode",
                                        &[("modifier", "Ctrl".into())],
                                    ),
                                    TagExcludeModifier::Shift => {
                                        fileatlas_ui_bridge::i18n::tr_args(
                                            "{modifier} + click toggles other mode",
                                            &[("modifier", "Shift".into())],
                                        )
                                    }
                                    TagExcludeModifier::Alt => fileatlas_ui_bridge::i18n::tr_args(
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
                                            .label(fileatlas_ui_bridge::i18n::tr_args(
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
                                            .label(fileatlas_ui_bridge::i18n::tr_args(
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
                    .text_color(rgb(theme::danger_text()))
                    .child(format!("Could not load library: {error}")),
            ),
        };
        let panel = if self.active_page == NavigationPage::Folders {
            self.render_favorite_groups(panel, cx)
        } else {
            panel
        };
        div()
            .h_flex()
            .size_full()
            .overflow_hidden()
            .child(rail)
            .child(panel.overflow_y_scrollbar())
    }
}

impl SidebarView {
    fn sync_plugin_slot_views(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let entries = self
            .cordis
            .try_service::<fileatlas_ui_bridge::UiBridgeService>()
            .ok()
            .and_then(|bridge| bridge.views.snapshot().ok())
            .unwrap_or_default();
        let live_ids = entries
            .iter()
            .filter(|entry| {
                matches!(
                    entry.contribution.slot.as_str(),
                    "navigation.extensions" | "navigation.launcher-shortcuts"
                )
            })
            .map(|entry| entry.contribution.id.clone())
            .collect::<HashSet<_>>();
        self.extension_views.retain(|(id, generation, _)| {
            entries.iter().any(|entry| {
                entry.contribution.slot == "navigation.extensions"
                    && entry.contribution.id == *id
                    && entry.generation == *generation
            })
        });
        self.project_tree_views.retain(|(id, generation, _)| {
            entries.iter().any(|entry| {
                entry.contribution.slot == "navigation.sidebar-project-tree"
                    && entry.contribution.id == *id
                    && entry.generation == *generation
            })
        });
        self.launcher_shortcuts.retain(|(id, generation, _)| {
            entries.iter().any(|entry| {
                entry.contribution.slot == "navigation.launcher-shortcuts"
                    && entry.contribution.id == *id
                    && entry.generation == *generation
            })
        });
        for entry in entries {
            let id = entry.contribution.id.clone();
            let generation = entry.generation;
            match entry.contribution.slot.as_str() {
                "navigation.extensions"
                    if !self.extension_views.iter().any(
                        |(current_id, current_generation, _)| {
                            current_id == &id && *current_generation == generation
                        },
                    ) =>
                {
                    self.extension_views.push((
                        id,
                        generation,
                        (entry.contribution.factory)(window, cx),
                    ));
                }
                "navigation.launcher-shortcuts"
                    if !self.launcher_shortcuts.iter().any(
                        |(current_id, current_generation, _)| {
                            current_id == &id && *current_generation == generation
                        },
                    ) =>
                {
                    self.launcher_shortcuts.push((
                        id,
                        generation,
                        (entry.contribution.factory)(window, cx),
                    ));
                }
                "navigation.sidebar-project-tree"
                    if !self.project_tree_views.iter().any(
                        |(current_id, current_generation, _)| {
                            current_id == &id && *current_generation == generation
                        },
                    ) =>
                {
                    self.project_tree_views.push((
                        id,
                        generation,
                        (entry.contribution.factory)(window, cx),
                    ));
                }
                _ => {}
            }
        }
        self.registered_sidebar_view_ids = live_ids;
        if self.active_page == NavigationPage::ProjectTree && self.project_tree_views.is_empty() {
            self.set_navigation_page(NavigationPage::Folders);
        }
    }
}
