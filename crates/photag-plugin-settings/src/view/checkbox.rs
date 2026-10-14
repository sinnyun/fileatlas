//! GPUI Kit owns focus, keyboard activation and accessibility. Source paints:
//! theme.rs:447-478 and pinned egui 0.29.1 Checkbox/Spacing defaults.
use super::*;
use gpui_kit::prelude::FluentBuilder as _;

impl SettingsView {
    pub(super) fn toggle_button(
        &self,
        key: &'static str,
        label: &str,
        value: bool,
        cx: &mut Context<Self>,
    ) -> gpui_kit::base::Checkbox {
        let id = format!("settings-{key}");
        let group = format!("{id}-group");
        let weak = cx.entity().downgrade();
        let label = photag_ui_bridge::i18n::tr(label);
        gpui_kit::base::Checkbox::new(id)
            .checked(value)
            .accessibility_label(label.clone())
            .group(group.clone())
            .h_flex()
            .items_center()
            .gap(px(4.))
            .min_h(px(18.))
            .text_size(px(13.5))
            .line_height(relative(1.))
            .child(
                div()
                    .id(format!("settings-{key}-indicator"))
                    .relative()
                    .size(px(14.))
                    .flex_shrink_0()
                    .rounded(px(2.))
                    .border_1()
                    .border_color(rgb(if value {
                        theme::accent()
                    } else {
                        theme::TEXT_SECONDARY
                    }))
                    .bg(rgb(if value {
                        theme::accent()
                    } else {
                        theme::SURFACE_MODAL
                    }))
                    .group_hover(group.clone(), |style| {
                        style.border_color(rgb(theme::TEXT_PRIMARY))
                    })
                    .group_active(group, |style| style.bg(rgb(theme::SURFACE_ELEVATED)))
                    .when(value, |indicator| {
                        indicator.child(
                            theme::small_icon(gpui_kit::assets::IconName::Check)
                                .text_color(rgb(theme::TEXT_PRIMARY)),
                        )
                    }),
            )
            .child(label)
            .on_change(move |_, _, _, cx| {
                let _ = weak.update(cx, |this, cx| this.toggle(key, cx));
            })
    }
}
