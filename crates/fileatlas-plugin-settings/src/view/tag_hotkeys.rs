use super::SettingsView;
use super::tag_choices::TagChoice;
use gpui_kit::component::button::Button;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::*;

pub(super) fn button(
    key: &'static str,
    selected: Option<i64>,
    choices: Vec<TagChoice>,
    width: f32,
    view: WeakEntity<SettingsView>,
) -> impl IntoElement {
    let selected_tag = selected.and_then(|id| choices.iter().find(|tag| tag.id == id).cloned());
    let label = selected_tag
        .as_ref()
        .map(|tag| tag.label.clone())
        .unwrap_or_else(|| {
            if selected.is_some() {
                fileatlas_ui_bridge::i18n::tr("(deleted tag)")
            } else {
                fileatlas_ui_bridge::i18n::tr("None")
            }
        });
    let mut button = Button::new(format!("settings-tag-hotkey-{key}"))
        .compact()
        // DropdownMenu wraps its trigger; flex on Button does not size that wrapper.
        .w(px(width))
        .min_w_0()
        .justify_start()
        .h(px(26.));
    if let Some(tag) = selected_tag {
        button = button.child(
            fileatlas_ui_bridge::tag_capsule::TagCapsule::new(tag.name)
                .color(tag.color.as_deref())
                .icon(tag.icon.as_deref())
                .render()
                .max_w(px((width - 48.).max(1.))),
        );
    } else {
        button = button.label(label);
    }
    button
        .dropdown_caret(true)
        .dropdown_menu(move |mut menu, _, _| {
            let none_view = view.clone();
            menu = menu.item(
                PopupMenuItem::new(fileatlas_ui_bridge::i18n::tr("None"))
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
            for tag in &choices {
                let tag_id = tag.id;
                let tag_label = tag.label.clone();
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
