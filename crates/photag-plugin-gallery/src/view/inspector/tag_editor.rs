//! Expanded draft editor. Only Confirm invokes the existing Navigation writes.
use super::*;
use photag_plugin_database::db::queries::Tag;
use photag_plugin_navigation::NavigationService;

#[derive(Clone)]
pub(crate) struct TagDraft {
    id: i64,
    color: Option<String>,
    icon: Option<String>,
    display: bool,
}

pub(super) fn render(tag: &Tag, view: &mut GalleryView, cx: &mut Context<GalleryView>) -> Div {
    if view
        .inspector_tag_draft
        .as_ref()
        .is_none_or(|d| d.id != tag.id)
    {
        view.inspector_tag_draft = Some(TagDraft {
            id: tag.id,
            color: tag.color.clone(),
            icon: tag.icon.clone(),
            display: tag.display,
        });
    }
    let draft = view.inspector_tag_draft.as_ref().unwrap().clone();
    well()
        .gap(px(10.))
        .child(label("Tag name"))
        .child(
            div()
                .w_full()
                .rounded(px(6.))
                .border_1()
                .border_color(rgba(theme::REGION_EDGE))
                .bg(rgb(theme::SURFACE_PANEL))
                .child(Input::new(&view.inspector_tag_name).w_full()),
        )
        .child(label("Color"))
        .child(swatches(&draft, cx))
        .child(label("Icon"))
        .child(icons(&draft, cx))
        .child(visibility_toggle(&draft, cx))
        .child(
            Button::new("gallery-inspector-confirm-tag")
                .primary()
                .w_full()
                .h(px(34.))
                .label(photag_ui_bridge::i18n::tr("Confirm changes"))
                .on_click(cx.listener(|this, _, _, cx| confirm(this, cx))),
        )
}

fn label(text: &str) -> Div {
    div()
        .text_size(px(12.))
        .text_color(rgb(theme::TEXT_SECONDARY))
        .child(photag_ui_bridge::i18n::tr(text))
}

fn swatches(draft: &TagDraft, cx: &mut Context<GalleryView>) -> Div {
    let mut row = div().h_flex().w_full().flex_wrap().gap(px(7.));
    for color in [
        "#3185ED", "#E36B6B", "#E3BD68", "#68B887", "#9B68D7", "#54AEB5", "#8B929E",
    ] {
        let value = parse_tag_color(Some(color));
        let selected = draft
            .color
            .as_deref()
            .is_some_and(|c| c.eq_ignore_ascii_case(color));
        row = row.child(
            div()
                .id(format!("gallery-inspector-color-{color}"))
                .size(px(27.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .border_1()
                .border_color(if selected {
                    rgb(theme::TEXT_PRIMARY)
                } else {
                    rgba(theme::SEPARATOR_LIGHT)
                })
                .cursor_pointer()
                .child(div().size(px(17.)).rounded_full().bg(rgb(value)))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(draft) = &mut this.inspector_tag_draft {
                        draft.color = Some(color.into());
                    }
                    cx.notify();
                })),
        );
    }
    row
}

fn icons(draft: &TagDraft, cx: &mut Context<GalleryView>) -> Div {
    let mut row = div().h_flex().w_full().flex_wrap().gap(px(6.));
    let selected = theme::tag_icon(draft.icon.as_deref());
    for icon in [
        IconName::Tag,
        IconName::Image,
        IconName::Folder,
        IconName::Video,
        IconName::Music,
        IconName::Star,
        IconName::Search,
        IconName::Library,
        IconName::Settings,
        IconName::Puzzle,
        IconName::ZoomIn,
        IconName::Eye,
        IconName::Calendar,
        IconName::ListFilter,
    ] {
        let value = format!("{icon:?}");
        let active = format!("{selected:?}") == value;
        row = row.child(
            Button::new(format!("gallery-inspector-icon-{value}"))
                .w(px(32.))
                .h(px(30.))
                .rounded(px(5.))
                .border_1()
                .border_color(if active {
                    rgb(theme::accent())
                } else {
                    rgba(theme::REGION_EDGE)
                })
                .bg(if active {
                    rgba(theme::accent_alpha(theme::accent(), 0.18))
                } else {
                    rgba(theme::FILL_SUBTLE)
                })
                .text_color(rgb(if active {
                    theme::accent()
                } else {
                    theme::TEXT_SECONDARY
                }))
                .icon(theme::small_icon(icon))
                .accessibility_label(value.clone())
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(draft) = &mut this.inspector_tag_draft {
                        draft.icon = Some(value.clone());
                    }
                    cx.notify();
                })),
        );
    }
    row
}

fn visibility_toggle(draft: &TagDraft, cx: &mut Context<GalleryView>) -> gpui_kit::base::Checkbox {
    let visible = draft.display;
    let weak = cx.entity().downgrade();
    let text = photag_ui_bridge::i18n::tr("Show on thumbnails");
    gpui_kit::base::Checkbox::new("gallery-inspector-toggle-tag-display")
        .checked(visible)
        .accessibility_label(text.clone())
        .h_flex()
        .w_full()
        .items_center()
        .justify_between()
        .min_h(px(32.))
        .text_size(px(12.))
        .text_color(rgb(theme::TEXT_SECONDARY))
        .child(text)
        .child(
            div()
                .relative()
                .w(px(34.))
                .h(px(20.))
                .flex_shrink_0()
                .rounded_full()
                .bg(rgb(if visible {
                    theme::accent()
                } else {
                    theme::SURFACE_ELEVATED
                }))
                .child(
                    div()
                        .absolute()
                        .top(px(2.))
                        .left(px(if visible { 16. } else { 2. }))
                        .size(px(16.))
                        .rounded_full()
                        .bg(rgb(0xffffff)),
                ),
        )
        .on_change(move |_, _, _, cx| {
            let _ = weak.update(cx, |this, cx| {
                if let Some(draft) = &mut this.inspector_tag_draft {
                    draft.display = !draft.display;
                }
                cx.notify();
            });
        })
}

fn confirm(view: &mut GalleryView, cx: &mut Context<GalleryView>) {
    let Some(draft) = view.inspector_tag_draft.clone() else {
        return;
    };
    let name = view.inspector_tag_name.read(cx).value().trim().to_string();
    let result = (|| -> Result<(), String> {
        if name.is_empty() {
            return Err(photag_ui_bridge::i18n::tr("Name cannot be empty").to_string());
        }
        // Revalidate the live action target; drafts never write from render.
        let nav = view
            .cordis
            .try_service::<NavigationService>()
            .map_err(|e| e.to_string())?;
        let current = nav.snapshot().map_err(|e| e.to_string())?;
        let tag = contexts::find_tag(&current.tags, draft.id)
            .ok_or_else(|| "Tag no longer exists".to_string())?;
        if tag.name != name {
            nav.rename_tag(draft.id, &name).map_err(|e| e.to_string())?;
        }
        if tag.color != draft.color || tag.icon != draft.icon {
            nav.set_tag_appearance(draft.id, draft.color.as_deref(), draft.icon.as_deref())
                .map_err(|e| e.to_string())?;
        }
        if tag.display != draft.display {
            nav.set_tag_display(draft.id, draft.display)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    view.operation_status = result.err();
    view.reload();
    cx.notify();
}
