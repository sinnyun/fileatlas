use super::SettingsView;
use crate::{AiTagGroup, Settings};
use fileatlas_ui_bridge::components::WorkbenchStyle as _;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
use fileatlas_ui_bridge::{i18n::tr, theme};
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::*;

impl SettingsView {
    pub(super) fn ai_groups(&self, settings: &Settings, cx: &mut Context<Self>) -> Div {
        let mut panel = div()
            .v_flex()
            .gap(px(fileatlas_ui_bridge::theme::space_standard()));
        let available = self
            .tag_choices
            .value()
            .and_then(|v| v.as_ref().ok())
            .cloned()
            .unwrap_or_default();
        if let Some(Err(error)) = self.tag_choices.value() {
            panel = panel.child(div().workbench_assistant().child(error.clone()));
        }
        if self.tag_choices.value().is_none() {
            panel = panel.child(div().workbench_assistant().child(tr("Loading file info…")));
        } else if available.is_empty() {
            panel=panel.child(div().workbench_assistant().child(tr("No tags are available for adding to groups while Navigation is stopped or no tags exist.")));
        }
        for (index, stored) in settings.ai.mandatory_groups.iter().enumerate() {
            let mut group = div()
                .v_flex()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                .py(px(theme::space_expanded()))
                .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Bottom);
            let mut header = div()
                .h_flex()
                .w_full()
                .items_center()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()));
            if self.editing_group == Some(index) {
                header = header
                    .child(
                        fileatlas_ui_bridge::components::text_input(&self.group_name_input)
                            .flex_1(),
                    )
                    .child(
                        fileatlas_ui_bridge::components::button(format!(
                            "settings-group-name-save-{index}"
                        ))
                        .primary()
                        .label(tr("Save name"))
                        .on_click(cx.listener(move |view, _, _, cx| {
                            let name = view.group_name_input.read(cx).value().trim().to_owned();
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
                                view.editing_group = None;
                            }
                            cx.notify();
                        })),
                    );
            } else {
                header = header
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .workbench_body()
                            .child(stored.name.clone()),
                    )
                    .child(
                        fileatlas_ui_bridge::components::button(format!(
                            "settings-group-name-edit-{index}"
                        ))
                        .ghost()
                        .label(tr("Rename"))
                        .on_click(cx.listener(
                            move |view, _, window, cx| {
                                let name = view.service.snapshot().ok().and_then(|s| {
                                    s.ai.mandatory_groups.get(index).map(|g| g.name.clone())
                                });
                                if let Some(name) = name {
                                    view.group_name_input
                                        .update(cx, |input, cx| input.set_value(&name, window, cx));
                                    view.editing_group = Some(index);
                                    cx.notify();
                                }
                            },
                        )),
                    );
            }
            group = group.child(header.child(self.remove_group_button(index, cx)));
            if !stored.tags.is_empty() {
                group = group.child(div().workbench_body().child(tr("Selected tags")));
            }
            let mut tags = div()
                .h_flex()
                .flex_wrap()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()));
            for (tag_index, tag) in stored.tags.iter().enumerate() {
                let chip = available
                    .iter()
                    .find(|choice| choice.name == *tag)
                    .map(|choice| {
                        fileatlas_ui_bridge::tag_capsule::TagCapsule::new(choice.name.clone())
                            .color(choice.color.as_deref())
                            .icon(choice.icon.as_deref())
                            .render()
                    })
                    .unwrap_or_else(|| fileatlas_ui_bridge::tag_capsule::name_only(tag.clone()));
                tags = tags.child(
                    div()
                        .h_flex()
                        .items_center()
                        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                        .child(chip)
                        .child(
                            fileatlas_ui_bridge::components::button(format!(
                                "settings-group-tag-remove-{index}-{tag_index}"
                            ))
                            .ghost()
                            .icon(theme::small_icon(gpui_kit::assets::IconName::X))
                            .tooltip(tr("Remove tag"))
                            .on_click(cx.listener(
                                move |view, _, _, cx| {
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
                                    );
                                },
                            )),
                        ),
                );
            }
            group = group.child(tags);
            let candidates = available
                .iter()
                .filter(|choice| !stored.tags.contains(&choice.name))
                .cloned()
                .collect::<Vec<_>>();
            if !candidates.is_empty() {
                let weak = cx.entity().downgrade();
                group = group.child(
                    div().h_flex().child(
                        fileatlas_ui_bridge::components::button(format!(
                            "settings-group-tag-add-{index}"
                        ))
                        .outline()
                        .justify_start()
                        .label(tr("Add tag…"))
                        .dropdown_caret(true)
                        .dropdown_menu(move |mut menu, _, _| {
                            for choice in &candidates {
                                let name = choice.name.clone();
                                let weak = weak.clone();
                                menu =
                                    menu.item(PopupMenuItem::new(choice.label.clone()).on_click(
                                        move |_, _, cx| {
                                            let _ = weak.update(cx, |view, cx| {
                                                view.edit(
                                                    |settings| {
                                                        if let Some(group) = settings
                                                            .ai
                                                            .mandatory_groups
                                                            .get_mut(index)
                                                        {
                                                            if !group.tags.contains(&name) {
                                                                group.tags.push(name.clone());
                                                            }
                                                        }
                                                    },
                                                    cx,
                                                )
                                            });
                                        },
                                    ));
                            }
                            menu
                        }),
                    ),
                );
            } else if available.is_empty() {
                group = group.child(
                    div()
                        .workbench_assistant()
                        .child(tr("No tags are available to add.")),
                );
            }
            panel = panel.child(group);
        }
        panel
    }
    fn remove_group_button(&self, index: usize, cx: &mut Context<Self>) -> Button {
        fileatlas_ui_bridge::components::button(format!("settings-group-remove-{index}"))
            .ghost()
            .danger()
            .label(tr("Delete"))
            .on_click(cx.listener(move |view, _, _, cx| {
                view.edit(
                    |settings| {
                        if index < settings.ai.mandatory_groups.len() {
                            settings.ai.mandatory_groups.remove(index);
                        }
                    },
                    cx,
                );
                view.editing_group = None;
            }))
    }
    pub(super) fn add_group_button(&self, cx: &mut Context<Self>) -> Button {
        fileatlas_ui_bridge::components::button("settings-group-add")
            .outline()
            .label(tr("+ Add group"))
            .on_click(cx.listener(|view, _, _, cx| {
                view.edit(
                    |settings| {
                        settings.ai.mandatory_groups.push(AiTagGroup {
                            name: format!("Group {}", settings.ai.mandatory_groups.len() + 1),
                            tags: vec![],
                        });
                    },
                    cx,
                )
            }))
    }
}
