//! Measured thumbnail slots and branch geometry shared by real and demo nodes.
use super::history::Version;
use super::*;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
use gpui_kit::prelude::FluentBuilder;

pub(super) fn thumbnail(path: Option<PathBuf>, width: f32, height: f32) -> Div {
    let mut slot = fileatlas_ui_bridge::components::card_surface(
        div()
            .w(px(width))
            .h(px(height))
            .flex_shrink_0()
            .overflow_hidden()
            .flex()
            .items_center()
            .justify_center(),
        false,
    );
    slot = if let Some(path) = path {
        slot.child(
            img(path)
                .w_full()
                .h_full()
                .object_fit(ObjectFit::Cover)
                .with_fallback(|| {
                    theme::icon(IconName::FileImage)
                        .size(px(24.))
                        .workbench_foreground(
                            fileatlas_ui_bridge::components::ForegroundRole::Muted,
                        )
                        .into_any_element()
                }),
        )
    } else {
        slot.child(
            theme::icon(IconName::FileImage)
                .size(px(24.))
                .workbench_foreground(fileatlas_ui_bridge::components::ForegroundRole::Muted),
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
    cx: &mut Context<FileManagerView>,
) -> Div {
    let line_color = if filter == 2 {
        theme::branch_alternate()
    } else {
        theme::accent()
    };
    let mut tree = div()
        .v_flex()
        .w_full()
        .min_w_0()
        .relative()
        .pb(px(fileatlas_ui_bridge::theme::space_expanded()))
        .child(
            fileatlas_ui_bridge::components::timeline_rail(line_color)
                .left(px(9.))
                .top(px(16.))
                .bottom(px(36.)),
        )
        .child(
            div()
                .h_flex()
                .pl(px(fileatlas_ui_bridge::theme::space_compact()))
                .pb(px(fileatlas_ui_bridge::theme::space_expanded()))
                .child(fileatlas_ui_bridge::components::colored_badge(
                    div()
                        .px(px(fileatlas_ui_bridge::theme::space_standard()))
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr(if filter == 2 {
                            "Alternate branch"
                        } else {
                            "Main branch"
                        })),
                    line_color,
                )),
        );
    for (index, version) in versions.iter().enumerate() {
        if (filter == 1 && version.branch) || (filter == 2 && !version.branch) {
            continue;
        }
        let color = if version.branch {
            theme::branch_alternate()
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
            .py(px(fileatlas_ui_bridge::theme::space_compact()));
        if nested {
            row = row.child(fileatlas_ui_bridge::components::timeline_connector(
                9., -33., 28., 70., color,
            ));
        }
        row = row
            .child(fileatlas_ui_bridge::components::timeline_marker(
                if nested { 31. } else { 3. },
                32.,
                14.,
                color,
                is_selected,
            ))
            .child(fileatlas_ui_bridge::components::accent_card(
                div()
                    .h_flex()
                    .w_full()
                    .min_w_0()
                    .items_center()
                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                    .p(px(fileatlas_ui_bridge::theme::space_standard()))
                    .child(thumbnail(version.thumbnail.clone(), 64., 48.))
                    .child(
                        div()
                            .v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                            .child(
                                div()
                                    .h_flex()
                                    .items_center()
                                    .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                                    .child(div().workbench_body().child(version.label.clone()))
                                    .when(index == active, |row| {
                                        row.child(div().workbench_assistant().child(
                                            fileatlas_ui_bridge::i18n::tr("Version baseline"),
                                        ))
                                    }),
                            )
                            .child(
                                div()
                                    .workbench_assistant()
                                    .child(card::format_timestamp(version.modified)),
                            ),
                    )
                    .child(
                        fileatlas_ui_bridge::components::button(format!(
                            "inspector-history-menu-{}",
                            version.version_id
                        ))
                        .ghost()
                        .icon(theme::icon(IconName::Ellipsis))
                        .disabled(!interactive || active == index)
                        .dropdown_menu(move |menu, _, _| {
                            let menu_view = menu_view.clone();
                            let revision = activation_revision.clone();
                            menu.item(
                                PopupMenuItem::new(fileatlas_ui_bridge::i18n::tr("Switch version"))
                                    .on_click(move |_, _, cx| {
                                        let _ = menu_view.update(cx, |view, cx| {
                                            view.activate_history_version(&revision, cx);
                                        });
                                    }),
                            )
                        }),
                    ),
                color,
                is_selected,
            ));
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
