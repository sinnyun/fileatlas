use std::path::{Path, PathBuf};
use std::sync::Arc;

use fileatlas_plugin_navigation::{NavigationService, NavigationUiRequest};
use fileatlas_ui_bridge::{UiViewContribution, read_model::ReadModel, theme};
use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::gpui::InteractiveElement as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::{ProjectFolder, ProjectsService, same_path};

#[derive(Clone, Debug)]
struct ProjectNode {
    path: String,
    name: String,
    depth: usize,
}

fn scan_projects(projects: &[ProjectFolder]) -> Result<Vec<ProjectNode>, String> {
    fn walk(
        path: &Path,
        depth: usize,
        out: &mut Vec<ProjectNode>,
        seen: &mut std::collections::HashSet<PathBuf>,
    ) -> std::io::Result<()> {
        let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        if !seen.insert(canonical) {
            return Ok(());
        }
        let mut dirs = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_dir() {
                dirs.push(entry.path());
            }
        }
        dirs.sort_by_key(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default()
        });
        for child in dirs {
            let name = child
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let child_path = child.to_string_lossy().into_owned();
            out.push(ProjectNode {
                path: child_path,
                name,
                depth,
            });
            walk(&child, depth + 1, out, seen)?;
        }
        Ok(())
    }
    let mut rows = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for project in projects {
        rows.push(ProjectNode {
            path: project.path.clone(),
            name: project.name.clone(),
            depth: 0,
        });
        walk(Path::new(&project.path), 1, &mut rows, &mut seen)
            .map_err(|error| format!("{}: {error}", project.path))?;
    }
    Ok(rows)
}

pub(super) fn sidebar_page(
    service: Arc<ProjectsService>,
    navigation: Arc<NavigationService>,
) -> UiViewContribution {
    UiViewContribution::new(
        "projects/sidebar-page-view",
        "navigation.sidebar-projects",
        0,
        move |window, app| page(Arc::clone(&service), Arc::clone(&navigation), window, app),
    )
}
fn page(
    service: Arc<ProjectsService>,
    navigation: Arc<NavigationService>,
    _window: &mut Window,
    app: &mut App,
) -> AnyView {
    app.new(|cx| {
        let wake = fileatlas_ui_bridge::refresh::Wake::default();
        let mut tree = ReadModel::<Vec<String>, Vec<ProjectNode>>::default();
        tree.set_wake(wake.clone());
        let projects = service.snapshot();
        let paths = projects.iter().map(|p| p.path.clone()).collect::<Vec<_>>();
        tree.request(paths, move || scan_projects(&projects));
        let task = fileatlas_ui_bridge::refresh::listen(cx, wake, |view: &mut ProjectsPage, _| {
            view.tree.poll()
        });
        ProjectsPage {
            _task: task,
            service,
            navigation,
            selected_path: None,
            tree,
            error: None,
        }
    })
    .into()
}

struct ProjectsPage {
    _task: Task<()>,
    service: Arc<ProjectsService>,
    navigation: Arc<NavigationService>,
    selected_path: Option<String>,
    tree: ReadModel<Vec<String>, Vec<ProjectNode>>,
    error: Option<String>,
}

