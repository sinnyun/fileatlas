use super::*;
use gpui_kit::assets::IconName;
use gpui_kit::base::Selectable as _;

/// Relative Chinese age of a project; older than a month falls back to the
/// absolute creation date (UTC civil date, matching the draft's "2026-10-19").
fn relative_time(created_at: u64, now: u64) -> String {
    let seconds = now.saturating_sub(created_at);
    match seconds {
        0..=59 => "刚刚".to_owned(),
        60..=3599 => format!("{} 分钟前", seconds / 60),
        3600..=86399 => format!("{} 小时前", seconds / 3600),
        _ => {
            let days = seconds / 86400;
            if days < 30 {
                format!("{days} 天前")
            } else {
                let (y, m, d) = civil_from_days((created_at / 86400) as i64);
                format!("{y:04}-{m:02}-{d:02}")
            }
        }
    }
}

/// Howard Hinnant's civil_from_days: days since 1970-01-01 to (y, m, d).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

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
            .px(px(28.))
            .py(px(16.));
        let mut items = self.items.clone();
        items.sort_by_key(|p| std::cmp::Reverse(p.created_at));
        if self.sort_by_name {
            items.sort_by_key(|p| p.name.to_lowercase());
        }
        if items.is_empty() {
            grid = grid.child(
                div()
                    .w_full()
                    .py(px(48.))
                    .text_color(rgb(theme::text_muted()))
                    .child(if self.query.is_empty() {
                        "还没有项目。点击右上角“新建工程”开始。"
                    } else {
                        "没有符合条件的项目。"
                    }),
            );
        }
        for project in items.into_iter().filter(|p| {
            p.archived == self.show_archived && p.name.to_lowercase().contains(&self.query)
        }) {
            let id = project.id.clone();
            let summary = self.summaries.get(&id).cloned().unwrap_or_default();
            let age = relative_time(project.created_at, now_secs());
            grid = grid.child(self.project_card(project, summary, age, width, cx));
        }
        let header = self.projects_header(cx);
        div()
            .flex_1()
            .min_h_0()
            .v_flex()
            .child(header)
            .child(grid)
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .justify_center()
                    .px(px(28.))
                    .pb(px(20.))
                    .child(self.add_project_tile(cx)),
            )
            .into_any_element()
    }

    /// Eyebrow, hero title and description from the draft, with the search /
    /// sort / archive toggles and the primary 新建工程 on the same top line.
    fn projects_header(&self, cx: &mut ViewContext<Self>) -> gpui::Div {
        div()
            .h_flex()
            .items_start()
            .gap_3()
            .px(px(28.))
            .pt(px(20.))
            .pb(px(4.))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(theme::FONT_SIZE_TINY))
                            .text_color(rgb(theme::text_muted()))
                            .child("TIER 1 · PROJECT REPOSITORY · VECTOR ENGINE READY"),
                    )
                    .child(
                        div()
                            .font_semibold()
                            .text_size(px(theme::SETTINGS_TITLE_SIZE))
                            .child("项目工程大盘"),
                    )
                    .child(
                        div()
                            .max_w(px(560.))
                            .text_size(px(theme::FONT_SIZE_SMALL))
                            .text_color(rgb(theme::text_secondary()))
                            .child(
                                "基于 VectorCraft 矢量画布构建的多画布物料工程。每个项目绑定一份核心矢量文档，通过统一物料模版对各画布进行批量规格定义与出版交付。",
                            ),
                    ),
            )
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_2()
                    .flex_shrink_0()
                    .pt(px(6.))
                    .child(super::tracking::control(
                        "workcrate-search-input",
                        fileatlas_ui_bridge::components::search_input(&self.search_input)
                            .w(px(220.)),
                        &self.surfaces,
                    ))
                    .child(super::tracking::control(
                        "workcrate-sort",
                        fileatlas_ui_bridge::components::icon_button(
                            "workcrate-sort",
                            IconName::ArrowUpDown,
                        )
                        .tooltip(if self.sort_by_name {
                            "当前按名称排序，点击改为最近创建"
                        } else {
                            "当前按最近创建排序，点击改为名称排序"
                        })
                        .selected(self.sort_by_name)
                        .on_click(cx.listener(|view, _, _, cx| {
                            view.sort_by_name = !view.sort_by_name;
                            cx.notify();
                        })),
                        &self.surfaces,
                    ))
                    .child(super::tracking::control(
                        "workcrate-archived",
                        fileatlas_ui_bridge::components::icon_button(
                            "workcrate-archived",
                            IconName::Archive,
                        )
                        .tooltip(if self.show_archived {
                            "当前显示已归档项目，点击返回进行中"
                        } else {
                            "查看已归档项目"
                        })
                        .selected(self.show_archived)
                        .on_click(cx.listener(|view, _, _, cx| {
                            view.show_archived = !view.show_archived;
                            cx.notify();
                        })),
                        &self.surfaces,
                    ))
                    .child(self.screen_action(
                        "workcrate-open-project-folder",
                        None,
                        "打开项目文件夹",
                        false,
                        |v, cx| v.open_project_folder(cx),
                        cx,
                    ))
                    .child(self.screen_action(
                        "workcrate-new-project",
                        Some(IconName::Plus),
                        "新建工程",
                        true,
                        |v, cx| v.create_project(cx),
                        cx,
                    )),
            )
    }

    fn project_card(
        &self,
        project: fileatlas_workcrate_contracts::WorkCrateProject,
        summary: super::ProjectSummary,
        age: String,
        width: f32,
        cx: &mut ViewContext<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let id = project.id.clone();
        let name = project.name.clone();
        let open = {
            let id = id.clone();
            cx.listener(move |view, _, _, cx| {
                view.selected_project = Some(id.clone());
                let _ = view.ui_state.navigate(Some(id.clone()), None);
                view.selected_board = None;
                view.manage_id = None;
                view.reload(cx);
            })
        };
        let mut card = div()
            .id(format!("workcrate-project-{id}"))
            .relative()
            .v_flex()
            .w(px(width))
            .workbench_card(false)
            .p(px(theme::SPACE_3))
            .gap_2();
        card = card.child(
            div()
                .h_flex()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .font_semibold()
                        .text_size(px(theme::FONT_SIZE_TITLE))
                        .child(name),
                )
                .child(self.card_icon(
                    &format!("project-rename-{id}"),
                    IconName::Pencil,
                    "重命名",
                    self.busy,
                    {
                        let id = id.clone();
                        let value = project.name.clone();
                        cx.listener(move |view, _, window, cx| {
                            view.begin_edit(id.clone(), "project", &value, window, cx)
                        })
                    },
                ))
                .child(self.card_icon(
                    &format!("project-duplicate-{id}"),
                    IconName::Copy,
                    "复制项目",
                    self.busy,
                    {
                        let id = id.clone();
                        cx.listener(move |view, _, _, cx| {
                            view.project_action(id.clone(), "duplicate", cx)
                        })
                    },
                ))
                .child(self.card_icon(
                    &format!("project-delete-{id}"),
                    IconName::Trash,
                    "删除项目",
                    self.busy,
                    {
                        let id = id.clone();
                        cx.listener(move |view, _, _, cx| {
                            view.project_action(id.clone(), "delete", cx)
                        })
                    },
                ))
                .child(super::tracking::control(
                    &format!("project-menu-{id}"),
                    fileatlas_ui_bridge::components::icon_button(
                        format!("project-menu-{id}"),
                        IconName::Ellipsis,
                    )
                    .tooltip("更多项目操作")
                    .disabled(self.busy)
                    .on_click({
                        let menu_id = id.clone();
                        cx.listener(move |view, _, _, cx| {
                            view.manage_id = if view.manage_id.as_ref() == Some(&menu_id) {
                                None
                            } else {
                                Some(menu_id.clone())
                            };
                            cx.notify();
                        })
                    }),
                    &self.surfaces,
                )),
        );
        card = card.child(
            div()
                .text_size(px(theme::FONT_SIZE_SMALL))
                .text_color(rgb(theme::text_muted()))
                .child(format!("{} 个画布 · {age}", summary.pages)),
        );
        if let Some(first) = &summary.first {
            let mut band = div()
                .h_flex()
                .items_center()
                .gap_2()
                .w_full()
                .rounded(px(theme::ROUND_MD))
                .bg(rgb(theme::selection_color()))
                .px_2()
                .h(px(26.))
                .text_size(px(theme::FONT_SIZE_TINY))
                .child(
                    div()
                        .font_semibold()
                        .min_w_0()
                        .truncate()
                        .child(format!("01 {}", first.title)),
                )
                .child(div().flex_1());
            let facts = [
                first.material.clone(),
                first.size.clone(),
                first.quantity.clone(),
            ]
            .into_iter()
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>()
            .join("  ");
            if !facts.is_empty() {
                band = band.child(
                    div()
                        .flex_shrink_0()
                        .text_color(rgb(theme::text_secondary()))
                        .child(facts),
                );
            }
            card = card.child(band);
        }
        card = card.child(
            div()
                .id(format!("project-cover-{id}"))
                .w_full()
                .h(px(width * 0.72))
                .rounded(px(theme::ROUND_LG))
                .overflow_hidden()
                .cursor_pointer()
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
                                .object_fit(if project.main_image.is_some() {
                                    ObjectFit::Cover
                                } else {
                                    ObjectFit::Contain
                                })
                                .into_any_element()
                        })
                        .unwrap_or_else(|| {
                            div()
                                .text_size(px(theme::FONT_SIZE_SMALL))
                                .text_color(rgb(theme::text_muted()))
                                .child("选择项目主图")
                                .into_any_element()
                        }),
                )
                .on_click(open),
        );
        card = card.child(
            div()
                .text_size(px(theme::FONT_SIZE_TINY))
                .text_color(rgb(theme::text_muted()))
                .child("项目主图 · 模版画布预览"),
        );
        card = card.child(
            div()
                .h_flex()
                .items_center()
                .gap_2()
                .pt_1()
                .border_t_1()
                .border_color(rgb(theme::separator()))
                .child(theme::small_icon(IconName::FileBox))
                .child(
                    div()
                        .id(format!("project-path-{id}"))
                        .relative()
                        .flex_1()
                        .min_w_0()
                        .child({
                            let path = summary
                                .document
                                .clone()
                                .unwrap_or_else(|| "未关联工程".into());
                            let copy = summary.document.clone();
                            Button::new(format!("project-copy-path-{id}"))
                                .ghost()
                                .compact()
                                .w_full()
                                .min_w_0()
                                .justify_start()
                                .px_0()
                                .tooltip(path.clone())
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .truncate()
                                        .text_size(px(theme::FONT_SIZE_TINY))
                                        .text_color(rgb(theme::text_muted()))
                                        .child(path),
                                )
                                .on_click(move |_, _, cx| {
                                    if let Some(path) = &copy {
                                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                            path.clone(),
                                        ));
                                    }
                                })
                        })
                        .child(super::tracking::measure(
                            &format!("project-path-{id}"),
                            self.surfaces.clone(),
                        )),
                )
                .child(super::tracking::control(
                    &format!("project-open-{id}"),
                    Button::new(format!("project-open-{id}"))
                        .ghost()
                        .compact()
                        .label("进入页面矩阵")
                        .icon(theme::small_icon(IconName::ArrowRight))
                        .disabled(self.busy)
                        .on_click({
                            let id = id.clone();
                            cx.listener(move |view, _, _, cx| {
                                view.selected_project = Some(id.clone());
                                let _ = view.ui_state.navigate(Some(id.clone()), None);
                                view.selected_board = None;
                                view.manage_id = None;
                                view.reload(cx);
                            })
                        }),
                    &self.surfaces,
                )),
        );
        if self.manage_id.as_ref() == Some(&project.id) {
            let target = project.id.clone();
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
                    &format!("project-cover-{target}-menu"),
                    Button::new(format!("project-cover-{target}-menu"))
                        .ghost()
                        .label("更换主图")
                        .disabled(self.busy)
                        .on_click(cx.listener(move |view, _, _, cx| {
                            view.manage_id = None;
                            view.project_action(target.clone(), "cover", cx)
                        })),
                    &self.surfaces,
                ));
            let target = project.id.clone();
            menu = menu.child(super::tracking::control(
                &format!("project-relocate-{target}"),
                Button::new(format!("project-relocate-{target}"))
                    .ghost()
                    .label("迁移项目数据")
                    .tooltip("将项目数据保存到原生工程文件旁")
                    .disabled(self.busy)
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.relocate_project(target.clone(), cx);
                    })),
                &self.surfaces,
            ));
            let target = project.id.clone();
            menu = menu.child(super::tracking::control(
                &format!("project-archive-{target}"),
                Button::new(format!("project-archive-{target}"))
                    .ghost()
                    .label(if project.archived {
                        "恢复项目"
                    } else {
                        "归档项目"
                    })
                    .disabled(self.busy)
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.manage_id = None;
                        view.project_action(target.clone(), "archive", cx)
                    })),
                &self.surfaces,
            ));
            card = card.child(menu);
        }
        if self
            .editing
            .as_ref()
            .is_some_and(|(target, key)| target == &project.id && key == "project")
        {
            card = card.child(self.edit_row(cx));
        }
        card
    }

    pub(super) fn card_icon(
        &self,
        id: &str,
        icon: IconName,
        tooltip: &str,
        disabled: bool,
        on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    ) -> gpui::Div {
        super::tracking::control(
            id,
            fileatlas_ui_bridge::components::icon_button(id, icon)
                .tooltip(tooltip)
                .disabled(disabled)
                .on_click(on_click),
            &self.surfaces,
        )
    }

    fn add_project_tile(&self, cx: &mut ViewContext<Self>) -> gpui::Div {
        super::tracking::control(
            "workcrate-add-project-tile",
            div()
                .id("workcrate-add-project-tile")
                .h_flex()
                .items_center()
                .justify_center()
                .gap_2()
                .w(px(480.))
                .h(px(72.))
                .workbench_card(false)
                .cursor_pointer()
                .on_click(cx.listener(|view, _, _, cx| view.create_project(cx)))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(28.))
                        .rounded_full()
                        .bg(rgb(theme::selection_color()))
                        .text_color(rgb(theme::accent()))
                        .child(theme::icon(IconName::Plus)),
                )
                .child(
                    div()
                        .font_semibold()
                        .text_size(px(theme::FONT_SIZE_BODY))
                        .child("创建新物料工程"),
                )
                .child(
                    div()
                        .text_size(px(theme::FONT_SIZE_TINY))
                        .text_color(rgb(theme::text_muted()))
                        .child("（自动加载 VectorCraft 模版）"),
                ),
            &self.surfaces,
        )
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| v.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{civil_from_days, relative_time};

    #[test]
    fn ages_use_the_chinese_relative_scale() {
        let now = 1_800_000_000;
        assert_eq!(relative_time(now - 30, now), "刚刚");
        assert_eq!(relative_time(now - 90, now), "1 分钟前");
        assert_eq!(relative_time(now - 2 * 3600, now), "2 小时前");
        assert_eq!(relative_time(now - 3 * 86400, now), "3 天前");
    }

    #[test]
    fn a_month_old_project_shows_its_creation_date() {
        let (y, m, d) = civil_from_days(0);
        assert_eq!((y, m, d), (1970, 1, 1));
        // 2026-10-19 is 20745 days after the epoch.
        let (y, m, d) = civil_from_days(20_745);
        assert_eq!((y, m, d), (2026, 10, 19));
        let stamp = 20_745 * 86_400;
        assert_eq!(relative_time(stamp, stamp + 40 * 86_400), "2026-10-19");
    }
}
