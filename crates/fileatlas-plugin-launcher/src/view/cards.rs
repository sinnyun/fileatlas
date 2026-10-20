use super::*;
use fileatlas_ui_bridge::components::WorkbenchStyle as _;
use gpui_kit::component::menu::ContextMenuExt;
use gpui_kit::prelude::FluentBuilder as _;

pub(super) fn category_name(name: &str) -> String {
    if name == "General" {
        tr("Favorite programs")
    } else {
        name.into()
    }
}

impl LauncherPanel {
    pub(super) fn icon(&self, entry: &LauncherEntry, size: f32) -> AnyElement {
        if let Some(image) = self
            .icons
            .value()
            .and_then(|r| r.as_ref().ok())
            .and_then(|icons| icons.images.get(&entry.launch_path))
            .and_then(|image| image.as_ref())
        {
            return img(image.clone())
                .size(px(size))
                .object_fit(ObjectFit::Contain)
                .into_any_element();
        }
        let category = self
            .snapshot
            .categories
            .iter()
            .find(|c| c.id == entry.category_id)
            .map(|c| c.name.as_str())
            .unwrap_or("General");
        div()
            .flex()
            .size(px(size))
            .items_center()
            .justify_center()
            .rounded(px(12.))
            .bg(rgba((theme::accent() << 8) | 0x18))
            .text_color(rgb(theme::accent_text()))
            .text_size(px(size * 0.6))
            .child(theme::icon(category_icon(category)).size(px(size * 0.5)))
            .into_any_element()
    }

    pub(super) fn program_card(&self, entry: LauncherEntry, cx: &mut Context<Self>) -> AnyElement {
        let id = entry.id;
        let selected = self.selected_entry == Some(id);
        let weak = cx.entity().downgrade();
        let destinations = self.snapshot.categories.clone();
        div()
            .id(format!("launcher-tile-{id}"))
            .relative()
            .v_flex()
            .w(px(112.))
            .h(px(132.))
            .flex_shrink_0()
            .items_center()
            .justify_start()
            .pt(px(10.))
            .px(px(5.))
            .gap(px(10.))
            .workbench_card(selected)
            .text_color(rgb(if entry.available {
                theme::text_primary()
            } else {
                theme::text_muted()
            }))
            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                this.select_entry(id, window, cx);
                if event.click_count() == 2 {
                    let result = this.service.launch(id);
                    this.report(result, cx);
                }
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, _, window, cx| this.select_entry(id, window, cx)),
            )
            .child(self.icon(&entry, 64.))
            .child(
                div()
                    .w_full()
                    .h(px(36.))
                    .overflow_hidden()
                    .text_center()
                    .text_size(px(12.))
                    .child(entry.display_name.clone()),
            )
            .when(entry.pin_scope != crate::PinScope::None, |tile| {
                tile.child(
                    div()
                        .absolute()
                        .top(px(5.))
                        .right(px(7.))
                        .text_color(rgb(theme::star_text()))
                        .child(theme::small_icon(
                            if entry.pin_scope == crate::PinScope::AllPrograms {
                                IconName::StarFill
                            } else {
                                IconName::Star
                            },
                        )),
                )
            })
            .context_menu(move |menu, window, cx| {
                super::menu::program_menu(
                    menu,
                    entry.clone(),
                    destinations.clone(),
                    weak.clone(),
                    window,
                    cx,
                )
            })
            .into_any_element()
    }
}
