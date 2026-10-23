use super::preview::page_preview;
use super::*;
use fileatlas_ui_bridge::components::WorkbenchForeground as _;
use fileatlas_ui_bridge::components::WorkbenchPaint as _;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
use gpui_kit::assets::IconName;
use gpui_kit::base::Selectable as _;

impl WorkCratePanel {
    pub(super) fn render_pages(&mut self, cx: &mut ViewContext<Self>) -> gpui::AnyElement {
        let width = ((self.panel_content_size().0 - 128.) / 3.).max(210.) as f32;
        let mut grid = div()
            .id("workcrate-page-grid")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .h_flex()
            .flex_wrap()
            .content_start()
            .items_start()
            .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
            .px(px(fileatlas_ui_bridge::theme::space_expanded()))
            .py(px(fileatlas_ui_bridge::theme::space_expanded()));
        if self.boards_view.is_empty() {
            grid = grid.child(
                div()
                    .w_full()
                    .py(px(fileatlas_ui_bridge::theme::space_expanded()))
                    .workbench_foreground(fileatlas_ui_bridge::components::ForegroundRole::Muted)
                    .child(if self.document.is_some() {
                        "本地没有画板目录。点击右上角“同步工程与预览”读取，或新增画布。"
                    } else {
                        "项目尚未关联 VectorCraft 工程。可关联已有工程，或新增画布。"
                    }),
            );
        }
        for (index, card) in self.boards_view.clone().into_iter().enumerate() {
            grid = grid.child(self.page_card(card, index, width, cx));
        }
        let header = self.pages_header(cx);
        div()
            .flex_1()
            .min_h_0()
            .v_flex()
            .child(header)
            .child(grid)
            .child(self.pages_export_bar(cx))
            .into_any_element()
    }