impl ProjectsPage {
    fn reload(&mut self) {
        let projects = self.service.snapshot();
        let paths = projects.iter().map(|p| p.path.clone()).collect::<Vec<_>>();
        self.tree.request(paths, move || scan_projects(&projects));
    }
    fn add_current_folder(&mut self, cx: &mut Context<Self>) {
        let path = self.navigation.sidebar_state().and_then(|state| {
            state
                .get("address_bar")
                .and_then(|address| address.get("path"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        });
        let Some(path) = path else {
            return;
        };
        match self.service.add(&path) {
            Ok(_) => {
                self.error = None;
                self.reload();
            }
            Err(error) => self.error = Some(error.to_string()),
        }
        cx.notify();
    }
}

impl Render for ProjectsPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let current_folder = self.navigation.sidebar_state().and_then(|state| {
            state
                .get("address_bar")
                .and_then(|address| address.get("path"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        });
        let selected_path = self
            .selected_path
            .clone()
            .or_else(|| current_folder.clone());
        let mut content = div().v_flex().size_full().gap(px(8.)).px(px(8.)).py(px(8.));
        content = content.child(
            div()
                .h_flex()
                .items_center()
                .justify_between()
                .px(px(8.))
                .py(px(5.))
                .child(
                    div()
                        .text_sm()
                        .font_semibold()
                        .child(fileatlas_ui_bridge::i18n::tr("Projects")),
                )
                .child(theme::small_icon(IconName::Folder)),
        );
        content = content.child(
            div()
                .h_flex()
                .items_center()
                .justify_between()
                .px(px(8.))
                .py(px(5.))
                .child(
                    Button::new("projects-add")
                        .ghost()
                        .icon(theme::small_icon(IconName::Plus))
                        .tooltip(fileatlas_ui_bridge::i18n::tr("Add folder"))
                        .accessibility_label(fileatlas_ui_bridge::i18n::tr("Add folder"))
                        .disabled(current_folder.is_none())
                        .on_click(cx.listener(|this, _, _, cx| this.add_current_folder(cx))),
                ),
        );
        if let Some(error) = &self.error {
            content = content.child(
                div()
                    .text_sm()
                    .text_color(rgb(theme::danger_text()))
                    .child(error.clone()),
            );
        }
        for (index, project) in self.service.snapshot().into_iter().enumerate() {
            let selected = selected_path
                .as_deref()
                .is_some_and(|path| same_path(path, &project.path));
            let select_path = project.path.clone();
            let remove_path = project.path.clone();
            content = content.child(
                div()
                    .id(format!("project-root-{index}"))
                    .h_flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .px(px(8.))
                    .py(px(5.))
                    .text_color(rgb(if selected {
                        theme::selection_text()
                    } else {
                        theme::text_primary()
                    }))
                    .when(selected, |style| style.bg(rgb(theme::selection_color())))
                    .hover(|style| style.bg(rgba(theme::fill_hover())))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected_path = Some(select_path.clone());
                        cx.notify();
                        if let Err(error) = this.navigation.request_ui(
                            NavigationUiRequest::SelectPathPreservingPage(select_path.clone()),
                        ) {
                            tracing::warn!(%error, "project navigation request rejected");
                        }
                    }))
                    .child(
                        div()
                            .h_flex()
                            .min_w_0()
                            .gap(px(8.))
                            .child(theme::small_icon(IconName::Folder))
                            .child(project.name),
                    )
                    .child(
                        Button::new(format!("remove-project-{index}"))
                            .ghost()
                            .icon(theme::small_icon(IconName::X))
                            .accessibility_label(fileatlas_ui_bridge::i18n::tr("Remove project"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Err(error) = this.service.remove(&remove_path) {
                                    this.error = Some(error.to_string());
                                }
                                this.reload();
                                cx.notify();
                            })),
                    ),
            );
        }
        match self.tree.value() {
            Some(Ok(rows)) => {
                for node in rows.iter().filter(|node| node.depth > 0) {
                    let path = node.path.clone();
                    let selected = selected_path
                        .as_deref()
                        .is_some_and(|selected| same_path(selected, &node.path));
                    let row = div()
                        .id(format!("project-node-{}", node.path))
                        .h_flex()
                        .items_center()
                        .gap(px(7.))
                        .px(px(8. + node.depth as f32 * 14.))
                        .py(px(3.))
                        .text_color(rgb(if selected {
                            theme::selection_text()
                        } else {
                            theme::text_secondary()
                        }))
                        .when(selected, |style| style.bg(rgb(theme::selection_color())))
                        .hover(|s| s.bg(rgba(theme::fill_hover())))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.selected_path = Some(path.clone());
                            cx.notify();
                            if let Err(error) = this.navigation.request_ui(
                                NavigationUiRequest::SelectPathPreservingPage(path.clone()),
                            ) {
                                tracing::warn!(%error, "project navigation request rejected");
                            }
                        }))
                        .child(theme::small_icon(IconName::Folder))
                        .child(node.name.clone());
                    content = content.child(row);
                }
            }
            Some(Err(error)) => {
                content = content.child(
                    div()
                        .text_sm()
                        .text_color(rgb(theme::danger_text()))
                        .child(error.clone()),
                )
            }
            None => {
                content = content.child(
                    div()
                        .text_sm()
                        .text_color(rgb(theme::text_muted()))
                        .child(fileatlas_ui_bridge::i18n::tr("Loading project folders…")),
                )
            }
        }
        content.overflow_y_scrollbar().into_any_element()
    }
}
