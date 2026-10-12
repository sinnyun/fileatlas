//! Native GPUI sidebar rendering for the navigation plugin.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use cordis::{Context as CordisContext, Routing};
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::*;
use photag_contracts::{
    DirectoryPathSelected, FolderSelectionChanged, MediaIndexChanged, TagSelectionChanged,
};
use photag_plugin_indexing::scan::ScanProgress;
use photag_ui_bridge::UiViewContribution;

use crate::{NavigationService, NavigationSnapshot, SubdirectoryNode};

pub(super) fn sidebar(
    service: Arc<NavigationService>,
    cordis: CordisContext,
) -> UiViewContribution {
    UiViewContribution::new(
        "navigation/sidebar-view",
        "workspace.sidebar",
        0,
        move |window, app| {
            let view_service = Arc::clone(&service);
            let view_cordis = cordis.clone();
            let folder_path = app.new(|cx| InputState::new(window, cx).placeholder("Folder path"));
            let tag_name = app.new(|cx| InputState::new(window, cx).placeholder("New tag name"));
            let tag_parent =
                app.new(|cx| InputState::new(window, cx).placeholder("Parent tag ID (optional)"));
            let tag_edit_name = app.new(|cx| InputState::new(window, cx).placeholder("Tag name"));
            let tag_color = app.new(|cx| {
                InputState::new(window, cx).placeholder("Color #RRGGBB (empty = default)")
            });
            let tag_icon = app.new(|cx| {
                InputState::new(window, cx).placeholder("Icon / emoji (empty = default)")
            });
            let tag_move_target =
                app.new(|cx| InputState::new(window, cx).placeholder("Parent tag ID"));
            app.new(|_| {
                SidebarView::new(
                    view_service,
                    view_cordis,
                    folder_path,
                    tag_name,
                    tag_parent,
                    tag_edit_name,
                    tag_color,
                    tag_icon,
                    tag_move_target,
                )
            })
            .into()
        },
    )
}

struct SidebarView {
    service: Arc<NavigationService>,
    cordis: CordisContext,
    folder_path: Entity<InputState>,
    tag_name: Entity<InputState>,
    tag_parent: Entity<InputState>,
    tag_edit_name: Entity<InputState>,
    tag_color: Entity<InputState>,
    tag_icon: Entity<InputState>,
    tag_move_target: Entity<InputState>,
    snapshot: Result<NavigationSnapshot, String>,
    active_scan: Option<std::sync::mpsc::Receiver<Option<ScanProgress>>>,
    scan_progress: Option<ScanProgress>,
    expanded_directories: HashMap<String, Vec<SubdirectoryNode>>,
}

impl SidebarView {
    fn new(
        service: Arc<NavigationService>,
        cordis: CordisContext,
        folder_path: Entity<InputState>,
        tag_name: Entity<InputState>,
        tag_parent: Entity<InputState>,
        tag_edit_name: Entity<InputState>,
        tag_color: Entity<InputState>,
        tag_icon: Entity<InputState>,
        tag_move_target: Entity<InputState>,
    ) -> Self {
        Self {
            service: Arc::clone(&service),
            cordis,
            folder_path,
            tag_name,
            tag_parent,
            tag_edit_name,
            tag_color,
            tag_icon,
            tag_move_target,
            snapshot: service.snapshot().map_err(|error| error.to_string()),
            active_scan: None,
            scan_progress: None,
            expanded_directories: HashMap::new(),
        }
    }

    fn refresh(&mut self) {
        self.snapshot = self.service.snapshot().map_err(|error| error.to_string());
    }

    fn rescan(&mut self, folder_id: i64, cx: &mut Context<Self>) {
        match self.service.rescan(folder_id) {
            Ok(progress) => {
                self.active_scan = Some(progress);
                self.scan_progress = None;
                self.refresh();
                self.monitor_scan(cx);
            }
            Err(error) => self.snapshot = Err(error.to_string()),
        }
    }

    fn add_folder(&mut self, cx: &mut Context<Self>) {
        let path = self.folder_path.read(cx).value().trim().to_owned();
        if path.is_empty() {
            return;
        }
        match self.service.add_folder(path) {
            Ok((_, progress)) => {
                self.active_scan = Some(progress);
                self.scan_progress = None;
                self.refresh();
                self.monitor_scan(cx);
            }
            Err(error) => self.snapshot = Err(error.to_string()),
        }
    }

