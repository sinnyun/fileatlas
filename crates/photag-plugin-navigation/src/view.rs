//! Native GPUI sidebar rendering for the navigation plugin.

use std::sync::Arc;
use std::time::Duration;

use cordis::{Context as CordisContext, Routing};
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::*;
use photag_contracts::{FolderSelectionChanged, MediaIndexChanged, TagSelectionChanged};
use photag_plugin_indexing::scan::ScanProgress;
use photag_ui_bridge::UiViewContribution;

use crate::{NavigationService, NavigationSnapshot};

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
            app.new(|_| SidebarView::new(view_service, view_cordis, folder_path))
                .into()
        },
    )
}

struct SidebarView {
    service: Arc<NavigationService>,
    cordis: CordisContext,
    folder_path: Entity<InputState>,
    snapshot: Result<NavigationSnapshot, String>,
    active_scan: Option<std::sync::mpsc::Receiver<Option<ScanProgress>>>,
    scan_progress: Option<ScanProgress>,
}

impl SidebarView {
    fn new(
        service: Arc<NavigationService>,
        cordis: CordisContext,
        folder_path: Entity<InputState>,
    ) -> Self {
        Self {
            service: Arc::clone(&service),
            cordis,
            folder_path,
            snapshot: service.snapshot().map_err(|error| error.to_string()),
            active_scan: None,
            scan_progress: None,
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
                        .w_full()
                        .px_2()
                        .py_1()
                        .text_sm()
                        .child(format!("{indent}{}  {}", tag.name, tag.media_count)),
                )
                .child(select),
        )
        .children(children)
}
