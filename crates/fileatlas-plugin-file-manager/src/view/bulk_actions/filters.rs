use super::{FileManagerView, date_row};
use crate::filter::MediaTypeFilter;
use fileatlas_plugin_database::MediaItem;
use fileatlas_ui_bridge::components::WorkbenchPaint as _;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
use fileatlas_ui_bridge::theme;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::*;
pub(super) fn section_title(label: &'static str) -> Div {
    fileatlas_ui_bridge::components::section_heading(fileatlas_ui_bridge::i18n::tr(label))
}

pub(super) fn filter_controls(view: &FileManagerView, cx: &mut Context<FileManagerView>) -> Div {
    let filters = div()
        .v_flex()
        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
        .child(section_title("3. Match all filters (AND)"))
        .child(
            div()
                .h_flex()
                .items_center()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                .child(
                    div()
                        .w(px(110.))
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr("Media type")),
                )
                .children(
                    [
                        (MediaTypeFilter::All, "All"),
                        (MediaTypeFilter::Photos, "Photos"),
                        (MediaTypeFilter::Videos, "Videos"),
                        (MediaTypeFilter::Audio, "Audio"),
                    ]
                    .into_iter()
                    .map(|(choice, label)| {
                        fileatlas_ui_bridge::components::button(format!(
                            "file_manager-bulk-type-{label}"
                        ))
                        .label(fileatlas_ui_bridge::i18n::tr(label))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.bulk_media_type = choice;
                            this.confirm_bulk_action = false;
                            this.confirm_bulk_action_ids = None;
                            cx.notify();
                        }))
                    }),
                ),
        )
        .child(
            div()
                .h_flex()
                .items_center()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                .child(
                    div()
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr("File name")),
                )
                .child(
                    fileatlas_ui_bridge::components::text_input(&view.bulk_inputs[4])
                        .flex_1()
                        .min_w_0(),
                ),
        )
        .child(
            div()
                .h_flex()
                .items_center()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                .child(
                    div()
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr("Extensions")),
                )
                .child(
                    fileatlas_ui_bridge::components::button("file_manager-bulk-extension-mode")
                        .label(fileatlas_ui_bridge::i18n::tr(
                            if view.bulk_extension_exclude {
                                "Exclude"
                            } else {
                                "Include"
                            },
                        ))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.bulk_extension_exclude = !this.bulk_extension_exclude;
                            this.confirm_bulk_action = false;
                            this.confirm_bulk_action_ids = None;
                            cx.notify();
                        })),
                )
                .child(
                    fileatlas_ui_bridge::components::text_input(&view.bulk_inputs[5])
                        .flex_1()
                        .min_w_0(),
                ),
        )
        .child(date_row(view, true, cx))
        .child(date_row(view, false, cx))
        .child(
            div()
                .h_flex()
                .items_center()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                .child(
                    div()
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr("Rating")),
                )
                .child(
                    div()
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr("Minimum rating")),
                )
                .child(fileatlas_ui_bridge::components::text_input(
                    &view.bulk_inputs[2],
                ))
                .child(
                    div()
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr("Maximum rating")),
                )
                .child(fileatlas_ui_bridge::components::text_input(
                    &view.bulk_inputs[3],
                ))
                .child(
                    div()
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr("0–5")),
                ),
        )
        .child(
            div()
                .h_flex()
                .items_start()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                .child(
                    div()
                        .pt(px(fileatlas_ui_bridge::theme::space_compact()))
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr("Tags")),
                )
                .child(bulk_tag_tree(view, cx)),
        )
        .child(section_title("Advanced filters"))
        .child(
            div()
                .h_flex()
                .items_center()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                .child(
                    div()
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr("Older / newer days")),
                )
                .child(fileatlas_ui_bridge::components::text_input(
                    &view.bulk_inputs[0],
                ))
                .child(fileatlas_ui_bridge::components::text_input(
                    &view.bulk_inputs[1],
                ))
                .child(
                    div()
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr("Minimum / maximum age")),
                ),
        )
        .child(
            div()
                .h_flex()
                .items_center()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                .child(
                    div()
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr("Size in MB")),
                )
                .child(fileatlas_ui_bridge::components::text_input(
                    &view.bulk_inputs[6],
                ))
                .child(fileatlas_ui_bridge::components::text_input(
                    &view.bulk_inputs[7],
                ))
                .child(
                    div()
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr("Minimum / maximum size")),
                ),
        );
    filters
}

