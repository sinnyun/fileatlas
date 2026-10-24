use super::*;

pub fn render_row(setting: SettingRow, handler: SettingHandler) -> Div {
    let SettingRow {
        id,
        label: title,
        description,
        control,
        disabled,
        error,
    } = setting;
    let mut content = row();
    if matches!(&control, SettingControl::InlineToggle { .. }) {
        content = content.justify_end();
    }
    if matches!(&control, SettingControl::Swatches { .. }) {
        content = content.items_start().pt(px(crate::theme::space_compact()));
    }
    if !title.is_empty() {
        content = content.child(label(&title, &description));
    }
    let control = match control {
        SettingControl::InlineToggle { value, label } => div()
            .w(px(360.))
            .h_flex()
            .items_center()
            .child(
                Checkbox::new(id.clone())
                    .with_size(Size::Large)
                    .label(tr(&label))
                    .checked(value)
                    .disabled(disabled)
                    .on_click(move |value, window, cx| {
                        handler(&id, SettingValue::Bool(*value), window, cx)
                    }),
            )
            .into_any_element(),
        SettingControl::Toggle(value) => Checkbox::new(id.clone())
            .with_size(Size::Large)
            .checked(value)
            .accessibility_label(tr(&title))
            .disabled(disabled)
            .on_click(move |value, window, cx| handler(&id, SettingValue::Bool(*value), window, cx))
            .into_any_element(),
        SettingControl::Status { value, enabled: _ } => {
            div().workbench_body().child(tr(&value)).into_any_element()
        }
        SettingControl::ReadOnly(value) => div()
            .workbench_assistant()
            .child(tr(&value))
            .into_any_element(),
        SettingControl::Action { label, primary } => {
            components::action(id.clone(), tr(&label), primary)
                .disabled(disabled)
                .on_click(move |_, window, cx| handler(&id, SettingValue::Invoke, window, cx))
                .into_any_element()
        }
        SettingControl::Text(state) => components::text_input_with_disabled(&state, disabled)
            .max_w(px(theme::CONTROL_WIDTH_EXTRA_WIDE))
            .flex_shrink_0()
            .into_any_element(),
        SettingControl::Path {
            value,
            browse_label,
        } => div()
            .h_flex()
            .items_center()
            .gap(px(crate::theme::space_standard()))
            .max_w(px(360.))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .workbench_assistant()
                    .child(value),
            )
            .child(
                crate::components::button(id.clone())
                    .label(tr(&browse_label))
                    .disabled(disabled)
                    .on_click(move |_, window, cx| handler(&id, SettingValue::Invoke, window, cx)),
            )
            .into_any_element(),
        SettingControl::Select { value, options } => {
            let selected = options
                .iter()
                .find(|option| option.value == value)
                .map(|option| option.label.as_str())
                .unwrap_or(&value);
            crate::components::button(id.clone())
                .label(tr(selected))
                .disabled(disabled)
                .dropdown_caret(true)
                .dropdown_menu(move |mut menu, _, _| {
                    for option in &options {
                        let edit = handler.clone();
                        let id = id.clone();
                        let next = option.value.clone();
                        menu = menu.item(
                            PopupMenuItem::new(tr(&option.label))
                                .checked(value == option.value)
                                .on_click(move |_, window, cx| {
                                    edit(&id, SettingValue::Choice(next.clone()), window, cx)
                                }),
                        );
                    }
                    menu
                })
                .into_any_element()
        }
        SettingControl::Segments { value, options } => {
            let mut choices = div().h_flex().gap_0();
            for option in options {
                let chosen = option.value == value;
                let edit = handler.clone();
                let row_id = id.clone();
                choices = choices.child(
                    crate::components::button(format!("{id}-{}", option.value))
                        .outline()
                        .disabled(disabled)
                        .selected(chosen)
                        .label(tr(&option.label))
                        .on_click(move |_, window, cx| {
                            edit(
                                &row_id,
                                SettingValue::Choice(option.value.clone()),
                                window,
                                cx,
                            )
                        }),
                );
            }
            choices.into_any_element()
        }
        SettingControl::Radios { value, options } => {
            let mut choices = div().v_flex().w_full().gap(px(theme::space_standard()));
            for option in options {
                let edit = handler.clone();
                let row_id = id.clone();
                let chosen = value == option.value;
                let option_id = format!("{id}-{}", option.value);
                let choice = crate::components::choice_card(chosen, disabled)
                    .id(option_id)
                    .min_h(px(if option.description.is_empty() {
                        32.
                    } else {
                        48.
                    }))
                    .child(
                        theme::icon(if chosen {
                            IconName::CircleDot
                        } else {
                            IconName::Circle
                        })
                        .text_color(rgb(if chosen {
                            theme::accent()
                        } else {
                            theme::text_secondary()
                        })),
                    )
                    .child(label(&option.label, &option.description));
                let choice = if disabled {
                    choice
                } else {
                    choice.on_click(move |_, window, cx| {
                        edit(
                            &row_id,
                            SettingValue::Choice(option.value.clone()),
                            window,
                            cx,
                        )
                    })
                };
                choices = choices.child(choice);
            }
            choices.into_any_element()
        }
        SettingControl::Swatches { value, options } => {
            let mut choices = div()
                .h_flex()
                .flex_wrap()
                .w(px(360.))
                .gap(px(theme::space_standard()));
            for option in options {
                let edit = handler.clone();
                let row_id = id.clone();
                let chosen = value == option.value;
                let color = option.color.unwrap_or_else(theme::accent);
                let mut swatch = div()
                    .size(px(22.))
                    .rounded_full()
                    .flex_shrink_0()
                    .bg(rgb(color));
                if chosen {
                    swatch = swatch.h_flex().items_center().justify_center().child(
                        theme::small_icon(IconName::Check)
                            .text_color(rgb(theme::contrasting_text(color))),
                    );
                }
                choices = choices.child(
                    crate::components::button(format!("{id}-{}", option.value))
                        .ghost()
                        .min_w_0()
                        .justify_start()
                        .disabled(disabled)
                        .tooltip(tr(&option.label))
                        .child(
                            div()
                                .h_flex()
                                .items_center()
                                .gap(px(crate::theme::space_standard()))
                                .child(swatch)
                                .child(div().workbench_body().truncate().child(tr(&option.label))),
                        )
                        .on_click(move |_, window, cx| {
                            edit(
                                &row_id,
                                SettingValue::Choice(option.value.clone()),
                                window,
                                cx,
                            )
                        }),
                );
            }
            choices.into_any_element()
        }
        SettingControl::Custom(element) => element,
    };
    content = content.child(control);
    let mut result = div().v_flex().w_full().min_w_0().child(content);
    if let Some(error) = error {
        result = result.child(div().workbench_assistant().child(error));
    }
    components::review_row(
        result,
        crate::geometry_audit::ComponentTokens {
            height: Some(48.),
            gap: Some(crate::theme::space_expanded()),
            ..Default::default()
        },
    )
}
