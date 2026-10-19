//! v5 media context menu: real thumbnail/metadata and grouped existing actions.
use super::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
use gpui_kit::prelude::FluentBuilder as _;

/// Render an application-owned action menu for explicit MCP or keyboard opens.
/// Actions delegate to the same GalleryView methods used by the native card menu.
pub(super) fn render_requested(
    media: &photag_plugin_database::MediaItem,
    cx: &mut Context<GalleryView>,
) -> Div {
    let id = media.id;
    let title = media.filename.clone();
    let rename_title = title.clone();
    let dimensions = match (media.width, media.height) {
        (Some(width), Some(height)) => format!("{width} × {height}"),
        _ => photag_ui_bridge::i18n::tr(media.media_type.as_str()).to_string(),
    };
    let thumbnail = if let Some(path) = &media.thumb_path {
        img(std::path::PathBuf::from(path))
            .w(px(76.))
            .h(px(72.))
            .rounded_sm()
            .object_fit(ObjectFit::Cover)
            .into_any_element()
    } else {
        div()
            .w(px(76.))
            .h(px(72.))
            .rounded_sm()
            .bg(rgb(theme::surface_faint()))
            .flex()
            .items_center()
            .justify_center()
            .child(
                theme::large_icon(match media.media_type.as_str() {
                    "video" => IconName::FileVideoCamera,
                    "audio" => IconName::FileMusic,
                    _ => IconName::FileImage,
                })
                .text_color(rgb(theme::text_muted())),
            )
            .into_any_element()
    };
    let ratings = (0..=5u8)
        .map(|rating| {
            Button::new(format!("gallery-requested-menu-rating-{rating}"))
                .ghost()
                .compact()
                .label(if rating == 0 {
                    "—".into()
                } else {
                    rating.to_string()
                })
                .when(rating > 0, |button| {
                    button.icon(theme::small_icon(IconName::Star))
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.apply_rating_to(&[id], rating);
                    this.media_actions_id = None;
                    cx.notify();
                }))
        })
        .collect::<Vec<_>>();
    div()
        .v_flex()
        .w(px(360.))
        .gap_1()
        .p_3()
        .rounded(px(8.))
        .border_1()
        .border_color(rgb(theme::separator()))
        .bg(rgb(theme::surface_modal()))
        .child(
            div()
                .h_flex()
                .items_start()
                .gap_2()
                .child(thumbnail)
                .child(
                    div()
                        .v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .font_medium()
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(title),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(theme::text_secondary()))
                                .child(format!(
                                    "{} · {}",
                                    dimensions,
                                    card::format_bytes(media.file_size)
                                )),
                        )
                        .child(
                            div()
                                .text_size(px(9.))
                                .text_color(rgb(theme::text_muted()))
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(media.full_path.clone()),
                        ),
                )
                .child(
                    Button::new("gallery-requested-menu-close")
                        .ghost()
                        .compact()
                        .icon(theme::icon(IconName::X))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.media_actions_id = None;
                            cx.notify();
                        })),
                ),
        )
        .child(div().h(px(1.)).bg(rgb(theme::separator())))
        .child(
            Button::new("gallery-locate-file")
                .ghost()
                .label(photag_ui_bridge::i18n::tr("Open containing folder"))
                .on_click(cx.listener(move |view, _, _, cx| view.locate_file(id, cx))),
        )
        .child(
            Button::new("gallery-requested-menu-preview")
                .ghost()
                .compact()
                .h(px(28.))
                .justify_start()
                .w_full()
                .label("QuickLook")
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.media_actions_id = None;
                    this.preview_media(id, cx);
                })),
        )
        .child(
            Button::new("gallery-requested-menu-open")
                .ghost()
                .compact()
                .h(px(28.))
                .justify_start()
                .w_full()
                .label(photag_ui_bridge::i18n::tr("Open in default app"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.media_actions_id = None;
                    this.open_media_action(id, false, cx);
                })),
        )
        .child(
            Button::new("gallery-requested-menu-reveal")
                .ghost()
                .compact()
                .h(px(28.))
                .justify_start()
                .w_full()
                .label(photag_ui_bridge::i18n::tr("Show in File Explorer"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.media_actions_id = None;
                    this.open_media_action(id, true, cx);
                })),
        )
        .child(
            div()
                .h_flex()
                .items_center()
                .gap_1()
                .child(div().text_xs().child(photag_ui_bridge::i18n::tr("Rating")))
                .children(ratings),
        )
        .child(
            Button::new("gallery-requested-menu-tags")
                .ghost()
                .compact()
                .h(px(28.))
                .justify_start()
                .w_full()
                .label(photag_ui_bridge::i18n::tr("Tags"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.show_tag_picker = true;
                    this.media_actions_id = None;
                    cx.notify();
                })),
        )
        .child(
            Button::new("gallery-requested-menu-ai-tags")
                .ghost()
                .compact()
                .h(px(28.))
                .justify_start()
                .w_full()
                .label(photag_ui_bridge::i18n::tr("AI Suggest Tags"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.media_actions_id = None;
                    this.suggest_tags(id, cx);
                })),
        )
        .child(
            Button::new("gallery-requested-menu-rename")
                .ghost()
                .compact()
                .h(px(28.))
                .justify_start()
                .w_full()
                .label(photag_ui_bridge::i18n::tr("Rename…"))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.selection.selected_media_ids = vec![id];
                    this.selection.anchor_id = Some(id);
                    this.rename_input
                        .update(cx, |input, cx| input.set_value(&rename_title, window, cx));
                    this.show_selection_details = true;
                    this.media_actions_id = None;
                    cx.notify();
                })),
        )
        .child(
            Button::new("gallery-requested-menu-copy")
                .ghost()
                .compact()
                .h(px(28.))
                .justify_start()
                .w_full()
                .label(photag_ui_bridge::i18n::tr("Copy to…"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.selection.selected_media_ids = vec![id];
                    this.copy_selected_with_dialog(cx);
                    this.media_actions_id = None;
                })),
        )
        .child(
            Button::new("gallery-requested-menu-move")
                .ghost()
                .compact()
                .h(px(28.))
                .justify_start()
                .w_full()
                .label(photag_ui_bridge::i18n::tr("Move to…"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.selection.selected_media_ids = vec![id];
                    this.move_selected_with_dialog(cx);
                    this.media_actions_id = None;
                })),
        )
        .child(
            Button::new("gallery-requested-menu-delete")
                .ghost()
                .compact()
                .h(px(28.))
                .justify_start()
                .w_full()
                .label(photag_ui_bridge::i18n::tr("Delete…"))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.selection.selected_media_ids = vec![id];
                    this.request_delete_confirmation(window, cx);
                    this.media_actions_id = None;
                })),
        )
}

