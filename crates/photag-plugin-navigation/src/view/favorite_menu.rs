//! Folder favorite actions remain available on unified bookmark rows.
use super::*;
use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
pub(super) fn favorite_menu(
    menu: PopupMenu,
    folder_id: i64,
    folder_path: String,
    view: WeakEntity<SidebarView>,
) -> PopupMenu {
    let create = view.clone();
    let rescan = view.clone();
    menu.item(
        PopupMenuItem::new(photag_ui_bridge::i18n::tr("New subfolder…")).on_click(
            move |_, window, cx| {
                let _ = create.update(cx, |this, cx| {
                    this.request_new_subfolder(folder_path.clone(), window, cx)
                });
            },
        ),
    )
    .item(
        PopupMenuItem::new(photag_ui_bridge::i18n::tr("Refresh folder")).on_click(
            move |_, _, cx| {
                let _ = rescan.update(cx, |this, cx| {
                    this.rescan(folder_id, cx);
                    cx.notify();
                });
            },
        ),
    )
    .item(
        PopupMenuItem::new(photag_ui_bridge::i18n::tr("Remove favorite")).on_click(
            move |_, _, cx| {
                let _ = view.update(cx, |this, cx| {
                    this.remove_folder(folder_id, cx);
                    cx.notify();
                });
            },
        ),
    )
}
