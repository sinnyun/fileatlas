use super::*;

pub(super) fn button(
    view: &mut PreviewView,
    settings: &ToolbarSettings,
    cx: &mut Context<PreviewView>,
) -> AnyElement {
    let weak = cx.entity().downgrade();
    let mut menu = div()
        .v_flex()
        .w(px(224.))
        .gap_1()
        .p_2()
        .rounded(px(4.))
        .bg(rgb(theme::SURFACE_MODAL))
        .border_1()
        .border_color(rgba(theme::SEPARATOR_LIGHT));
    for (enabled, id, label, icon) in [
        (
            settings.open_external_button,
            "open",
            "Open in default app",
            IconName::ExternalLink,
        ),
        (
            settings.explorer_button,
            "reveal",
            "Show in Explorer",
            IconName::FolderOpen,
        ),
        (settings.copy_button, "copy", "Copy to…", IconName::Copy),
        (
            settings.move_button,
            "move",
            "Move to…",
            IconName::ArrowRightLeft,
        ),
        (
            settings.delete_button,
            "delete",
            "Delete (Del/Backspace)",
            IconName::Trash,
        ),
    ] {
        if !enabled {
            continue;
        }
        menu = menu.child(
            Button::new(format!("preview-more-{id}"))
                .ghost()
                .compact()
                .w_full()
                .child(
                    div()
                        .w_full()
                        .h_flex()
                        .gap_2()
                        .child(gpui_kit::component::Icon::new(icon))
                        .child(tr(label)),
                )
                .text_color(rgb(if id == "delete" {
                    theme::DANGER
                } else {
                    theme::TEXT_PRIMARY
                }))
                .on_click(cx.listener(move |view, _, window, cx| {
                    view.more_actions_open = false;
                    let _ = view.surfaces.set_open("preview.more", false);
                    match id {
                        "open" => view.open_in_default_app(cx),
                        "reveal" => view.reveal_in_file_manager(cx),
                        "copy" => view.copy_current_to_file(cx),
                        "move" => view.move_current_to_file(cx),
                        "delete" => view.request_delete(window, cx),
                        _ => unreachable!(),
                    }
                    cx.notify();
                })),
        );
    }
    Popover::new("preview-more-popup")
        .anchor(Anchor::BottomRight)
        .arrow(false)
        .open(view.more_actions_open)
        .on_open_change(move |open: &bool, _, app| {
            let _ = weak.update(app, |view, cx| {
                view.more_actions_open = *open;
                let _ = view.surfaces.set_open("preview.more", *open);
                cx.notify();
            });
        })
        .trigger(tool_button("more", tr("More")).icon(IconName::Ellipsis))
        .child(menu)
        .into_any_element()
}