fn bulk_tag_tree(view: &FileManagerView, cx: &mut Context<FileManagerView>) -> impl IntoElement {
    let tags = view
        .snapshot
        .as_ref()
        .ok()
        .map(|snapshot| snapshot.tag_tree.clone())
        .unwrap_or_default();
    div()
        .v_flex()
        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
        .max_h(px(120.))
        .overflow_y_scrollbar()
        .children(bulk_tag_rows(&tags, 0, &view.bulk_tag_ids, cx))
}

fn bulk_tag_rows(
    tags: &[fileatlas_plugin_database::Tag],
    depth: usize,
    selected: &[i64],
    cx: &mut Context<FileManagerView>,
) -> Vec<Div> {
    let mut rows = Vec::new();
    for tag in tags {
        let id = tag.id;
        let checked = selected.contains(&id);
        rows.push(
            div()
                .h_flex()
                .items_center()
                .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                .pl(px((depth as f32) * 16.))
                .child(
                    fileatlas_ui_bridge::components::button(format!("file_manager-bulk-tag-{id}"))
                        .icon(theme::small_icon(if checked {
                            gpui_kit::assets::IconName::Check
                        } else {
                            gpui_kit::assets::IconName::Square
                        }))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(index) =
                                this.bulk_tag_ids.iter().position(|tag_id| *tag_id == id)
                            {
                                this.bulk_tag_ids.remove(index);
                            } else {
                                this.bulk_tag_ids.push(id);
                            }
                            this.confirm_bulk_action = false;
                            this.confirm_bulk_action_ids = None;
                            cx.notify();
                        })),
                )
                .child(
                    fileatlas_ui_bridge::tag_capsule::TagCapsule::new(tag.name.clone())
                        .color(tag.color.as_deref())
                        .icon(tag.icon.as_deref())
                        .selected(checked)
                        .render()
                        .flex_1()
                        .min_w_0(),
                ),
        );
        rows.extend(bulk_tag_rows(&tag.children, depth + 1, selected, cx));
    }
    rows
}

pub(super) fn match_results_list(
    media: &[MediaItem],
    count: usize,
    height: f32,
) -> impl IntoElement {
    let mut list = fileatlas_ui_bridge::components::card_surface(
        div()
            .v_flex()
            .gap(px(fileatlas_ui_bridge::theme::space_standard()))
            .w(px(240.))
            .flex_shrink_0()
            .overflow_y_scrollbar()
            .min_h_0()
            .h(px(height))
            .p(px(fileatlas_ui_bridge::theme::space_standard())),
        false,
    )
    .child(
        div()
            .workbench_assistant()
            .child(fileatlas_ui_bridge::i18n::tr_args(
                "Matching media ({count})",
                &[("count", count.to_string())],
            )),
    );
    if media.is_empty() {
        list = list.child(
            div()
                .workbench_assistant()
                .child(fileatlas_ui_bridge::i18n::tr(
                    "No matching media preview is available.",
                )),
        );
    } else {
        for item in media {
            let mut row = div()
                .h_flex()
                .items_center()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()));
            if let Some(path) = &item.thumb_path {
                row = row.child(
                    img(std::path::PathBuf::from(path))
                        .w(px(48.))
                        .h(px(theme::ui_height(theme::CONTROL_HEIGHT_FOOTER)))
                        .object_fit(ObjectFit::Cover)
                        .workbench_corner(fileatlas_ui_bridge::components::CornerRole::Small),
                );
            } else {
                row = row.child(
                    div()
                        .w(px(48.))
                        .h(px(theme::ui_height(theme::CONTROL_HEIGHT_FOOTER)))
                        .flex()
                        .items_center()
                        .justify_center()
                        .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Faint)
                        .child(theme::large_icon(if item.media_type == "video" {
                            gpui_kit::assets::IconName::Play
                        } else if item.media_type == "audio" {
                            gpui_kit::assets::IconName::Music
                        } else {
                            gpui_kit::assets::IconName::Image
                        })),
                );
            }
            list = list.child(
                row.child(
                    div()
                        .v_flex()
                        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                        .overflow_hidden()
                        .child(
                            div()
                                .workbench_assistant()
                                .text_ellipsis()
                                .child(item.filename.clone()),
                        )
                        .child(
                            div()
                                .workbench_assistant()
                                .child(fileatlas_ui_bridge::i18n::tr(item.media_type.as_str())),
                        ),
                ),
            );
        }
    }
    list
}
