use super::*;
use crate::PinScope;
use gpui_kit::component::menu::PopupMenu;

pub(super) fn program_menu(
    mut menu: PopupMenu,
    entry: LauncherEntry,
    categories: Vec<LauncherCategory>,
    view: WeakEntity<LauncherPanel>,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let id = entry.id;
    let run = view.clone();
    menu = menu.item(PopupMenuItem::new(tr("Launch")).on_click(move |_, _, cx| {
        let _ = run.update(cx, |this, cx| {
            let result = this.service.launch(id);
            this.report(result, cx);
        });
    }));
    let rename = view.clone();
    menu = menu.item(
        PopupMenuItem::new(tr("Rename")).on_click(move |_, window, cx| {
            let _ = rename.update(cx, |this, cx| {
                this.select_entry(id, window, cx);
                this.entry_name.focus_handle(cx).focus(window, cx);
            });
        }),
    );
    let pinning = view.clone();
    menu = menu.submenu(tr("Pin to top"), window, cx, move |mut submenu, _, _| {
        for (scope, label) in [
            (PinScope::Category, "Pin to category top"),
            (PinScope::AllPrograms, "Pin to all programs top"),
            (PinScope::None, "Unpin"),
        ] {
            let pinning = pinning.clone();
            let marker = if entry.pin_scope == scope { "✓ " } else { "" };
            submenu = submenu.item(
                PopupMenuItem::new(format!("{marker}{}", tr(label))).on_click(move |_, _, cx| {
                    let _ = pinning.update(cx, |this, cx| {
                        this.report(this.service.set_pin_scope(id, scope), cx);
                    });
                }),
            );
        }
        submenu
    });
    let moving = view.clone();
    menu = menu.submenu(
        tr("Move to category"),
        window,
        cx,
        move |mut submenu, _, _| {
            for category in categories.iter().filter(|c| c.id != entry.category_id) {
                let moving = moving.clone();
                let category_id = category.id;
                submenu = submenu.item(
                    PopupMenuItem::new(cards::category_name(&category.name))
                        .icon(theme::icon(category_icon(&category.name)))
                        .on_click(move |_, _, cx| {
                            let _ = moving.update(cx, |this, cx| {
                                let result = this.service.move_entry(id, category_id);
                                if result.is_ok() && this.selected_entry == Some(id) {
                                    this.draft_category = category_id;
                                }
                                this.report(result, cx);
                            });
                        }),
                );
            }
            submenu
        },
    );
    let hiding = view.clone();
    menu = menu.item(
        PopupMenuItem::new(tr(if entry.hidden { "Show" } else { "Hide" })).on_click(
            move |_, _, cx| {
                let _ = hiding.update(cx, |this, cx| {
                    let result = this.service.set_hidden(id, !entry.hidden);
                    if result.is_ok() && this.selected_entry == Some(id) {
                        this.draft_hidden = !entry.hidden;
                    }
                    this.report(result, cx);
                });
            },
        ),
    );
    if entry.uninstall_command.is_some() {
        let uninstall = view.clone();
        menu =
            menu.separator().item(
                PopupMenuItem::new(tr("Uninstall")).on_click(move |_, _, cx| {
                    let _ = uninstall.update(cx, |this, cx| {
                        let result = this.service.uninstall(id);
                        this.report(result, cx);
                    });
                }),
            );
    }
    menu.separator()
        .item(PopupMenuItem::new(tr("Remove")).on_click(move |_, _, cx| {
            let _ = view.update(cx, |this, cx| {
                let result = this.service.remove_entry(id);
                this.report(result, cx);
            });
        }))
}
