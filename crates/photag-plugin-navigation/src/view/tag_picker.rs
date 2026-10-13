use super::*;
use gpui_kit::component::scroll::ScrollableElement;

pub(super) fn parent_picker(view: &SidebarView, cx: &mut Context<SidebarView>) -> Div {
    let query = view.tag_parent.read(cx).value().trim().to_owned();
    let tags = view
        .snapshot
        .as_ref()
        .ok()
        .map(|s| s.tags.clone())
        .unwrap_or_default();
    let selected = view
        .parent_tag_id
        .and_then(|id| find_tag(&tags, id).map(|tag| tag.full_path.clone()));
    let mut matching = Vec::new();
    collect_matches(&tags, &query, &HashSet::new(), &mut matching);
    let rows = matching
        .into_iter()
        .map(|(id, path, depth)| {
            Button::new(format!("navigation-parent-tag-{id}"))
                .ghost()
                .w_full()
                .label(photag_ui_bridge::i18n::tr(&format!(
                    "{}{}",
                    "  ".repeat(depth),
                    path.replace('>', " › ")
                )))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.parent_tag_id = Some(id);
                    this.tag_parent
                        .update(cx, |input, cx| input.set_value("", window, cx));
                    cx.notify();
                }))
        })
        .collect::<Vec<_>>();
    div()
        .v_flex()
        .gap_1()
        .child(
            div()
                .text_xs()
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child(
                    selected
                        .map(|path| format!("{} {path}", photag_ui_bridge::i18n::tr("Parent")))
                        .unwrap_or_else(|| photag_ui_bridge::i18n::tr("Root tag")),
                ),
        )
        .child(Input::new(&view.tag_parent))
        .child(
            Button::new("navigation-parent-root")
                .ghost()
                .w_full()
                .label(photag_ui_bridge::i18n::tr("No parent (root)"))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.parent_tag_id = None;
                    this.tag_parent
                        .update(cx, |input, cx| input.set_value("", window, cx));
                    cx.notify();
                })),
        )
        .child(
            div()
                .v_flex()
                .max_h(px(190.))
                .overflow_y_scrollbar()
                .children(rows),
        )
}

pub(super) fn move_target_picker(view: &SidebarView, cx: &mut Context<SidebarView>) -> Div {
    let query = view.tag_move_target.read(cx).value().trim().to_owned();
    let tags = view
        .snapshot
        .as_ref()
        .ok()
        .map(|s| s.tags.clone())
        .unwrap_or_default();
    let source = view.move_source_id;
    let mut excluded = HashSet::new();
    if let Some(source) = source.and_then(|id| find_tag(&tags, id)) {
        collect_tag_ids(source, &mut excluded);
    }
    let selected = view
        .move_target_tag_id
        .and_then(|id| find_tag(&tags, id).map(|tag| tag.full_path.clone()));
    let mut matching = Vec::new();
    collect_matches(&tags, &query, &excluded, &mut matching);
    let rows = matching
        .into_iter()
        .map(|(id, path, depth)| {
            Button::new(format!("navigation-move-target-{id}"))
                .ghost()
                .w_full()
                .label(photag_ui_bridge::i18n::tr(&format!(
                    "{}{}",
                    "  ".repeat(depth),
                    path.replace('>', " › ")
                )))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.move_target_tag_id = Some(id);
                    this.tag_move_target
                        .update(cx, |input, cx| input.set_value("", window, cx));
                    cx.notify();
                }))
        })
        .collect::<Vec<_>>();
    div()
        .v_flex()
        .gap_1()
        .child(
            div()
                .text_xs()
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child(
                    selected
                        .map(|path| format!("{} {path}", photag_ui_bridge::i18n::tr("Target")))
                        .unwrap_or_else(|| photag_ui_bridge::i18n::tr("Choose a target tag")),
                ),
        )
        .child(Input::new(&view.tag_move_target))
        .child(
            div()
                .v_flex()
                .max_h(px(260.))
                .overflow_y_scrollbar()
                .children(rows),
        )
}

fn collect_matches(
    tags: &[photag_plugin_database::db::queries::Tag],
    query: &str,
    excluded: &HashSet<i64>,
    rows: &mut Vec<(i64, String, usize)>,
) {
    for tag in tags {
        if !excluded.contains(&tag.id) && tag_matches(tag, query, TagSearchMatch::Contains) {
            rows.push((
                tag.id,
                tag.full_path.clone(),
                tag.full_path.matches('>').count(),
            ));
        }
        collect_matches(&tag.children, query, excluded, rows);
    }
}
