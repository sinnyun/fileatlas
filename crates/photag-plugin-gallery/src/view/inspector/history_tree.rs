//! Measured thumbnail slots and branch geometry shared by real and demo nodes.
use super::history::Version;
use super::*;
use gpui_kit::prelude::FluentBuilder;

pub(super) fn thumbnail(path: Option<PathBuf>, width: f32, height: f32) -> Div {
    let mut slot = div()
        .w(px(width))
        .h(px(height))
        .flex_shrink_0()
        .rounded(px(5.))
        .overflow_hidden()
        .border_1()
        .border_color(rgba(theme::SEPARATOR_LIGHT))
        .bg(rgb(theme::SURFACE_FAINT))
        .flex()
        .items_center()
        .justify_center();
    slot = if let Some(path) = path {
        slot.child(
            img(path)
                .w_full()
                .h_full()
                .object_fit(ObjectFit::Cover)
                .with_fallback(|| {
                    theme::icon(IconName::FileImage)
                        .size(px(24.))
                        .text_color(rgb(theme::TEXT_MUTED))
                        .into_any_element()
                }),
        )
    } else {
        slot.child(
            theme::icon(IconName::FileImage)
                .size(px(24.))
                .text_color(rgb(theme::TEXT_MUTED)),
        )
    };
    slot
}

pub(super) fn render(
    versions: &[Version],
    selected: usize,
    active: usize,
    filter: u8,
    interactive: bool,
    cx: &mut Context<GalleryView>,
) -> Div {
    let line_color = if filter == 2 {
        theme::BRANCH_ALTERNATE
    } else {
        theme::accent()
    };
    let mut tree = div()
        .v_flex()
        .w_full()
        .min_w_0()
        .relative()
        .pb(px(14.))
        .child(
            div()
                .absolute()
                .left(px(9.))
                .top(px(16.))
                .bottom(px(36.))
                .w(px(2.))
                .bg(rgb(line_color)),
        )
        .child(
            div().h_flex().pl(px(1.)).pb(px(12.)).child(
                div()
                    .px(px(9.))
                    .py(px(3.))
                    .rounded_full()
                    .border_1()
                    .border_color(rgba((line_color << 8) | 0x90))
                    .bg(rgb(theme::SURFACE_FAINT))
                    .text_size(px(12.))
                    .text_color(rgb(line_color))
                    .child(photag_ui_bridge::i18n::tr(if filter == 2 {
                        "Alternate branch"
                    } else {
                        "Main branch"
                    })),
            ),
        );
    for (index, version) in versions.iter().enumerate() {
        if (filter == 1 && version.branch) || (filter == 2 && !version.branch) {
            continue;
        }
        let color = if version.branch {
            theme::BRANCH_ALTERNATE
        } else {
            theme::accent()
        };
        let is_selected = selected == index;
        let nested = version.branch && filter != 2;
        let menu_view = cx.entity().downgrade();
        let activation_revision = version.version_id.clone();
        let selection_revision = version.version_id.clone();
        let mut row = div()
            .id(format!("inspector-history-version-{}", version.version_id))
            .relative()
            .h_flex()
            .w_full()
            .min_w_0()
            .min_h(px(76.))
            .pl(px(if nested { 48. } else { 28. }))
            .py(px(5.));
        if nested {
            row = row.child(
                div()
                    .absolute()
                    .left(px(9.))
                    .top(px(-33.))
                    .w(px(28.))
                    .h(px(70.))
                    .border_l_2()
                    .border_b_2()
                    .border_color(rgb(color))
                    .rounded_bl(px(25.)),
            );
        }
        row = row
            .child(
                div()
                    .absolute()
                    .left(px(if nested { 31. } else { 3. }))
                    .top(px(32.))
                    .size(px(14.))
                    .rounded_full()
                    .border_2()
                    .border_color(rgb(color))
                    .bg(rgb(if is_selected {
                        color
                    } else {
                        theme::SURFACE_CARD
                    })),
            )
            .child(
                div()
                    .h_flex()
                    .w_full()
                    .min_w_0()
                    .items_center()
                    .gap(px(10.))
                    .p(px(9.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(if is_selected { rgb(color) } else { rgba(0) })
                    .bg(rgba(if is_selected { (color << 8) | 0x18 } else { 0 }))
                    .child(thumbnail(version.thumbnail.clone(), 64., 48.))
                    .child(
                        div()
                            .v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap(px(6.))
                            .child(
                                div()
                                    .h_flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .text_size(px(14.))
                                            .font_family(theme::SEMIBOLD_FONT_FAMILY)
                                            .text_color(rgb(theme::TEXT_PRIMARY))
                                            .child(version.label.clone()),
                                    )
                                    .when(index == active, |row| {
                                        row.child(
                                            div().text_size(px(10.)).text_color(rgb(color)).child(
                                                photag_ui_bridge::i18n::tr("Version baseline"),
                                            ),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(theme::TEXT_SECONDARY))
                                    .child(card::format_timestamp(version.modified)),
                            ),
                    )
                    .child(
                        Button::new(format!("inspector-history-menu-{}", version.version_id))
                            .ghost()
                            .compact()
                            .icon(theme::icon(IconName::Ellipsis))
                            .disabled(!interactive || active == index)
                            .dropdown_menu(move |menu, _, _| {
                                let menu_view = menu_view.clone();
                                let revision = activation_revision.clone();
                                menu.item(
                                    PopupMenuItem::new(photag_ui_bridge::i18n::tr(
                                        "Switch version",
                                    ))
                                    .on_click(
                                        move |_, _, cx| {
                                            let _ = menu_view.update(cx, |view, cx| {
                                                view.activate_history_version(&revision, cx);
                                            });
                                        },
                                    ),
                                )
                            }),
                    ),
            );
        if interactive {
            row = row
                .cursor_pointer()
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.select_history_version(&selection_revision, cx)
                }));
        }
        tree = tree.child(row);
    }
    tree
}
