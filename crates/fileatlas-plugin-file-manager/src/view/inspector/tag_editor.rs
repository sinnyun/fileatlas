//! Expanded draft editor. Only Confirm invokes the existing Navigation writes.
use super::*;
use fileatlas_plugin_database::Tag;
use fileatlas_plugin_navigation::NavigationService;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;

#[derive(Clone)]
pub(crate) struct TagDraft {
    id: i64,
    color: Option<String>,
    icon: Option<String>,
    display: bool,
}

pub(super) fn render(
    tag: &Tag,
    view: &mut FileManagerView,
    cx: &mut Context<FileManagerView>,
) -> Div {
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
        .gap(px(theme::space_standard()))
        .child(label("Tag name"))
        .child(
            div().w_full().workbench_control_surface(false).child(
                fileatlas_ui_bridge::components::text_input(&view.inspector_tag_name).w_full(),
            ),
        )
        .child(label("Color"))
        .child(swatches(&draft, cx))
        .child(label("Icon"))
        .child(icons(&draft, cx))
        .child(visibility_toggle(&draft, cx))
        .child(
            fileatlas_ui_bridge::components::button("file_manager-inspector-confirm-tag")
                .primary()
                .label(fileatlas_ui_bridge::i18n::tr("Confirm changes"))
                .on_click(cx.listener(|this, _, _, cx| confirm(this, cx))),
        )
}

fn label(text: &str) -> Div {
    div()
        .workbench_assistant()
        .child(fileatlas_ui_bridge::i18n::tr(text))
}

fn swatches(draft: &TagDraft, cx: &mut Context<FileManagerView>) -> Div {
    let mut row = div()
        .h_flex()
        .w_full()
        .flex_wrap()
        .gap(px(fileatlas_ui_bridge::theme::space_standard()));
    for tint in [
        theme::accent(),
        theme::danger(),
        theme::warning(),
        theme::success(),
        theme::category(),
        theme::data(),
        theme::text_muted(),
    ] {
        let color = format!("#{tint:06x}");
        let value = parse_tag_color(Some(&color));
        let selected = draft
            .color
            .as_deref()
            .is_some_and(|c| c.eq_ignore_ascii_case(&color));
        row = row.child(
            fileatlas_ui_bridge::components::color_swatch(
                format!("file_manager-inspector-color-{color}"),
                value,
                selected,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(draft) = &mut this.inspector_tag_draft {
                    draft.color = Some(color.clone());
                }
                cx.notify();
            })),
        );
    }
    row
}

fn icons(draft: &TagDraft, cx: &mut Context<FileManagerView>) -> Div {
    let mut row = div()
        .h_flex()
        .w_full()
        .flex_wrap()
        .gap(px(fileatlas_ui_bridge::theme::space_compact()));
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
            fileatlas_ui_bridge::components::icon_choice_button(
                format!("file_manager-inspector-icon-{value}"),
                icon,
                active,
            )
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

fn visibility_toggle(draft: &TagDraft, cx: &mut Context<FileManagerView>) -> Div {
    let visible = draft.display;
    let weak = cx.entity().downgrade();
    let text = fileatlas_ui_bridge::i18n::tr("Show on thumbnails");
    let checkbox = fileatlas_ui_bridge::components::checkbox(
        "file_manager-inspector-toggle-tag-display",
        visible,
    )
    .accessibility_label(text.clone())
    .on_change(move |_, _, cx| {
        let _ = weak.update(cx, |this, cx| {
            if let Some(draft) = &mut this.inspector_tag_draft {
                draft.display = !draft.display;
            }
            cx.notify();
        });
    });
    div()
        .h_flex()
        .w_full()
        .items_center()
        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
        .child(checkbox)
        .child(div().workbench_assistant().child(text))
}

fn confirm(view: &mut FileManagerView, cx: &mut Context<FileManagerView>) {
    let Some(draft) = view.inspector_tag_draft.clone() else {
        return;
    };
    let name = view.inspector_tag_name.read(cx).value().trim().to_string();
    let result = (|| -> Result<(), String> {
        if name.is_empty() {
            return Err(fileatlas_ui_bridge::i18n::tr("Name cannot be empty").to_string());
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
