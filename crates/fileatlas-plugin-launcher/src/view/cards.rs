use super::*;
use fileatlas_ui_bridge::components::WorkbenchForeground as _;
use fileatlas_ui_bridge::components::WorkbenchPaint as _;
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
            .workbench_corner(fileatlas_ui_bridge::components::CornerRole::ExtraLarge)
            .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Transparent(
                (theme::accent() << 8) | 0x18,
            ))
            .workbench_foreground(fileatlas_ui_bridge::components::ForegroundRole::AccentText)
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
            .h(px(theme::PROGRAM_CARD_HEIGHT))
            .flex_shrink_0()
            .items_center()
            .justify_start()
            .pt(px(fileatlas_ui_bridge::theme::space_standard()))
            .px(px(fileatlas_ui_bridge::theme::space_compact()))
            .gap(px(fileatlas_ui_bridge::theme::space_standard()))
            .workbench_card(selected)
            .workbench_foreground(if entry.available {
                fileatlas_ui_bridge::components::ForegroundRole::Primary
            } else {
                fileatlas_ui_bridge::components::ForegroundRole::Muted
            })
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
                    .h(px(theme::PROGRAM_CARD_NAME_HEIGHT))
                    .overflow_hidden()
                    .text_center()
                    .workbench_body()
                    .child(entry.display_name.clone()),
            )
            .when(entry.pin_scope != crate::PinScope::None, |tile| {
                tile.child(
                    div()
                        .absolute()
                        .top(px(5.))
                        .right(px(7.))
                        .workbench_foreground(fileatlas_ui_bridge::components::ForegroundRole::Star)
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
