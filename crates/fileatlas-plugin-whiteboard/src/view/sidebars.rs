use super::*;
use gpui_kit::assets::IconName;
use gpui_kit::base::{Disableable, StyledExt};
use std::path::Path;
use url::Url;

mod eagle;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LeftPanel {
    Boards,
    Resources,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RightPanel {
    Layers,
    Object,
}

pub(super) fn left_rail(host: &Host, board_title: &str, cx: &mut Context<Host>) -> AnyElement {
    div()
        .id("whiteboard-left-panel-rail")
        .w(px(64.))
        .h_full()
        .min_h_0()
        .v_flex()
        .items_center()
        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
        .workbench_region(true)
        .p(px(fileatlas_ui_bridge::theme::space_compact()))
        .child(
            div()
                .w_full()
                .min_w_0()
                .v_flex()
                .items_center()
                .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                .child(
                    div()
                        .w_full()
                        .truncate()
                        .text_center()
                        .workbench_title()
                        .child("画布"),
                )
                .child(
                    div()
                        .w_full()
                        .truncate()
                        .text_center()
                        .workbench_assistant()
                        .child(board_title.to_owned()),
                ),
        )
        .child(
            components::rail_shortcut(
                "canvas-panel-boards",
                "画板",
                fileatlas_ui_bridge::theme::icon(IconName::LayoutTemplate),
                host.left_panel == Some(LeftPanel::Boards),
            )
            .on_click(cx.listener(|h, _, _, cx| h.toggle_left_panel(LeftPanel::Boards, cx))),
        )
        .child(
            components::rail_shortcut(
                "canvas-panel-resources",
                "资源",
                fileatlas_ui_bridge::theme::icon(IconName::FolderOpen),
                host.left_panel == Some(LeftPanel::Resources),
            )
            .on_click(cx.listener(|h, _, _, cx| h.toggle_left_panel(LeftPanel::Resources, cx))),
        )
        .into_any_element()
}
pub(super) fn right_rail(host: &Host, cx: &mut Context<Host>) -> AnyElement {
    div()
        .id("whiteboard-right-panel-rail")
        .w(px(64.))
        .h_full()
        .min_h_0()
        .v_flex()
        .items_center()
        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
        .workbench_region(true)
        .p(px(fileatlas_ui_bridge::theme::space_compact()))
        .child(
            components::rail_shortcut(
                "canvas-panel-layers",
                "图层",
                fileatlas_ui_bridge::theme::icon(IconName::Layers),
                host.right_panel == Some(RightPanel::Layers),
            )
            .on_click(cx.listener(|h, _, _, cx| h.toggle_right_panel(RightPanel::Layers, cx))),
        )
        .child(
            components::rail_shortcut(
                "canvas-panel-object",
                "对象",
                fileatlas_ui_bridge::theme::icon(IconName::Square),
                host.right_panel == Some(RightPanel::Object),
            )
            .on_click(cx.listener(|h, _, _, cx| h.toggle_right_panel(RightPanel::Object, cx))),
        )
        .into_any_element()
}
pub(super) fn left_drawer(
    host: &Host,
    library: &store::Library,
    panel: LeftPanel,
    cx: &mut Context<Host>,
) -> AnyElement {
    let (title, content) = match panel {
        LeftPanel::Boards => ("画板", board_content(host, library, cx)),
        LeftPanel::Resources => ("资源", resource_content(library, cx)),
    };
    let close = components::icon_button("canvas-collapse-left-drawer", IconName::ChevronLeft)
        .tooltip("折叠面板")
        .on_click(cx.listener(move |h, _, _, cx| h.toggle_left_panel(panel, cx)));
    drawer("whiteboard-left-drawer", title, 248., close, content)
}
pub(super) fn right_drawer(
    host: &Host,
    library: &store::Library,
    panel: RightPanel,
    cx: &mut Context<Host>,
) -> AnyElement {
    let (title, width, content) = match panel {
        RightPanel::Layers => ("图层", 260., layer_content(host, cx)),
        RightPanel::Object => ("对象", 300., object_content(host, library, cx)),
    };
    let close = components::icon_button("canvas-collapse-right-drawer", IconName::ChevronRight)
        .tooltip("折叠面板")
        .on_click(cx.listener(move |h, _, _, cx| h.toggle_right_panel(panel, cx)));
    drawer("whiteboard-right-drawer", title, width, close, content)
}
fn drawer(
    id: &'static str,
    title: &str,
    width: f32,
    close: impl IntoElement,
    content: impl IntoElement,
) -> AnyElement {
    div()
        .id(id)
        .w(px(width))
        .h_full()
        .min_w_0()
        .min_h_0()
        .v_flex()
        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
        .workbench_region(true)
        .p(px(fileatlas_ui_bridge::theme::space_standard()))
        .child(
            div()
                .w_full()
                .h_flex()
                .items_center()
                .justify_between()
                .child(components::heading_block(title, "画布面板"))
                .child(close),
        )
        .child(
            div()
                .flex_1()
                .min_h_0()
                .overflow_y_scrollbar()
                .child(content),
        )
        .into_any_element()
}
fn board_content(host: &Host, library: &store::Library, cx: &mut Context<Host>) -> AnyElement {
    let rows = library.boards.iter().map(|record| {
        let id = record.id;
        div()
            .id(format!("whiteboard-item-{id}"))
            .w_full()
            .min_w_0()
            .h_flex()
            .items_center()
            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
            .px(px(fileatlas_ui_bridge::theme::space_compact()))
            .py(px(fileatlas_ui_bridge::theme::space_compact()))
            .workbench_nav(host.active_id == Some(id))
            .child(fileatlas_ui_bridge::theme::icon(IconName::LayoutTemplate).flex_shrink_0())
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .child(record.title.clone()),
            )
            .on_click(cx.listener(move |h, _, _, cx| {
                let result = h.service.activate(id);
                h.result(result, cx);
            }))
            .into_any_element()
    });
    let can_edit = host.active_id.is_some();
    div()
        .id("whiteboard-board-content")
        .v_flex()
        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
        .child(components::text_input(&host.name))
        .child(
            div()
                .h_flex()
                .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                .child(
                    components::action("whiteboard-new", "添加画板", true)
                        .on_click(cx.listener(|h, _, _, cx| h.create(cx))),
                )
                .child(
                    components::action("whiteboard-rename", "重命名", false)
                        .disabled(!can_edit)
                        .on_click(cx.listener(|h, _, _, cx| h.rename(cx))),
                ),
        )
        .child(
            div()
                .h_flex()
                .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                .child(
                    components::icon_button("whiteboard-import", IconName::FileInput)
                        .tooltip("导入画板")
                        .on_click(cx.listener(|h, _, _, cx| h.import(cx))),
                )
                .child(
                    components::icon_button("whiteboard-export", IconName::FileOutput)
                        .tooltip("导出当前画板")
                        .disabled(!can_edit)
                        .on_click(cx.listener(|h, _, _, cx| h.export(cx))),
                )
                .child(
                    components::icon_button("whiteboard-remove", IconName::Trash)
                        .tooltip("移除当前画板")
                        .disabled(!can_edit)
                        .on_click(cx.listener(|h, _, _, cx| h.remove(cx))),
                )
                .child(
                    components::icon_button("whiteboard-refresh", IconName::RefreshCw)
                        .tooltip("刷新图片缓存")
                        .on_click(cx.listener(|h, _, _, cx| {
                            h.images.borrow_mut().clear();
                            if let Some(b) = &h.board {
                                b.update(cx, |_, cx| cx.notify());
                            }
                            cx.notify();
                        })),
                ),
        )
        .child(components::section_heading("工作区"))
        .children(rows)
        .into_any_element()
}
fn resource_content(library: &store::Library, cx: &mut Context<Host>) -> AnyElement {
    let rows = library.links.iter().map(|file| {
        let id = file.id;
        let name = resource_name(&file.uri);
        let ext = Path::new(&name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_ascii_uppercase();
        let (kind, icon) = resource_kind_and_icon(&file.uri);
        let subtitle = if ext.is_empty() {
            kind.to_owned()
        } else {
            format!("{kind} · {ext}")
        };
        div()
            .id(format!("canvas-resource-{id}"))
            .w_full()
            .min_w_0()
            .h_flex()
            .items_center()
            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
            .px(px(fileatlas_ui_bridge::theme::space_compact()))
            .py(px(fileatlas_ui_bridge::theme::space_compact()))
            .workbench_nav(false)
            .child(fileatlas_ui_bridge::theme::icon(icon).flex_shrink_0())
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .v_flex()
                    .child(div().truncate().child(name.clone()))
                    .child(div().workbench_assistant().child(subtitle.clone())),
            )
            .child(
                components::icon_button(
                    format!("canvas-resource-open-{id}"),
                    IconName::ExternalLink,
                )
                .tooltip(format!("打开资源：{name} · {subtitle}"))
                .on_click(cx.listener(move |h, _, _, cx| h.open_file(id, cx))),
            )
            .into_any_element()
    });
    div()
        .id("whiteboard-resource-content")
        .v_flex()
        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
        .child(components::section_heading("文件与视频卡片"))
        .children(rows)
        .when(library.links.is_empty(), |el| {
            el.child(div().workbench_assistant().child("暂无资源"))
        })
        .into_any_element()
}
fn layer_content(host: &Host, cx: &mut Context<Host>) -> AnyElement {
    let Some(board) = host.board.clone() else {
        return div().into_any_element();
    };
    let selected = board.read(cx).selected_ids().to_vec();
    let elements = board.read(cx).elements().to_vec();
    let rows = elements
        .iter()
        .enumerate()
        .rev()
        .map(|(index, item)| {
            let id = item.id;
            let label = layer_label(item);
            let icon = layer_icon(item);
            let chosen = selected.contains(&id);
            let a = board.clone();
            let b = board.clone();
            let c = board.clone();
            let count = elements.len();
            div()
                .id(format!("canvas-layer-row-{id}"))
                .w_full()
                .min_w_0()
                .h_flex()
                .items_center()
                .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                .child(
                    div()
                        .id(format!("canvas-layer-select-{id}"))
                        .flex_1()
                        .min_w_0()
                        .h_flex()
                        .items_center()
                        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                        .px(px(fileatlas_ui_bridge::theme::space_compact()))
                        .py(px(fileatlas_ui_bridge::theme::space_compact()))
                        .workbench_nav(chosen)
                        .child(fileatlas_ui_bridge::theme::icon(icon).flex_shrink_0())
                        .child(div().flex_1().min_w_0().truncate().child(label))
                        .on_click(cx.listener(move |_, _, _, cx| {
                            a.update(cx, |v, cx| v.select_element(id, cx))
                        })),
                )
                .child(
                    components::icon_button(format!("canvas-layer-up-{id}"), IconName::ArrowUp)
                        .tooltip("上移一层")
                        .disabled(index + 1 >= count)
                        .on_click(cx.listener(move |_, _, w, cx| {
                            b.update(cx, |v, cx| v.move_element_to(id, index + 1, w, cx))
                        })),
                )
                .child(
                    components::icon_button(format!("canvas-layer-down-{id}"), IconName::ArrowDown)
                        .tooltip("下移一层")
                        .disabled(index == 0)
                        .on_click(cx.listener(move |_, _, w, cx| {
                            c.update(cx, |v, cx| v.move_element_to(id, index - 1, w, cx))
                        })),
                )
                .into_any_element()
        })
        .collect::<Vec<_>>();
    div()
        .id("canvas-layer-content")
        .v_flex()
        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
        .child(components::heading_block("图层", "置顶图层显示在前"))
        .children(rows)
        .when(elements.is_empty(), |el| {
            el.child(div().workbench_assistant().child("画布暂无对象"))
        })
        .into_any_element()
}
fn object_content(host: &Host, library: &store::Library, cx: &mut Context<Host>) -> AnyElement {
    let Some(board) = host.board.clone() else {
        return div().into_any_element();
    };
    let selected = board.read(cx).selected_ids().to_vec();
    let elements = board.read(cx).elements().to_vec();
    let active = library.boards.iter().find(|b| Some(b.id) == host.active_id);
    let source = active.and_then(|r| {
        selected.first().and_then(|id| {
            let e = elements.iter().find(|e| e.id == *id)?;
            let ElementKind::Image(image) = &e.kind else {
                return None;
            };
            r.eagle_sources.iter().find(|s| s.original_uri == image.src)
        })
    });
    let content = selected.first().and_then(|id| {
        let e = elements.iter().find(|e| e.id == *id)?;
        let [x0, y0, x1, y1] = board.read(cx).element_bounds(*id)?;
        let details = if let Some(source) = source {
            eagle::eagle_rows(source, host.eagle_links.clone(), cx)
        } else if let ElementKind::Image(image) = &e.kind {
            let name = resource_name(&image.src);
            div()
                .v_flex()
                .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                .child(eagle::value_row(
                    IconName::Image,
                    "图片",
                    name.clone(),
                    Some(name),
                    cx,
                ))
        } else {
            div().child(eagle::value_row(
                layer_icon(e),
                "对象",
                layer_label(e),
                None,
                cx,
            ))
        };
        Some(
            div()
                .v_flex()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                .child(components::heading_block("选中对象", "图标与参数"))
                .child(details)
                .child(
                    div()
                        .v_flex()
                        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                        .child(eagle::value_row(
                            IconName::Move,
                            "位置",
                            format!("X {:.0}  ·  Y {:.0}", x0, y0),
                            None,
                            cx,
                        ))
                        .child(eagle::value_row(
                            IconName::Scaling,
                            "大小",
                            format!("{:.0} × {:.0}", x1 - x0, y1 - y0),
                            None,
                            cx,
                        )),
                )
                .into_any_element(),
        )
    });
    div()
        .id("canvas-object-content")
        .v_flex()
        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
        .child(content.unwrap_or_else(|| {
            div()
                .workbench_assistant()
                .child("未选择图层")
                .into_any_element()
        }))
        .into_any_element()
}
pub(super) fn resource_name(uri: &str) -> String {
    if let Ok(url) = Url::parse(uri) {
        if let Ok(path) = url.to_file_path() {
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                return name.to_owned();
            }
        }
        if let Some(name) = url.path_segments().and_then(|mut p| p.next_back()) {
            return name.to_owned();
        }
    }
    uri.rsplit(['/', '\\']).next().unwrap_or(uri).to_owned()
}
fn resource_kind_and_icon(uri: &str) -> (&'static str, IconName) {
    let name = resource_name(uri);
    let ext = Path::new(&name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "tif" | "tiff" | "avif" => {
            ("图片", IconName::FileImage)
        }
        "mp4" | "m4v" | "mov" | "mkv" | "webm" | "avi" => ("视频", IconName::FileVideoCamera),
        "mp3" | "wav" | "flac" => ("音频", IconName::FileMusic),
        "pdf" | "txt" | "md" | "doc" | "docx" | "xls" | "xlsx" => ("文档", IconName::FileText),
        "zip" | "rar" | "7z" => ("压缩包", IconName::FileArchive),
        _ => ("文件", IconName::File),
    }
}
fn layer_icon(e: &CanvasElement) -> IconName {
    match &e.kind {
        ElementKind::Image(_) => IconName::Image,
        ElementKind::Embed(_) => IconName::File,
        ElementKind::Text(_) => IconName::Type,
        ElementKind::Draw(_) => IconName::Pencil,
        ElementKind::Line(_) | ElementKind::Arrow(_) => IconName::Waypoints,
        _ => IconName::Shapes,
    }
}
