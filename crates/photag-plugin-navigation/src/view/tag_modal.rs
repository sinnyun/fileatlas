use super::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::scroll::ScrollableElement;

impl SidebarView {
    pub(super) fn render_tag_modal(&mut self, cx: &mut Context<Self>) -> Div {
        if let Some(id) = self.rename_favorite {
            return modal_shell(
                photag_ui_bridge::i18n::tr("Rename favorite"),
                div()
                    .v_flex()
                    .gap_3()
                    .child(Input::new(&self.tag_name))
                    .child(
                        Button::new("favorite-rename-save")
                            .primary()
                            .label(photag_ui_bridge::i18n::tr("Save"))
                            .on_click(cx.listener(move |view, _, _, cx| {
                                let name = view.tag_name.read(cx).value().to_string();
                                if let Err(error) = view.service.rename_favorite(id, &name) {
                                    view.snapshot = Err(error.to_string());
                                } else {
                                    view.rename_favorite = None;
                                    view.refresh();
                                }
                                cx.notify();
                            })),
                    ),
                modal_close_button(cx),
                430.,
            );
        }
        if self.tag_create_open {
            let create = Button::new("navigation-create-tag-submit")
                .primary()
                .label(photag_ui_bridge::i18n::tr("Create"))
                .on_click(cx.listener(|this, _, _, cx| this.create_tag(cx)));
            return modal_shell(
                photag_ui_bridge::i18n::tr("Create tag"),
                div()
                    .v_flex()
                    .gap_3()
                    .child(field_label("Tag name"))
                    .child(Input::new(&self.tag_name))
                    .child(field_label("Parent tag"))
                    .child(tag_picker::parent_picker(self, cx))
                    .child(
                        div()
                            .h_flex()
                            .pt(px(10.))
                            .border_t_1()
                            .border_color(rgb(theme::SEPARATOR))
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("navigation-create-tag-cancel")
                                    .label(photag_ui_bridge::i18n::tr("Cancel"))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.tag_create_open = false;
                                        cx.notify();
                                    })),
                            )
                            .child(create),
                    ),
                modal_close_button(cx),
                430.,
            );
        }

        if let Some(id) = self.associated_edit_tag {
            if self.tag_edit_name_only {
                return modal_shell(
                    photag_ui_bridge::i18n::tr("Rename tag"),
                    div()
                        .v_flex()
                        .gap_3()
                        .child(field_label("Tag name"))
                        .child(Input::new(&self.tag_edit_name))
                        .child(
                            div()
                                .h_flex()
                                .pt(px(10.))
                                .border_t_1()
                                .border_color(rgb(theme::SEPARATOR))
                                .justify_end()
                                .gap_2()
                                .child(
                                    Button::new("navigation-rename-tag-cancel")
                                        .label(photag_ui_bridge::i18n::tr("Cancel"))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.associated_edit_tag = None;
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("navigation-rename-tag-save")
                                        .primary()
                                        .label(photag_ui_bridge::i18n::tr("Save"))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.rename_tag(id, cx);
                                            if this.snapshot.is_ok() {
                                                this.associated_edit_tag = None;
                                            }
                                            cx.notify();
                                        })),
                                ),
                        ),
                    modal_close_button(cx),
                    430.,
                );
            }
            let tags = self
                .snapshot
                .as_ref()
                .ok()
                .map(|s| s.tags.clone())
                .unwrap_or_default();
            let mut association_rows = Vec::new();
            collect_association_rows(&tags, id, &mut association_rows);
            let associations = association_rows.into_iter().fold(
                div()
                    .v_flex()
                    .gap_1()
                    .max_h(px(286.))
                    .overflow_y_scrollbar(),
                |rows, (tag_id, path)| {
                    let checked = self.associated_tag_ids.contains(&tag_id);
                    let depth = path.matches('>').count();
                    rows.child(
                        Button::new(format!("navigation-associated-tag-{tag_id}"))
                            .ghost()
                            .w_full()
                            .pl(px(6. + depth as f32 * 12.))
                            .icon(theme::small_icon(if checked {
                                IconName::Check
                            } else {
                                IconName::Square
                            }))
                            .label(photag_ui_bridge::i18n::tr(&path.replace('>', " › ")))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.toggle_associated_tag(tag_id, cx)
                            })),
                    )
                },
            );
            let selected_color = self.tag_color.read(cx).value().trim().to_ascii_lowercase();
            let swatches = [
                [
                    "#E85D5D", "#F07855", "#F08C46", "#D9A541", "#D9C044", "#9DAA47", "#69A96B",
                    "#4FAF9F", "#4AAFB5", "#4A9CC7", "#5C7FE0",
                ],
                [
                    "#716DDD", "#8A69CE", "#A168C7", "#C168A6", "#E06FA3", "#A77C5A", "#BE8C63",
                    "#858A92", "#A6ABB2", "#D1D5DA", "#E6E9ED",
                ],
            ]
            .into_iter()
            .map(|row| {
                div()
                    .h_flex()
                    .gap(px(4.))
                    .children(row.into_iter().map(|color| {
                        let value = color.to_owned();
                        let selected = selected_color == color.to_ascii_lowercase();
                        let parsed = crate::parse_hex_color(color).unwrap_or([120, 120, 130]);
                        let tint = (u32::from(parsed[0]) << 16)
                            | (u32::from(parsed[1]) << 8)
                            | u32::from(parsed[2]);
                        Button::new(format!("navigation-tag-color-{color}"))
                            .when(selected, |button| {
                                button.icon(theme::small_icon(IconName::Check))
                            })
                            .text_color(rgb(0xffffff))
                            .w(px(22.))
                            .h(px(22.))
                            .rounded(px(12.))
                            .border_2()
                            .border_color(rgb(if selected {
                                theme::TEXT_PRIMARY
                            } else {
                                theme::SURFACE_MODAL
                            }))
                            .bg(rgb(tint))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.tag_color
                                    .update(cx, |state, cx| state.set_value(&value, window, cx));
                            }))
                    }))
            })
            .collect::<Vec<_>>();
            let selected_icon = self.tag_icon.read(cx).value().trim().to_owned();
            let selected_icon = (!selected_icon.is_empty())
                .then(|| format!("{:?}", theme::tag_icon(Some(&selected_icon))));
            let icons = [
                [
                    IconName::Tag,
                    IconName::Image,
                    IconName::Folder,
                    IconName::Video,
                    IconName::Music,
                    IconName::Star,
                    IconName::Search,
                    IconName::Library,
                ],
                [
                    IconName::Settings,
                    IconName::Puzzle,
                    IconName::ZoomIn,
                    IconName::Eye,
                    IconName::Calendar,
                    IconName::ListFilter,
                    IconName::ArrowUpDown,
                    IconName::Plus,
                ],
            ]
            .into_iter()
            .map(|row| {
                div()
                    .h_flex()
                    .gap(px(5.))
                    .children(row.into_iter().map(|icon| {
                        let value = format!("{icon:?}");
                        let selected = selected_icon.as_deref() == Some(value.as_str());
                        Button::new(format!("navigation-tag-icon-{value}"))
                            .w(px(30.))
                            .h(px(30.))
                            .rounded(px(4.))
                            .text_color(rgb(if selected {
                                theme::accent_hover()
                            } else {
                                theme::TEXT_PRIMARY
                            }))
                            .bg(rgba(if selected {
                                theme::accent_alpha(theme::accent(), 70. / 255.)
                            } else {
                                theme::FILL_SUBTLE
                            }))
                            .border_1()
                            .border_color(rgb(if selected {
                                theme::accent()
                            } else {
                                theme::SEPARATOR
                            }))
                            .icon(theme::small_icon(icon))
                            .accessibility_label(value.clone())
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.tag_icon
                                    .update(cx, |state, cx| state.set_value(&value, window, cx));
                            }))
                    }))
            })
            .collect::<Vec<_>>();
            return modal_shell(
                photag_ui_bridge::i18n::tr("Edit tag"),
                div()
                    .v_flex()
                    .gap_3()
                    .child(
                        div()
                            .h_flex()
                            .items_start()
                            .gap(px(18.))
                            .child(
                                div()
                                    .v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap_2()
                                    .child(field_label("Tag name"))
                                    .child(Input::new(&self.tag_edit_name))
                                    .child(field_label("Tag color"))
                                    .child(div().v_flex().gap(px(4.)).children(swatches))
                                    .child(Input::new(&self.tag_color))
                                    .child(field_label("Tag icon"))
                                    .child(div().v_flex().gap(px(5.)).children(icons))
                                    .child(
                                        Button::new("navigation-tag-edit-display")
                                            .ghost()
                                            .w_full()
                                            .justify_start()
                                            .when(self.tag_edit_display, |button| {
                                                button.icon(theme::small_icon(IconName::Check))
                                            })
                                            .label(photag_ui_bridge::i18n::tr(
                                                "Display on thumbnails",
                                            ))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.tag_edit_display = !this.tag_edit_display;
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap_2()
                                    .child(field_label("Associated tags"))
                                    .child(
                                        div()
                                            .v_flex()
                                            .gap_1()
                                            .p(px(8.))
                                            .rounded(px(4.))
                                            .bg(rgb(theme::SURFACE_FAINT))
                                            .border_1()
                                            .border_color(rgb(theme::SEPARATOR))
                                            .child(associations),
                                    )
                                    .child(
                                        Button::new("navigation-tag-appearance-children")
                                            .ghost()
                                            .w_full()
                                            .justify_start()
                                            .when(self.apply_tag_appearance_to_children, |button| {
                                                button.icon(theme::small_icon(IconName::Check))
                                            })
                                            .label(photag_ui_bridge::i18n::tr("Apply to children"))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.apply_tag_appearance_to_children =
                                                    !this.apply_tag_appearance_to_children;
                                                cx.notify();
                                            })),
                                    )
                                    .when(self.apply_tag_appearance_to_children, |body| {
                                        body.child(
                                            Button::new("navigation-tag-appearance-skip-edited")
                                                .ghost()
                                                .w_full()
                                                .justify_start()
                                                .when(self.exclude_edited_tag_children, |button| {
                                                    button.icon(theme::small_icon(IconName::Check))
                                                })
                                                .label(photag_ui_bridge::i18n::tr(
                                                    "Skip edited children",
                                                ))
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.exclude_edited_tag_children =
                                                        !this.exclude_edited_tag_children;
                                                    cx.notify();
                                                })),
                                        )
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .h_flex()
                            .pt(px(10.))
                            .border_t_1()
                            .border_color(rgb(theme::SEPARATOR))
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("navigation-edit-tag-cancel")
                                    .label(photag_ui_bridge::i18n::tr("Cancel"))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.associated_edit_tag = None;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("navigation-edit-tag-save")
                                    .primary()
                                    .label(photag_ui_bridge::i18n::tr("Save"))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.update_tag_appearance(id, cx);
                                        let current_display = this
                                            .snapshot
                                            .as_ref()
                                            .ok()
                                            .and_then(|snapshot| find_tag(&snapshot.tags, id))
                                            .map(|tag| tag.display);
                                        if this.snapshot.is_ok()
                                            && current_display != Some(this.tag_edit_display)
                                        {
                                            this.toggle_tag_display(id, this.tag_edit_display, cx);
                                        }
                                        this.associated_edit_tag = None;
                                        cx.notify();
                                    })),
                            ),
                    ),
                modal_close_button(cx),
                760.,
            );
        }

        if let Some((id, name, count)) = self.confirm_delete.clone() {
            return modal_shell(
                photag_ui_bridge::i18n::tr("Delete tag"),
                div()
                    .v_flex()
                    .gap_3()
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .text_color(rgb(theme::DANGER))
                                    .child(theme::large_icon(IconName::TriangleAlert)),
                            )
                            .child(
                                div()
                                    .v_flex()
                                    .gap_1()
                                    .child(
                                        div().font_family(theme::SEMIBOLD_FONT_FAMILY).child(name),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(rgb(theme::TEXT_SECONDARY))
                                            .child(photag_ui_bridge::i18n::tr_args(
                                                "Used by {count} media items",
                                                &[("count", count.to_string())],
                                            )),
                                    ),
                            ),
                    )
                    .child(field_label("When deleting this tag"))
                    .child(
                        Button::new("navigation-delete-tag-keep")
                            .ghost()
                            .w_full()
                            .justify_start()
                            .icon(theme::small_icon(if !self.delete_clean_parents {
                                IconName::CircleDot
                            } else {
                                IconName::Circle
                            }))
                            .label(photag_ui_bridge::i18n::tr("Keep parent tags"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.delete_clean_parents = false;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("navigation-delete-tag-clean")
                            .ghost()
                            .w_full()
                            .justify_start()
                            .icon(theme::small_icon(if self.delete_clean_parents {
                                IconName::CircleDot
                            } else {
                                IconName::Circle
                            }))
                            .label(photag_ui_bridge::i18n::tr("Also remove parent tags"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.delete_clean_parents = true;
                                cx.notify();
                            })),
                    )
                    .child(div().text_sm().text_color(rgb(theme::TEXT_MUTED)).child(
                        photag_ui_bridge::i18n::tr(
                            "This action removes tag assignments from matching media.",
                        ),
                    ))
                    .child(
                        div()
                            .h_flex()
                            .pt(px(10.))
                            .border_t_1()
                            .border_color(rgb(theme::SEPARATOR))
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("navigation-delete-tag-cancel")
                                    .label(photag_ui_bridge::i18n::tr("Cancel"))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.confirm_delete = None;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("navigation-delete-tag-confirm")
                                    .danger()
                                    .label(photag_ui_bridge::i18n::tr("Delete"))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        let clean_parents = this.delete_clean_parents;
                                        this.confirm_delete = None;
                                        this.delete_tag(id, clean_parents, cx);
                                    })),
                            ),
                    ),
                modal_close_button(cx),
                430.,
            );
        }

        if let Some(source_id) = self.merge_source {
            let source_tag = self
                .snapshot
                .as_ref()
                .ok()
                .and_then(|s| find_tag(&s.tags, source_id))
                .cloned();
            let source_name = source_tag
                .as_ref()
                .map(|tag| tag.name.clone())
                .unwrap_or_else(|| "tag".into());
            let targets = super::render::merge_target_buttons(self, source_id, cx);
            let child_count = self
                .snapshot
                .as_ref()
                .ok()
                .and_then(|snapshot| find_tag(&snapshot.tags, source_id))
                .map(|tag| tag.children.len())
                .unwrap_or_default();
            return modal_shell(
                photag_ui_bridge::i18n::tr("Merge tag"),
                div()
                    .v_flex()
                    .gap_3()
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap_2()
                            .child(div().font_family(theme::SEMIBOLD_FONT_FAMILY).child(photag_ui_bridge::i18n::tr("Source tag")))
                            .child(
                                photag_ui_bridge::tag_capsule::TagCapsule::new(source_name)
                                    .color(
                                        source_tag
                                            .as_ref()
                                            .and_then(|tag| tag.color.as_deref()),
                                    )
                                    .icon(
                                        source_tag
                                            .as_ref()
                                            .and_then(|tag| tag.icon.as_deref()),
                                    )
                                    .render(),
                            ),
                    )
                    .child(
                        div()
                            .h_flex()
                            .items_start()
                            .gap(px(16.))
                            .child(
                                div()
                                    .v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap_2()
                                    .child(field_label("Choose a merge target"))
                                    .child(Input::new(&self.merge_search))
                                    .child(
                                        div()
                                            .v_flex()
                                            .gap_1()
                                            .p(px(6.))
                                            .max_h(px(310.))
                                            .overflow_y_scrollbar()
                                            .rounded(px(4.))
                                            .bg(rgb(theme::SURFACE_FAINT))
                                            .border_1()
                                            .border_color(rgb(theme::SEPARATOR))
                                            .children(targets),
                                    ),
                            )
                            .child(
                                div()
                                    .v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap_2()
                                    .child(field_label("Child tag handling"))
                                    .child(div().text_sm().text_color(rgb(theme::TEXT_SECONDARY)).child(
                                        photag_ui_bridge::i18n::tr_args(
                                            "This tag has {count} direct child tags.",
                                            &[("count", child_count.to_string())],
                                        ),
                                    ))
                                    .child(
                                        Button::new("navigation-merge-children-reparent")
                                            .ghost()
                                            .w_full()
                                            .justify_start()
                                            .icon(theme::small_icon(if self.merge_reparent_children {
                                                IconName::CircleDot
                                            } else {
                                                IconName::Circle
                                            }))
                                            .label(photag_ui_bridge::i18n::tr("Re-parent children under target"))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.merge_reparent_children = true;
                                                cx.notify();
                                            })),
                                    )
                                    .child(div().pl(px(24.)).text_xs().text_color(rgb(theme::TEXT_MUTED)).child(
                                        photag_ui_bridge::i18n::tr("Existing child tags are kept and moved below the target."),
                                    ))
                                    .child(
                                        Button::new("navigation-merge-children-delete")
                                            .ghost()
                                            .w_full()
                                            .justify_start()
                                            .icon(theme::small_icon(if self.merge_reparent_children {
                                                IconName::Circle
                                            } else {
                                                IconName::CircleDot
                                            }))
                                            .label(photag_ui_bridge::i18n::tr("Delete child tags"))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.merge_reparent_children = false;
                                                cx.notify();
                                            })),
                                    )
                                    .child(div().pl(px(24.)).text_xs().text_color(rgb(theme::TEXT_MUTED)).child(
                                        photag_ui_bridge::i18n::tr("Child tag assignments are removed with the source tag."),
                                    )),
                            ),
                    )
                    .child(
                        Button::new("navigation-merge-cancel")
                            .self_end()
                            .label(photag_ui_bridge::i18n::tr("Cancel"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.merge_source = None;
                                cx.notify();
                            })),
                    ),
                modal_close_button(cx),
                720.,
            );
        }

        if let Some(source_id) = self.move_source_id {
            return modal_shell(
                photag_ui_bridge::i18n::tr("Move tag"),
                div()
                    .v_flex()
                    .gap_2()
                    .child(tag_picker::move_target_picker(self, cx))
                    .child(
                        div()
                            .h_flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("navigation-move-tag-cancel")
                                    .label(photag_ui_bridge::i18n::tr("Cancel"))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.move_source_id = None;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("navigation-move-tag-submit")
                                    .primary()
                                    .label(photag_ui_bridge::i18n::tr("Move"))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.move_tag_into(source_id, cx);
                                        if this.snapshot.is_ok() {
                                            this.move_source_id = None;
                                        }
                                        cx.notify();
                                    })),
                            ),
                    ),
                modal_close_button(cx),
                480.,
            );
        }
        div()
    }
}

