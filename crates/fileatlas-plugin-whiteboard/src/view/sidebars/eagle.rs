use super::*;

pub(super) fn eagle_rows(
    source: &fileatlas_plugin_eagle_links::EagleImageSource,
    eagle_links: Option<Arc<fileatlas_plugin_eagle_links::EagleLinkService>>,
    cx: &mut Context<Host>,
) -> Div {
    let name = format!("{}  ·  {}", source.name, source.item_id);
    let folders = if source.folders.is_empty() {
        "未分类".to_owned()
    } else {
        source
            .folders
            .iter()
            .map(|folder| folder.name.as_str())
            .collect::<Vec<_>>()
            .join(" / ")
    };
    let explorer_path = windows_explorer_path(&source.original_path);
    let open_button = eagle_links.map(|service| {
        let item_id = source.item_id.clone();
        components::icon_button("canvas-open-eagle-item", IconName::ExternalLink)
            .tooltip("在 Eagle 中打开并定位此图片")
            .on_click(cx.listener(move |view, _, _, cx| {
                view.result(service.open_item(&item_id), cx);
            }))
            .into_any_element()
    });
    div()
        .v_flex()
        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
        .children(open_button.map(|button| div().w_full().h_flex().justify_end().child(button)))
        .child(value_row(
            IconName::Image,
            "Eagle 条目",
            name.clone(),
            Some(name),
            cx,
        ))
        .child(value_row(
            IconName::Folder,
            "所属文件夹",
            folders.clone(),
            Some(folders),
            cx,
        ))
        .child(value_row(
            IconName::MapPin,
            "本地文件路径",
            path_tail(&explorer_path),
            Some(explorer_path),
            cx,
        ))
        .child(value_row(
            IconName::Link2,
            "画布资源 URI",
            "FileAtlas URI".to_owned(),
            Some(source.original_uri.clone()),
            cx,
        ))
        .child(value_row(
            IconName::FileImage,
            "缩略图路径",
            path_tail(&source.thumbnail_path),
            Some(source.thumbnail_path.clone()),
            cx,
        ))
        .child(value_row(
            IconName::Globe,
            "Eagle API",
            format!("条目 {}", source.item_id),
            Some(source.api_url.clone()),
            cx,
        ))
}

pub(super) fn value_row(
    icon: IconName,
    label: &str,
    value: String,
    copy: Option<String>,
    cx: &mut Context<Host>,
) -> Div {
    let mut row = div()
        .w_full()
        .min_w_0()
        .h_flex()
        .items_center()
        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
        .child(fileatlas_ui_bridge::theme::icon(icon).flex_shrink_0())
        .child(
            div()
                .flex_1()
                .min_w_0()
                .h_flex()
                .items_center()
                .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                .child(div().workbench_assistant().child(label.to_owned()))
                .child(div().flex_1().min_w_0().truncate().child(value.clone())),
        );
    if let Some(text) = copy {
        let clipboard = text.clone();
        let tooltip = format!("复制{}：{}", label, text);
        row = row.child(
            components::icon_button(format!("canvas-copy-{label}"), IconName::Copy)
                .tooltip(tooltip)
                .on_click(cx.listener(move |_, _, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(clipboard.clone()));
                })),
        );
    }
    row
}

fn path_tail(path: &str) -> String {
    let parts = Path::new(path)
        .components()
        .filter_map(|part| part.as_os_str().to_str())
        .collect::<Vec<_>>();
    let tail = parts.iter().rev().take(2).copied().collect::<Vec<_>>();
    if tail.is_empty() {
        path.to_owned()
    } else {
        format!("…/{}", tail.into_iter().rev().collect::<Vec<_>>().join("/"))
    }
}

fn windows_explorer_path(path: &str) -> String {
    if let Some(path) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{path}")
    } else {
        path.strip_prefix(r"\\?\").unwrap_or(path).to_owned()
    }
}
