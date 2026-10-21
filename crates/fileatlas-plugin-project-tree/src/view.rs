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

use crate::{ProjectTreeRoot, ProjectTreeService, same_path};

#[derive(Clone, Debug)]
struct ProjectNode {
    root_path: String,
    path: String,
    name: String,
    depth: usize,
    ancestor_continues: Vec<bool>,
    has_next_sibling: bool,
}

fn scan_project_tree(projects: &[ProjectTreeRoot]) -> Result<Vec<ProjectNode>, String> {
    fn walk(
        path: &Path,
        depth: usize,
        root_path: &str,
        ancestor_continues: &[bool],
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
        for (index, child) in dirs.iter().enumerate() {
            let name = child
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let child_path = child.to_string_lossy().into_owned();
            let has_next_sibling = index + 1 < dirs.len();
            out.push(ProjectNode {
                root_path: root_path.to_owned(),
                path: child_path.clone(),
                name,
                depth,
                ancestor_continues: ancestor_continues.to_vec(),
                has_next_sibling,
            });
            let mut child_ancestors = ancestor_continues.to_vec();
            child_ancestors.push(has_next_sibling);
            walk(child, depth + 1, root_path, &child_ancestors, out, seen)?;
        }
        Ok(())
    }
    let mut rows = Vec::new();
    for project in projects {
        let mut seen = std::collections::HashSet::new();
        rows.push(ProjectNode {
            root_path: project.path.clone(),
            path: project.path.clone(),
            name: project.name.clone(),
            depth: 0,
            ancestor_continues: Vec::new(),
            has_next_sibling: false,
        });
        walk(
            Path::new(&project.path),
            1,
            &project.path,
            &[],
            &mut rows,
            &mut seen,
        )
        .map_err(|error| format!("{}: {error}", project.path))?;
    }
    Ok(rows)
}

pub(super) fn sidebar_page(
    service: Arc<ProjectTreeService>,
    navigation: Arc<NavigationService>,
) -> UiViewContribution {
    UiViewContribution::new(
        "project-tree/sidebar-page-view",
        "navigation.sidebar-project-tree",
        0,
        move |window, app| page(Arc::clone(&service), Arc::clone(&navigation), window, app),
    )
}
fn page(
    service: Arc<ProjectTreeService>,
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
        let current_folder = navigation.sidebar_state().and_then(|state| {
            state
                .get("address_bar")
                .and_then(|address| address.get("path"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        });
        let expanded_project = current_folder
            .as_deref()
            .and_then(|path| {
                projects
                    .iter()
                    .filter(|project| path_is_within(&project.path, path))
                    .max_by_key(|project| project.path.len())
                    .map(|project| project.path.clone())
            })
            .or_else(|| projects.first().map(|project| project.path.clone()));
        tree.request(paths, move || scan_project_tree(&projects));
        let task =
            fileatlas_ui_bridge::refresh::listen(cx, wake, |view: &mut ProjectTreePage, _| {
                view.tree.poll()
            });
        ProjectTreePage {
            _task: task,
            service,
            navigation,
            selected_path: current_folder,
            expanded_project,
            tree,
            error: None,
        }
    })
    .into()
}

fn path_is_within(root: &str, candidate: &str) -> bool {
    let normalize = |path: &str| {
        let path = path
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase();
        path.strip_prefix("\\\\?\\").unwrap_or(&path).to_owned()
    };
    let root = normalize(root);
    let candidate = normalize(candidate);
    candidate == root
        || candidate
            .strip_prefix(&root)
            .is_some_and(|suffix| suffix.starts_with('\\'))
}

struct ProjectTreePage {
    _task: Task<()>,
    service: Arc<ProjectTreeService>,
    navigation: Arc<NavigationService>,
    selected_path: Option<String>,
    expanded_project: Option<String>,
    tree: ReadModel<Vec<String>, Vec<ProjectNode>>,
    error: Option<String>,
}

