use super::*;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::scroll::ScrollableElement;

impl SidebarView {
    pub(super) fn render_tag_modal(&mut self, cx: &mut Context<Self>) -> Div {
        if self.tag_create_open {
            let create = Button::new("navigation-create-tag-submit")
                .primary()
                .label(photag_ui_bridge::i18n::tr("Create"))
                .on_click(cx.listener(|this, _, _, cx| this.create_tag(cx)));
            return modal_shell(
                photag_ui_bridge::i18n::tr("Create tag"),
                div()
                    .v_flex()
                    .gap_2()
                    .child(Input::new(&self.tag_name))
                    .child(tag_picker::parent_picker(self, cx))
                    .child(
                        div()
                            .h_flex()
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
            );
        }

        if let Some(id) = self.associated_edit_tag {
            if self.tag_edit_name_only {
                return modal_shell(
                    photag_ui_bridge::i18n::tr("Rename tag"),
                    div()
                        .v_flex()
                        .gap_2()
                        .child(Input::new(&self.tag_edit_name))
                        .child(
                            div()
                                .h_flex()
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
                    .max_h(px(150.))
                    .overflow_y_scrollbar(),
                |rows, (tag_id, path)| {
                    let checked = self.associated_tag_ids.contains(&tag_id);
                    rows.child(
                        Button::new(format!("navigation-associated-tag-{tag_id}"))
                            .ghost()
                            .w_full()
                            .label(photag_ui_bridge::i18n::tr(&format!(
                                "{} {path}",
                                if checked { "☑" } else { "☐" }
                            )))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.toggle_associated_tag(tag_id, cx)
                            })),
                    )
                },
            );
            let swatches = [
                "#E85D5D", "#F08C46", "#D9B443", "#69A96B", "#4FAF9F", "#4A9CC7", "#5C7FE0",
                "#8A69CE", "#C168A6", "#A77C5A", "#858A92", "#E6E9ED",
            ]
            .into_iter()
            .map(|color| {
                let value = color.to_owned();
                let parsed = crate::parse_hex_color(color).unwrap_or([120, 120, 130]);
                let tint = (u32::from(parsed[0]) << 16)
                    | (u32::from(parsed[1]) << 8)
                    | u32::from(parsed[2]);
                Button::new(format!("navigation-tag-color-{color}"))
                    .label(" ")
                    .w(px(22.))
                    .h(px(22.))
                    .bg(rgb(tint))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.tag_color
                            .update(cx, |state, cx| state.set_value(&value, window, cx));
                    }))
            })
            .collect::<Vec<_>>();
            let icons = ["🏷", "🌿", "🏔", "✈", "⭐", "📷", "❤️", "📁"]
                .into_iter()
                .map(|icon| {
                    let value = icon.to_owned();
                    Button::new(format!("navigation-tag-icon-{icon}"))
                        .ghost()
                        .label(icon)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.tag_icon
                                .update(cx, |state, cx| state.set_value(&value, window, cx));
                        }))
                })
                .collect::<Vec<_>>();
            return modal_shell(
                photag_ui_bridge::i18n::tr("Edit tag"),
                div()
                    .v_flex()
                    .gap_2()
                    .child(Input::new(&self.tag_edit_name))
                    .child(Input::new(&self.tag_color))
                    .child(div().h_flex().gap_1().children(swatches))
                    .child(Input::new(&self.tag_icon))
                    .child(div().h_flex().gap_1().children(icons))
                    .child(
                        Button::new("navigation-tag-edit-display")
                            .label(photag_ui_bridge::i18n::tr(if self.tag_edit_display {
                                "✓ Display on thumbnails"
                            } else {
                                "Display on thumbnails"
                            }))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.tag_edit_display = !this.tag_edit_display;
                                cx.notify();
                            })),
                    )
                    .child(associations)
                    .child(
                        Button::new("navigation-tag-appearance-children")
                            .label(photag_ui_bridge::i18n::tr(
                                if self.apply_tag_appearance_to_children {
                                    "✓ Apply to children"
                                } else {
                                    "Apply to children"
                                },
                            ))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.apply_tag_appearance_to_children =
                                    !this.apply_tag_appearance_to_children;
                                cx.notify();
                            })),
                    )
                    .when(self.apply_tag_appearance_to_children, |body| {
                        body.child(
                            Button::new("navigation-tag-appearance-skip-edited")
                                .label(photag_ui_bridge::i18n::tr(
                                    if self.exclude_edited_tag_children {
                                        "✓ Skip edited children"
                                    } else {
                                        "Skip edited children"
                                    },
                                ))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.exclude_edited_tag_children =
                                        !this.exclude_edited_tag_children;
                                    cx.notify();
                                })),
                        )
                    })
                    .child(
                        div()
                            .h_flex()
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
            );
        }

        if let Some((id, name, count)) = self.confirm_delete.clone() {
            return modal_shell(
                photag_ui_bridge::i18n::tr("Delete tag"),
                div()
                    .v_flex()
                    .gap_2()
                    .child(div().text_size(px(13.)).child(name))
                    .child(
                        div()
                            .h_flex()
                            .gap_1()
                            .child(count.to_string())
                            .child(photag_ui_bridge::i18n::tr("media items")),
                    )
                    .child(div().text_color(rgb(theme::TEXT_SECONDARY)).child(
                        photag_ui_bridge::i18n::tr("Also remove parent tags from those media?"),
                    ))
                    .child(
                        div()
                            .h_flex()
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
                                Button::new("navigation-delete-tag-keep")
                                    .label(photag_ui_bridge::i18n::tr("Keep parent tags"))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.confirm_delete = None;
                                        this.delete_tag(id, false, cx);
                                    })),
                            )
                            .child(
                                Button::new("navigation-delete-tag-clean")
                                    .danger()
                                    .label(photag_ui_bridge::i18n::tr("Remove parent tags too"))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.confirm_delete = None;
                                        this.delete_tag(id, true, cx);
                                    })),
                            ),
                    ),
            );
        }

        if let Some(source_id) = self.merge_source {
            let source_name = self
                .snapshot
                .as_ref()
                .ok()
                .and_then(|s| find_tag(&s.tags, source_id))
                .map(|tag| tag.name.clone())
                .unwrap_or_else(|| "tag".into());
            let targets = super::render::merge_target_buttons(self, source_id, cx);
            return modal_shell(
                photag_ui_bridge::i18n::tr("Merge tag"),
                div()
                    .v_flex()
                    .gap_2()
                    .child(div().child(source_name))
                    .child(
                        div()
                            .text_color(rgb(theme::TEXT_SECONDARY))
                            .child(photag_ui_bridge::i18n::tr("Choose a merge target")),
                    )
                    .child(Input::new(&self.merge_search))
                    .child(
                        Button::new("navigation-merge-children-mode")
                            .label(photag_ui_bridge::i18n::tr(
                                if self.merge_reparent_children {
                                    "Children will be re-parented under target"
                                } else {
                                    "Children will be deleted"
                                },
                            ))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.merge_reparent_children = !this.merge_reparent_children;
                                cx.notify();
                            })),
                    )
                    .children(targets)
                    .child(
                        Button::new("navigation-merge-cancel")
                            .label(photag_ui_bridge::i18n::tr("Cancel"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.merge_source = None;
                                cx.notify();
                            })),
                    ),
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
            );
        }
        div()
    }
}

fn modal_shell(title: String, body: Div) -> Div {
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
                .w(px(440.))
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
                        .text_size(px(15.))
                        .font_family(theme::SEMIBOLD_FONT_FAMILY)
                        .child(title),
                )
                .child(body),
        )
}

fn collect_association_rows(
    tags: &[photag_plugin_database::db::queries::Tag],
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