    fn monitor_scan(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                let active = this
                    .update(cx, |view, cx| {
                        if view.poll_scan() {
                            view.notify_media_index(cx);
                        }
                        cx.notify();
                        view.active_scan.is_some()
                    })
                    .unwrap_or(false);
                if !active {
                    break;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(200))
                    .await;
            }
        })
        .detach();
    }

    fn poll_scan(&mut self) -> bool {
        let Some(receiver) = &self.active_scan else {
            return false;
        };
        let mut completed = false;
        loop {
            match receiver.try_recv() {
                Ok(Some(progress)) => self.scan_progress = Some(progress),
                Ok(None) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    completed = true;
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
            }
        }
        if completed {
            self.active_scan = None;
            self.scan_progress = None;
            self.refresh();
        }
        completed
    }

    fn remove_folder(&mut self, folder_id: i64, cx: &mut Context<Self>) {
        match self.service.remove_folder(folder_id) {
            Ok(_) => {
                self.refresh();
                self.notify_media_index(cx);
            }
            Err(error) => self.snapshot = Err(error.to_string()),
        }
    }

    fn notify_media_index(&self, cx: &mut Context<Self>) {
        let cordis = self.cordis.clone();
        cx.spawn(async move |_, _| {
            if let Err(error) = cordis
                .emit::<MediaIndexChanged>(Routing::Unscoped, ())
                .await
            {
                eprintln!("media index change dispatch failed: {error}");
            }
        })
        .detach();
    }

    fn select_folder(&self, folder_id: Option<i64>, cx: &mut Context<Self>) {
        let cordis = self.cordis.clone();
        cx.spawn(async move |_, _| {
            if let Err(error) = cordis
                .emit::<FolderSelectionChanged>(Routing::Unscoped, folder_id)
                .await
            {
                eprintln!("folder selection dispatch failed: {error}");
            }
        })
        .detach();
    }

    fn select_tag(&self, tag_id: Option<i64>, cx: &mut Context<Self>) {
        let cordis = self.cordis.clone();
        cx.spawn(async move |_, _| {
            if let Err(error) = cordis
                .emit::<TagSelectionChanged>(Routing::Unscoped, tag_id)
                .await
            {
                eprintln!("tag selection dispatch failed: {error}");
            }
        })
        .detach();
    }

    fn toggle_directory(&mut self, path: &str) {
        if self.expanded_directories.remove(path).is_none() {
            self.expanded_directories
                .insert(path.to_owned(), self.service.subdirectories(path));
        }
    }

    fn select_path(&self, path: String, cx: &mut Context<Self>) {
        let cordis = self.cordis.clone();
        cx.spawn(async move |_, _| {
            if let Err(error) = cordis
                .emit::<DirectoryPathSelected>(Routing::Unscoped, Some(path))
                .await
            {
                eprintln!("directory selection dispatch failed: {error}");
            }
        })
        .detach();
    }

    fn create_tag(&mut self, cx: &mut Context<Self>) {
        let name = self.tag_name.read(cx).value().trim().to_owned();
        if name.is_empty() {
            return;
        }
        let parent_text = self.tag_parent.read(cx).value().trim().to_owned();
        let parent = if parent_text.is_empty() {
            None
        } else {
            parent_text.parse::<i64>().ok()
        };
        if !parent_text.is_empty() && parent.is_none() {
            self.snapshot = Err("Parent tag ID must be an integer".into());
            return;
        }
        match self.service.create_tag(&name, parent) {
            Ok(_) => self.refresh(),
            Err(error) => self.snapshot = Err(error.to_string()),
        }
        cx.notify();
    }

    fn rename_tag(&mut self, id: i64, cx: &mut Context<Self>) {
        let name = self.tag_edit_name.read(cx).value().trim().to_owned();
        if name.is_empty() {
            return;
        }
        match self.service.rename_tag(id, &name) {
            Ok(()) => self.refresh(),
            Err(error) => self.snapshot = Err(error.to_string()),
        }
        cx.notify();
    }

    fn update_tag_appearance(&mut self, id: i64, cx: &mut Context<Self>) {
        let name = self.tag_edit_name.read(cx).value().trim().to_owned();
        let color = self.tag_color.read(cx).value().trim().to_owned();
        let icon = self.tag_icon.read(cx).value().trim().to_owned();
        if !color.is_empty() && crate::parse_hex_color(&color).is_none() {
            self.snapshot = Err("Tag color must use #RRGGBB format".into());
            cx.notify();
            return;
        }
        if !name.is_empty() {
            if let Err(error) = self.service.rename_tag(id, &name) {
                self.snapshot = Err(error.to_string());
                cx.notify();
                return;
            }
        }
        let color = if color.is_empty() { None } else { Some(color) };
        let icon = if icon.is_empty() { None } else { Some(icon) };
        match self
            .service
            .set_tag_appearance(id, color.as_deref(), icon.as_deref())
        {
            Ok(()) => self.refresh(),
            Err(error) => self.snapshot = Err(error.to_string()),
        }
        cx.notify();
    }

    fn delete_tag(&mut self, id: i64, cx: &mut Context<Self>) {
        match self.service.delete_tag(id) {
            Ok(()) => {
                self.refresh();
                self.select_tag(None, cx);
                self.notify_media_index(cx);
            }
            Err(error) => self.snapshot = Err(error.to_string()),
        }
        cx.notify();
    }

    fn move_tag_into(&mut self, id: i64, cx: &mut Context<Self>) {
        let target = self.tag_move_target.read(cx).value().trim().parse::<i64>();
        match target {
            Ok(target) => match self.service.move_tag(
                id,
                photag_plugin_database::db::queries::TagDropPos::Into(target),
            ) {
                Ok(_) => {
                    self.refresh();
                    self.notify_media_index(cx);
                }
                Err(error) => self.snapshot = Err(error.to_string()),
            },
            Err(_) => self.snapshot = Err("Parent tag ID must be an integer".into()),
        }
        cx.notify();
    }

    fn toggle_tag_display(&mut self, id: i64, display: bool, cx: &mut Context<Self>) {
        match self.service.set_tag_display(id, display) {
            Ok(()) => self.refresh(),
            Err(error) => self.snapshot = Err(error.to_string()),
        }
        cx.notify();
    }

    fn use_tag_as_parent(&self, id: i64, window: &mut Window, cx: &mut Context<Self>) {
        self.tag_parent.update(cx, |state, cx| {
            state.set_value(&id.to_string(), window, cx);
        });
    }
}

