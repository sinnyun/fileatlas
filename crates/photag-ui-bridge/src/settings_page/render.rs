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
        content = content.items_start().pt_1();
    }
    if !title.is_empty() {
        let mut field_label = label(&title, &description);
        if matches!(&control, SettingControl::Status { .. }) {
            field_label = field_label
                .font_semibold()
                .text_size(px(theme::SETTINGS_SECTION_SIZE));
        }
        content = content.child(field_label);
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
        SettingControl::Status { value, enabled } => div()
            .text_size(px(theme::FONT_SIZE_BODY))
            .text_color(rgb(if enabled {
                theme::SUCCESS
            } else {
                theme::TEXT_MUTED
            }))
            .child(tr(&value))
            .into_any_element(),
        SettingControl::ReadOnly(value) => div()
            .text_size(px(theme::FONT_SIZE_SMALL))
            .text_color(rgb(theme::TEXT_SECONDARY))
            .child(tr(&value))
            .into_any_element(),
        SettingControl::Action { label, primary } => {
            components::action(id.clone(), tr(&label), primary)
                .disabled(disabled)
                .on_click(move |_, window, cx| handler(&id, SettingValue::Invoke, window, cx))
                .into_any_element()
        }
        SettingControl::Text(state) => components::text_input(&state)
            .w(px(240.))
            .disabled(disabled)
            .into_any_element(),
        SettingControl::Path {
            value,
            browse_label,
        } => div()
            .h_flex()
            .items_center()
            .gap_2()
            .max_w(px(360.))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(theme::FONT_SIZE_SMALL))
                    .child(value),
            )
            .child(
                Button::new(id.clone())
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
            Button::new(id.clone())
                .label(tr(selected))
                .w(px(200.))
                .h(px(theme::SETTINGS_CONTROL_HEIGHT))
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
                    Button::new(format!("{id}-{}", option.value))
                        .outline()
                        .h(px(theme::SETTINGS_CONTROL_HEIGHT))
                        .w(px(96.))
                        .disabled(disabled)
                        .border_color(rgb(if chosen {
                            theme::accent()
                        } else {
                            theme::SEPARATOR
                        }))
                        .bg(rgb(if chosen {
                            theme::selection_color()
                        } else {
                            theme::SURFACE_FAINT
                        }))
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
            let mut choices = div().v_flex().w_full().gap(px(theme::SPACE_2));
            for option in options {
                let edit = handler.clone();
                let row_id = id.clone();
                let chosen = value == option.value;
                let option_id = format!("{id}-{}", option.value);
                choices = choices.child(
                    Button::new(option_id)
                        .ghost()
                        .disabled(disabled)
                        .w_full()
                        .min_h(px(if option.description.is_empty() {
                            32.
                        } else {
                            48.
                        }))
                        .justify_start()
                        .icon(
                            theme::icon(if chosen {
                                IconName::CircleDot
                            } else {
                                IconName::Circle
                            })
                            .text_color(rgb(if chosen {
                                theme::accent()
                            } else {
                                theme::TEXT_SECONDARY
                            })),
                        )
                        .child(label(&option.label, &option.description))
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
        SettingControl::Swatches { value, options } => {
            let mut choices = div()
                .h_flex()
                .flex_wrap()
                .w(px(360.))
                .gap(px(theme::SPACE_2));
            for option in options {
                let edit = handler.clone();
                let row_id = id.clone();
                let chosen = value == option.value;
                let mut swatch = div()
                    .size(px(22.))
                    .rounded_full()
                    .flex_shrink_0()
                    .bg(rgb(option.color.unwrap_or_else(theme::accent)));
                if chosen {
                    swatch = swatch.h_flex().items_center().justify_center().child(
                        theme::small_icon(IconName::Check).text_color(rgb(theme::TEXT_ON_ACCENT)),
                    );
                }
                choices = choices.child(
                    Button::new(format!("{id}-{}", option.value))
                        .ghost()
                        .p_0()
                        .w(px(84.))
                        .min_w_0()
                        .h(px(36.))
                        .justify_start()
                        .disabled(disabled)
                        .tooltip(tr(&option.label))
                        .child(
                            div().h_flex().items_center().gap_2().child(swatch).child(
                                div()
                                    .text_size(px(theme::FONT_SIZE_SMALL))
                                    .truncate()
                                    .child(tr(&option.label)),
                            ),
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
        result = result.child(
            div()
                .text_size(px(theme::FONT_SIZE_SMALL))
                .text_color(rgb(theme::DANGER))
                .child(error),
        );
    }
    result
}