pub(super) fn build(
    mut menu: PopupMenu,
    media: photag_plugin_database::MediaItem,
    menu_targets: Vec<i64>,
    rating_target_ids: Vec<i64>,
    tag_tree: Arc<Vec<photag_plugin_database::Tag>>,
    menu_view: WeakEntity<GalleryView>,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let media_id = media.id;
    let filename = media.filename.clone();
    let dimensions = match (media.width, media.height) {
        (Some(w), Some(h)) => format!("{w} × {h}"),
        _ => String::new(),
    };
    let bytes = card::format_bytes(media.file_size);
    let thumb = media.thumb_path.clone();
    menu = menu
        .min_w(px(250.))
        .item(PopupMenuItem::element(move |_, _| {
            let mut row = div().h_flex().items_center().gap(px(10.)).py(px(8.));
            if let Some(path) = &thumb {
                row = row.child(
                    img(PathBuf::from(path))
                        .w(px(68.))
                        .h(px(48.))
                        .object_fit(ObjectFit::Cover)
                        .rounded(px(theme::ROUND_SM)),
                );
            }
            row.child(
                div()
                    .v_flex()
                    .min_w_0()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(theme::FONT_SIZE_BODY))
                            .child(filename.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(theme::FONT_SIZE_SMALL))
                            .text_color(rgb(theme::text_secondary()))
                            .child(dimensions.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(theme::FONT_SIZE_SMALL))
                            .text_color(rgb(theme::text_muted()))
                            .child(bytes.clone()),
                    ),
            )
        }))
        .item(PopupMenuItem::separator());
    let preview_view = menu_view.clone();
    menu = menu.item(PopupMenuItem::new("QuickLook").on_click(move |_, _, cx| {
        let _ = preview_view.update(cx, |this, cx| {
            this.preview_media(media_id, cx);
        });
    }));
    let open_view = menu_view.clone();
    menu = menu.item(
        PopupMenuItem::new(photag_ui_bridge::i18n::tr("Open in default app")).on_click(
            move |_, _, cx| {
                let _ = open_view.update(cx, |this, cx| {
                    this.open_media_action(media_id, false, cx);
                });
            },
        ),
    );
    let reveal_view = menu_view.clone();
    menu = menu.item(
        PopupMenuItem::new(photag_ui_bridge::i18n::tr("Show in File Explorer")).on_click(
            move |_, _, cx| {
                let _ = reveal_view.update(cx, |this, cx| {
                    this.open_media_action(media_id, true, cx);
                });
            },
        ),
    );
    menu = menu.item(PopupMenuItem::separator());
    let tags_view = menu_view.clone();
    let tags_targets = menu_targets.clone();
    menu = menu.submenu(
        photag_ui_bridge::i18n::tr("Tags"),
        window,
        cx,
        move |mut menu, _, _| {
            for tag in card::flatten_tags(&tag_tree) {
                let view = tags_view.clone();
                let targets = tags_targets.clone();
                menu = menu.item(PopupMenuItem::new(tag.label).on_click(move |_, _, cx| {
                    let _ =
                        view.update(cx, |this, cx| this.assign_context_tag(&targets, tag.id, cx));
                }));
            }
            menu
        },
    );

    let rating_view = menu_view.clone();
    let targets = rating_target_ids.clone();
    menu = menu.submenu(
        photag_ui_bridge::i18n::tr("Rating"),
        window,
        cx,
        move |mut menu, _, _| {
            for rating in 0..=5u8 {
                let view = rating_view.clone();
                let targets = targets.clone();
                let label = if rating == 0 {
                    photag_ui_bridge::i18n::tr("No rating")
                } else {
                    rating.to_string()
                };
                let item = PopupMenuItem::new(label);
                let item = if rating == 0 {
                    item
                } else {
                    item.icon(theme::small_icon(IconName::Star))
                };
                menu = menu.item(item.on_click(move |_, _, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.apply_rating_to(&targets, rating);
                        cx.notify();
                    });
                }));
            }
            menu
        },
    );
    let locate_view = menu_view.clone();
    menu = menu.item(
        PopupMenuItem::new(photag_ui_bridge::i18n::tr("Open containing folder")).on_click(
            move |_, _, cx| {
                let _ = locate_view.update(cx, |view, cx| view.locate_file(media_id, cx));
            },
        ),
    );
    let ai_view = menu_view.clone();
    menu = menu.item(
        PopupMenuItem::new(photag_ui_bridge::i18n::tr("AI Suggest Tags"))
            .icon(theme::icon(IconName::Sparkles))
            .on_click(move |_, _, cx| {
                let _ = ai_view.update(cx, |this, cx| this.suggest_tags(media_id, cx));
            }),
    );
    menu = menu.item(PopupMenuItem::separator());
    let rename_view = menu_view.clone();
    let rename_filename = media.filename.clone();
    menu = menu.item(
        PopupMenuItem::new(photag_ui_bridge::i18n::tr("Rename…"))
            .icon(theme::icon(IconName::Pencil))
            .on_click(move |_, window, cx| {
                let _ = rename_view.update(cx, |this, cx| {
                    this.selection.selected_media_ids = vec![media_id];
                    this.selection.anchor_id = Some(media_id);
                    this.rename_input.update(cx, |input, cx| {
                        input.set_value(&rename_filename, window, cx)
                    });
                    this.show_selection_details = true;
                    cx.notify();
                });
            }),
    );
    let copy_view = menu_view.clone();
    let copy_targets = menu_targets.clone();
    menu = menu.item(
        PopupMenuItem::new(photag_ui_bridge::i18n::tr("Copy to…"))
            .icon(theme::icon(IconName::Copy))
            .on_click(move |_, _, cx| {
                let _ = copy_view.update(cx, |this, cx| {
                    this.selection.selected_media_ids = copy_targets.clone();
                    this.copy_selected_with_dialog(cx);
                });
            }),
    );
    let move_view = menu_view.clone();
    let move_targets = menu_targets.clone();
    menu = menu.item(
        PopupMenuItem::new(photag_ui_bridge::i18n::tr("Move to…"))
            .icon(theme::icon(IconName::Folder))
            .on_click(move |_, _, cx| {
                let _ = move_view.update(cx, |this, cx| {
                    this.selection.selected_media_ids = move_targets.clone();
                    this.move_selected_with_dialog(cx);
                });
            }),
    );
    let delete_view = menu_view.clone();
    let delete_targets = menu_targets;
    menu.item(PopupMenuItem::separator()).item(
        PopupMenuItem::new(photag_ui_bridge::i18n::tr("Delete…"))
            .icon(theme::icon(IconName::Trash))
            .on_click(move |_, window, cx| {
                let _ = delete_view.update(cx, |this, cx| {
                    this.selection.selected_media_ids = delete_targets.clone();
                    this.selection.anchor_id = delete_targets.first().copied();
                    this.request_delete_confirmation(window, cx);
                });
            }),
    )
}
