use super::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable;

fn action_button(
    id: &'static str,
    icon: IconName,
    label: &'static str,
    enabled: bool,
    action: R,
    cx: &mut Context<SidebarView>,
) -> Button {
    Button::new(id)
        .ghost()
        .compact()
        .w(px(28.))
        .h(px(28.))
        .icon(theme::small_icon(icon))
        .tooltip(photag_ui_bridge::i18n::tr(label))
        .accessibility_label(photag_ui_bridge::i18n::tr(label))
        .disabled(!enabled)
        .on_click(cx.listener(move |view, _, window, cx| {
            view.apply_address_request(action.clone(), window, cx)
        }))
}

impl SidebarView {
    pub(in crate::view) fn render_breadcrumb(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let path = self.address_path();
        let mut address = div()
            .h_flex()
            .flex_1()
            .min_w_0()
            .h(px(30.))
            .items_center()
            .gap(px(3.))
            .rounded(px(5.))
            .border_1()
            .border_color(rgb(if self.address.error.is_some() {
                theme::DANGER
            } else {
                theme::CONTROL_BOX
            }))
            .px(px(5.));
        if self.address.editing {
            address = address
                .child(
                    photag_ui_bridge::components::text_input(&self.address.input)
                        .flex_1()
                        .min_w_0(),
                )
                .child(action_button(
                    "navigation-address-submit",
                    IconName::ArrowRight,
                    "Go to folder",
                    true,
                    R::Submit,
                    cx,
                ))
                .child(action_button(
                    "navigation-address-cancel",
                    IconName::X,
                    "Cancel",
                    true,
                    R::Cancel,
                    cx,
                ));
        } else {
            let mut trail = div()
                .id("navigation-address-trail")
                .h_flex()
                .flex_shrink_1()
                .min_w_0()
                .items_center()
                .overflow_x_scroll();
            if let Some(path) = &path {
                trail = trail.child(theme::small_icon(IconName::FolderOpen));
                for (i, crumb) in crumbs(path).into_iter().enumerate() {
                    if i > 0 {
                        trail = trail.child(
                            theme::small_icon(IconName::ChevronRight)
                                .text_color(rgb(theme::TEXT_MUTED)),
                        );
                    }
                    let target = crumb.path.clone();
                    trail = trail.child(
                        Button::new(format!("navigation-address-crumb-{i}"))
                            .ghost()
                            .compact()
                            .flex_shrink_0()
                            .label(crumb.label)
                            .tooltip(crumb.path)
                            .on_click(cx.listener(move |view, _, window, cx| {
                                view.apply_address_request(
                                    R::Crumb {
                                        path: target.clone(),
                                    },
                                    window,
                                    cx,
                                )
                            })),
                    );
                }
            } else {
                match self.address.history.current() {
                    Location::Tags { included, excluded } => {
                        trail = trail.child(
                            div()
                                .text_sm()
                                .text_color(rgb(theme::TEXT_SECONDARY))
                                .child(photag_ui_bridge::i18n::tr("Tags")),
                        );
                        if let Some(snapshot) = self.snapshot.as_ref().ok() {
                            for (id, selected) in included
                                .iter()
                                .map(|id| (*id, true))
                                .chain(excluded.iter().map(|id| (*id, false)))
                            {
                                if let Some(tag) = find_tag(&snapshot.tags, id) {
                                    trail = trail.child(
                                        photag_ui_bridge::tag_capsule::TagCapsule::new(
                                            tag.name.clone(),
                                        )
                                        .color(tag.color.as_deref())
                                        .icon(tag.icon.as_deref())
                                        .selected(selected)
                                        .render(),
                                    );
                                }
                            }
                        }
                    }
                    _ => {
                        trail = trail.child(
                            div()
                                .text_sm()
                                .text_color(rgb(theme::TEXT_SECONDARY))
                                .child(photag_ui_bridge::i18n::tr("Favorites")),
                        );
                    }
                }
            }
            address = address
                .child(trail)
                .child(
                    div()
                        .id("navigation-address-edit-space")
                        .flex_1()
                        .min_w(px(12.))
                        .h_full()
                        .on_click(cx.listener(|view, _, window, cx| {
                            view.apply_address_request(R::Edit, window, cx)
                        })),
                )
                .child(
                    Button::new("navigation-address-edit")
                        .ghost()
                        .compact()
                        .label("…")
                        .tooltip(photag_ui_bridge::i18n::tr("Edit folder address"))
                        .on_click(cx.listener(|view, _, window, cx| {
                            view.apply_address_request(R::Edit, window, cx)
                        })),
                );
        }
        if let Some(error) = &self.address.error {
            address = address.child(
                Button::new("navigation-address-error")
                    .ghost()
                    .compact()
                    .label("!")
                    .text_color(rgb(theme::DANGER))
                    .tooltip(error.clone()),
            );
        }
        div()
            .h_flex()
            .w_full()
            .min_w_0()
            .items_center()
            .gap(px(3.))
            .track_focus(&self.address.focus)
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                let action = if event.keystroke.key == "escape" && view.address.editing {
                    Some(R::Cancel)
                } else if event.keystroke.modifiers.control
                    && event.keystroke.key.eq_ignore_ascii_case("l")
                {
                    Some(R::Edit)
                } else if event.keystroke.modifiers.alt {
                    match event.keystroke.key.as_str() {
                        "left" => Some(R::Back),
                        "right" => Some(R::Forward),
                        "up" => Some(R::Up),
                        _ => None,
                    }
                } else {
                    None
                };
                if let Some(action) = action {
                    view.apply_address_request(action, window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(action_button(
                "navigation-back",
                IconName::ArrowLeft,
                "Back",
                self.address.history.target(false).is_some(),
                R::Back,
                cx,
            ))
            .child(action_button(
                "navigation-forward",
                IconName::ArrowRight,
                "Forward",
                self.address.history.target(true).is_some(),
                R::Forward,
                cx,
            ))
            .child(action_button(
                "navigation-up",
                IconName::ArrowUp,
                "Up one folder",
                path.as_ref().and_then(|p| parent(p)).is_some(),
                R::Up,
                cx,
            ))
            .child(address)
    }
}