    /// Hero header from the Tier 2 draft: project title, page/selection
    /// statistics and the unified action cluster on the same top line.
    fn pages_header(&self, cx: &mut ViewContext<Self>) -> gpui::Div {
        let project = self
            .selected_project
            .as_ref()
            .and_then(|id| self.items.iter().find(|p| &p.id == id))
            .map(|p| p.name.clone())
            .unwrap_or_default();
        div()
            .h_flex()
            .items_start()
            .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
            .px(px(fileatlas_ui_bridge::theme::space_expanded()))
            .pt(px(fileatlas_ui_bridge::theme::space_expanded()))
            .pb(px(fileatlas_ui_bridge::theme::space_compact()))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .v_flex()
                    .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                    .child(
                        div()
                            .workbench_assistant()
                            .child(format!("TIER 2 · PAGE MATRIX · {project}")),
                    )
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                            .child(div().workbench_title().truncate().child(project.clone()))
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .workbench_corner(
                                        fileatlas_ui_bridge::components::CornerRole::Medium,
                                    )
                                    .workbench_surface(
                                        fileatlas_ui_bridge::components::SurfaceRole::Selection,
                                    )
                                    .workbench_assistant()
                                    .px(px(fileatlas_ui_bridge::theme::space_standard()))
                                    .py(px(fileatlas_ui_bridge::theme::space_compact()))
                                    .child("页面管理 (Tier 2)"),
                            ),
                    )
                    .child(div().workbench_assistant().child(format!(
                        "VectorCraft 内部多画布 · 共 {} 个独立物料 · 已选中 {} 个",
                        self.boards_view.len(),
                        self.selected_pages.len()
                    ))),
            )
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                    .flex_shrink_0()
                    .pt(px(fileatlas_ui_bridge::theme::space_standard()))
                    .child(super::tracking::control(
                        "pages-multi-select",
                        fileatlas_ui_bridge::components::icon_button(
                            "pages-multi-select",
                            IconName::ListChecks,
                        )
                        .tooltip(if self.multi_select {
                            "已开启多选，点击退出多选"
                        } else {
                            "开启多选模式"
                        })
                        .selected(self.multi_select)
                        .on_click(cx.listener(|view, _, _, cx| {
                            view.multi_select = !view.multi_select;
                            if !view.multi_select {
                                view.selected_pages.clear();
                            }
                            cx.notify();
                        })),
                        &self.surfaces,
                    ))
                    .child(super::tracking::control(
                        "pages-select-all",
                        fileatlas_ui_bridge::components::button("pages-select-all")
                            .ghost()
                            .label("全选")
                            .disabled(self.boards_view.is_empty())
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.multi_select = true;
                                view.selected_pages = view
                                    .boards_view
                                    .iter()
                                    .map(|c| c.board.id.clone())
                                    .collect();
                                cx.notify();
                            })),
                        &self.surfaces,
                    ))
                    .child(super::tracking::control(
                        "pages-clear-selection",
                        fileatlas_ui_bridge::components::button("pages-clear-selection")
                            .ghost()
                            .label("取消全选")
                            .disabled(self.selected_pages.is_empty())
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.selected_pages.clear();
                                cx.notify();
                            })),
                        &self.surfaces,
                    ))
                    .child(self.screen_action(
                        "attach-document",
                        Some(IconName::Link2),
                        "关联工程",
                        false,
                        |v, cx| v.pick_document(cx),
                        cx,
                    ))
                    .child(self.screen_action(
                        "refresh-native",
                        Some(IconName::RefreshCw),
                        "同步工程与预览",
                        false,
                        |v, cx| v.sync_project(cx),
                        cx,
                    ))
                    .child(self.screen_action(
                        "workcrate-new-page",
                        Some(IconName::Plus),
                        "新增画布",
                        true,
                        |v, cx| v.create_board(cx),
                        cx,
                    )),
            )
    }

    /// One page card: checkbox + title + spec line, optional material band,
    /// preview and a footer with the page number and the edit action.
    fn page_card(
        &self,
        card: super::BoardCard,
        index: usize,
        width: f32,
        cx: &mut ViewContext<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let id = card.board.id.clone();
        let selected = self.selected_pages.contains(&id);
        let select_id = id.clone();
        let facts = [
            card.board.info.material.clone(),
            card.board.info.size.clone(),
            card.board.info.quantity.clone(),
        ]
        .into_iter()
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
        .join("  ");
        div()
            .id(format!("page-card-{id}"))
            .drag_over::<super::page_order::PageDrag>(|style, _, _, _| {
                fileatlas_ui_bridge::components::drop_target_style(style)
            })
            .on_drop::<super::page_order::PageDrag>(cx.listener({
                let id = id.clone();
                move |view, payload, _, cx| view.drop_page(payload, &id, cx)
            }))
            .v_flex()
            .w(px(width))
            .workbench_card(selected)
            .p(px(theme::space_expanded()))
            .gap(px(fileatlas_ui_bridge::theme::space_standard()))
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                    .child(self.card_icon(
                        &format!("select-page-{id}"),
                        if selected {
                            IconName::SquareCheck
                        } else {
                            IconName::Square
                        },
                        if selected {
                            "取消选择本页"
                        } else {
                            "选择本页"
                        },
                        false,
                        cx.listener(move |view, _, _, cx| {
                            if !view.selected_pages.remove(&select_id) {
                                view.selected_pages.insert(select_id.clone());
                                view.multi_select = true;
                            }
                            cx.notify();
                        }),
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .workbench_title()
                            .child(card.board.title.clone()),
                    )
                    .child(self.page_order_controls(&id, index, cx)),
            )
            .child({
                let mut band = div()
                    .h_flex()
                    .items_center()
                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                    .w_full()
                    .workbench_corner(fileatlas_ui_bridge::components::CornerRole::Medium)
                    .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Selection)
                    .px(px(fileatlas_ui_bridge::theme::space_standard()))
                    .h(px(theme::ui_height(theme::CONTROL_HEIGHT_COMPACT)))
                    .workbench_assistant()
                    .child(
                        div()
                            .workbench_body()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .child(format!("{:02} {}", index + 1, card.board.title)),
                    );
                if !facts.is_empty() {
                    band = band.child(div().flex_shrink_0().workbench_assistant().child(facts));
                }
                band
            })
            .child(
                fileatlas_ui_bridge::components::card_surface(
                    div()
                        .id(format!("page-open-preview-{}", card.board.id))
                        .w_full()
                        .overflow_hidden()
                        .cursor_pointer(),
                    selected,
                )
                .child(super::tracking::control(
                    &format!("page-preview-{}", card.board.id),
                    page_preview(&card, width - 24.),
                    &self.surfaces,
                ))
                .on_click(cx.listener({
                    let id = card.board.id.clone();
                    move |view, _, _, cx| {
                        if view.multi_select {
                            if !view.selected_pages.remove(&id) {
                                view.selected_pages.insert(id.clone());
                            }
                            cx.notify();
                        } else {
                            view.open_page(&id, cx);
                        }
                    }
                })),
            )
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                    .pt(px(fileatlas_ui_bridge::theme::space_compact()))
                    .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Top)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .workbench_assistant()
                            .child(format!("第 {} 页", index + 1)),
                    )
                    .child(super::tracking::control(
                        &format!("open-page-{id}"),
                        fileatlas_ui_bridge::components::button(format!("open-page-{id}"))
                            .ghost()
                            .label("编辑")
                            .icon(theme::small_icon(IconName::Pencil))
                            .on_click(cx.listener(move |view, _, _, cx| view.open_page(&id, cx))),
                        &self.surfaces,
                    )),
            )
    }

    /// Export bar from the draft: hint and selection count left, the format
    /// toggle and the two export actions right.
    fn pages_export_bar(&self, cx: &mut ViewContext<Self>) -> gpui::Div {
        div()
            .h_flex()
            .items_center()
            .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
            .p(px(fileatlas_ui_bridge::theme::space_expanded()))
            .px(px(fileatlas_ui_bridge::theme::space_expanded()))
            .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Top)
            .flex_wrap()
            .child(
                div()
                    .workbench_assistant()
                    .child("本地预览 · 外部编辑保存后请同步"),
            )
            .child(
                div()
                    .workbench_assistant()
                    .child(format!("已选 {} 页", self.selected_pages.len())),
            )
            .child(div().flex_1())
            .child(super::tracking::control(
                "pages-exit-multi-select",
                fileatlas_ui_bridge::components::button("pages-exit-multi-select")
                    .ghost()
                    .label("取消多选")
                    .disabled(!self.multi_select)
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.multi_select = false;
                        view.selected_pages.clear();
                        cx.notify();
                    })),
                &self.surfaces,
            ))
            .child(super::tracking::control(
                "export-image-format",
                fileatlas_ui_bridge::components::button("export-image-format")
                    .ghost()
                    .label(if self.image_export_jpeg {
                        "JPEG"
                    } else {
                        "PNG"
                    })
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.image_export_jpeg = !view.image_export_jpeg;
                        cx.notify();
                    })),
                &self.surfaces,
            ))
            .child(super::tracking::control(
                "export-selected-images",
                fileatlas_ui_bridge::components::button("export-selected-images")
                    .outline()
                    .workbench_corner(fileatlas_ui_bridge::components::CornerRole::Medium)
                    .label(format!("导出选中页为图片 ({})", self.selected_pages.len()))
                    .icon(theme::icon(IconName::FileImage))
                    .disabled(self.busy || self.selected_pages.is_empty())
                    .on_click(cx.listener(|view, _, _, cx| view.export_catalog(true, cx))),
                &self.surfaces,
            ))
            .child(super::tracking::control(
                "export-all-pages",
                fileatlas_ui_bridge::components::button("export-all-pages")
                    .primary()
                    .workbench_corner(fileatlas_ui_bridge::components::CornerRole::Medium)
                    .label(format!("导出全部为 PDF ({} 页)", self.boards_view.len()))
                    .disabled(self.busy || self.boards_view.is_empty())
                    .on_click(cx.listener(|view, _, _, cx| view.export_catalog(false, cx))),
                &self.surfaces,
            ))
    }
}
