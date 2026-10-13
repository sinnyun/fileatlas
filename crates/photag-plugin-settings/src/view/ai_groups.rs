use super::SettingsView;
use crate::{AiTagGroup, Settings};
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::Input;
use gpui_kit::*;
use std::sync::Weak;

impl SettingsView {
    pub(super) fn ai_groups(&self, settings: &Settings, cx: &mut Context<Self>) -> Div {
        let mut panel = div().v_flex().gap_2();
        let mut available_tags = Vec::new();
        if let Some(Ok(snapshot)) = self
            .navigation
            .as_ref()
            .and_then(Weak::upgrade)
            .map(|navigation| navigation.snapshot())
        {
            let mut pending: Vec<_> = snapshot.tags.into_iter().rev().collect();
            while let Some(tag) = pending.pop() {
                available_tags.push((
                    tag.name.clone(),
                    format!(
                        "{} {}",
                        tag.icon
                            .as_deref()
                            .filter(|icon| !icon.is_empty())
                            .unwrap_or("🏷"),
                        tag.full_path.replace('>', " > ")
                    ),
                ));
                pending.extend(tag.children.into_iter().rev());
            }
        }
        if available_tags.is_empty() {
            panel = panel.child(div().child(
                "No library tags are available for adding to groups while Navigation is stopped or the tag library is empty.",
            ));
        }

        for (index, group_settings) in settings.ai.mandatory_groups.iter().enumerate() {
            let mut group = div().v_flex().gap_1().p_2().child(
                div()
                    .h_flex()
                    .gap_1()
                    .items_center()
                    .child(format!("Group {}", index + 1)),
            );
            if self.editing_group == Some(index) {
                group = group.child(
                    div()
                        .h_flex()
                        .gap_1()
                        .child(Input::new(&self.group_name_input).flex_1())
                        .child(
                            Button::new(format!("settings-group-name-save-{index}"))
                                .label("Save name")
                                .on_click(cx.listener(move |view, _, _, cx| {
                                    let name =
                                        view.group_name_input.read(cx).value().trim().to_owned();
                                    if !name.is_empty() {
                                        view.edit(
                                            |settings| {
                                                if let Some(group) =
                                                    settings.ai.mandatory_groups.get_mut(index)
                                                {
                                                    group.name = name.clone();
                                                }
                                            },
                                            cx,
                                        );
                                    }
                                    view.editing_group = None;
                                    cx.notify();
                                })),
                        ),
                );
            } else {
                group = group.child(div().child(group_settings.name.clone())).child(
                    Button::new(format!("settings-group-name-edit-{index}"))
                        .label("Rename")
                        .on_click(cx.listener(move |view, _, window, cx| {
                            let name = view
                                .service
                                .snapshot()
                                .ok()
                                .and_then(|settings| {
                                    settings
                                        .ai
                                        .mandatory_groups
                                        .get(index)
                                        .map(|group| group.name.clone())
                                })
                                .unwrap_or_default();
                            view.group_name_input
                                .update(cx, |input, cx| input.set_value(&name, window, cx));
                            view.editing_group = Some(index);
                            cx.notify();
                        })),
                );
            }

            for (tag_index, tag) in group_settings.tags.iter().enumerate() {
                group = group.child(
                    div()
                        .h_flex()
                        .gap_1()
                        .items_center()
                        .child(format!("🏷 {tag}"))
                        .child(
                            Button::new(format!("settings-group-tag-remove-{index}-{tag_index}"))
                                .label("×")
                                .on_click(cx.listener(move |view, _, _, cx| {
                                    view.edit(
                                        |settings| {
                                            if let Some(group) =
                                                settings.ai.mandatory_groups.get_mut(index)
                                            {
                                                if tag_index < group.tags.len() {
                                                    group.tags.remove(tag_index);
                                                }
                                            }
                                        },
                                        cx,
                                    )
                                })),
                        ),
                );
            }

            for (name, display) in available_tags
                .iter()
                .filter(|(name, _)| !group_settings.tags.contains(name))
            {
                let name = name.clone();
                group = group.child(
                    Button::new(format!("settings-group-tag-add-{index}-{name}"))
                        .label(format!("+ {display}"))
                        .on_click(cx.listener(move |view, _, _, cx| {
                            view.edit(
                                |settings| {
                                    if let Some(group) = settings.ai.mandatory_groups.get_mut(index)
                                    {
                                        if !group.tags.contains(&name) {
                                            group.tags.push(name.clone());
                                        }
                                    }
                                },
                                cx,
                            )
                        })),
                );
            }

            panel = panel.child(group).child(
                Button::new(format!("settings-group-remove-{index}"))
                    .label("Remove group")
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.edit(
                            |settings| {
                                if index < settings.ai.mandatory_groups.len() {
                                    settings.ai.mandatory_groups.remove(index);
                                }
                            },
                            cx,
                        )
                    })),
            );
        }
        panel.child(
            Button::new("settings-group-add")
                .label("+ Add group")
                .on_click(cx.listener(|view, _, _, cx| {
                    view.edit(
                        |settings| {
                            settings.ai.mandatory_groups.push(AiTagGroup {
                                name: format!("Group {}", settings.ai.mandatory_groups.len() + 1),
                                tags: vec![],
                            })
                        },
                        cx,
                    )
                })),
        )
    }
}
