use super::*;
use fileatlas_ui_bridge::components::WorkbenchForeground as _;
use fileatlas_ui_bridge::components::{WorkbenchStyle as _, WorkbenchTypography as _};
use gpui_kit::assets::IconName;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::scroll::ScrollableElement;

impl SidebarView {
    pub(super) fn render_tag_modal(&mut self, cx: &mut Context<Self>) -> Div {
        if self.tag_create_open {
            let create = fileatlas_ui_bridge::components::button("navigation-create-tag-submit")
                .primary()
                .label(fileatlas_ui_bridge::i18n::tr("Create"))
                .on_click(cx.listener(|this, _, _, cx| this.create_tag(cx)));
            return modal_shell(
                fileatlas_ui_bridge::i18n::tr("Create tag"),
                div()
                    .v_flex()
                    .gap(px(theme::space_standard()))
                    .child(field_label("Tag name"))
                    .child(fileatlas_ui_bridge::components::text_input(&self.tag_name))
                    .child(field_label("Parent tag"))
                    .child(tag_picker::parent_picker(self, cx))
                    .child(
                        div()
                            .h_flex()
                            .pt(px(fileatlas_ui_bridge::theme::space_standard()))
                            .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Top)
                            .justify_end()
                            .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                            .child(
                                fileatlas_ui_bridge::components::button(
                                    "navigation-create-tag-cancel",
                                )
                                .label(fileatlas_ui_bridge::i18n::tr("Cancel"))
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.tag_create_open = false;
                                        cx.notify();
                                    },
                                )),
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
                    fileatlas_ui_bridge::i18n::tr("Rename tag"),
                    div()
                        .v_flex()
                        .gap(px(theme::space_standard()))
                        .child(field_label("Tag name"))
                        .child(fileatlas_ui_bridge::components::text_input(
                            &self.tag_edit_name,
                        ))
                        .child(
                            div()
                                .h_flex()
                                .pt(px(fileatlas_ui_bridge::theme::space_standard()))
                                .workbench_separator(
                                    fileatlas_ui_bridge::components::DividerEdge::Top,
                                )
                                .justify_end()
                                .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                                .child(
                                    fileatlas_ui_bridge::components::button(
                                        "navigation-rename-tag-cancel",
                                    )
                                    .label(fileatlas_ui_bridge::i18n::tr("Cancel"))
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.associated_edit_tag = None;
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    fileatlas_ui_bridge::components::button(
                                        "navigation-rename-tag-save",
                                    )
                                    .primary()
                                    .label(fileatlas_ui_bridge::i18n::tr("Save"))
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.rename_tag(id, cx);
                                            if this.snapshot.is_ok() {
                                                this.associated_edit_tag = None;
                                            }
                                            cx.notify();
                                        },
                                    )),
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
            let associations =
                association_rows.into_iter().fold(
                    div()
                        .v_flex()
                        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                        .max_h(px(286.))
                        .overflow_y_scrollbar(),
                    |rows, (tag_id, path)| {
                        let checked = self.associated_tag_ids.contains(&tag_id);
                        let depth = path.matches('>').count();
                        rows.child(
                            fileatlas_ui_bridge::components::button(format!(
                                "navigation-associated-tag-{tag_id}"
                            ))
                            .ghost()
                            .pl(px(fileatlas_ui_bridge::theme::space_compact()
                                + depth as f32
                                    * fileatlas_ui_bridge::theme::space_expanded()))
                            .icon(theme::small_icon(if checked {
                                IconName::Check
                            } else {
                                IconName::Square
                            }))
                            .label(fileatlas_ui_bridge::i18n::tr(&path.replace('>', " › ")))
                            .on_click(cx.listener(
                                move |this, _, _, cx| this.toggle_associated_tag(tag_id, cx),
                            )),
                        )
                    },
                );
            let selected_color = self.tag_color.read(cx).value().trim().to_ascii_lowercase();
            let swatches = [
                [
                    theme::accent(),
                    theme::danger(),
                    theme::warning(),
                    theme::success(),
                    theme::info(),
                ],
                [
                    theme::category(),
                    theme::data(),
                    theme::text_primary(),
                    theme::text_muted(),
                    theme::surface_elevated(),
                ],
            ]
            .into_iter()
            .map(|row| {
                div()
                    .h_flex()
                    .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                    .children(row.into_iter().map(|color| {
                        let value = format!("#{color:06x}");
                        let selected = selected_color == value;
                        let tint = color;
                        fileatlas_ui_bridge::components::color_swatch(
                            format!("navigation-tag-color-{color}"),
                            tint,
                            selected,
                        )
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                this.tag_color
                                    .update(cx, |state, cx| state.set_value(&value, window, cx));
                            },
                        ))
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
                    .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                    .children(row.into_iter().map(|icon| {
                        let value = format!("{icon:?}");
                        let selected = selected_icon.as_deref() == Some(value.as_str());
                        fileatlas_ui_bridge::components::icon_choice_button(
                            format!("navigation-tag-icon-{value}"),
                            icon,
                            selected,
                        )
                        .accessibility_label(value.clone())
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                this.tag_icon
                                    .update(cx, |state, cx| state.set_value(&value, window, cx));
                            },
                        ))
                    }))
            })
            .collect::<Vec<_>>();
            return modal_shell(
                fileatlas_ui_bridge::i18n::tr("Edit tag"),
                div()
                    .v_flex()
                    .gap(px(theme::space_standard()))
                    .child(
                        div()
                            .h_flex()
                            .items_start()
                            .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
                            .child(
                                div()
                                    .v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                                    .child(field_label("Tag name"))
                                    .child(fileatlas_ui_bridge::components::text_input(
                                        &self.tag_edit_name,
                                    ))
                                    .child(field_label("Tag color"))
                                    .child(
                                        div()
                                            .v_flex()
                                            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                                            .children(swatches),
                                    )
                                    .child(fileatlas_ui_bridge::components::text_input(
                                        &self.tag_color,
                                    ))
                                    .child(field_label("Tag icon"))
                                    .child(
                                        div()
                                            .v_flex()
                                            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                                            .children(icons),
                                    )
                                    .child(
                                        fileatlas_ui_bridge::components::button(
                                            "navigation-tag-edit-display",
                                        )
                                        .ghost()
                                        .justify_start()
                                        .when(self.tag_edit_display, |button| {
                                            button.icon(theme::small_icon(IconName::Check))
                                        })
                                        .label(fileatlas_ui_bridge::i18n::tr(
                                            "Display on thumbnails",
                                        ))
                                        .on_click(
                                            cx.listener(|this, _, _, cx| {
                                                this.tag_edit_display = !this.tag_edit_display;
                                                cx.notify();
                                            }),
                                        ),
                                    ),
                            )
                            .child(
                                div()
                                    .v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                                    .child(field_label("Associated tags"))
                                    .child(
                                        div()
                                            .v_flex()
                                            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                                            .p(px(fileatlas_ui_bridge::theme::space_standard()))
                                            .workbench_section()
                                            .child(associations),
                                    )
                                    .child(
                                        fileatlas_ui_bridge::components::button(
                                            "navigation-tag-appearance-children",
                                        )
                                        .ghost()
                                        .justify_start()
                                        .when(self.apply_tag_appearance_to_children, |button| {
                                            button.icon(theme::small_icon(IconName::Check))
                                        })
                                        .label(fileatlas_ui_bridge::i18n::tr("Apply to children"))
                                        .on_click(
                                            cx.listener(|this, _, _, cx| {
                                                this.apply_tag_appearance_to_children =
                                                    !this.apply_tag_appearance_to_children;
                                                cx.notify();
                                            }),
                                        ),
                                    )
                                    .when(self.apply_tag_appearance_to_children, |body| {
                                        body.child(
                                            fileatlas_ui_bridge::components::button(
                                                "navigation-tag-appearance-skip-edited",
                                            )
                                            .ghost()
                                            .justify_start()
                                            .when(self.exclude_edited_tag_children, |button| {
                                                button.icon(theme::small_icon(IconName::Check))
                                            })
                                            .label(fileatlas_ui_bridge::i18n::tr(
                                                "Skip edited children",
                                            ))
                                            .on_click(
                                                cx.listener(|this, _, _, cx| {
                                                    this.exclude_edited_tag_children =
                                                        !this.exclude_edited_tag_children;
                                                    cx.notify();
                                                }),
                                            ),
                                        )
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .h_flex()
                            .pt(px(fileatlas_ui_bridge::theme::space_standard()))
                            .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Top)
                            .justify_end()
                            .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                            .child(
                                fileatlas_ui_bridge::components::button(
                                    "navigation-edit-tag-cancel",
                                )
                                .label(fileatlas_ui_bridge::i18n::tr("Cancel"))
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.associated_edit_tag = None;
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                fileatlas_ui_bridge::components::button("navigation-edit-tag-save")
                                    .primary()
                                    .label(fileatlas_ui_bridge::i18n::tr("Save"))
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
                fileatlas_ui_bridge::i18n::tr("Delete tag"),
                div()
                    .v_flex()
                    .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
                            .child(
                                div()
                                    .workbench_foreground(
                                        fileatlas_ui_bridge::components::ForegroundRole::Danger,
                                    )
                                    .child(theme::large_icon(IconName::TriangleAlert)),
                            )
                            .child(
                                div()
                                    .v_flex()
                                    .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                                    .child(div().workbench_body().child(name))
                                    .child(div().workbench_assistant().child(
                                        fileatlas_ui_bridge::i18n::tr_args(
                                            "Used by {count} media items",
                                            &[("count", count.to_string())],
                                        ),
                                    )),
                            ),
                    )
                    .child(field_label("When deleting this tag"))
                    .child(
                        fileatlas_ui_bridge::components::button("navigation-delete-tag-keep")
                            .ghost()
                            .justify_start()
                            .icon(theme::small_icon(if !self.delete_clean_parents {
                                IconName::CircleDot
                            } else {
                                IconName::Circle
                            }))
                            .label(fileatlas_ui_bridge::i18n::tr("Keep parent tags"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.delete_clean_parents = false;
                                cx.notify();
                            })),
                    )
                    .child(
                        fileatlas_ui_bridge::components::button("navigation-delete-tag-clean")
                            .ghost()
                            .justify_start()
                            .icon(theme::small_icon(if self.delete_clean_parents {
                                IconName::CircleDot
                            } else {
                                IconName::Circle
                            }))
                            .label(fileatlas_ui_bridge::i18n::tr("Also remove parent tags"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.delete_clean_parents = true;
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .workbench_assistant()
                            .child(fileatlas_ui_bridge::i18n::tr(
                                "This action removes tag assignments from matching media.",
                            )),
                    )
                    .child(
                        div()
                            .h_flex()
                            .pt(px(fileatlas_ui_bridge::theme::space_standard()))
                            .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Top)
                            .justify_end()
                            .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                            .child(
                                fileatlas_ui_bridge::components::button(
                                    "navigation-delete-tag-cancel",
                                )
                                .label(fileatlas_ui_bridge::i18n::tr("Cancel"))
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.confirm_delete = None;
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                fileatlas_ui_bridge::components::button(
                                    "navigation-delete-tag-confirm",
                                )
                                .danger()
                                .label(fileatlas_ui_bridge::i18n::tr("Delete"))
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        let clean_parents = this.delete_clean_parents;
                                        this.confirm_delete = None;
                                        this.delete_tag(id, clean_parents, cx);
                                    },
                                )),
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
                fileatlas_ui_bridge::i18n::tr("Merge tag"),
                div()
                    .v_flex()
                    .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                            .child(div().workbench_body().child(fileatlas_ui_bridge::i18n::tr("Source tag")))
                            .child(
                                fileatlas_ui_bridge::tag_capsule::TagCapsule::new(source_name)
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
                            .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
                            .child(
                                div()
                                    .v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                                    .child(field_label("Choose a merge target"))
                                    .child(fileatlas_ui_bridge::components::text_input(&self.merge_search))
                                    .child(
                                        fileatlas_ui_bridge::components::card_surface(div()
                                            .v_flex()
                                            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                                            .p(px(fileatlas_ui_bridge::theme::space_standard()))
                                            .max_h(px(310.))
                                            .overflow_y_scrollbar()
                                            .children(targets), false),
                                    ),
                            )
                            .child(
                                div()
                                    .v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                                    .child(field_label("Child tag handling"))
                                    .child(div().workbench_assistant().child(
                                        fileatlas_ui_bridge::i18n::tr_args(
                                            "This tag has {count} direct child tags.",
                                            &[("count", child_count.to_string())],
                                        ),
                                    ))
                                    .child(
                                        fileatlas_ui_bridge::components::button("navigation-merge-children-reparent")
                                            .ghost()

                                            .justify_start()
                                            .icon(theme::small_icon(if self.merge_reparent_children {
                                                IconName::CircleDot
                                            } else {
                                                IconName::Circle
                                            }))
                                            .label(fileatlas_ui_bridge::i18n::tr("Re-parent children under target"))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.merge_reparent_children = true;
                                                cx.notify();
                                            })),
                                    )
                                    .child(div().pl(px(fileatlas_ui_bridge::theme::space_expanded())).workbench_assistant().child(
                                        fileatlas_ui_bridge::i18n::tr("Existing child tags are kept and moved below the target."),
                                    ))
                                    .child(
                                        fileatlas_ui_bridge::components::button("navigation-merge-children-delete")
                                            .ghost()

                                            .justify_start()
                                            .icon(theme::small_icon(if self.merge_reparent_children {
                                                IconName::Circle
                                            } else {
                                                IconName::CircleDot
                                            }))
                                            .label(fileatlas_ui_bridge::i18n::tr("Delete child tags"))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.merge_reparent_children = false;
                                                cx.notify();
                                            })),
                                    )
                                    .child(div().pl(px(fileatlas_ui_bridge::theme::space_expanded())).workbench_assistant().child(
                                        fileatlas_ui_bridge::i18n::tr("Child tag assignments are removed with the source tag."),
                                    )),
                            ),
                    )
                    .child(
                        fileatlas_ui_bridge::components::button("navigation-merge-cancel")
                            .self_end()
                            .label(fileatlas_ui_bridge::i18n::tr("Cancel"))
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
                fileatlas_ui_bridge::i18n::tr("Move tag"),
                div()
                    .v_flex()
                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                    .child(tag_picker::move_target_picker(self, cx))
                    .child(
                        div()
                            .h_flex()
                            .justify_end()
                            .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                            .child(
                                fileatlas_ui_bridge::components::button(
                                    "navigation-move-tag-cancel",
                                )
                                .label(fileatlas_ui_bridge::i18n::tr("Cancel"))
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.move_source_id = None;
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                fileatlas_ui_bridge::components::button(
                                    "navigation-move-tag-submit",
                                )
                                .primary()
                                .label(fileatlas_ui_bridge::i18n::tr("Move"))
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.move_tag_into(source_id, cx);
                                        if this.snapshot.is_ok() {
                                            this.move_source_id = None;
                                        }
                                        cx.notify();
                                    },
                                )),
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
    fileatlas_ui_bridge::components::modal_overlay(
        div()
            .items_start()
            .h_auto()
            .w(px(width))
            .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
            .p(px(fileatlas_ui_bridge::theme::space_expanded()))
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .justify_between()
                    .pb(px(fileatlas_ui_bridge::theme::space_standard()))
                    .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Bottom)
                    .child(div().workbench_title().child(title))
                    .child(close),
            )
            .child(div().max_h(px(640.)).overflow_y_scrollbar().child(body)),
    )
}

fn modal_close_button(cx: &mut Context<SidebarView>) -> Button {
    fileatlas_ui_bridge::components::button("navigation-tag-modal-close")
        .ghost()
        .icon(theme::small_icon(IconName::X))
        .on_click(cx.listener(|this, _, _, cx| {
            this.close_tag_modal();
            cx.notify();
        }))
}

fn field_label(label: &str) -> Div {
    div()
        .workbench_body()
        .child(fileatlas_ui_bridge::i18n::tr(label))
}

fn collect_association_rows(
    tags: &[fileatlas_plugin_database::Tag],
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