impl ProjectTreePage {
    fn reload(&mut self) {
        let projects = self.service.snapshot();
        let paths = projects.iter().map(|p| p.path.clone()).collect::<Vec<_>>();
        self.tree
            .request(paths, move || scan_project_tree(&projects));
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
            Ok(projects) => {
                self.selected_path = projects
                    .iter()
                    .find(|project| same_path(&project.path, &path))
                    .map(|project| project.path.clone());
                self.expanded_project = self.selected_path.clone();
                self.error = None;
                self.reload();
            }
            Err(error) => self.error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn toggle_project(&mut self, path: &str, cx: &mut Context<Self>) {
        if self
            .expanded_project
            .as_deref()
            .is_some_and(|expanded| same_path(expanded, path))
        {
            self.expanded_project = None;
        } else {
            self.expanded_project = Some(path.to_owned());
        }
        self.selected_path = Some(path.to_owned());
        cx.notify();
    }
}

impl Render for ProjectTreePage {
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
                        .child(fileatlas_ui_bridge::i18n::tr("Project tree")),
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
                    Button::new("project-tree-add")
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
        let projects = self.service.snapshot();
        let tree = self.tree.value().cloned();
        for (index, project) in projects.into_iter().enumerate() {
            let selected = selected_path
                .as_deref()
                .is_some_and(|path| same_path(path, &project.path));
            let expanded = self
                .expanded_project
                .as_deref()
                .is_some_and(|path| same_path(path, &project.path));
            let select_path = project.path.clone();
            let toggle_path = project.path.clone();
            let remove_path = project.path.clone();
            let root_row = div()
                .id(format!("project-tree-root-{index}"))
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
                    this.expanded_project = Some(select_path.clone());
                    cx.notify();
                    if let Err(error) =
                        this.navigation
                            .request_ui(NavigationUiRequest::SelectPathPreservingPage(
                                select_path.clone(),
                            ))
                    {
                        tracing::warn!(%error, "project navigation request rejected");
                    }
                }))
                .child(
                    div()
                        .h_flex()
                        .min_w_0()
                        .gap(px(8.))
                        .child(
                            Button::new(format!("project-tree-expand-{index}"))
                                .ghost()
                                .icon(theme::small_icon(if expanded {
                                    IconName::ChevronDown
                                } else {
                                    IconName::ChevronRight
                                }))
                                .tooltip(fileatlas_ui_bridge::i18n::tr(if expanded {
                                    "Collapse project folders"
                                } else {
                                    "Expand project folders"
                                }))
                                .accessibility_label(fileatlas_ui_bridge::i18n::tr(if expanded {
                                    "Collapse project folders"
                                } else {
                                    "Expand project folders"
                                }))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.toggle_project(&toggle_path, cx);
                                })),
                        )
                        .child(theme::small_icon(IconName::Folder))
                        .child(project.name),
                )
                .child(
                    Button::new(format!("remove-project-tree-root-{index}"))
                        .ghost()
                        .icon(theme::small_icon(IconName::X))
                        .accessibility_label(fileatlas_ui_bridge::i18n::tr("Remove project"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            match this.service.remove(&remove_path) {
                                Ok(remaining) => {
                                    if this
                                        .expanded_project
                                        .as_deref()
                                        .is_some_and(|path| same_path(path, &remove_path))
                                    {
                                        this.expanded_project =
                                            remaining.first().map(|project| project.path.clone());
                                    }
                                    if this
                                        .selected_path
                                        .as_deref()
                                        .is_some_and(|path| same_path(path, &remove_path))
                                    {
                                        this.selected_path = None;
                                    }
                                    this.error = None;
                                }
                                Err(error) => this.error = Some(error.to_string()),
                            }
                            this.reload();
                            cx.notify();
                        })),
                );
            let mut project_block = div().v_flex().gap_0().child(root_row);
            if expanded {
                match &tree {
                    Some(Ok(rows)) => {
                        let children = rows
                            .iter()
                            .filter(|node| {
                                node.depth > 0 && same_path(&node.root_path, &project.path)
                            })
                            .map(|node| project_tree_row(node, selected_path.as_deref(), cx))
                            .collect::<Vec<_>>();
                        project_block =
                            project_block.child(div().v_flex().gap_0().children(children));
                    }
                    Some(Err(error)) => {
                        project_block = project_block.child(
                            div()
                                .ml(px(22.))
                                .text_sm()
                                .text_color(rgb(theme::danger_text()))
                                .child(error.clone()),
                        );
                    }
                    None => {
                        project_block = project_block.child(
                            div()
                                .ml(px(22.))
                                .text_sm()
                                .text_color(rgb(theme::text_muted()))
                                .child(fileatlas_ui_bridge::i18n::tr("Loading project folders…")),
                        );
                    }
                }
            }
            content = content.child(project_block);
        }
        content.overflow_y_scrollbar().into_any_element()
    }
}