impl Render for SidebarView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut panel = div()
            .v_flex()
            .size_full()
            .gap_1()
            .bg(rgb(0x20232b))
            .p_3()
            .child(div().text_sm().font_semibold().child("LIBRARY"));
        match &self.snapshot {
            Ok(snapshot) => {
                panel = panel.child(
                    div()
                        .h_flex()
                        .justify_between()
                        .pt_2()
                        .child(div().text_xs().text_color(rgb(0x9aa0ad)).child("Folders"))
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
                            Button::new("navigation-add-folder")
                                .label("Add")
                                .on_click(cx.listener(|this, _, _, cx| this.add_folder(cx))),
                        ),
                );
                panel = panel.child(
                    Button::new("navigation-all-media")
                        .label("All Media")
                        .on_click(cx.listener(|this, _, _, cx| this.select_folder(None, cx))),
                );
                for node in &snapshot.folders {
                    let expanded = self.expanded_directories.get(&node.folder.path);
                    panel =
                        panel.child(
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
                                        ),
                                ),
                        );
                    if let Some(children) = expanded {
                        panel = panel.children(children.iter().map(|child| {
                            render_subdirectory(child, &self.expanded_directories, cx)
                        }));
                    }
                }
                if let Some(progress) = &self.scan_progress {
                    panel = panel.child(div().text_xs().text_color(rgb(0x9aa0ad)).child(format!(
                        "{}: {} / {} files",
                        progress.phase, progress.scanned, progress.total
                    )));
                }
                panel = panel.child(
                    div()
                        .h_flex()
                        .justify_between()
                        .pt_3()
                        .child(div().text_xs().text_color(rgb(0x9aa0ad)).child("Tags"))
                        .child(
                            Button::new("navigation-clear-tag-filter")
                                .label("Clear")
                                .on_click(cx.listener(|this, _, _, cx| this.select_tag(None, cx))),
                        ),
                );
                panel = panel.child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x9aa0ad))
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
                        .child(Input::new(&self.tag_move_target).flex_1())
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x9aa0ad))
                                .child("Set fields, then use a tag row action"),
                        ),
                );
                panel.children(snapshot.tags.iter().map(|tag| render_tag(tag, cx)))
            }
            Err(error) => panel.child(
                div()
                    .text_sm()
                    .text_color(rgb(0xef7777))
                    .child(format!("Could not load library: {error}")),
            ),
        }
    }
}

