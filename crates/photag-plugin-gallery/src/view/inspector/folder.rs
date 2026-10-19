//! Filesystem browsing details; counts come from the current projection.
use super::*;
use photag_plugin_navigation::{NavigationService, NavigationUiRequest};

pub(super) fn render(
    folder: &contexts::FolderInfo,
    _: &GalleryView,
    cx: &mut Context<GalleryView>,
) -> AnyElement {
    let path = folder.path.clone();
    let copy_path = display_path(&path);
    let refresh_path = path.clone();
    let count = folder
        .count
        .map(|n| n.to_string())
        .unwrap_or_else(|| "—".into());
    let children = folder
        .subfolders
        .map(|n| n.to_string())
        .unwrap_or_else(|| "—".into());
    let mut body = div()
        .v_flex()
        .w_full()
        .min_w_0()
        .px(px(16.))
        .pb(px(20.))
        .child(section_title("Folder details"))
        .child(detail_row(
            IconName::Folder,
            "Folder name",
            folder.name.clone(),
        ))
        .child(detail_row(IconName::File, "Current results", count))
        .child(detail_row(IconName::Folder, "Subfolders", children))
        .child(inspector_divider())
        .child(section_title("Location"))
        .child(
            well()
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(theme::text_secondary()))
                        .child(display_path(&folder.path)),
                )
                .child(
                    Button::new("inspector-folder-copy")
                        .ghost()
                        .w_full()
                        .label(photag_ui_bridge::i18n::tr("Copy path"))
                        .on_click(cx.listener(move |_, _, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(copy_path.clone()))
                        })),
                ),
        )
        .child(inspector_divider())
        .child(section_title("Browsing state"))
        .child(detail_row(
            IconName::Info,
            "Browsing state",
            photag_ui_bridge::i18n::tr(if folder.error.is_some() {
                "Failed"
            } else {
                "Ready"
            }),
        ))
        .child(inspector_divider())
        .child(section_title("Actions"))
        .child(
            Button::new("inspector-folder-open")
                .outline()
                .w_full()
                .h(px(36.))
                .icon(theme::icon(IconName::FolderOpen))
                .label(photag_ui_bridge::i18n::tr("Open folder"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Ok(nav) = this.cordis.try_service::<NavigationService>() {
                        if let Err(error) =
                            nav.request_ui(NavigationUiRequest::SelectPath(path.clone()))
                        {
                            this.operation_status = Some(error.to_string());
                        }
                    }
                    cx.notify();
                })),
        )
        .child(div().h(px(8.)).flex_shrink_0())
        .child(
            Button::new("inspector-folder-refresh")
                .outline()
                .w_full()
                .h(px(36.))
                .icon(theme::icon(IconName::RefreshCw))
                .label(photag_ui_bridge::i18n::tr("Refresh"))
                .on_click(cx.listener(move |this, _, _, _| {
                    let current = this
                        .snapshot
                        .as_ref()
                        .ok()
                        .and_then(|s| s.browser.directory.as_deref());
                    if current.is_some_and(|path| contexts::same_path(path, &refresh_path)) {
                        this.service.request_ui(crate::GalleryUiRequest::Refresh);
                    } else if let Ok(nav) = this.cordis.try_service::<NavigationService>() {
                        let _ =
                            nav.request_ui(NavigationUiRequest::SelectPath(refresh_path.clone()));
                    }
                })),
        );
    if let Some(error) = &folder.error {
        body = body.child(inspector_message(error));
    }
    if folder.count.is_none() {
        body = body.child(inspector_message(
            "Open this folder to see its current contents.",
        ));
    }
    body.into_any_element()
}
