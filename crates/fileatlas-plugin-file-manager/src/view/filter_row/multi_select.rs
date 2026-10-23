//! Multi-select tag and extension filters with live include/exclude mode.
use super::*;

pub(super) fn tags_chip(
    view: &FileManagerView,
    weak: WeakEntity<FileManagerView>,
    cx: &mut Context<FileManagerView>,
) -> AnyElement {
    let included = view.filter.selected_tag_ids.clone();
    let excluded = view.filter.excluded_tag_ids.clone();
    let active = !included.is_empty() || !excluded.is_empty();
    let exclude_mode = view.tag_filter_exclude_mode;
    let tags = flatten_tag_tree(
        view.snapshot
            .as_ref()
            .map(|snapshot| snapshot.tag_tree.as_slice())
            .unwrap_or_default(),
    );
    let clear = active.then(|| {
        clear_control(
            "file_manager-filter-tags-clear",
            cx.listener(|this, _, _, _| this.emit_tag_filter_selection(Vec::new(), Vec::new())),
        )
    });
    controlled_popover(
        view,
        weak.clone(),
        "file_manager-filter-tags-popover",
        "tags",
    )
    .trigger(chip(
        "file_manager-filter-tags",
        IconName::Tag,
        "Tags".into(),
        active,
        clear,
    ))
    .content(move |_, _, cx| {
        let weak = weak.clone();
        let (included, excluded, exclude_mode) = weak
            .upgrade()
            .map(|view| {
                let view = view.read(cx);
                (
                    view.filter.selected_tag_ids.clone(),
                    view.filter.excluded_tag_ids.clone(),
                    view.tag_filter_exclude_mode,
                )
            })
            .unwrap_or((included.clone(), excluded.clone(), exclude_mode));
        let mut panel = div()
            .v_flex()
            .w(px(250.))
            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
            .child(popover_title("Tags"))
            .child(segment_pair(
                "file_manager-tag-filter-include",
                "file_manager-tag-filter-exclude",
                "Include selected",
                "Exclude selected",
                exclude_mode,
                weak.clone(),
                true,
                cx,
            ));
        let mut list = div()
            .v_flex()
            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
            .max_h(px(240.))
            .overflow_y_scrollbar();
        if tags.is_empty() {
            list = list.child(empty_hint("No tags yet"));
        }
        for (tag, depth) in &tags {
            let tag_id = tag.id;
            let checked = if exclude_mode {
                excluded.contains(&tag_id)
            } else {
                included.contains(&tag_id)
            };
            let weak = weak.clone();
            list = list.child(
                menu_row(format!("file_manager-filter-tag-{tag_id}"))
                    .pl(px(fileatlas_ui_bridge::theme::space_compact()
                        + fileatlas_ui_bridge::theme::space_expanded()
                            * *depth as f32))
                    .child(fileatlas_ui_bridge::components::checkbox(
                        format!("file_manager-filter-tag-box-{tag_id}"),
                        checked,
                    ))
                    .child(
                        fileatlas_ui_bridge::tag_capsule::TagCapsule::new(tag.name.clone())
                            .color(tag.color.as_deref())
                            .icon(tag.icon.as_deref())
                            .selected(checked)
                            .render()
                            .flex_1()
                            .min_w_0(),
                    )
                    .on_click(cx.listener(move |_, _, _, cx| {
                        let _ = weak.update(cx, |this, cx| {
                            this.toggle_tag_filter(tag_id, this.tag_filter_exclude_mode, cx)
                        });
                        cx.notify();
                    })),
            );
        }
        panel = panel.child(list);
        panel
    })
    .into_any_element()
}

pub(super) fn extensions_chip(
    view: &FileManagerView,
    weak: WeakEntity<FileManagerView>,
    cx: &mut Context<FileManagerView>,
) -> AnyElement {
    let include_mode = !view.extension_exclude_mode;
    let active = !view.filter.extensions.is_empty() || !view.filter.excluded_extensions.is_empty();
    let included = view.filter.extensions.clone();
    let excluded = view.filter.excluded_extensions.clone();
    let extensions = view
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.available_extensions.clone())
        .unwrap_or_default();
    let clear = active.then(|| {
        clear_control(
            "file_manager-filter-extensions-clear",
            cx.listener(|this, _, _, cx| this.clear_extensions(cx)),
        )
    });
    controlled_popover(
        view,
        weak.clone(),
        "file_manager-filter-extensions-popover",
        "extensions",
    )
    .trigger(chip(
        "file_manager-filter-extensions",
        IconName::File,
        "Extensions".into(),
        active,
        clear,
    ))
    .content(move |_, _, cx| {
        let popover = cx.entity();
        let weak = weak.clone();
        let (included, excluded, include_mode) = weak
            .upgrade()
            .map(|view| {
                let view = view.read(cx);
                (
                    view.filter.extensions.clone(),
                    view.filter.excluded_extensions.clone(),
                    !view.extension_exclude_mode,
                )
            })
            .unwrap_or((included.clone(), excluded.clone(), include_mode));
        let mut panel = div()
            .v_flex()
            .w(px(280.))
            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
            .child(popover_title("Extensions"))
            .child(segment_pair(
                "file_manager-ext-filter-include",
                "file_manager-ext-filter-exclude",
                "Include selected",
                "Exclude selected",
                !include_mode,
                weak.clone(),
                false,
                cx,
            ));
        let mut list = div()
            .v_flex()
            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
            .max_h(px(240.))
            .overflow_y_scrollbar();
        if extensions.is_empty() {
            list = list.child(empty_hint("No extensions indexed yet"));
        }
        for extension in &extensions {
            let extension = extension.clone();
            let checked = if include_mode {
                included.contains(&extension)
            } else {
                excluded.contains(&extension)
            };
            let weak = weak.clone();
            list = list.child(
                menu_row(format!("file_manager-ext-filter-{extension}"))
                    .child(fileatlas_ui_bridge::components::checkbox(
                        format!("file_manager-ext-filter-box-{extension}"),
                        checked,
                    ))
                    .child(row_label(format!(
                        "{} (.{extension})",
                        extension.to_uppercase()
                    )))
                    .on_click(cx.listener(move |_, _, _, cx| {
                        let extension = extension.clone();
                        let _ = weak.update(cx, |this, cx| this.cycle_extension(extension, cx));
                        cx.notify();
                    })),
            );
        }
        panel = panel.child(list).child(
            div()
                .h_flex()
                .justify_end()
                .pt(px(fileatlas_ui_bridge::theme::space_compact()))
                .child(
                    fileatlas_ui_bridge::components::button("file_manager-ext-filter-clear-all")
                        .ghost()
                        .label(fileatlas_ui_bridge::i18n::tr("Clear all"))
                        .on_click(move |_, window, cx| {
                            let _ = weak.update(cx, |this, cx| this.clear_extensions(cx));
                            popover.update(cx, |state, cx| state.dismiss(window, cx));
                        }),
                ),
        );
        panel
    })
    .into_any_element()
}
