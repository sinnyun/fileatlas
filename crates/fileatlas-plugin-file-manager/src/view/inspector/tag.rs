//! Inline editing uses the existing Navigation commands and real tag fields.
use super::*;
use fileatlas_plugin_database::Tag;
use fileatlas_plugin_navigation::{NavigationService, NavigationUiRequest};

pub(super) fn render(
    tag: &Tag,
    view: &mut FileManagerView,
    window: &mut Window,
    cx: &mut Context<FileManagerView>,
) -> AnyElement {
    let id = tag.id;
    let stored = view.inspector_tag_loaded.as_ref();
    if stored.is_none_or(|(loaded_id, name)| *loaded_id != id || name != &tag.name) {
        let input = view.inspector_tag_name.read(cx).value().to_string();
        let replace = stored.is_none_or(|(loaded_id, name)| *loaded_id != id || input == *name);
        if replace {
            view.inspector_tag_name
                .update(cx, |state, cx| state.set_value(&tag.name, window, cx));
        }
        view.inspector_tag_loaded = Some((id, tag.name.clone()));
    }
    let parent = tag
        .full_path
        .rsplit_once('>')
        .map(|(p, _)| p)
        .unwrap_or("—");
    let mut body = div()
        .v_flex()
        .w_full()
        .min_w_0()
        .px(px(16.))
        .pb(px(20.))
        .child(super::tag_editor::render(tag, view, cx))
        .child(inspector_divider())
        .child(section_title("Tag details"))
        .child(detail_row(
            IconName::Folder,
            "Parent tag",
            parent.to_string(),
        ))
        .child(detail_row(
            IconName::File,
            "Media count",
            tag.media_count.to_string(),
        ))
        .child(detail_row(
            IconName::Tag,
            "Child tags",
            tag.children.len().to_string(),
        ))
        .child(inspector_divider())
        .child(
            section_title("Child tags").child(
                Button::new("inspector-tag-add-child")
                    .ghost()
                    .compact()
                    .icon(theme::icon(IconName::Plus))
                    .tooltip(fileatlas_ui_bridge::i18n::tr("Create tag"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        request(this, NavigationUiRequest::OpenCreate, cx)
                    })),
            ),
        );
    if tag.children.is_empty() {
        body = body.child(inspector_message("No child tags."));
    } else {
        for child in &tag.children {
            let child_id = child.id;
            let child_menu = cx.entity().downgrade();
            body = body.child(
                div()
                    .h_flex()
                    .w_full()
                    .min_w_0()
                    .items_center()
                    .gap(px(9.))
                    .min_h(px(38.))
                    .px(px(9.))
                    .my(px(3.))
                    .rounded(px(7.))
                    .border_1()
                    .border_color(rgba(theme::hairline().to_rgba()))
                    .bg(rgb(theme::surface_base()))
                    .child(
                        fileatlas_ui_bridge::tag_capsule::TagCapsule::new(child.name.clone())
                            .color(child.color.as_deref())
                            .icon(child.icon.as_deref())
                            .render()
                            .flex_1()
                            .min_w_0(),
                    )
                    .child(
                        div()
                            .text_right()
                            .text_size(px(12.))
                            .text_color(rgb(theme::text_secondary()))
                            .child(child.media_count.to_string()),
                    )
                    .child(
                        Button::new(format!("inspector-child-edit-{child_id}"))
                            .ghost()
                            .compact()
                            .icon(theme::icon(IconName::Ellipsis))
                            .tooltip(fileatlas_ui_bridge::i18n::tr("Edit tag"))
                            .dropdown_menu(move |menu, _, _| {
                                let edit = child_menu.clone();
                                let remove = child_menu.clone();
                                menu.item(
                                    PopupMenuItem::new(fileatlas_ui_bridge::i18n::tr("Edit tag"))
                                        .on_click(move |_, _, cx| {
                                            let _ = edit.update(cx, |this, cx| {
                                                request(
                                                    this,
                                                    NavigationUiRequest::OpenEdit(child_id),
                                                    cx,
                                                )
                                            });
                                        }),
                                )
                                .item(
                                    PopupMenuItem::new(fileatlas_ui_bridge::i18n::tr("Delete tag"))
                                        .on_click(move |_, _, cx| {
                                            let _ = remove.update(cx, |this, cx| {
                                                request(
                                                    this,
                                                    NavigationUiRequest::OpenDelete(child_id),
                                                    cx,
                                                )
                                            });
                                        }),
                                )
                            }),
                    ),
            );
        }
    }
    if let Some(error) = &view.operation_status {
        body = body.child(inspector_message(error));
    }
    body.into_any_element()
}

fn request(
    view: &mut FileManagerView,
    request: NavigationUiRequest,
    cx: &mut Context<FileManagerView>,
) {
    if let Ok(nav) = view.cordis.try_service::<NavigationService>() {
        if let Err(error) = nav.request_ui(request) {
            view.operation_status = Some(error.to_string());
        }
    }
    cx.notify();
}
