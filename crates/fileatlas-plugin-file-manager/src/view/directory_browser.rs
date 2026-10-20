//! Native directory tiles and Navigation-service routing for ungrouped views.
use super::*;
use fileatlas_plugin_navigation::{NavigationService, NavigationUiRequest};
use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::prelude::FluentBuilder as _;

fn parent_path(path: &str) -> Option<String> {
    std::path::Path::new(path)
        .parent()
        .map(|parent| parent.to_string_lossy().into_owned())
        .filter(|parent| !parent.is_empty() && !same_path(parent, path))
}

fn same_path(left: &str, right: &str) -> bool {
    left.trim_end_matches(['\\', '/'])
        .eq_ignore_ascii_case(right.trim_end_matches(['\\', '/']))
}

fn address_crumbs(path: &str) -> Vec<(String, String)> {
    let path = path
        .strip_prefix(r"\\?\UNC\")
        .map(|tail| format!(r"\\{tail}"))
        .unwrap_or_else(|| path.strip_prefix(r"\\?\").unwrap_or(path).to_owned());
    let mut parts = std::path::Path::new(&path)
        .ancestors()
        .filter(|part| part.has_root())
        .map(|part| {
            let value = part.to_string_lossy().into_owned();
            let label = part
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| value.clone());
            (label, value)
        })
        .collect::<Vec<_>>();
    parts.reverse();
    parts
}

impl FileManagerView {
    pub(super) fn render_column_address(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let current = self.filter.path_prefix.clone().unwrap_or_default();
        if self.address.read(cx).value().is_empty() && !current.is_empty() {
            self.address
                .update(cx, |input, cx| input.set_value(&current, window, cx));
        }
        if current.is_empty() && !self.address_editing {
            self.address_editing = true;
        }
        let path = self.filter.path_prefix.clone();
        let parent = path.as_deref().and_then(parent_path);
        let mut row = div()
            .h_flex()
            .items_center()
            .gap(px(3.))
            .w_full()
            .min_w_0()
            .px(px(6.))
            .child(self.address_nav_button(
                "back",
                IconName::ArrowLeft,
                "Back",
                self.address_history_index > 0,
                cx,
            ))
            .child(self.address_nav_button(
                "forward",
                IconName::ArrowRight,
                "Forward",
                self.address_history_index + 1 < self.address_history.len(),
                cx,
            ))
            .child(self.address_nav_button(
                "up",
                IconName::ArrowUp,
                "Up one folder",
                parent.is_some(),
                cx,
            ));
        if self.address_editing {
            row = row.child(
                div()
                    .h_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(3.))
                    .child(fileatlas_ui_bridge::components::text_input(&self.address))
                    .child(
                        Button::new(format!(
                            "file_manager-column-{0}-address-go",
                            self.column_index
                        ))
                        .ghost()
                        .compact()
                        .label(fileatlas_ui_bridge::i18n::tr("Go"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            let path = this.address.read(cx).value().to_string();
                            if !path.trim().is_empty() {
                                this.open_column_path(path, cx);
                            }
                        })),
                    ),
            );
        } else {
            let mut trail = div()
                .id(format!(
                    "file_manager-column-{}-address-trail",
                    self.column_index
                ))
                .h_flex()
                .flex_shrink_1()
                .min_w_0()
                .items_center()
                .gap(px(2.))
                .overflow_x_scroll();
            let mut current_crumb = None;
            if let Some(path) = path.as_deref() {
                trail = trail.child(theme::small_icon(IconName::FolderOpen));
                let mut crumbs = address_crumbs(path);
                let current = crumbs.pop();
                let has_ancestors = !crumbs.is_empty();
                for (index, (label, target)) in crumbs.into_iter().enumerate() {
                    if index > 0 {
                        trail = trail.child(
                            theme::small_icon(IconName::ChevronRight)
                                .text_color(rgb(theme::text_muted())),
                        );
                    }
                    let destination = target.clone();
                    trail = trail.child(
                        Button::new(format!(
                            "file_manager-column-{}-crumb-{index}",
                            self.column_index
                        ))
                        .ghost()
                        .compact()
                        .flex_shrink_0()
                        .label(label)
                        .tooltip(target)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.open_column_path(destination.clone(), cx);
                        })),
                    );
                }
                if let Some((label, target)) = current {
                    let destination = target.clone();
                    let mut current_view =
                        div().h_flex().flex_shrink_0().items_center().gap(px(2.));
                    if has_ancestors {
                        current_view = current_view.child(
                            theme::small_icon(IconName::ChevronRight)
                                .text_color(rgb(theme::text_muted())),
                        );
                    }
                    current_crumb = Some(
                        current_view.child(
                            Button::new(format!(
                                "file_manager-column-{}-crumb-current",
                                self.column_index
                            ))
                            .ghost()
                            .compact()
                            .flex_shrink_0()
                            .max_w(px(140.))
                            .label(label)
                            .tooltip(target)
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.open_column_path(destination.clone(), cx);
                                },
                            )),
                        ),
                    );
                }
            }
            row = row.child(trail);
            if let Some(current_crumb) = current_crumb {
                row = row.child(current_crumb);
            }
            row = row.child(div().flex_1().min_w_0());
            row = row.child(
                Button::new(format!(
                    "file_manager-column-{}-address-edit",
                    self.column_index
                ))
                .ghost()
                .compact()
                .w(px(26.))
                .h(px(28.))
                .icon(theme::small_icon(IconName::Pencil))
                .tooltip(fileatlas_ui_bridge::i18n::tr("Edit folder address"))
                .accessibility_label(fileatlas_ui_bridge::i18n::tr("Edit folder address"))
                .on_click(cx.listener(|this, _, window, cx| {
                    let value = this.filter.path_prefix.clone().unwrap_or_default();
                    this.address
                        .update(cx, |input, cx| input.set_value(&value, window, cx));
                    this.address_editing = true;
                    cx.notify();
                })),
            );
        }
        row.into_any_element()
    }

    fn address_nav_button(
        &mut self,
        action: &'static str,
        icon: IconName,
        label: &'static str,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> Button {
        let column = self.column_index;
        Button::new(format!("file_manager-column-{column}-address-{action}"))
            .ghost()
            .compact()
            .flex_shrink_0()
            .w(px(28.))
            .h(px(28.))
            .icon(theme::small_icon(icon))
            .tooltip(fileatlas_ui_bridge::i18n::tr(label))
            .accessibility_label(fileatlas_ui_bridge::i18n::tr(label))
            .disabled(!enabled)
            .on_click(cx.listener(move |this, _, _, cx| match action {
                "back" => this.address_history_step(false, cx),
                "forward" => this.address_history_step(true, cx),
                "up" => {
                    if let Some(path) = this.filter.path_prefix.as_deref().and_then(parent_path) {
                        this.open_column_path(path, cx);
                    }
                }
                _ => {}
            }))
    }

    fn open_column_path(&mut self, path: String, cx: &mut Context<Self>) {
        self.address_editing = false;
        self.navigate_column_path(path, true, cx);
    }

    fn navigate_column_path(&mut self, path: String, record_history: bool, cx: &mut Context<Self>) {
        let path = path.trim().to_string();
        if path.is_empty() {
            return;
        }
        if !std::path::Path::new(&path).is_dir() {
            self.operation_status = Some("Folder does not exist".into());
            cx.notify();
            return;
        }
        if record_history {
            self.record_address_history(&path);
        }
        if let Ok(mut selected) = self.selected_path.0.lock() {
            *selected = Some(path.clone());
        }
        if let Ok(mut selected) = self.selected_folder.0.lock() {
            *selected = None;
        }
        if let Ok(mut selected) = self.selected_tag.0.lock() {
            *selected = Default::default();
        }
        self.filter.folder_id = None;
        self.filter.path_prefix = Some(path);
        self.selected_directory = None;
        self.service
            .navigation_revision
            .fetch_add(1, std::sync::atomic::Ordering::Release);
        self.reload();
        self.persist_filter_session();
        cx.notify();
    }

    fn address_history_step(&mut self, forward: bool, cx: &mut Context<Self>) {
        let target = if forward {
            self.address_history_index.checked_add(1)
        } else {
            self.address_history_index.checked_sub(1)
        };
        let Some(index) = target.filter(|index| *index < self.address_history.len()) else {
            return;
        };
        let Some(path) = self.address_history.get(index).cloned() else {
            return;
        };
        self.address_history_index = index;
        self.navigate_column_path(path, false, cx);
    }

    pub(super) fn record_address_history(&mut self, path: &str) {
        if self
            .address_history
            .get(self.address_history_index)
            .is_some_and(|current| same_path(current, path))
        {
            return;
        }
        self.address_history
            .truncate(self.address_history_index + 1);
        self.address_history.push(path.to_owned());
        if self.address_history.len() > 200 {
            self.address_history.remove(0);
        }
        self.address_history_index = self.address_history.len().saturating_sub(1);
    }

    pub(super) fn locate_file(&mut self, id: i64, cx: &mut Context<Self>) {
        let Some(item) = self.service.media_by_id(id).ok().flatten() else {
            return;
        };
        let Some(parent) = std::path::Path::new(&item.full_path)
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
        else {
            return;
        };
        self.restored_file_paths = vec![item.full_path];
        self.restore_in_directory = true;
        self.selection_restore = Default::default();
        self.route_directory(NavigationUiRequest::SelectPath(parent), cx);
        self.media_actions_id = None;
    }

    pub(super) fn pick_browse_location(&mut self, cx: &mut Context<Self>) {
        if self.location_picker_pending {
            return;
        }
        self.location_picker_pending = true;
        let task = self.runtime.spawn(async {
            rfd::AsyncFileDialog::new()
                .pick_folder()
                .await
                .map(|p| p.path().to_string_lossy().into_owned())
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |view, cx| {
                view.location_picker_pending = false;
                if let Ok(Some(path)) = result {
                    view.route_directory(NavigationUiRequest::SelectPath(path), cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn set_descendants(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.service.set_show_descendants(enabled);
        if enabled {
            self.before_descendants_grouping = Some(self.grouping_mode);
            self.grouping_mode = fileatlas_plugin_grouping::GroupingMode::Subfolder;
        } else {
            self.grouping_mode = self
                .before_descendants_grouping
                .take()
                .unwrap_or(fileatlas_plugin_grouping::GroupingMode::None);
        }
        if let Some(settings) = &self.settings {
            let mode = self.grouping_mode;
            let _ = settings.update(|s| {
                s.show_date_dividers = mode == fileatlas_plugin_grouping::GroupingMode::Month
            });
        }
        self.refresh_index();
        cx.notify();
    }

    #[tracing::instrument(skip_all, fields(path=?path))]
    pub(super) fn open_directory(&mut self, path: String, cx: &mut Context<Self>) {
        if !self.service.browser_enabled()
            && !self.snapshot.as_ref().is_ok_and(|s| {
                s.browser.active && s.browser.directories.iter().any(|f| f.path == path)
            })
        {
            self.operation_status =
                Some("Folder is no longer visible. Refresh and try again.".into());
            cx.notify();
            return;
        }
        self.route_directory(NavigationUiRequest::SelectPath(path), cx);
    }
    #[tracing::instrument(skip_all)]
    pub(super) fn directory_up(&mut self, cx: &mut Context<Self>) {
        let Ok(snapshot) = &self.snapshot else {
            return;
        };
        if !snapshot.browser.active || snapshot.browser.directory.is_none() {
            return;
        }
        let request = snapshot
            .browser
            .parent
            .clone()
            .map(NavigationUiRequest::SelectPath)
            .unwrap_or(NavigationUiRequest::AllFiles);
        self.route_directory(request, cx);
    }
    fn route_directory(&mut self, request: NavigationUiRequest, cx: &mut Context<Self>) {
        if self.column_index > 0 {
            match request {
                NavigationUiRequest::SelectPath(path) => self.open_column_path(path, cx),
                NavigationUiRequest::AllFiles => {
                    if let Ok(mut selected) = self.selected_path.0.lock() {
                        *selected = None;
                    }
                    if let Ok(mut selected) = self.selected_folder.0.lock() {
                        *selected = None;
                    }
                    if let Ok(mut selected) = self.selected_tag.0.lock() {
                        *selected = Default::default();
                    }
                    self.filter.path_prefix = None;
                    self.filter.folder_id = None;
                    self.reload();
                    cx.notify();
                }
                _ => {
                    self.operation_status =
                        Some("This action is not available in a column address.".into())
                }
            }
            return;
        }
        let result = self
            .cordis
            .try_service::<NavigationService>()
            .map_err(|e| e.to_string())
            .and_then(|service| service.request_ui(request).map_err(|e| e.to_string()));
        if let Err(error) = result {
            self.operation_status = Some(format!("Could not open folder: {error}"));
        } else {
            self.selected_directory = None;
            self.operation_status = None;
        }
        cx.notify();
    }
}

pub(super) fn refresh_button(cx: &mut Context<FileManagerView>) -> Button {
    Button::new("file_manager-directory-refresh")
        .ghost()
        .compact()
        .icon(theme::icon(IconName::RefreshCw))
        .tooltip(fileatlas_ui_bridge::i18n::tr("Refresh folder"))
        .on_click(cx.listener(|this, _, _, cx| {
            this.service.invalidate_directory();
            this.refresh_index();
            cx.notify();
        }))
}

pub(super) fn tile(
    folder: &crate::FileManagerDirectory,
    selected: bool,
    thumb: f32,
    list_width: Option<f32>,
    cx: &mut Context<FileManagerView>,
) -> AnyElement {
    let path = folder.path.clone();
    let list = list_width.is_some();
    let icon_area = div()
        .relative()
        .flex_shrink_0()
        .size(px(thumb))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(theme::THUMB_ROUND))
        .bg(rgb(theme::surface_faint()))
        .border_2()
        .border_color(if selected {
            rgb(theme::accent())
        } else {
            rgba(0)
        })
        .child(
            theme::icon(IconName::Folder)
                .size(px(if list {
                    34.
                } else {
                    (thumb * 0.46).clamp(40., 96.)
                }))
                .text_color(rgb(theme::warning_text())),
        );
    let caption = div()
        .min_w_0()
        .overflow_hidden()
        .text_ellipsis()
        .text_size(px(theme::FONT_SIZE_BUTTON))
        .child(folder.name.clone());
    div()
        .id(format!("file_manager-directory-{}", folder.path))
        .relative()
        .w(px(list_width.unwrap_or(thumb)))
        .h(px(if list {
            56.
        } else {
            thumb + card::CARD_CAPTION_HEIGHT
        }))
        .when(list, |row| {
            row.h_flex()
                .items_center()
                .rounded(px(4.))
                .bg(rgb(theme::surface_card()))
        })
        .when(!list, |row| row.v_flex())
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
            window.focus(&this.focus_handle, cx);
            this.selection = FileManagerSelection::default();
            this.selected_directory = Some(path.clone());
            this.inspector_deselected = false;
            this.persist_filter_session();
            if event.click_count() >= 2 {
                this.open_directory(path.clone(), cx);
            }
            cx.notify();
        }))
        .child(icon_area)
        .child(if list {
            div()
                .flex_1()
                .min_w_0()
                .v_flex()
                .px_3()
                .gap_1()
                .child(caption)
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(theme::text_muted()))
                        .child(fileatlas_ui_bridge::i18n::tr("Folder")),
                )
        } else {
            div()
                .h(px(card::CARD_CAPTION_HEIGHT))
                .w_full()
                .pt(px(9.))
                .text_center()
                .child(caption)
        })
        .hover(|style| style.bg(rgba(theme::fill_hover())))
        .cursor_pointer()
        .into_any_element()
}

pub(super) fn empty(
    view: &FileManagerView,
    message: Option<&str>,
    cx: &mut Context<FileManagerView>,
) -> AnyElement {
    let loading = view.service.directory_status().2;
    let message = if loading {
        Some("Loading folder…")
    } else {
        message
    };
    div()
        .track_focus(&view.focus_handle)
        .on_key_down(cx.listener(|this, event, window, cx| this.handle_key(event, window, cx)))
        .v_flex()
        .flex_1()
        .size_full()
        .items_center()
        .justify_center()
        .gap_3()
        .child(
            theme::icon(IconName::FolderOpen)
                .size(px(48.))
                .text_color(rgb(theme::warning_text())),
        )
        .child(
            div().text_color(rgb(theme::text_secondary())).child(
                message
                    .map(|text| fileatlas_ui_bridge::i18n::tr(text).to_string())
                    .unwrap_or_else(|| {
                        fileatlas_ui_bridge::i18n::tr("This folder is empty").to_string()
                    }),
            ),
        )
        .into_any_element()
}
