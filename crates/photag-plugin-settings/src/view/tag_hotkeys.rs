use super::SettingsView;
use gpui_kit::component::button::Button;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::*;

pub(super) fn button(
    key: &'static str,
    selected: Option<i64>,
    choices: Vec<(i64, String)>,
    view: WeakEntity<SettingsView>,
) -> impl IntoElement {
    let label = selected
        .and_then(|id| choices.iter().find(|(tag_id, _)| *tag_id == id))
        .map(|(_, label)| label.clone())
        .unwrap_or_else(|| {
            if selected.is_some() {
                photag_ui_bridge::i18n::tr("(deleted tag)")
            } else {
                photag_ui_bridge::i18n::tr("None")
            }
        });
    Button::new(format!("settings-tag-hotkey-{key}"))
        .compact()
        .w(px(360.))
        .h(px(28.))
        .label(label)
        .dropdown_caret(true)
        .dropdown_menu(move |mut menu, _, _| {
            let none_view = view.clone();
            menu = menu.item(
                PopupMenuItem::new(photag_ui_bridge::i18n::tr("None"))
                    .checked(selected.is_none())
                    .on_click(move |_, _, cx| {
                        let _ = none_view.update(cx, |this, cx| {
                            this.edit(
                                |settings| {
                                    settings.tag_hotkeys.retain(|hotkey| hotkey.key != key);
                                },
                                cx,
                            );
                        });
                    }),
            );
            for (tag_id, tag_label) in &choices {
                let tag_id = *tag_id;
                let tag_label = tag_label.clone();
                let view = view.clone();
                menu = menu.item(
                    PopupMenuItem::new(tag_label)
                        .checked(selected == Some(tag_id))
                        .on_click(move |_, _, cx| {
                            let _ = view.update(cx, |this, cx| {
                                this.edit(
                                    |settings| {
                                        settings.tag_hotkeys.retain(|hotkey| hotkey.key != key);
                                        settings.tag_hotkeys.push(crate::TagHotkey {
                                            key: key.to_owned(),
                                            tag_id,
                                        });
                                    },
                                    cx,
                                );
                            });
                        }),
                );
            }
            menu
        })
}