fn project_tree_row(
    node: &ProjectNode,
    selected_path: Option<&str>,
    cx: &mut Context<ProjectTreePage>,
) -> AnyElement {
    const ROW_HEIGHT: f32 = 24.;
    const DOT_STEP: f32 = 5.;
    const DOT_SIZE: f32 = 2.;
    let selected = selected_path.is_some_and(|path| same_path(path, &node.path));
    let path = node.path.clone();
    let mut guides = Vec::new();
    let mut add_vertical = |x: f32, start: f32, end: f32| {
        let mut y = start;
        while y <= end {
            guides.push(
                div()
                    .absolute()
                    .left(px(x))
                    .top(px(y))
                    .w(px(DOT_SIZE))
                    .h(px(DOT_SIZE))
                    .rounded(px(DOT_SIZE / 2.))
                    .bg(rgb(theme::separator()))
                    .into_any_element(),
            );
            y += DOT_STEP;
        }
    };
    for (level, continues) in node.ancestor_continues.iter().enumerate() {
        if *continues {
            add_vertical(14. + level as f32 * 14., 1., ROW_HEIGHT - DOT_SIZE);
        }
    }
    let branch_x = 14. + (node.depth.saturating_sub(1) as f32) * 14.;
    add_vertical(branch_x, 1., ROW_HEIGHT / 2. - DOT_SIZE / 2.);
    if node.has_next_sibling {
        add_vertical(branch_x, ROW_HEIGHT / 2. + DOT_SIZE, ROW_HEIGHT - DOT_SIZE);
    }
    let mut x = branch_x + DOT_STEP;
    while x < 8. + node.depth as f32 * 14. {
        guides.push(
            div()
                .absolute()
                .left(px(x))
                .top(px(ROW_HEIGHT / 2. - DOT_SIZE / 2.))
                .w(px(DOT_SIZE))
                .h(px(DOT_SIZE))
                .rounded(px(DOT_SIZE / 2.))
                .bg(rgb(theme::separator()))
                .into_any_element(),
        );
        x += DOT_STEP;
    }
    div()
        .id(format!("project-tree-node-{}", node.path))
        .relative()
        .h_flex()
        .items_center()
        .gap(px(7.))
        .px(px(8. + node.depth as f32 * 14.))
        .h(px(ROW_HEIGHT))
        .text_color(rgb(if selected {
            theme::selection_text()
        } else {
            theme::text_secondary()
        }))
        .when(selected, |style| style.bg(rgb(theme::selection_color())))
        .hover(|style| style.bg(rgba(theme::fill_hover())))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.selected_path = Some(path.clone());
            cx.notify();
            if let Err(error) = this
                .navigation
                .request_ui(NavigationUiRequest::SelectPathPreservingPage(path.clone()))
            {
                tracing::warn!(%error, "project navigation request rejected");
            }
        }))
        .child(theme::small_icon(IconName::Folder))
        .child(node.name.clone())
        .children(guides)
        .into_any_element()
}