fn render_tag(
    tag: &photag_plugin_database::db::queries::Tag,
    cx: &mut Context<SidebarView>,
) -> Div {
    let indent = "  ".repeat(tag.full_path.matches('>').count());
    let tag_id = tag.id;
    let select = Button::new(format!("navigation-tag-{}", tag.id))
        .label("Filter")
        .on_click(cx.listener(move |this, _, _, cx| this.select_tag(Some(tag_id), cx)));
    let tag_name = tag.name.clone();
    let tag_color = tag.color.clone().unwrap_or_default();
    let tag_icon = tag.icon.clone().unwrap_or_default();
    let [red, green, blue] = tag
        .color
        .as_deref()
        .and_then(crate::parse_hex_color)
        .unwrap_or([120, 120, 130]);
    let select_name_id = tag.id;
    let name = Button::new(format!("navigation-tag-name-{}", tag.id))
        .label(format!(
            "{indent}{} {}  {}",
            tag.icon
                .as_deref()
                .filter(|icon| !icon.is_empty())
                .unwrap_or("🏷"),
            tag.name,
            tag.media_count
        ))
        .on_click(cx.listener(move |this, _, _, cx| this.select_tag(Some(select_name_id), cx)));
    let edit_name = tag_name;
    let edit_color = tag_color;
    let edit_icon = tag_icon;
    let edit_id = tag.id;
    let edit = Button::new(format!("navigation-tag-edit-{edit_id}"))
        .label("Edit fields")
        .on_click(cx.listener(move |this, _, window, cx| {
            let name = edit_name.clone();
            let color = edit_color.clone();
            let icon = edit_icon.clone();
            this.tag_edit_name
                .update(cx, |state, cx| state.set_value(&name, window, cx));
            this.tag_color
                .update(cx, |state, cx| state.set_value(&color, window, cx));
            this.tag_icon
                .update(cx, |state, cx| state.set_value(&icon, window, cx));
        }));
    let rename = Button::new(format!("navigation-tag-rename-{edit_id}"))
        .label("Rename")
        .on_click(cx.listener(move |this, _, _, cx| this.rename_tag(edit_id, cx)));
    let appearance_id = tag.id;
    let appearance = Button::new(format!("navigation-tag-appearance-{appearance_id}"))
        .label("Save style")
        .on_click(cx.listener(move |this, _, _, cx| this.update_tag_appearance(appearance_id, cx)));
    let display_id = tag.id;
    let was_displayed = tag.display;
    let display = Button::new(format!("navigation-tag-display-{display_id}"))
        .label(if was_displayed {
            "Hide chips"
        } else {
            "Show chips"
        })
        .on_click(cx.listener(move |this, _, _, cx| {
            this.toggle_tag_display(display_id, !was_displayed, cx)
        }));
    let delete_id = tag.id;
    let delete = Button::new(format!("navigation-tag-delete-{delete_id}"))
        .label("Delete")
        .on_click(cx.listener(move |this, _, _, cx| this.delete_tag(delete_id, cx)));
    let move_id = tag.id;
    let move_into = Button::new(format!("navigation-tag-move-{move_id}"))
        .label("Move into ID")
        .on_click(cx.listener(move |this, _, _, cx| this.move_tag_into(move_id, cx)));
    let parent_id = tag.id;
    let use_parent = Button::new(format!("navigation-tag-parent-{parent_id}"))
        .label("Use as parent")
        .on_click(cx.listener(move |this, _, window, cx| {
            this.use_tag_as_parent(parent_id, window, cx);
            cx.notify();
        }));
    let children = tag
        .children
        .iter()
        .map(|child| render_tag(child, cx))
        .collect::<Vec<_>>();
    div()
        .v_flex()
        .child(
            div()
                .h_flex()
                .justify_between()
                .child(
                    div()
                        .h_flex()
                        .gap_1()
                        .child(div().w_2().h_2().rounded_sm().bg(rgb((u32::from(red) << 16)
                            | (u32::from(green) << 8)
                            | u32::from(blue))))
                        .child(name),
                )
                .child(select),
        )
        .child(
            div()
                .h_flex()
                .gap_1()
                .child(edit)
                .child(rename)
                .child(appearance)
                .child(display)
                .child(use_parent)
                .child(move_into)
                .child(delete),
        )
        .children(children)
}

fn render_subdirectory(
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
    let mut row = div()
        .h_flex()
        .pl_3()
        .gap_1()
        .child(
            div()
                .flex_1()
                .text_xs()
                .child(format!("{}  {}", node.name, node.media_count)),
        )
        .child(
            Button::new(format!("navigation-open-dir-{}", node.path))
                .label("Open")
                .on_click(
                    cx.listener(move |this, _, _, cx| this.select_path(selected_path.clone(), cx)),
                ),
        );
    if node.has_children {
        let directory_path = node.path.clone();
        row = row.child(
            Button::new(format!("navigation-expand-dir-{}", node.path))
                .label(if expanded.contains_key(&node.path) {
                    "−"
                } else {
                    "+"
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.toggle_directory(&directory_path);
                    cx.notify();
                })),
        );
    }
    row.children(children)
}
