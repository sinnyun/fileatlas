use super::*;
impl WorkCratePanel {
    pub(super) fn render_projects(&mut self, cx: &mut ViewContext<Self>) -> gpui::AnyElement {
        let width = ((self.panel_content_size().0 - 96.) / 3.).max(210.) as f32;
        let mut grid = div()
            .id("workcrate-project-grid")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .h_flex()
            .flex_wrap()
            .content_start()
            .items_start()
            .gap(px(24.))
            .p(px(24.));
        let mut items = self.items.clone();
        items.sort_by_key(|p| std::cmp::Reverse(p.created_at));
        if self.sort_by_name {
            items.sort_by_key(|p| p.name.to_lowercase());
        }
        for project in items.into_iter().filter(|p| {
            p.archived == self.show_archived && p.name.to_lowercase().contains(&self.query)
        }) {
            let id = project.id.clone();
            let menu_id = id.clone();
            let name = project.name.clone();
            let mut card = div()
                .id(format!("workcrate-project-{id}"))
                .relative()
                .v_flex()
                .w(px(width))
                .workbench_card(false)
                .p_3()
                .gap_3()
                .child(
                    div()
                        .h_flex()
                        .items_center()
                        .child(
                            Button::new(format!("project-title-{id}"))
                                .ghost()
                                .font_semibold()
                                .text_size(px(20.))
                                .label(name.clone())
                                .on_click(cx.listener(move |view, _, _, cx| {
                                    view.selected_project = Some(id.clone());
                                    let _ = view.ui_state.navigate(Some(id.clone()), None);
                                    view.selected_board = None;
                                    view.manage_id = None;
                                    view.reload(cx);
                                })),
                        )
                        .child(div().flex_1())
                        .child(super::tracking::control(
                            &format!("project-menu-{menu_id}"),
                            Button::new(format!("project-menu-{menu_id}"))
                                .ghost()
                                .icon(theme::small_icon(gpui_kit::assets::IconName::Ellipsis))
                                .tooltip("项目操作")
                                .on_click(cx.listener(move |view, _, _, cx| {
                                    view.manage_id = if view.manage_id.as_ref() == Some(&menu_id) {
                                        None
                                    } else {
                                        Some(menu_id.clone())
                                    };
                                    cx.notify();
                                })),
                            &self.surfaces,
                        )),
                );
            let id = project.id.clone();
            card = card.child(
                div()
                    .id(format!("project-cover-{id}"))
                    .w_full()
                    .h(px((width - 24.) * 0.92))
                    .rounded(px(8.))
                    .overflow_hidden()
                    .bg(rgb(theme::surface_faint()))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        self.project_covers
                            .get(&id)
                            .cloned()
                            .map(|image| {
                                img(image)
                                    .size_full()
                                    .object_fit(ObjectFit::Cover)
                                    .into_any_element()
                            })
                            .unwrap_or_else(|| {
                                div()
                                    .text_color(rgb(theme::text_muted()))
                                    .child("选择项目主图")
                                    .into_any_element()
                            }),
                    )
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.selected_project = Some(id.clone());
                        let _ = view.ui_state.navigate(Some(id.clone()), None);
                        view.selected_board = None;
                        view.reload(cx);
                    })),
            );
            if self.manage_id.as_ref() == Some(&project.id) {
                let target = project.id.clone();
                let value = name.clone();
                let mut menu = div()
                    .absolute()
                    .top(px(38.))
                    .right(px(6.))
                    .w(px(160.))
                    .v_flex()
                    .gap_1()
                    .workbench_section()
                    .p_2()
                    .child(super::tracking::control(
                        &format!("project-rename-{target}"),
                        Button::new(format!("project-rename-{target}"))
                            .ghost()
                            .label("重命名")
                            .on_click(cx.listener(move |view, _, window, cx| {
                                view.begin_edit(target.clone(), "project", &value, window, cx)
                            })),
                        &self.surfaces,
                    ));
                for (action, label) in [
                    ("cover", "更换主图"),
                    ("duplicate", "复制项目"),
                    (
                        "archive",
                        if project.archived {
                            "恢复项目"
                        } else {
                            "归档项目"
                        },
                    ),
                    ("delete", "删除项目"),
                ] {
                    let target = project.id.clone();
                    let action = action.to_owned();
                    let control_id = format!("project-{action}-{target}");
                    menu = menu.child(super::tracking::control(
                        &control_id,
                        Button::new(control_id.clone())
                            .ghost()
                            .label(label)
                            .disabled(self.busy)
                            .on_click(cx.listener(move |view, _, _, cx| {
                                view.project_action(target.clone(), &action, cx)
                            })),
                        &self.surfaces,
                    ));
                }
                card = card.child(menu);
            }
            if self
                .editing
                .as_ref()
                .is_some_and(|(target, key)| target == &project.id && key == "project")
            {
                card = card.child(self.edit_row(cx));
            }
            grid = grid.child(card);
        }
        grid.child(
            Button::new("workcrate-add-project-tile")
                .outline()
                .w(px(width))
                .h(px(180.))
                .label("＋ 新建项目")
                .on_click(cx.listener(|view, _, _, cx| view.create_project(cx))),
        )
        .into_any_element()
    }
}