fn modal_shell(title: String, body: Div, close: Button, width: f32) -> Div {
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(rgba(theme::SCRIM))
        .child(
            div()
                .v_flex()
                .items_start()
                .h_auto()
                .w(px(width))
                .max_h(px(640.))
                .overflow_y_scrollbar()
                .gap_3()
                .p(px(18.))
                .rounded(px(6.))
                .bg(rgb(theme::SURFACE_MODAL))
                .border_1()
                .border_color(rgb(theme::SEPARATOR))
                .child(
                    div()
                        .h_flex()
                        .items_center()
                        .justify_between()
                        .w_full()
                        .pb(px(10.))
                        .border_b_1()
                        .border_color(rgb(theme::SEPARATOR))
                        .child(
                            div()
                                .text_size(px(15.))
                                .font_family(theme::SEMIBOLD_FONT_FAMILY)
                                .child(title),
                        )
                        .child(close),
                )
                .child(body.w_full()),
        )
}

fn modal_close_button(cx: &mut Context<SidebarView>) -> Button {
    Button::new("navigation-tag-modal-close")
        .ghost()
        .compact()
        .w(px(28.))
        .h(px(28.))
        .icon(theme::small_icon(IconName::X))
        .on_click(cx.listener(|this, _, _, cx| {
            this.close_tag_modal();
            cx.notify();
        }))
}

fn field_label(label: &str) -> Div {
    div()
        .text_size(px(11.))
        .text_color(rgb(theme::TEXT_SECONDARY))
        .child(photag_ui_bridge::i18n::tr(label))
}

fn collect_association_rows(
    tags: &[photag_plugin_database::Tag],
    excluded_id: i64,
    rows: &mut Vec<(i64, String)>,
) {
    for tag in tags {
        if tag.id != excluded_id {
            rows.push((tag.id, tag.full_path.replace('>', " > ")));
        }
        collect_association_rows(&tag.children, excluded_id, rows);
    }
}
