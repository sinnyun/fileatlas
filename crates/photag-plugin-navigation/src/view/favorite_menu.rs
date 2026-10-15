use super::*;
use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
pub(super) fn favorite_menu(
    mut menu: PopupMenu,
    folder_id: i64,
    folder_path: String,
    folder_view: WeakEntity<SidebarView>,
) -> PopupMenu {
    let rename_view = folder_view.clone();
    menu = menu.item(
        PopupMenuItem::new(photag_ui_bridge::i18n::tr("Rename favorite")).on_click(
            move |_, window, cx| {
                let _ = rename_view.update(cx, |view, cx| {
                    let name = view
                        .snapshot
                        .as_ref()
                        .ok()
                        .and_then(|s| s.folders.iter().find(|n| n.folder.id == folder_id))
                        .map(|n| n.folder.name.clone())
                        .unwrap_or_default();
                    view.tag_name
                        .update(cx, |input, cx| input.set_value(name, window, cx));
                    view.rename_favorite = Some(folder_id);
                    cx.notify();
                });
            },
        ),
    );
    for (label, delta) in [("Move favorite up", -1), ("Move favorite down", 1)] {
        let target_view = folder_view.clone();
        menu = menu.item(
            PopupMenuItem::new(photag_ui_bridge::i18n::tr(label)).on_click(move |_, _, cx| {
                let _ = target_view.update(cx, |view, cx| {
                    if let Err(error) = view.service.move_favorite(folder_id, delta) {
                        view.snapshot = Err(error.to_string());
                    } else {
                        view.refresh();
                    }
                    cx.notify();
                });
            }),
        );
    }
    let create_path = folder_path.clone();
    let create_view = folder_view.clone();
    menu = menu.item(
        PopupMenuItem::new(photag_ui_bridge::i18n::tr("New subfolder…")).on_click(
            move |_, window, cx| {
                let _ = create_view.update(cx, |this, cx| {
                    this.request_new_subfolder(create_path.clone(), window, cx)
                });
            },
        ),
    );
    let rescan_view = folder_view.clone();
    menu = menu.item(
        PopupMenuItem::new(photag_ui_bridge::i18n::tr("Refresh folder")).on_click(
            move |_, _, cx| {
                let _ = rescan_view.update(cx, |this, cx| {
                    this.rescan(folder_id, cx);
                    cx.notify();
                });
            },
        ),
    );
    let open_path = folder_path.clone();
    let open_view = folder_view.clone();
    menu = menu.item(
        PopupMenuItem::new(photag_ui_bridge::i18n::tr("Open in file manager")).on_click(
            move |_, _, cx| {
                let _ = open_view.update(cx, |this, _| this.open_folder_in_explorer(&open_path));
            },
        ),
    );
    let remove_view = folder_view.clone();
    menu.item(
        PopupMenuItem::new(photag_ui_bridge::i18n::tr("Remove")).on_click(move |_, _, cx| {
            let _ = remove_view.update(cx, |this, cx| {
                this.remove_folder(folder_id, cx);
                cx.notify();
            });
        }),
    )
}
