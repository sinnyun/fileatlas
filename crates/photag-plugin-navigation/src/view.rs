//! Native GPUI sidebar rendering for the navigation plugin.

use std::sync::Arc;
use std::time::Duration;

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::*;
use photag_plugin_indexing::scan::ScanProgress;
use photag_ui_bridge::UiViewContribution;

use crate::{NavigationService, NavigationSnapshot};

pub(super) fn sidebar(service: Arc<NavigationService>) -> UiViewContribution {
    UiViewContribution::new(
        "navigation/sidebar-view",
        "workspace.sidebar",
        0,
        move |_, app| {
            let view_service = Arc::clone(&service);
            app.new(|_| SidebarView::new(view_service)).into()
        },
    )
}

struct SidebarView {
    service: Arc<NavigationService>,
    snapshot: Result<NavigationSnapshot, String>,
    active_scan: Option<std::sync::mpsc::Receiver<Option<ScanProgress>>>,
    scan_progress: Option<ScanProgress>,
}

impl SidebarView {
    fn new(service: Arc<NavigationService>) -> Self {
        Self {
            service: Arc::clone(&service),
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

    fn monitor_scan(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                let active = this
                    .update(cx, |view, cx| {
                        view.poll_scan();
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

    fn poll_scan(&mut self) {
        let Some(receiver) = &self.active_scan else {
            return;
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
    }

    fn remove_folder(&mut self, folder_id: i64) {
        match self.service.remove_folder(folder_id) {
            Ok(_) => self.refresh(),
            Err(error) => self.snapshot = Err(error.to_string()),
        }
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
                                                    this.remove_folder(folder_id);
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
                        .pt_3()
                        .text_xs()
                        .text_color(rgb(0x9aa0ad))
                        .child("Tags"),
                );
                panel.children(snapshot.tags.iter().map(render_tag))
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

fn render_tag(tag: &photag_plugin_database::db::queries::Tag) -> Div {
    let indent = "  ".repeat(tag.full_path.matches('>').count());
    div()
        .v_flex()
        .child(
            div()
                .w_full()
                .px_2()
                .py_1()
                .text_sm()
                .child(format!("{indent}{}  {}", tag.name, tag.media_count)),
        )
        .children(tag.children.iter().map(render_tag))
}
