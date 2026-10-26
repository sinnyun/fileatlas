use super::*;
use fileatlas_ui_bridge::components::WorkbenchStyle as _;
use gpui_kit::prelude::FluentBuilder as _;

impl LauncherPanel {
    pub(super) fn batch_list(
        &self,
        entries: &[LauncherEntry],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let all_selected =
            !entries.is_empty() && entries.iter().all(|e| self.batch_selected.contains(&e.id));
        let view = cx.entity().downgrade();
        let visible_ids = entries.iter().map(|e| e.id).collect::<Vec<_>>();
        let mut rows = div().v_flex().w_full();
        rows = rows.child(
            div()
                .h_flex()
                .items_center()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                .px(px(fileatlas_ui_bridge::theme::space_standard()))
                .h(px(38.))
                .workbench_assistant()
                .child(
                    fileatlas_ui_bridge::components::button("launcher-batch-select-all")
                        .label(tr(if all_selected {
                            "Clear visible selection"
                        } else {
                            "Select visible"
                        }))
                        .on_click({
                            let view = view.clone();
                            move |_, _, cx| {
                                let _ = view.update(cx, |this, cx| {
                                    if all_selected {
                                        for id in &visible_ids {
                                            this.batch_selected.remove(id);
                                        }
                                    } else {
                                        for id in &visible_ids {
                                            this.batch_selected.insert(*id);
                                        }
                                    }
                                    cx.notify();
                                });
                            }
                        }),
                )
                .child(div().workbench_assistant().child(format!(
                    "{} · {}",
                    tr("Batch selected"),
                    self.batch_selected.len()
                )))
                .child(div().flex_1().child(tr("Program")))
                .child(div().w(px(130.)).child(tr("Current category")))
                .child(div().w(px(90.)).child(tr("Display status")))
                .child(div().w(px(220.)).child(tr("Path"))),
        );
        for entry in entries {
            let id = entry.id;
            let selected = self.batch_selected.contains(&id);
            let category = self
                .snapshot
                .categories
                .iter()
                .find(|c| c.id == entry.category_id)
                .map(|c| cards::category_name(&c.name))
                .unwrap_or_else(|| tr("Other"));
            let path = entry.launch_path.to_string_lossy().to_string();
            rows = rows.child(
                div()
                    .id(format!("launcher-batch-row-{id}"))
                    .h_flex()
                    .items_center()
                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                    .px(px(fileatlas_ui_bridge::theme::space_standard()))
                    .h(px(40.))
                    .overflow_hidden()
                    .workbench_card(selected)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.batch_selected.remove(&id) {
                            this.batch_selected.insert(id);
                        }
                        cx.notify();
                    }))
                    .child(theme::small_icon(if selected {
                        IconName::SquareCheck
                    } else {
                        IconName::Square
                    }))
                    .child(self.icon(entry, 24.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .workbench_body()
                            .child(entry.display_name.clone()),
                    )
                    .child(div().w(px(130.)).workbench_assistant().child(category))
                    .child(
                        div()
                            .w(px(90.))
                            .workbench_assistant()
                            .child(tr(if entry.hidden {
                                "Hidden program status"
                            } else {
                                "Visible program status"
                            })),
                    )
                    .child(
                        div()
                            .w(px(220.))
                            .h(px(24.))
                            .min_w_0()
                            .overflow_hidden()
                            .text_ellipsis()
                            .workbench_assistant()
                            .child(path),
                    )
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, _, _, cx| {
                            this.batch_selected.insert(id);
                            cx.notify();
                        }),
                    ),
            );
        }
        rows.into_any_element()
    }

    pub(super) fn batch_inspector(&self, cx: &mut Context<Self>) -> AnyElement {
        let categories = self.snapshot.categories.clone();
        let weak = cx.entity().downgrade();
        let category_label = self
            .batch_category
            .and_then(|id| categories.iter().find(|c| c.id == id))
            .map(|c| cards::category_name(&c.name))
            .unwrap_or_else(|| tr("Do not change"));
        let mut panel = div()
            .v_flex()
            .w(px(250.))
            .h_full()
            .flex_shrink_0()
            .p(px(fileatlas_ui_bridge::theme::space_expanded()))
            .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
            .workbench_region(true)
            .child(div().workbench_title().child(tr("Batch edit")))
            .child(
                div()
                    .v_flex()
                    .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                    .child(div().workbench_assistant().child(tr("Target category")))
                    .child(
                        fileatlas_ui_bridge::components::button("launcher-batch-category")
                            .label(category_label)
                            .dropdown_caret(true)
                            .dropdown_menu(move |mut menu, _, _| {
                                let view = weak.clone();
                                menu =
                                    menu.item(PopupMenuItem::new(tr("Do not change")).on_click({
                                        let view = view.clone();
                                        move |_, _, cx| {
                                            let _ = view.update(cx, |this, cx| {
                                                this.batch_category = None;
                                                cx.notify();
                                            });
                                        }
                                    }));
                                for cat in &categories {
                                    let id = cat.id;
                                    let label = cards::category_name(&cat.name);
                                    let view = view.clone();
                                    menu = menu.item(PopupMenuItem::new(label).on_click(
                                        move |_, _, cx| {
                                            let _ = view.update(cx, |this, cx| {
                                                this.batch_category = Some(id);
                                                cx.notify();
                                            });
                                        },
                                    ));
                                }
                                menu
                            }),
                    ),
            )
            .child(
                div()
                    .v_flex()
                    .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                    .child(div().workbench_assistant().child(tr("Name prefix")))
                    .child(
                        fileatlas_ui_bridge::components::text_input(&self.batch_prefix).w_full(),
                    ),
            )
            .child(
                div()
                    .v_flex()
                    .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                    .child(div().workbench_assistant().child(tr("Name suffix")))
                    .child(
                        fileatlas_ui_bridge::components::text_input(&self.batch_suffix).w_full(),
                    ),
            )
            .child(div().workbench_assistant().child(tr("Visibility")));
        for (hidden, label) in [
            (None, "Keep unchanged"),
            (Some(false), "Show"),
            (Some(true), "Hide"),
        ] {
            panel = panel.child(
                fileatlas_ui_bridge::components::button(format!(
                    "launcher-batch-visibility-{label}"
                ))
                .label(tr(label))
                .when(self.batch_hidden == hidden, |button| button.primary())
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.batch_hidden = hidden;
                    cx.notify();
                })),
            );
        }
        panel = panel.child(
            fileatlas_ui_bridge::components::button("launcher-batch-apply")
                .primary()
                .label(tr("Apply to selected programs"))
                .disabled(self.batch_selected.is_empty())
                .on_click(cx.listener(|this, _, window, cx| this.apply_batch(window, cx))),
        );
        panel.into_any_element()
    }

    pub(super) fn apply_batch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ids = self.batch_selected.iter().copied().collect::<Vec<_>>();
        let prefix = self.batch_prefix.read(cx).value().to_string();
        let suffix = self.batch_suffix.read(cx).value().to_string();
        let result = self.service.batch_update_entries(
            &ids,
            self.batch_category,
            &prefix,
            &suffix,
            self.batch_hidden,
        );
        if result.is_ok() {
            self.batch_selected.clear();
            self.batch_category = None;
            self.batch_hidden = None;
            self.batch_prefix
                .update(cx, |input, cx| input.set_value("", window, cx));
            self.batch_suffix
                .update(cx, |input, cx| input.set_value("", window, cx));
        }
        self.report(result, cx);
    }
}
