//! Information views for the four selection contexts in the right inspector.
use super::*;
use fileatlas_ui_bridge::components::WorkbenchPaint as _;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
use gpui_kit::assets::IconName;

pub(super) enum InspectorContext {
    Program,
    Tag(fileatlas_plugin_database::Tag),
    Folder(FolderInfo),
    File,
}

pub(super) struct FolderInfo {
    pub(super) name: String,
    pub(super) path: String,
    pub(super) count: Option<i64>,
    pub(super) subfolders: Option<usize>,
    pub(super) error: Option<String>,
}

impl InspectorContext {
    pub(super) fn key(&self) -> &'static str {
        match self {
            Self::Program => "program",
            Self::Tag(_) => "tag",
            Self::Folder(_) => "folder",
            Self::File => "file",
        }
    }
    pub(super) fn title(&self) -> &'static str {
        match self {
            Self::Program => "Program information",
            Self::Tag(_) => "Tag information",
            Self::Folder(_) => "Folder information",
            Self::File => "File information",
        }
    }

    pub(super) fn hero(&self) -> AnyElement {
        match self {
            Self::Program => hero(
                IconName::Image,
                "FileAtlas",
                &fileatlas_ui_bridge::i18n::tr("Local file browser and organizer"),
                Some(theme::accent()),
            ),
            Self::Tag(tag) => tag_hero(tag),
            Self::Folder(folder) => hero(
                IconName::Folder,
                &folder.name,
                &display_path(&folder.path),
                Some(theme::warning()),
            ),
            Self::File => div().into_any_element(),
        }
    }

    pub(super) fn body(
        &self,
        view: &mut FileManagerView,
        window: &mut Window,
        cx: &mut Context<FileManagerView>,
    ) -> AnyElement {
        match self {
            Self::Program => super::program::render(view),
            Self::Tag(tag) => super::tag::render(tag, view, window, cx),
            Self::Folder(folder) => super::folder::render(folder, view, cx),
            Self::File => inspector_empty(),
        }
    }
}

fn tag_hero(tag: &fileatlas_plugin_database::Tag) -> AnyElement {
    div()
        .w_full()
        .flex_shrink_0()
        .p(px(fileatlas_ui_bridge::theme::space_expanded()))
        .child(
            well()
                .v_flex()
                .min_h(px(108.))
                .items_start()
                .justify_center()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                .p(px(fileatlas_ui_bridge::theme::space_expanded()))
                .child(
                    fileatlas_ui_bridge::tag_capsule::TagCapsule::new(tag.name.clone())
                        .color(tag.color.as_deref())
                        .icon(tag.icon.as_deref())
                        .render(),
                )
                .child(div().workbench_assistant().child(format!(
                    "{} · {}",
                    fileatlas_ui_bridge::i18n::tr_args(
                        "{count} files",
                        &[("count", tag.media_count.to_string())]
                    ),
                    tag.full_path
                ))),
        )
        .into_any_element()
}

impl FileManagerView {
    pub(super) fn inspector_context(&self) -> InspectorContext {
        if !self.selection.selected_media_ids.is_empty() {
            return InspectorContext::File;
        }
        if self.inspector_deselected {
            return InspectorContext::Program;
        }
        let snapshot = match &self.snapshot {
            Ok(snapshot) => snapshot,
            Err(_) => return InspectorContext::Program,
        };
        let selected_tags = self
            .selected_tag
            .0
            .lock()
            .map(|selection| selection.included.clone())
            .unwrap_or_default();
        if let Some(id) = selected_tags.first() {
            if let Some(tag) = find_tag(&snapshot.tag_tree, *id) {
                return InspectorContext::Tag(tag.clone());
            }
        }

        let selected_path = self
            .selected_path
            .0
            .lock()
            .ok()
            .and_then(|path| path.clone());
        let selected_folder_id = self
            .selected_folder
            .0
            .lock()
            .ok()
            .and_then(|folder| *folder);
        let folder_path = self
            .selected_directory
            .clone()
            .or(selected_path)
            .or_else(|| {
                selected_folder_id.and_then(|id| {
                    snapshot
                        .folders
                        .iter()
                        .find(|folder| folder.id == id)
                        .map(|folder| folder.path.clone())
                })
            });
        if let Some(path) = folder_path {
            let root = snapshot
                .folders
                .iter()
                .find(|folder| std::path::Path::new(&folder.path) == std::path::Path::new(&path));
            let current = snapshot
                .browser
                .directory
                .as_deref()
                .is_some_and(|directory| same_path(directory, &path));
            let name = root.map(|folder| folder.name.clone()).unwrap_or_else(|| {
                std::path::Path::new(&path)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.clone())
            });
            let count = current.then_some(snapshot.media_count);
            let subfolders = current.then_some(snapshot.browser.directories.len());
            let error = current.then(|| snapshot.browser.error.clone()).flatten();
            return InspectorContext::Folder(FolderInfo {
                name,
                path,
                count,
                subfolders,
                error,
            });
        }
        InspectorContext::Program
    }
}

pub(super) fn same_path(left: &str, right: &str) -> bool {
    left.trim_start_matches(r"\\?\")
        .trim_end_matches(['/', '\\'])
        .eq_ignore_ascii_case(
            right
                .trim_start_matches(r"\\?\")
                .trim_end_matches(['/', '\\']),
        )
}

fn hero(icon: IconName, title: &str, subtitle: &str, accent: Option<u32>) -> AnyElement {
    let color = accent.unwrap_or(theme::accent());
    let solid = matches!(icon, IconName::Tag);
    div()
        .w_full()
        .flex_shrink_0()
        .p(px(fileatlas_ui_bridge::theme::space_expanded()))
        .child(
            well()
                .h_flex()
                .items_center()
                .min_h(px(108.))
                .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
                .p(px(fileatlas_ui_bridge::theme::space_expanded()))
                .child(
                    div()
                        .size(px(56.))
                        .flex_shrink_0()
                        .workbench_corner(fileatlas_ui_bridge::components::CornerRole::ExtraLarge)
                        .workbench_surface(
                            fileatlas_ui_bridge::components::SurfaceRole::Transparent(
                                (color << 8) | if solid { 0xff } else { 0x26 },
                            ),
                        )
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(theme::icon(icon).size(px(30.)).workbench_foreground(
                            fileatlas_ui_bridge::components::ForegroundRole::Custom(if solid {
                                theme::contrasting_text(color)
                            } else {
                                color
                            }),
                        )),
                )
                .child(
                    div()
                        .v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                        .child(
                            div()
                                .workbench_title()
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(title.to_string()),
                        )
                        .child(
                            div()
                                .workbench_assistant()
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(subtitle.to_string()),
                        ),
                ),
        )
        .into_any_element()
}

pub(super) fn count_tags(tags: &[fileatlas_plugin_database::Tag]) -> usize {
    tags.iter().map(|tag| 1 + count_tags(&tag.children)).sum()
}

pub(super) fn find_tag(
    tags: &[fileatlas_plugin_database::Tag],
    id: i64,
) -> Option<&fileatlas_plugin_database::Tag> {
    tags.iter().find_map(|tag| {
        if tag.id == id {
            Some(tag)
        } else {
            find_tag(&tag.children, id)
        }
    })
}
